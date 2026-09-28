// Kurucunun araç zinciri hazırlığının ağsız testleri: `node --test "crates/kalfa/kurucu/*.test.mjs"`.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, test } from 'node:test';

import { hazirla, ozetDenetle, tarCalistir } from './araclar.mjs';

const kok = mkdtempSync(join(tmpdir(), 'kalfa-araclar-'));
after(() => rmSync(kok, { recursive: true, force: true }));

let sayac = 0;
function yeniDizin() {
  const dizin = join(kok, String(sayac++));
  mkdirSync(dizin, { recursive: true });
  return dizin;
}

function ozet(dosya) {
  return createHash('sha256').update(readFileSync(dosya)).digest('hex');
}

// Windows'un tar'ı (bsdtar) zip üretir ve açar; kurucu yalnızca Windows'ta derlenir.
const zipYok = process.platform !== 'win32' && 'zip arşivi Windows tar.exe ister';

// `<dizin>/paket.zip` içinde `ic/arac.txt` ve `ic/LISANS` olan bir arşiv üretir.
function ornekArsiv(dizin) {
  const kaynak = join(dizin, 'kaynak');
  mkdirSync(join(kaynak, 'ic'), { recursive: true });
  writeFileSync(join(kaynak, 'ic', 'arac.txt'), 'arac');
  writeFileSync(join(kaynak, 'ic', 'LISANS'), 'lisans');
  const arsiv = join(dizin, 'paket.zip');
  tarCalistir(['-a', '-cf', arsiv, '-C', kaynak, 'ic']);
  return arsiv;
}

test('ozet_eslesmezse_dosya_silinir_ve_hata_atilir', () => {
  const dosya = join(yeniDizin(), 'paket.zip');
  writeFileSync(dosya, 'bozuk');
  assert.throws(() => ozetDenetle(dosya, '0'.repeat(64)), /SHA-256/);
  assert.equal(existsSync(dosya), false);
});

test('ozet_eslesirse_dosya_kalir', () => {
  const dosya = join(yeniDizin(), 'paket.zip');
  writeFileSync(dosya, 'saglam');
  ozetDenetle(dosya, ozet(dosya).toUpperCase());
  assert.equal(existsSync(dosya), true);
});

test('lisans_yoksa_hazirla_basarisiz', { skip: zipYok }, async () => {
  const dizin = yeniDizin();
  const onbellek = join(dizin, 'onbellek');
  mkdirSync(onbellek);
  const arsiv = ornekArsiv(dizin);
  const kopya = join(onbellek, 'paket.zip');
  writeFileSync(kopya, readFileSync(arsiv));
  const manifest = {
    arac: {
      adres: 'https://gecersiz.invalid/paket.zip',
      sha256: ozet(kopya),
      ic_dizin: 'ic',
      lisans: 'LICENSE.txt',
    },
  };
  await assert.rejects(hazirla(manifest, onbellek, join(dizin, 'hedef')), /LICENSE\.txt/);
});

test('onbellekteki_dosya_yeniden_indirilmez', { skip: zipYok }, async () => {
  const dizin = yeniDizin();
  const onbellek = join(dizin, 'onbellek');
  mkdirSync(onbellek);
  const arsiv = ornekArsiv(dizin);
  const kopya = join(onbellek, 'paket.zip');
  writeFileSync(kopya, readFileSync(arsiv));
  const hedef = join(dizin, 'hedef');
  // Önceki bir derlemenin kalıntısı silinir.
  mkdirSync(join(hedef, 'arac'), { recursive: true });
  writeFileSync(join(hedef, 'arac', 'eski.txt'), 'eski');
  const manifest = {
    arac: {
      adres: 'https://gecersiz.invalid/paket.zip',
      sha256: ozet(kopya),
      ic_dizin: 'ic',
      lisans: 'LISANS',
    },
  };
  await hazirla(manifest, onbellek, hedef);
  assert.equal(readFileSync(join(hedef, 'arac', 'arac.txt'), 'utf8'), 'arac');
  assert.equal(existsSync(join(hedef, 'arac', 'LISANS')), true);
  assert.equal(existsSync(join(hedef, 'arac', 'eski.txt')), false);
});

test('gecersiz_ad_ve_ic_dizin_reddedilir', async () => {
  const dizin = yeniDizin();
  const girdi = { adres: 'https://x.invalid/a.zip', sha256: '0'.repeat(64), lisans: 'L' };
  await assert.rejects(
    hazirla({ '../kacak': { ...girdi, ic_dizin: 'ic' } }, dizin, join(dizin, 'hedef')),
    /araç adı/,
  );
  await assert.rejects(
    hazirla({ arac: { ...girdi, ic_dizin: '../ic' } }, dizin, join(dizin, 'hedef')),
    /iç dizin/,
  );
  await assert.rejects(
    hazirla({ arac: { ...girdi, adres: 'http://x.invalid/a.zip', ic_dizin: 'ic' } }, dizin, join(dizin, 'hedef')),
    /https/,
  );
});
