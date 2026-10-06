// Ağ çıkışı (S13, aşama 3 tasarımı §7.1, §8; 3d planı Görev 7, Windows alt planı W4):
// ağlı komut izinsiz bir hosta bağlanmaya çalışınca bağlantı reddedilir,
// sonuçta "[ağ] … engellendi (izin listesinde yok)" yazar.
// İzinli host yerel sahte sunucuya çözülür; komut yanıtı alır ve tur
// özetinde ağ özeti görünür.
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { after, before, describe, it } from 'node:test';
import {
  GECERLI_ANAHTAR,
  Kalfa,
  kalfaProfilSayisi,
  sayfaMetni,
  sureyleBekle,
  terminalMetni,
} from '../yardimci/kalfa.mjs';

/**
 * Ağlı komut onay sayfasında risklidir (güvenlik analizi, #16): "Tümünü
 * onayla" kapalıdır ve öğe seçili gelmez. Kullanıcı öğeyi işaretleyip
 * "Seçilenleri onayla"ya basar.
 */
async function agliKomutuOnayla(k) {
  assert.equal(await (await k.b.$('button=Tümünü onayla')).isEnabled(), false);
  await (await k.b.$('[role="dialog"] .dosya label.onay-kutusu')).click();
  await k.dugme('Seçilenleri onayla');
}

describe('ağ çıkışı (S13)', { skip: !['linux', 'win32'].includes(process.platform) }, () => {
  let k;
  let sahteSunucu;
  let sahtePort;
  let ilkProfilSayisi;

  before(async () => {
    sahteSunucu = createServer((req, res) => {
      res.writeHead(200, { 'Content-Type': 'text/plain' });
      res.end('merhaba-e2e-ag');
    });
    await new Promise((resolve) => sahteSunucu.listen(0, '127.0.0.1', resolve));
    sahtePort = sahteSunucu.address().port;

    const senaryolar = {
      'izinsiz-ag': {
        yanitlar: [
          [
            {
              tur: 'arac',
              ad: 'kabuk',
              argumanlar: {
                program: 'node',
                argumanlar: [
                  '-e',
                  "fetch('https://evil.com/').then(() => console.log('ulasildi')).catch((e) => console.log('hata=' + e.message))",
                ],
                ag: true,
              },
            },
          ],
          [{ tur: 'metin', metin: 'İzinsiz ağ denendi.' }],
        ],
      },
      'izinli-ag': {
        yanitlar: [
          [
            {
              tur: 'arac',
              ad: 'kabuk',
              argumanlar: {
                program: 'node',
                argumanlar: [
                  '-e',
                  `fetch('http://fake.crates.io:${sahtePort}/').then((r) => r.text()).then((t) => console.log('cevap=' + t)).catch((e) => console.log('hata=' + e.message))`,
                ],
                ag: true,
                ag_hostlari: [`fake.crates.io:${sahtePort}`],
              },
            },
          ],
          [{ tur: 'metin', metin: 'İzinli ağ denendi.' }],
        ],
      },
    };

    ilkProfilSayisi = kalfaProfilSayisi();
    process.env.KALFA_E2E_SAHTE_DNS = `fake.crates.io:${sahtePort}=127.0.0.1:${sahtePort}`;

    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar,
    });
    await k.pencere('ana');
    await k.yeniOturum();
  });

  after(async () => {
    delete process.env.KALFA_E2E_SAHTE_DNS;
    if (sahteSunucu) {
      await new Promise((resolve) => sahteSunucu.close(resolve));
    }
    let temiz = false;
    try {
      await k?.hazirlananAlanlariKaldir();
      if (ilkProfilSayisi !== null) {
        assert.equal(kalfaProfilSayisi(), ilkProfilSayisi, 'Kalfa AppContainer profili kalmamalı');
      }
      temiz = true;
    } finally {
      await k?.kapat({ veriyiKoru: !temiz });
    }
  });

  it('S13: izinsiz host engellenir, izinli host vekilden çıkar ve tur özetinde görünür', async (t) => {
    const durum = await k.invoke('yalitimi_algila');
    if (durum.tip !== 'Hazir') {
      t.skip('bu makinede yalıtım kurulu değil');
      return;
    }

    // 1. İzinsiz host denemesi
    await k.gonder('izinsiz-ag');
    await k.onaySayfasiniAc();
    assert.match(
      await sayfaMetni(k, '[role="dialog"]'),
      new RegExp(`Yalıtımlı \\(${durum.arka_uc}\\)`),
    );
    await agliKomutuOnayla(k);
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('İzinsiz ağ denendi.');

    // `[ağ]` satırı modele giden sonuçtadır; kayıtta görünür.
    const kayit1 = JSON.stringify(k.kayit(await k.seciliOturum()));
    assert.match(kayit1, /\[ağ\] `evil\.com:443` engellendi/);

    // Tur özetinde engelleme görünür
    const ozetMetni1 = await sayfaMetni(k, '.tur-ozeti-blogu');
    assert.match(ozetMetni1, /evil\.com/);

    // 2. İzinli host denemesi
    await k.gonder('izinli-ag');
    await k.onaySayfasiniAc();
    const onayMetni = await sayfaMetni(k, '[role="dialog"]');
    assert.match(onayMetni, /fake\.crates\.io/);
    await agliKomutuOnayla(k);
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('İzinli ağ denendi.');

    const terminal2 = await terminalMetni(k);
    assert.match(terminal2, /cevap=merhaba-e2e-ag/);

    // Tur özetinde izinli host görünür
    const ozetMetni2 = await sayfaMetni(k, '.tur-ozeti-blogu');
    assert.match(ozetMetni2, /fake\.crates\.io/);
  });
});
