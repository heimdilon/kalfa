// Kabuk (aşama 2 tasarımı §12.5): onay sayfası → yerel onay → canlı çıktı →
// sonuç; süren komutun iptali süreç ağacını durdurur; çıktı seli arayüzü
// kilitlemez ve turu iptal ettirmez. 2a'da yalıtım yoktur: her komut yerel
// pencerede onaylanır (sahte iletişim kutusu `onay: true` ile onaylar).
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

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

/** Konuşmadaki son kabuk çağrısının terminali. */
async function terminalMetni(k) {
  return k.b.execute(
    () => [...document.querySelectorAll('[aria-label="Komut çıktısı"]')].at(-1)?.textContent ?? '',
  );
}

async function onayla(k) {
  await k.dugme('İncele');
  await k.dugme('Tümünü onayla');
}

describe('kabuk', () => {
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

  it('onaylanan komutun çıktısı canlı akar, sonucu modele gider', async () => {
    await k.gonder('kabuk');
    await k.metniBekle('Komutu çalıştırıyorum.');
    await onayla(k);
    assert.equal(await k.turSonuBekle(1), 'Normal');
    const terminal = await terminalMetni(k);
    assert.match(terminal, /kabuktan-merhaba/);
    assert.match(terminal, /uyari-satiri/);
    await k.metniBekle('Çıkış kodu: 0');
    await k.metniBekle('[kaynak: kabuk:node]');
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
