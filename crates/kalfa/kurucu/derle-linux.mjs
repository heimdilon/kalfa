// Linux .deb ve .rpm paketlerini aynı sürümden derler (Linux paket tasarımı).
// .deb Kalfa'nın kendi bwrap kopyasını ve onun AppArmor profilini taşır;
// .rpm sistemin bubblewrap paketine bağlıdır.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

if (process.platform !== 'linux') {
  throw new Error('Linux paketleri Linux üzerinde derlenmelidir.');
}

const kalfa = join(dirname(fileURLToPath(import.meta.url)), '..');
const kok = join(kalfa, '..', '..');
const arayuz = join(kok, 'ui');
const hedef = execFileSync('rustc', ['--print', 'host-tuple'], { encoding: 'utf8' }).trim();
if (hedef !== 'x86_64-unknown-linux-gnu') {
  throw new Error(`Desteklenmeyen paket hedefi: ${hedef}`);
}

// Kabuk kullanılmaz: argümanlar birleştirilince boşluklu yollar bölünür.
function calistir(program, argumanlar, dizin) {
  const sonuc = spawnSync(program, argumanlar, { cwd: dizin, stdio: 'inherit' });
  if (sonuc.error) throw sonuc.error;
  if (sonuc.status !== 0) {
    throw new Error(`${program} başarısız oldu (çıkış kodu ${sonuc.status ?? 'yok'}).`);
  }
}

calistir(
  'cargo',
  ['build', '-p', 'kalfa-sandbox', '--bin', 'kalfa-kopru', '--release', '--locked'],
  kok,
);
const ikililer = join(kalfa, 'binaries');
mkdirSync(ikililer, { recursive: true });
copyFileSync(
  join(kok, 'target', 'release', 'kalfa-kopru'),
  join(ikililer, `kalfa-kopru-${hedef}`),
);

// Yerleşik eklenti `kalfa-rag` (aşama 4 tasarımı §13.1): paket
// `/usr/lib/Kalfa/eklentiler/kalfa-rag/`'a koyar.
calistir(
  'cargo',
  ['build', '-p', 'kalfa-rag', '--target', 'wasm32-wasip2', '--release', '--locked'],
  kok,
);
const rag = join(kalfa, 'eklentiler', 'kalfa-rag');
mkdirSync(rag, { recursive: true });
copyFileSync(
  join(kok, 'target', 'wasm32-wasip2', 'release', 'kalfa_rag.wasm'),
  join(rag, 'eklenti.wasm'),
);
copyFileSync(join(kok, 'crates', 'kalfa-rag', 'kalfa.toml'), join(rag, 'kalfa.toml'));

// Kalfa'nın bwrap kopyası derleme makinesinin dağıtım paketinden alınır;
// sürümü ve LGPL kaynak bilgisi pakete yazılır.
const surum = execFileSync('dpkg-query', ['-W', '-f', '${Version}', 'bubblewrap'], {
  encoding: 'utf8',
}).trim();
if (!surum) throw new Error('bubblewrap paketi kurulu değil.');
copyFileSync('/usr/bin/bwrap', join(ikililer, 'bwrap'));
copyFileSync('/usr/share/doc/bubblewrap/copyright', join(ikililer, 'bubblewrap-copyright'));
writeFileSync(
  join(ikililer, 'bubblewrap-KAYNAK'),
  `/usr/lib/kalfa/bwrap, dağıtımın bubblewrap ${surum} paketinin değiştirilmemiş ikilisidir.\n` +
    `Lisans: LGPL-2.0-or-later (copyright dosyası). Kaynak: apt-get source bubblewrap=${surum}\n`,
);

calistir('npm', ['run', 'build'], arayuz);
const tauri = join(arayuz, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
if (!existsSync(tauri)) {
  throw new Error('Tauri CLI bulunamadı. ui/ dizininde npm ci çalıştırın.');
}
calistir(process.execPath, [tauri, 'build', '--config', 'tauri.linux-paket.conf.json'], kalfa);
