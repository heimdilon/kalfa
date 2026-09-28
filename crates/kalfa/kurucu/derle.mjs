// Windows NSIS kurucusunu ve yanındaki yönetici yardımcısını aynı sürümden derler.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { hazirla } from './araclar.mjs';

if (process.platform !== 'win32') {
  throw new Error('NSIS kurucusu Windows üzerinde derlenmelidir.');
}

const kalfa = join(dirname(fileURLToPath(import.meta.url)), '..');
const kok = join(kalfa, '..', '..');
const arayuz = join(kok, 'ui');
const hedef = execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();
if (!/^[a-z0-9_]+-pc-windows-msvc$/.test(hedef)) {
  throw new Error(`Desteklenmeyen kurucu hedefi: ${hedef}`);
}

// Kabuk kullanılmaz: argümanlar birleştirilince boşluklu yollar bölünür.
function calistir(program, argumanlar, dizin) {
  const sonuc = spawnSync(program, argumanlar, {
    cwd: dizin,
    stdio: 'inherit',
  });
  if (sonuc.error) throw sonuc.error;
  if (sonuc.status !== 0) {
    throw new Error(`${program} başarısız oldu (çıkış kodu ${sonuc.status ?? 'yok'}).`);
  }
}

calistir('cargo', ['build', '-p', 'kalfa-yalitim', '--release', '--locked'], kok);
const ikililer = join(kalfa, 'binaries');
mkdirSync(ikililer, { recursive: true });
copyFileSync(
  join(kok, 'target', 'release', 'kalfa-yalitim.exe'),
  join(ikililer, `kalfa-yalitim-${hedef}.exe`),
);
// Python ve Node sabit sürüm ve SHA-256 ile `araclar/`'a açılır; kurucu onları
// `$INSTDIR\araclar\`'a koyar. Python'un yalıtımda çalışması Kalfa'nın
// sitecustomize.py'sini ister (A16).
const kurucu = join(kalfa, 'kurucu');
const araclar = join(kalfa, 'araclar');
await hazirla(
  JSON.parse(readFileSync(join(kurucu, 'araclar.json'), 'utf8')),
  join(kok, 'target', 'kurucu-onbellek'),
  araclar,
);
copyFileSync(
  join(kurucu, 'python', 'sitecustomize.py'),
  join(araclar, 'python', 'Lib', 'sitecustomize.py'),
);
// `npm` bir .cmd dosyasıdır ve kabuksuz başlatılamaz. Komut metni sabittir;
// yol yalnızca çalışma dizini olarak geçer.
calistir('cmd.exe', ['/d', '/s', '/c', 'npm run build'], arayuz);
// CLI, ana tauri.conf.json dosyasını çalışma dizininde bulur; --config yalnızca
// ek ayarları birleştirir. Yerel CLI'yi Node ile doğrudan çağırmak örtük
// indirmeyi ve .cmd sarmalayıcısını önler.
const tauri = join(arayuz, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
if (!existsSync(tauri)) {
  throw new Error('Tauri CLI bulunamadı. ui/ dizininde npm ci çalıştırın.');
}
calistir(
  process.execPath,
  [tauri, 'build', '--config', 'tauri.nsis.conf.json'],
  kalfa,
);
