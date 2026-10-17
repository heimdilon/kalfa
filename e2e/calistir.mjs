// Uçtan uca testlerin koşucusu (1c tasarımı §11.4).
//
//   node calistir.mjs [--derleme-yok] [testler/<dosya>.test.mjs …]
//
// 1. Arayüzü test kancasıyla derler (`ui/dist-e2e`).
// 2. Uygulamayı `e2e` özelliği ve gömülü arayüzle, ayrı bir hedef dizinine
//    sürüm profilinde derler (`target/e2e/release/kalfa`). Uygulama kimliği
//    `dev.kalfa.e2e` olur: geliştiricinin açık Kalfa'sıyla çakışmaz.
// 3. Testleri sırayla koşar; her test kendi Kalfa sürecini açar.
//
// Windows'ta `msedgedriver` (WebView2 sürümüyle aynı) PATH'te ya da
// `KALFA_E2E_EDGEDRIVER`'da olmalıdır. Linux'ta `WebKitWebDriver` ve bir ekran
// (`xvfb-run -a npm test`) gerekir. `tauri-driver` iki platformda da PATH'te
// olmalıdır.
import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const e2e = dirname(fileURLToPath(import.meta.url));
const kok = join(e2e, '..');
const hedef = join(kok, 'target', 'e2e');
const uygulama = join(hedef, 'release', process.platform === 'win32' ? 'kalfa.exe' : 'kalfa');
// Yerleşik eklenti `kalfa-rag` (aşama 4 tasarımı §13.1): kurulu sürümdeki
// `eklentiler/kalfa-rag/` yerleşimi; uygulama `KALFA_YERLESIK_EKLENTILER`'den okur.
const yerlesik = join(hedef, 'yerlesik-eklentiler');

const argumanlar = process.argv.slice(2);
const derleme = !argumanlar.includes('--derleme-yok');
const secilen = argumanlar.filter((a) => !a.startsWith('--'));

function calistir(komut, args, secenekler = {}) {
  console.log(`> ${komut} ${args.join(' ')}`);
  const sonuc = spawnSync(komut, args, {
    stdio: 'inherit',
    shell: process.platform === 'win32',
    ...secenekler,
  });
  if (sonuc.status !== 0) process.exit(sonuc.status ?? 1);
}

if (derleme) {
  calistir('npm', ['run', 'build:e2e'], { cwd: join(kok, 'ui') });
  calistir(
    'cargo',
    [
      'build',
      '--release',
      '--locked',
      '-p',
      'kalfa',
      '-p',
      'kalfa-sandbox',
      '-p',
      'kalfa-mcp',
      // Uygulama, ağ köprüsü (3d) ve MCP test sunucusu (1.1a-2); `--bin`
      // yalnızca adı geçenleri derler. Test sunucusu kurucuya girmez.
      '--bin',
      'kalfa',
      '--bin',
      'kalfa-kopru',
      '--bin',
      'kalfa-mcp-test-sunucusu',
      '--features',
      'e2e,tauri/custom-protocol',
    ],
    {
      cwd: kok,
      env: {
        ...process.env,
        CARGO_TARGET_DIR: hedef,
        TAURI_CONFIG: JSON.stringify({
          identifier: 'dev.kalfa.e2e',
          build: { frontendDist: '../../ui/dist-e2e' },
        }),
      },
    },
  );
  calistir(
    'cargo',
    ['build', '--release', '--locked', '-p', 'kalfa-rag', '--target', 'wasm32-wasip2'],
    { cwd: kok, env: { ...process.env, CARGO_TARGET_DIR: hedef } },
  );
  const rag = join(yerlesik, 'kalfa-rag');
  mkdirSync(rag, { recursive: true });
  copyFileSync(join(hedef, 'wasm32-wasip2', 'release', 'kalfa_rag.wasm'), join(rag, 'eklenti.wasm'));
  copyFileSync(join(kok, 'crates', 'kalfa-rag', 'kalfa.toml'), join(rag, 'kalfa.toml'));
}

const testler = secilen.length
  ? secilen
  : readdirSync(join(e2e, 'testler'))
      .filter((d) => d.endsWith('.test.mjs'))
      .sort()
      .map((d) => join('testler', d));

calistir(
  process.execPath,
  ['--test', '--test-concurrency=1', '--test-reporter=spec', ...testler],
  {
    cwd: e2e,
    shell: false,
    env: { ...process.env, KALFA_E2E_UYGULAMA: uygulama, KALFA_YERLESIK_EKLENTILER: yerlesik },
  },
);
