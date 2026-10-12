// Oturum bütçesi (5b tasarımı §7, §8, §9; plan Görev 7 e2e): istek sınırı soru
// ile yükseltilince aynı iş sürer; kullanımı bilinmeyen yanıt kart çıkarır ve
// turu durdurmak turu iptal eder; 5b öncesi kayıt açılınca ilk istekten önce
// soru çıkar. Canlı EVREN gerekmez: sahte sağlayıcı senaryosu kullanılır.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  'liste oku': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_listele', argumanlar: { yol: '.' } }],
      [{ tur: 'metin', metin: 'Liste okundu.' }],
    ],
  },
  'kullanımsız': {
    yanitlar: [[{ tur: 'metin', metin: 'Kullanımsız yanıt.' }, { tur: 'kullanim_yok' }]],
  },
  selam: { yanitlar: [[{ tur: 'metin', metin: 'Merhaba.' }]] },
};

/** Bütçe kararı kartındaki düğme; yalnızca o kartın içinde aranır. */
async function butceDugmesi(k, metin) {
  const dugme = await k.b.$(
    `//section[contains(., "Bütçe kararı")]//button[normalize-space()="${metin}"]`,
  );
  await dugme.waitForClickable({ timeout: 15000 });
  await dugme.click();
}

describe('oturum bütçesi: istek sınırı soru ile yükselir', () => {
  let k;
  let oturum;

  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    await k.pencere('ana');
    // Yeni oturumlar bir istekle sınırlanır (5b tasarımı §9).
    await k.invoke('oturum_butcesini_ayarla', { istek: 1, token: 5000000 });
    await k.yeniOturum();
    oturum = await k.seciliOturum();
  });

  after(async () => {
    await k?.kapat();
  });

  it('istek sınırı dolunca soru çıkar; sınır yükseltilince aynı iş sürer', async () => {
    await k.gonder('liste oku');
    await k.metniBekle('İstek sınırına ulaşıldı.');
    await butceDugmesi(k, 'Bütçeyle devam et');
    await k.metniBekle('Liste okundu.');
    assert.equal(await k.turSonuBekle(1), 'Normal');

    const gunluk = await k.invoke('gonderim_gunlugu', { oturum, once_sira: null });
    assert.equal(gunluk.satirlar.length, 2, 'iki model isteği günlükte');
    assert.ok(
      gunluk.satirlar.every((s) => s.token !== null),
      'her yeni isteğin tokenı kendi bitişinden gelir',
    );
  });
});

describe('oturum bütçesi: kullanımı bilinmeyen yanıt', () => {
  let k;

  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    await k.pencere('ana');
    await k.yeniOturum();
  });

  after(async () => {
    await k?.kapat();
  });

  it('kullanımsız yanıttan sonra yeni istek kart ister; durdurulunca tur iptal olur', async () => {
    await k.gonder('kullanımsız');
    assert.equal(await k.turSonuBekle(1), 'Normal');

    await k.gonder('selam');
    await k.metniBekle('Kullanımı bilinmeyen 1 istek var.');
    await butceDugmesi(k, 'Turu durdur');
    assert.equal(await k.turSonuBekle(2), 'Iptal');
  });
});

describe('oturum bütçesi: 5b öncesi kayıt', () => {
  let k;
  const oturumKimligi = '6f1d0a4e-2b8c-4c1e-9a55-0d6b1f3e2a10';

  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, onay: true });
    // Bütçe kaydı olmayan (5b öncesi) iki istekli kayıt; liste yenilenince görünür.
    const dizin = join(k.veri, 'oturumlar');
    mkdirSync(dizin, { recursive: true });
    const satir = (sira, yuk) =>
      JSON.stringify({ sema: 1, oturum: oturumKimligi, tur: null, sira, yuk });
    const istek = {
      tip: 'Istek',
      amac: 'ana',
      istek_id: null,
      kredi: null,
      onceki: 0,
      sonuc: { tip: 'Basarili' },
    };
    writeFileSync(
      join(dizin, `${oturumKimligi}.jsonl`),
      [
        satir(1, {
          tip: 'OturumAcildi',
          model: 'glm-5.3',
          kip: 'Siki',
          calisma_alani: k.alan,
        }),
        satir(2, istek),
        satir(3, istek),
        satir(4, { tip: 'TurBitti', neden: { tip: 'Normal' } }),
      ].join('\n') + '\n',
    );
    await k.pencere('ana');
    await k.b.refresh();
    await k.pencere('ana');
  });

  after(async () => {
    await k?.kapat();
  });

  it('5b öncesi kayıt açılınca ilk istekten önce soru çıkar', async () => {
    const satir = await k.b.$('button.oturum-satiri');
    await satir.waitForClickable({ timeout: 15000 });
    await satir.click();
    await k.metniBekle('Kullanımı bilinmeyen 1 istek var.');
    await butceDugmesi(k, 'Bütçeyle devam et');
    await k.metniBekle('Yanıtlandı');
  });
});
