// Kurucuya girecek araç zincirini (araclar.json) indirir, özetini denetler ve açar.
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  cpSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
} from 'node:fs';
import { basename, isAbsolute, join, normalize, sep } from 'node:path';
import { Readable } from 'node:stream';
import { pipeline } from 'node:stream/promises';

// Git for Windows'un GNU tar'ı zip açamaz; Windows'ta sistemin bsdtar'ı kullanılır.
function tarYolu() {
  if (process.platform !== 'win32') return 'tar';
  return join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe');
}

export function tarCalistir(argumanlar) {
  const sonuc = spawnSync(tarYolu(), argumanlar, { stdio: ['ignore', 'ignore', 'pipe'] });
  if (sonuc.error) throw sonuc.error;
  if (sonuc.status !== 0) {
    throw new Error(`tar başarısız oldu (çıkış kodu ${sonuc.status ?? 'yok'}): ${sonuc.stderr}`);
  }
}

export function ozetDenetle(dosya, beklenen) {
  const gercek = createHash('sha256').update(readFileSync(dosya)).digest('hex');
  if (gercek !== beklenen.toLowerCase()) {
    rmSync(dosya, { force: true });
    throw new Error(`${dosya}: SHA-256 eşleşmedi (beklenen ${beklenen}, bulunan ${gercek}); dosya silindi.`);
  }
}

function dogrula(ad, arac) {
  if (!/^[a-z0-9_-]+$/.test(ad)) {
    throw new Error(`Geçersiz araç adı: ${ad}`);
  }
  const ic = normalize(arac.ic_dizin ?? '');
  if (!arac.ic_dizin || isAbsolute(ic) || ic === '..' || ic.startsWith(`..${sep}`)) {
    throw new Error(`${ad}: geçersiz iç dizin: ${arac.ic_dizin}`);
  }
  const adres = new URL(arac.adres);
  if (adres.protocol !== 'https:') {
    throw new Error(`${ad}: adres https olmalı: ${arac.adres}`);
  }
  const dosya = basename(adres.pathname);
  if (!/^[A-Za-z0-9._-]+$/.test(dosya)) {
    throw new Error(`${ad}: adresten dosya adı çıkarılamadı: ${arac.adres}`);
  }
  if (!/^[0-9a-fA-F]{64}$/.test(arac.sha256 ?? '')) {
    throw new Error(`${ad}: geçersiz SHA-256: ${arac.sha256}`);
  }
  return dosya;
}

async function indir(adres, dosya) {
  const yanit = await fetch(adres);
  if (!yanit.ok || !yanit.body) {
    throw new Error(`${adres} indirilemedi: HTTP ${yanit.status}`);
  }
  const parca = `${dosya}.parca`;
  await pipeline(Readable.fromWeb(yanit.body), createWriteStream(parca));
  renameSync(parca, dosya);
}

// Her araç önbellekte yoksa (ya da özeti tutmuyorsa) indirilir; `ic_dizin`'i
// `hedef/<ad>`'a konur. Önceki derlemenin `hedef/<ad>`'ı silinir.
export async function hazirla(manifest, onbellek, hedef) {
  const araclar = Object.entries(manifest).map(([ad, arac]) => [ad, arac, dogrula(ad, arac)]);
  mkdirSync(onbellek, { recursive: true });
  mkdirSync(hedef, { recursive: true });
  for (const [ad, arac, dosyaAdi] of araclar) {
    const arsiv = join(onbellek, dosyaAdi);
    if (existsSync(arsiv)) {
      try {
        ozetDenetle(arsiv, arac.sha256);
      } catch {
        // Bozuk ya da yarım dosya silindi; yeniden indirilir.
      }
    }
    if (!existsSync(arsiv)) {
      await indir(arac.adres, arsiv);
      ozetDenetle(arsiv, arac.sha256);
    }

    const acilan = join(onbellek, `acilan-${ad}`);
    rmSync(acilan, { recursive: true, force: true });
    mkdirSync(acilan);
    tarCalistir(['-xf', arsiv, '-C', acilan]);
    const ic = join(acilan, arac.ic_dizin);
    if (!existsSync(ic)) {
      throw new Error(`${ad}: arşivde ${arac.ic_dizin} dizini yok.`);
    }
    const yer = join(hedef, ad);
    rmSync(yer, { recursive: true, force: true });
    // Taşımak yerine kopyalanır: yeni açılmış .exe'leri virüs taraması
    // tutarken Windows dizin taşımasını EPERM ile reddediyor.
    cpSync(ic, yer, { recursive: true });
    rmSync(acilan, { recursive: true, force: true });
    if (!existsSync(join(yer, arac.lisans))) {
      throw new Error(`${ad}: lisans dosyası ${arac.lisans} bulunamadı.`);
    }
  }
}
