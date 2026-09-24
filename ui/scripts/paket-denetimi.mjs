// Derlenmiş paketin denetimi (1c tasarımı §3.1, §6). `npm run build` sonunda
// koşar; bir kural bozulursa derleme başarısız olur.
//
// - HTML'de satır içi betik, stil özniteliği, <style> etiketi ve dış adres yok.
// - Şart penceresinin açılışta yüklediği dosyalar ana pencerenin sohbet
//   komutlarını içermez.
// - Uçtan uca test kancası (`src/e2e/`) sürüm paketine girmez.
import { readdirSync, readFileSync } from 'node:fs';

const dist = new URL('../dist/', import.meta.url);
const oku = (ad) => readFileSync(new URL(ad, dist), 'utf8');
const hatalar = [];

for (const sayfa of ['index.html', 'sartlar.html']) {
  const html = oku(sayfa);
  for (const [etiket] of html.matchAll(/<script\b[^>]*>/g)) {
    if (!/\bsrc=/.test(etiket)) hatalar.push(`${sayfa}: satır içi betik: ${etiket}`);
  }
  if (/\sstyle=/.test(html)) hatalar.push(`${sayfa}: style özniteliği`);
  if (/<style\b/.test(html)) hatalar.push(`${sayfa}: <style> etiketi`);
  if (/(?:src|href)="(?:https?:)?\/\//.test(html)) hatalar.push(`${sayfa}: dış adres`);
}

// §6.1'de yalnızca `baglanti_ac` iki pencere tarafından da kullanılır.
// Geri kalan ana pencere komutları şart penceresinin paketine girmemelidir.
const ANA_PENCERE_OZEL = [
  'uygulama_durumu',
  'oturumlari_listele',
  'calisma_alani_sec',
  'oturum_baslat',
  'oturum_ac',
  'oturumu_dinle',
  'paketi_isledim',
  'gecmis_sayfasi',
  'mesaj_gonder',
  'turu_iptal_et',
  'arac_onayi',
  'model_degistir',
  'izin_kipi_degistir',
  'oturumu_kapat',
  'anahtar_penceresini_ac',
  'sart_penceresini_ac',
];

function javascriptGrafi(baslangic, manifest) {
  const gorulen = new Set();
  const bekleyen = [...baslangic];
  while (bekleyen.length) {
    const anahtar = bekleyen.pop();
    if (gorulen.has(anahtar)) continue;
    const girdi = manifest[anahtar];
    if (!girdi) {
      hatalar.push(`sartlar.html → ${anahtar}: paket manifestinde girdi yok`);
      continue;
    }
    gorulen.add(anahtar);
    for (const baglanti of [...(girdi.imports ?? []), ...(girdi.dynamicImports ?? [])]) {
      bekleyen.push(baglanti);
    }
  }
  return [...gorulen]
    .map((anahtar) => manifest[anahtar].file)
    .filter((dosya) => /\.m?js$/.test(dosya));
}

const manifest = JSON.parse(oku('.vite/manifest.json'));
const sartlarGrafigi = javascriptGrafi(['sartlar.html'], manifest);
for (const dosya of sartlarGrafigi) {
  const icerik = oku(dosya);
  for (const komut of ANA_PENCERE_OZEL) {
    if (icerik.includes(komut)) {
      hatalar.push(`sartlar.html → ${dosya}: ana pencere komutu "${komut}" içeriyor`);
    }
  }
}

// Test kancası yalnızca `npm run build:e2e` derlemesindedir (1c tasarımı §11.4).
for (const dosya of readdirSync(new URL('assets/', dist))) {
  if (!/\.m?js$/.test(dosya)) continue;
  const icerik = oku(`assets/${dosya}`);
  for (const iz of ['__kalfaE2e', '[[e2e:']) {
    if (icerik.includes(iz)) hatalar.push(`assets/${dosya}: uçtan uca test kancası ("${iz}") içeriyor`);
  }
}

if (hatalar.length) {
  console.error('Paket denetimi başarısız:\n' + hatalar.map((h) => `  - ${h}`).join('\n'));
  process.exit(1);
}
console.log('Paket denetimi temiz.');
