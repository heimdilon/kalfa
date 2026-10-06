// Kabuk (aşama 2 tasarımı §12.5): onay sayfası → onay → canlı çıktı → sonuç;
// süren komutun iptali süreç ağacını durdurur; çıktı seli arayüzü kilitlemez ve
// turu iptal ettirmez. Kurulumlu Windows'ta komutlar AppContainer'da,
// Linux'ta bwrap ile çalışır. Kurulum yoksa yalıtımsız anahtar açıktır;
// yerel pencereleri sahte iletişim kutusu `onay: true` ile onaylar.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import {
  GECERLI_ANAHTAR,
  Kalfa,
  kalfaProfilSayisi,
  sayfaMetni,
  sureyleBekle,
  terminalMetni,
} from '../yardimci/kalfa.mjs';

/** İptal testindeki süreçleri bulmak için komut satırına eklenen işaret. */
const ISARET = `kalfa-e2e-kabuk-${process.pid}`;

const node = (betik, ...ek) => ({ program: 'node', argumanlar: ['-e', betik, ...ek] });

const SENARYOLAR = {
  kabuk: {
    yanitlar: [
      [
        { tur: 'metin', metin: 'Komutu çalıştırıyorum.' },
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node("console.log('kabuktan-merhaba'); console.error('uyari-satiri')"),
        },
      ],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  'kabuk-iptal': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "require('child_process').spawn(process.execPath, " +
              "['-e', 'setInterval(() => {}, 1000)', process.argv[1] + '-torun'], { stdio: 'ignore' }); " +
              "setInterval(() => console.log('tik'), 200)",
            ISARET,
          ),
        },
      ],
      [{ tur: 'metin', metin: 'olmamalı' }],
    ],
  },
  'kabuk-gizli': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "const fs = require('fs'); " +
              "try { console.log('env=' + fs.readFileSync('.env', 'utf8')); } " +
              "catch (e) { console.log('env-okunamadi'); } " +
              "require('net').connect(80, '1.1.1.1')" +
              ".on('error', () => console.log('ag-kapali'))" +
              ".on('connect', () => { console.log('ag-acik'); process.exit(0); });",
          ),
        },
      ],
      [{ tur: 'metin', metin: 'Yalıtım denendi.' }],
    ],
  },
  'kabuk-sel': {
    yanitlar: [
      [
        {
          tur: 'arac',
          ad: 'kabuk',
          argumanlar: node(
            "const s = 'x'.repeat(65536) + '\\n'; const son = Date.now() + 3000; " +
              "function yaz() { while (Date.now() < son) { if (!process.stdout.write(s)) " +
              "return process.stdout.once('drain', yaz); } } yaz();",
          ),
        },
      ],
      [{ tur: 'metin', metin: 'Sel bitti.' }],
    ],
  },
};

/** Komut satırında işareti taşıyan süreçler (torun dahil). */
function isaretliSurecler() {
  if (process.platform === 'win32') {
    const cikti = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*${ISARET}*' -and $_.Name -eq 'node.exe' } | ForEach-Object { $_.ProcessId }; exit 0`,
      ],
      { encoding: 'utf8' },
    );
    return cikti.split(/\r?\n/).filter(Boolean);
  }
  return readdirSync('/proc').filter((ad) => {
    if (!/^\d+$/.test(ad)) return false;
    try {
      return readFileSync(`/proc/${ad}/cmdline`, 'utf8').includes(ISARET);
    } catch {
      return false;
    }
  });
}

async function onayla(k) {
  await k.onaySayfasiniAc();
  await k.dugme('Tümünü onayla');
}

describe('kabuk', () => {
  let k;
  let ilkProfilSayisi;
  before(async () => {
    ilkProfilSayisi = kalfaProfilSayisi();
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

  it('onaylanan komutun çıktısı canlı akar, sonucu modele gider', async () => {
    await k.gonder('kabuk');
    await k.metniBekle('Komutu çalıştırıyorum.');
    await onayla(k);
    assert.equal(await k.turSonuBekle(1), 'Normal');
    const terminal = await terminalMetni(k);
    assert.match(terminal, /kabuktan-merhaba/);
    assert.match(terminal, /uyari-satiri/);
    await k.metniBekle('Çıkış kodu: 0');
    // Sahte model araç sonucunu yanıtına yansıtır: sonuç sarmalıyla modele gitti
    // (güvenlik analizi Bulgu 5).
    await k.metniBekle('<arac_sonucu kaynak="kabuk:node" sinir="');
    if (
      process.platform === 'win32' &&
      ilkProfilSayisi !== null &&
      (await k.invoke('yalitimi_algila')).tip === 'Hazir'
    ) {
      assert.ok(kalfaProfilSayisi() > ilkProfilSayisi, 'yalıtımlı komut profil oluşturmalı');
    }
  });

  it('süren komut iptal edilir; torunuyla birlikte durur', async () => {
    await k.gonder('kabuk-iptal');
    await onayla(k);
    await sureyleBekle(
      async () => (await terminalMetni(k)).includes('tik'),
      20000,
      'komut çıktı üretmedi',
    );
    assert.ok(isaretliSurecler().length >= 2, 'komut ve torunu çalışıyor olmalı');
    await k.ikonDugmesi('Turu durdur');
    assert.equal(await k.turSonuBekle(2), 'Iptal');
    await sureyleBekle(
      () => isaretliSurecler().length === 0,
      10000,
      `süreç ağacı durmadı: ${isaretliSurecler().join(', ')}`,
    );
  });

  it('çıktı seli arayüzü kilitlemez ve turu iptal ettirmez', async () => {
    await k.gonder('kabuk-sel');
    await onayla(k);
    assert.equal(await k.turSonuBekle(3, 60000), 'Normal');
    await k.metniBekle('Sel bitti.');
    const durdu = await k.b.execute(() => document.body.textContent.includes('Aktarım durdu'));
    assert.equal(durdu, false);
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(4), 'Normal');
  });
});

describe('kabuk yalıtımı', { skip: !['linux', 'win32'].includes(process.platform) }, () => {
  let k;
  let ilkProfilSayisi;
  before(async () => {
    ilkProfilSayisi = kalfaProfilSayisi();
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    writeFileSync(join(k.alan, '.env'), 'GIZLI_DEGER=e2e-sir\n');
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
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

  it('komut yalıtımda çalışır: gizli dosya okunamaz, ağ kapalıdır', async (t) => {
    const durum = await k.invoke('yalitimi_algila');
    if (durum.tip !== 'Hazir') {
      t.skip('bu makinede yalıtım kurulu değil');
      return;
    }
    await sureyleBekle(
      async () => (await sayfaMetni(k, '.durum-cubugu')).includes('Yalıtımlı'),
      10000,
      'durum çubuğu yalıtımı göstermedi',
    );
    await k.gonder('kabuk-gizli');
    await k.onaySayfasiniAc();
    assert.match(await sayfaMetni(k, '[role="dialog"]'), new RegExp(`Yalıtımlı \\(${durum.arka_uc}\\)`));
    await k.dugme('Tümünü onayla');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('Yalıtım denendi.');
    const terminal = await terminalMetni(k);
    assert.match(terminal, /env-okunamadi/);
    assert.match(terminal, /ag-kapali/);
    assert.doesNotMatch(terminal, /e2e-sir/);
    if (process.platform === 'win32' && ilkProfilSayisi !== null) {
      assert.ok(kalfaProfilSayisi() > ilkProfilSayisi, 'yalıtımlı komut profil oluşturmalı');
    }
  });
});
