# Kalfa 1c-3a — Arayüz Temeli Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `ui/` altında Svelte 5 arayüzünün temelini kurmak: iskelet, tasarım tokenları ve fontlar, dil kataloğu, komut sözleşmesinin tipleri ve çağrıları, oturum durumu işlevi, güvenli Markdown ve tasarım sisteminin 1c'de kullanılan bileşenleri.

**Architecture:** Vite 8 + Svelte 5 (runes) + TypeScript 6; SvelteKit yok. İki pencere iki HTML giriş noktasıdır (`index.html`, `sartlar.html`); ekranlar 1c-3b'de gelir, bu planda yer tutucudur. Oturum durumu DOM'a dokunmayan saf bir işlevle güncellenir (`durum/oturum.ts`); Markdown sözdizimi ağacı Svelte bileşenleriyle çizilir, HTML olarak hiçbir şey eklenmez. Bileşenler tasarım sistemindeki (`design/kalfa-ds/components`) adları ve prop'ları taşır; stiller sınıflarla verilir çünkü CSP satır içi stil özniteliğine izin vermez.

**Tech Stack:** Node `^22.22.2 || >=24.15.0` · npm · Vite 8.3 · Svelte 5.57 · TypeScript 6.0 · svelte-check 4.7 · Vitest 5 + @testing-library/svelte + jsdom · @tauri-apps/api 2.11 · @lucide/svelte · shiki 4 (JavaScript motoru) · mdast-util-from-markdown + GFM · @fontsource-variable

**Spec:** `docs/superpowers/specs/2026-09-22-kalfa-1c-design.md` (1c tasarımı) — §3.1, §6, §6.1, §8, §9, §11.3, §13.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-23, Windows 11, Node 24.20): temiz kopyada `npm ci`, `npm run check` (0 hata, 0 uyarı), `npm test` (10 dosya, 102 test), `npm run build` ve paket denetimi geçti; bileşen vitrini tarayıcıda tasarım sistemiyle karşılaştırıldı. Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:** iki pencerenin ekranları ve komutlara bağlanmaları (1c-3b) · Rust tarafı (`kalfa-uygulama`, `kalfa`; 1c-2) · gerçek pencerede uçtan uca testler (1c-4).

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `1c-3a/arayuz-temeli`, PR'ın hedefi `kalfa-asama-1`. Ayrı bir PR açman gerekirse hedef dal olarak `1c-3a/arayuz-temeli`'ni seç.
- **Hazır gelenler:** `ui/package.json`, `ui/package-lock.json` ve `ui/.gitignore` bu dalın ilk commit'inde, doğrulanmış sürümlerle hazır. Değiştirme. Bağımlılık eklemen ya da sürüm değiştirmen gerekirse dur ve PR'a yorum yaz.
- **Ortam:** Node `^22.22.2 || >=24.15.0` (jsdom 30 ve Vitest 5 bunu ister), npm 10 ya da üstü. `npm ci` için ağ erişimi gerekir. Rust gerekmez; `cargo` komutları bu PR'da koşturulmaz.
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar, Windows'ta doğrular ve vitrinle görsel karşılaştırma yapar; inceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Node `^22.22.2 || >=24.15.0`; paket yöneticisi npm; kurulum `npm ci`. Sürümler `package.json`'da sabittir (aralık yok).
- **CSP (1c tasarımı §6):** şablonda sabit `style="…"` özniteliği yok; değişken stil `style:` yönergesiyle verilir. `{@html}`, `innerHTML`, `outerHTML`, `insertAdjacentHTML`, `document.write`, `eval`, `new Function` yok. Dış kaynaktan font, ikon ya da stil yüklenmez. `src/csp.test.ts` kaynağı tarar; `scripts/paket-denetimi.mjs` derlenmiş paketi denetler.
- **Markdown (1c tasarımı §9.1):** ham HTML metin olarak görünür; yalnızca mutlak `http`/`https` adresleri bağlantı olur ve bağlantı bir düğmedir (`baglanti_ac` yolundan açılır); görselin yerine alt metni yazılır, hiçbir görsel yüklenmez.
- **Dil (1c tasarımı §9.4):** kullanıcıya görünen her metin `src/dil/tr.ts`'de; arayüz "siz" diliyle; ünlem ve emoji yok; biten iş "Tamamlandı".
- **Bileşenler:** adları ve prop'ları tasarım sistemindeki `.d.ts` dosyalarındaki gibi; `style` prop'u yok, yerine `class`. İkonlar yalnızca `src/bilesenler/ikonlar.ts`'deki kayıttan.
- **Komut sözleşmesi (1c tasarımı §6.1):** komut ve argüman adları snake_case, Rust'takiyle aynı.
- **Tokenlar:** `design/kalfa-ds/tokens`'tan kopya; iki bilinçli değişiklik var: font aileleri fontsource adlarını da taşır, başlık yazı boyutu `--text-title-size` adını alır (tasarım sisteminde `--text-title` hem renk hem boyuttu).
- **Her görevin sonunda (`ui/` içinde):** `npm run check` 0 hata ve 0 uyarı; `npm test` yeşil; `npm run build` (paket denetimi dahil) temiz.
- **Commit:** mesajlar Türkçe, `ui: …` biçiminde. Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:** `design/`, `crates/`, `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`.

## Dosya Yapısı

```
ui/
  package.json, package-lock.json   bağımlılıklar (plan commit'inde hazır)
  .gitignore                        node_modules, dist (plan commit'inde hazır)
  vite.config.ts                    iki giriş noktası, Vitest (jsdom)
  svelte.config.js, tsconfig.json
  index.html, sartlar.html          ana pencere, şart penceresi
  vitrin.html                       bileşen vitrini (yalnızca geliştirmede)
  scripts/paket-denetimi.mjs        derlenmiş paketin denetimi
  src/
    csp.test.ts                     kaynak kodu CSP ile uyumlu mu
    stiller/                        tokenlar (kopya), kalfa.css, index.ts (fontlar), tokenlar.test.ts
    dil/                            tr.ts (katalog), dil.test.ts
    ipc/                            tipler.ts, ana.ts, sartlar.ts, hata.ts, ipc.test.ts
    durum/                          oturum.ts, dinleyici.ts, oturum.test.ts
    markdown/                       cozumle.ts, adres.ts, vurgula.ts, Markdown.svelte, Dugum.svelte, markdown.test.ts
    bilesenler/                     25 bileşen, ikonlar.ts, diff.ts; temel/veri/sohbet/diff testleri
    ana/, sartlar/                  giris.ts ve yer tutucu ekran (1c-3b'de değişir)
    vitrin/                         giris.ts, Vitrin.svelte
docs/GELISTIRME_DURUMU.md           1c-3a sonuçları (değişir)
```

---

### Task 1: İskelet, tokenlar, sözleşme tipleri ve dil kataloğu

Vite + Svelte + TypeScript iskeleti, tasarım tokenları ve fontlar, komut sözleşmesinin tipleri ve Türkçe katalog. İki pencerenin giriş noktası bu görevde yer tutucu bir ekran gösterir.

**Files:**
- Var (plan commit'inde): `ui/package.json`, `ui/package-lock.json`, `ui/.gitignore`
- Create: `ui/vite.config.ts`, `ui/svelte.config.js`, `ui/tsconfig.json`, `ui/index.html`, `ui/sartlar.html`, `ui/scripts/paket-denetimi.mjs`
- Create (kopya): `ui/src/stiller/{base,colors,elevation,motion,radius,spacing}.css`
- Create: `ui/src/stiller/typography.css`, `ui/src/stiller/kalfa.css`, `ui/src/stiller/index.ts`
- Create: `ui/src/ipc/tipler.ts`, `ui/src/dil/tr.ts`
- Create: `ui/src/ana/giris.ts`, `ui/src/ana/Ana.svelte`, `ui/src/sartlar/giris.ts`, `ui/src/sartlar/Sartlar.svelte`
- Test: `ui/src/stiller/tokenlar.test.ts`, `ui/src/dil/dil.test.ts`, `ui/src/csp.test.ts`

**Interfaces:**
- Produces: `ipc/tipler.ts`'deki bütün sözleşme tipleri (`Olay`, `Zarf`, `KanalMesaji`, `GorunumOgesi`, `GecmisSayfasi`, `DurumOzeti`, `OturumAcilisi`, `SaltOkunurNedeni`, `UygulamaDurumu`, `OturumSatiri`, `CalismaAlani`, `OnayOgesi`, `OnayKarari`, `OnayGirdisi`, `SartMetni`, `SartPenceresiDurumu`, `AnahtarSonucu`, `KabulYaniti`, `KomutHatasi`, `Kip`, `KIPLER`, `AracDurumu`, `BitisNedeni`, `AktarimDurmaNedeni`, `ModelProfili` …); `dil/tr.ts`: `tr`, `MetinAnahtari`, `m(anahtar, degerler?)`, `KIP_ADI`, `MODEL_PROFILI_ADI`, `ARAC_DURUMU_ADI`, `AKTARIM_DURMA_METNI`.

- [ ] **Step 1: Bağımlılıkları kur**

Run: `cd ui && npm ci`
Expected: `added … packages`; hata yok. Node sürümü uymazsa `npm` uyarı verir; `node --version` ile `^22.22.2 || >=24.15.0` olduğunu doğrula.

- [ ] **Step 2: Yapılandırma dosyalarını yaz**

`ui/vite.config.ts`:

```ts
/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

// İki pencere iki ayrı giriş noktasıdır (1c tasarımı §3.1). `vitrin.html`
// yalnızca geliştirme sunucusunda açılır; derlemeye girmez.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    target: 'es2022',
    manifest: true,
    rolldownOptions: {
      input: { index: 'index.html', sartlar: 'sartlar.html' },
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
```

`ui/svelte.config.js`:

```js
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default { preprocess: vitePreprocess() };
```

`ui/tsconfig.json` (`ES2023` kütüphanesi `findLastIndex` için; `vite/client` CSS içe aktarımlarının tipleri için):

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "isolatedModules": true,
    "verbatimModuleSyntax": true,
    "skipLibCheck": true,
    "types": ["vitest/globals", "svelte", "vite/client"],
    "lib": ["ES2023", "DOM", "DOM.Iterable"]
  },
  "include": ["src/**/*.ts", "src/**/*.svelte", "vite.config.ts"]
}
```

`ui/index.html`:

```html
<!doctype html>
<html lang="tr">
  <head>
    <meta charset="utf-8" />
    <title>Kalfa</title>
  </head>
  <body class="kalfa-root">
    <div id="kok"></div>
    <script type="module" src="/src/ana/giris.ts"></script>
  </body>
</html>
```

`ui/sartlar.html` (şart penceresi açık temayla açılır):

```html
<!doctype html>
<html lang="tr">
  <head>
    <meta charset="utf-8" />
    <title>Kalfa — EVREN şartları</title>
  </head>
  <body class="kalfa-root kalfa-light">
    <div id="kok"></div>
    <script type="module" src="/src/sartlar/giris.ts"></script>
  </body>
</html>
```

`ui/scripts/paket-denetimi.mjs` (`npm run build` bunu derlemeden sonra koşturur):

```js
// Derlenmiş paketin denetimi (1c tasarımı §3.1, §6). `npm run build` sonunda
// koşar; bir kural bozulursa derleme başarısız olur.
//
// - HTML'de satır içi betik, stil özniteliği, <style> etiketi ve dış adres yok.
// - Şart penceresinin girişinden erişilebilen bütün JavaScript parçaları ana
//   pencereye özel §6.1 komutlarını içermez.
import { readFileSync } from 'node:fs';

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

if (hatalar.length) {
  console.error('Paket denetimi başarısız:\n' + hatalar.map((h) => `  - ${h}`).join('\n'));
  process.exit(1);
}
console.log('Paket denetimi temiz.');
```

- [ ] **Step 3: Tokenları kopyala**

Run (depo kökünden):

```bash
mkdir -p ui/src/stiller
cp design/kalfa-ds/tokens/{base,colors,elevation,motion,radius,spacing}.css ui/src/stiller/
```

`fonts.css` kopyalanmaz (Google Fonts'tan yükler). `typography.css` iki değişiklikle yazılır: font aileleri fontsource paketlerinin tanımladığı "… Variable" adlarını önce taşır; başlık yazı boyutu `--text-title` yerine `--text-title-size` adını alır (tasarım sisteminde `--text-title` `colors.css`'te başlık rengi, `typography.css`'te `17px`'ti; sonra yüklenen boyut rengi eziyor, koyu temada başlıklar renksiz kalıyordu). `ui/src/stiller/typography.css`:

```css
:root{
--font-sans:"Source Sans 3 Variable","Source Sans 3","Segoe UI",system-ui,sans-serif;
--font-serif:"Source Serif 4 Variable","Source Serif 4",Georgia,serif;
--font-mono:"JetBrains Mono Variable","JetBrains Mono","Cascadia Mono",ui-monospace,monospace;

--text-wordmark:38px;
--text-display:28px;
--text-title-lg:22px;
--text-title-size:17px;
--text-subtitle:15px;
--text-body-size:14px;
--text-small:13px;
--text-micro:12px;
--text-nano:11px;

--lh-tight:1.15;/* @kind other */
--lh-snug:1.35;/* @kind other */
--lh-normal:1.55;/* @kind other */
--lh-relaxed:1.7;/* @kind other */

--fw-light:300;/* @kind other */--fw-regular:400;/* @kind other */--fw-medium:500;/* @kind other */--fw-semibold:600;/* @kind other */--fw-bold:700;/* @kind other */

--tracking-wordmark:-0.02em;/* @kind other */
--tracking-tight:-0.01em;/* @kind other */
--tracking-normal:0;/* @kind other */
--tracking-label:0.06em;/* @kind other */  /* sidebar section labels, all-caps */
--tracking-mono:0.01em;/* @kind other */

/* role aliases */
--type-wordmark:var(--fw-light) var(--text-wordmark)/var(--lh-tight) var(--font-sans);
--type-tagline:var(--fw-regular) var(--text-micro)/var(--lh-snug) var(--font-mono);
--type-display:var(--fw-semibold) var(--text-display)/var(--lh-tight) var(--font-sans);
--type-title:var(--fw-semibold) var(--text-title-size)/var(--lh-snug) var(--font-sans);
--type-subtitle:var(--fw-regular) var(--text-subtitle)/var(--lh-snug) var(--font-sans);
--type-body:var(--fw-regular) var(--text-body-size)/var(--lh-normal) var(--font-sans);
--type-small:var(--fw-regular) var(--text-small)/var(--lh-normal) var(--font-sans);
--type-label:var(--fw-semibold) var(--text-nano)/var(--lh-snug) var(--font-sans);
--type-code:var(--fw-regular) var(--text-small)/1.65 var(--font-mono);
--type-meta:var(--fw-regular) var(--text-micro)/var(--lh-snug) var(--font-mono);
--type-doc:var(--fw-regular) 16px/var(--lh-relaxed) var(--font-serif);
}
```

`ui/src/stiller/kalfa.css`:

```css
/* Kalfa arayüzünün tek stil girişi. Tokenlar design/kalfa-ds/tokens'tan
   kopyalandı (fonts.css hariç; fontlar npm paketlerinden gelir). */
@import './colors.css';
@import './typography.css';
@import './spacing.css';
@import './radius.css';
@import './elevation.css';
@import './motion.css';
@import './base.css';

html,
body {
  margin: 0;
  height: 100%;
}

#kok {
  height: 100%;
}

/* Ekranda görünmeyen ama ekran okuyucunun okuduğu metin. */
.gizli-erisilebilir {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}

@keyframes kalfa-imlec {
  50% {
    opacity: 0;
  }
}

@keyframes kalfa-don {
  to {
    transform: rotate(360deg);
  }
}
```

`ui/src/stiller/index.ts`:

```ts
// Fontlar npm paketlerinden gelir; Vite dosyaları pakete gömer (1c tasarımı
// §3.1). Dış kaynaktan font yüklenmez.
import '@fontsource-variable/source-sans-3/wght.css';
import '@fontsource-variable/source-sans-3/wght-italic.css';
import '@fontsource-variable/source-serif-4/wght.css';
import '@fontsource-variable/jetbrains-mono/wght.css';
import './kalfa.css';
```

- [ ] **Step 4: Testleri yaz**

`ui/src/stiller/tokenlar.test.ts`:

```ts
import { describe, expect, it } from 'vitest';

const dosyalar = import.meta.glob('/src/stiller/*.css', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

function tanimlar(secici: string): Map<string, string[]> {
  const yerler = new Map<string, string[]>();
  for (const [dosya, icerik] of Object.entries(dosyalar)) {
    const yorumsuz = icerik.replace(/\/\*[\s\S]*?\*\//g, '');
    const blok = new RegExp(`${secici.replace('.', '\\.')}\\s*\\{([^}]*)\\}`, 'g');
    for (const [, govde] of yorumsuz.matchAll(blok)) {
      for (const [, ad] of govde.matchAll(/(--[\w-]+)\s*:/g)) {
        yerler.set(ad, [...(yerler.get(ad) ?? []), dosya]);
      }
    }
  }
  return yerler;
}

describe('tasarım tokenları', () => {
  it('bir token :root içinde yalnızca bir kez tanımlanır', () => {
    // Tasarım sistemindeki `--text-title` hem renk hem yazı boyutuydu; sonra
    // yüklenen boyut rengi eziyordu. Boyut `--text-title-size` oldu.
    const cakisan = [...tanimlar(':root')].filter(([, yerler]) => yerler.length > 1);
    expect(cakisan).toEqual([]);
  });

  it('açık tema yalnızca var olan tokenları yeniden tanımlar', () => {
    const kok = tanimlar(':root');
    const acik = [...tanimlar('.kalfa-light').keys()];
    expect(acik.filter((ad) => !kok.has(ad))).toEqual([]);
  });
});
```

`ui/src/csp.test.ts`:

```ts
import { describe, expect, it } from 'vitest';

// CSP satır içi stili ve betiği engeller (1c tasarımı §6). Bu tarama, kurala
// uymayan kodun derlemeye girmeden yakalanması içindir.
const kaynaklar = import.meta.glob(['/src/**/*.{ts,svelte}', '!/src/**/*.test.ts'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const YASAKLAR: [RegExp, string][] = [
  [/\sstyle=["']/, 'sabit style özniteliği (sınıf ya da style: yönergesi kullanın)'],
  [/\{@html\b/, '{@html}'],
  [/\.(innerHTML|outerHTML)\s*=/, 'innerHTML/outerHTML ataması'],
  [/insertAdjacentHTML|document\.write/, 'HTML enjeksiyonu'],
  [/\beval\(|new Function\(/, 'eval / new Function'],
  [/https?:\/\/fonts\.googleapis|cdn\.|unpkg\.com/, 'dış kaynak'],
];

describe('kaynak kodu CSP ile uyumlu', () => {
  for (const [dosya, icerik] of Object.entries(kaynaklar)) {
    it(dosya, () => {
      const bulunan = YASAKLAR.filter(([desen]) => desen.test(icerik)).map(([, ad]) => ad);
      expect(bulunan).toEqual([]);
    });
  }
});
```

`ui/src/dil/dil.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { m, tr } from './tr';

// Kaynak dosyalar metin olarak (testler hariç).
const kaynaklar = import.meta.glob(['/src/**/*.{ts,svelte}', '!/src/**/*.test.ts'], {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

describe('dil kataloğu', () => {
  it('kodda kullanılan her metin anahtarı katalogda var (1c tasarımı §11.3)', () => {
    const eksik: string[] = [];
    for (const [dosya, icerik] of Object.entries(kaynaklar)) {
      for (const [, anahtar] of icerik.matchAll(/\bm\(\s*'([^']+)'/g)) {
        if (!(anahtar in tr)) eksik.push(`${dosya}: ${anahtar}`);
      }
    }
    expect(eksik).toEqual([]);
  });

  it('değişkenleri doldurur, bilinmeyeni olduğu gibi bırakır', () => {
    expect(m('hata.calisma_alani_yok', { yol: 'C:\\proje' })).toBe(
      'Çalışma alanı bulunamadı: C:\\proje',
    );
    expect(m('hata.calisma_alani_yok')).toBe('Çalışma alanı bulunamadı: {yol}');
  });

  it('metinlerde ünlem ve emoji yok (1c tasarımı §9.4)', () => {
    for (const metin of Object.values(tr)) {
      expect(metin).not.toMatch(/!|\p{Extended_Pictographic}/u);
    }
  });
});
```

- [ ] **Step 5: Testlerin başarısız olduğunu gör**

Run: `cd ui && npm test`
Expected: FAIL — `dil.test.ts` `./tr` modülünü bulamaz. `tokenlar.test.ts` ve `csp.test.ts` geçer.

- [ ] **Step 6: Sözleşme tiplerini ve kataloğu yaz**

`ui/src/ipc/tipler.ts`:

```ts
// Komut sözleşmesinin tipleri (1c tasarımı §6.1). JSON biçimi Rust'taki serde
// tanımlarından gelir: etiketli enum'lar `tip` alanı taşır, birim değerli
// enum'lar düz metindir, `u64` alanları sayıdır. Rust tarafı değişirse burası
// da aynı commit'te değişir.

/** UUID metni. */
export type OturumId = string;
export type TurId = string;
export type CagriId = string;
export type IslemId = string;

// ── kalfa-core ────────────────────────────────────────────────────────────

export type Kip = 'Inceleme' | 'Siki' | 'Proje' | 'Serbest';

/** Kiplerin sırası: sağa her geçiş yükseltmedir (1c tasarımı §6). */
export const KIPLER: readonly Kip[] = ['Inceleme', 'Siki', 'Proje', 'Serbest'];

export type AracDurumu =
  | 'Basarili'
  | 'Basarisiz'
  | 'Reddedildi'
  | 'IptalEdildi'
  | 'SonucuBilinmiyor';

export type BitisNedeni =
  | { tip: 'Normal' }
  | { tip: 'Iptal' }
  | { tip: 'Sinir'; aciklama: string }
  | { tip: 'Hata' };

export type AktarimDurmaNedeni = 'TuketiciYetismedi' | 'OlayCokBuyuk' | 'Diger';

export interface KrediGozlemi {
  /** Ondalık sayı metni. */
  kalan: string | null;
  tutulan: string | null;
  istek_id: string | null;
  /** Unix epoch milisaniyesi. */
  zaman_ms: number;
}

export interface OnayOgesi {
  cagri: CagriId;
  arac: string;
  aciklama: string;
  hedefler: string[];
  diff: string | null;
  uyarilar: string[];
  muhur: string;
}

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };

export type Olay =
  | { tip: 'TurBasladi' }
  | { tip: 'TokenParcasi'; metin: string }
  | { tip: 'DusunceParcasi'; metin: string }
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  | { tip: 'AracSonucu'; cagri: CagriId; arac: string; durum: AracDurumu; icerik: string }
  | { tip: 'OnayGerekiyor'; islem: IslemId; ogeler: OnayOgesi[] }
  | { tip: 'KrediGozlemi'; gozlem: KrediGozlemi }
  | { tip: 'TokenKullanimi'; istem: number; tamamlama: number; dusunce: number }
  | { tip: 'SartOnayiGerekiyor'; surum: number; metin: string }
  | { tip: 'Hata'; mesaj: string }
  | { tip: 'Tamamlandi'; neden: BitisNedeni }
  | { tip: 'AktarimDurdu'; neden: AktarimDurmaNedeni }
  | { tip: 'IstekBilgisi'; istek_id: string | null; yonlendirilen_model: string | null };

export interface Zarf<T> {
  sema: number;
  oturum: OturumId;
  tur: TurId | null;
  sira: number;
  yuk: T;
}

export type KanalMesaji =
  | { tip: 'Olaylar'; no: number; zarflar: Zarf<Olay>[] }
  | { tip: 'Kapandi'; no: number };

export type GorunumOgesi =
  | { tip: 'KullaniciMesaji'; sira: number; tur: TurId | null; metin: string }
  | {
      tip: 'Yanit';
      sira: number;
      tur: TurId | null;
      metin: string | null;
      dusunce: string | null;
    }
  | {
      tip: 'AracCagrisi';
      sira: number;
      tur: TurId | null;
      model_cagri_id: string;
      arac: string;
      argumanlar: string;
      /** `null`: sonuç kaydı yok, çağrı hiç başlamadı. */
      durum: AracDurumu | null;
      sonuc: string | null;
    }
  | { tip: 'TurSonu'; sira: number; tur: TurId | null; neden: BitisNedeni };

export interface GecmisSayfasi {
  ogeler: GorunumOgesi[];
  daha_eski_var: boolean;
}

export interface DurumOzeti {
  model: string | null;
  kip: Kip | null;
  yonlendirilen_model: string | null;
  son_kredi: KrediGozlemi | null;
  istem: number;
  tamamlama: number;
  dusunce: number;
}

export type OturumDurumu =
  | { tip: 'Bos' }
  | { tip: 'Tamamlandi' }
  | { tip: 'IptalEdildi' }
  | { tip: 'Hata' }
  | { tip: 'YarimKaldi' }
  | { tip: 'Okunamadi'; neden: string };

export interface SartMetni {
  surum: number;
  /** EVREN'in `is_material` bilgisi. */
  esasli: boolean;
  metin: string;
}

// ── kalfa-uygulama ────────────────────────────────────────────────────────

export type ModelProfili = 'Hizli' | 'Dengeli' | 'Derin' | 'Otomatik';

export interface ModelSecenegi {
  kimlik: string;
  profil: ModelProfili;
}

export type DepoTuru = 'Kalici' | 'Bellekte';

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
  anahtar_deposu: DepoTuru;
  modeller: ModelSecenegi[];
  varsayilan_model: string;
  son_oturum: OturumId | null;
}

export interface OturumSatiri {
  oturum: OturumId | null;
  dosya: string;
  baslik: string | null;
  calisma_alani: string | null;
  son_degisiklik_ms: number;
  durum: OturumDurumu;
}

export interface CalismaAlani {
  kimlik: number;
  yol: string;
}

export type SaltOkunurNedeni =
  | { tip: 'CalismaAlaniYok'; yol: string }
  | { tip: 'CalismaAlaniKayitsiz' };

export interface OturumAcilisi {
  oturum: OturumId;
  calisma_alani: string | null;
  gecmis: GecmisSayfasi;
  ozet: DurumOzeti;
  salt_okunur: SaltOkunurNedeni | null;
}

export interface OnayGirdisi {
  cagri: CagriId;
  muhur: string;
  karar: OnayKarari;
}

export type AnahtarNedeni = 'Yok' | 'KayitliGecersiz' | 'Degistir';

export type SartPenceresiDurumu =
  | { tip: 'AnahtarGerekli'; neden: AnahtarNedeni }
  | { tip: 'SartGerekli'; metin: SartMetni }
  | { tip: 'Kilitli' }
  | { tip: 'Tamam' };

export type AnahtarSonucu =
  | { tip: 'Gecersiz' }
  | { tip: 'Ulasilamadi' }
  | { tip: 'Kaydedildi'; depo: DepoTuru; sonraki: SartPenceresiDurumu };

export type KabulYaniti = { tip: 'Kabul' } | { tip: 'YeniMetin'; metin: SartMetni };

export type HataKodu =
  | 'OturumYok'
  | 'KayitMesgul'
  | 'SaltOkunur'
  | 'GecersizGirdi'
  | 'Evren'
  | 'Ic';

export interface KomutHatasi {
  kod: HataKodu;
  mesaj: string;
}
```

`ui/src/dil/tr.ts`:

```ts
// Arayüzün Türkçe metin kataloğu (1c tasarımı §9.4). Kurallar: "siz" dili;
// ünlem ve emoji yok; biten iş "Tamamlandı" ile özetlenir; hata metni ne
// olduğunu ve ne yapılacağını söyler. Değişkenler `{ad}` biçimindedir.
import type { AktarimDurmaNedeni, AracDurumu, Kip, ModelProfili } from '../ipc/tipler';

export const tr = {
  'uygulama.adi': 'Kalfa',

  'mesaj.kullanici': 'Siz',
  'mesaj.kalfa': 'Kalfa',

  'dusunce.baslik': 'Düşünce',
  'dusunce.ozet': 'Modelin bu yanıt için düşündükleri',

  'composer.yer_tutucu': 'Bir şey sorun ya da bir görev tanımlayın…',
  'composer.gonder': 'Gönder',
  'composer.durdur': 'Turu durdur',
  'composer.model': 'Model',
  'composer.kip': 'İzin kipi',

  'kip.Inceleme': 'İnceleme',
  'kip.Siki': 'Sıkı',
  'kip.Proje': 'Proje',
  'kip.Serbest': 'Serbest',

  'model.Hizli': 'Hızlı',
  'model.Dengeli': 'Dengeli',
  'model.Derin': 'Derin',
  'model.Otomatik': 'Otomatik',

  'arac.durum.Suruyor': 'Çalışıyor',
  'arac.durum.Basarili': 'Tamamlandı',
  'arac.durum.Basarisiz': 'Başarısız',
  'arac.durum.Reddedildi': 'Reddedildi',
  'arac.durum.IptalEdildi': 'İptal edildi',
  'arac.durum.SonucuBilinmiyor': 'Sonucu bilinmiyor',
  'arac.durum.Baslamadi': 'Çalıştırılmadı',

  'tur.bitti.Normal': 'Tamamlandı',
  'tur.bitti.Iptal': 'İptal edildi',
  'tur.bitti.Sinir': 'Sınıra ulaşıldı: {aciklama}',
  'tur.bitti.Hata': 'Hatayla bitti',

  'aktarim.TuketiciYetismedi':
    'Arayüz uzun süre yanıt vermediği için oturum durduruldu. Oturumu yeniden açabilirsiniz.',
  'aktarim.OlayCokBuyuk':
    'Bu adımın çıktısı gösterilemeyecek kadar büyük olduğu için oturum durduruldu. Oturumu yeniden açabilirsiniz.',
  'aktarim.Diger': 'Oturumun olay aktarımı durdu. Oturumu yeniden açabilirsiniz.',

  'hata.anahtar_gecersiz':
    'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
  'hata.evren_ulasilamiyor': "EVREN'e ulaşılamıyor. Bağlantınızı kontrol edip yeniden deneyin.",
  'hata.anahtar_bellekte':
    'Anahtar bu oturum boyunca bellekte tutulacak. Kalfa kapanınca yeniden girmeniz gerekecek.',
  'hata.kayit_mesgul': 'Bu oturum başka bir Kalfa sürecinde açık.',
  'hata.calisma_alani_yok': 'Çalışma alanı bulunamadı: {yol}',
  'hata.calisma_alani_kayitsiz':
    'Bu kayıt çalışma alanını yazmıyor; yalnızca geçmişi gösterilebilir.',
  'hata.sart_guncellendi': 'Şart metni bu sırada güncellendi.',
  'hata.baglanti_yok':
    'EVREN bağlantısı yok. Geçmiş oturumları okuyabilirsiniz; mesajları bağlantı gelince gönderin.',

  'markdown.gorsel': 'görsel',
  'kod.kirpildi': 'Çıktının devamı gösterilmiyor',
} as const;

export type MetinAnahtari = keyof typeof tr;

/** Katalogdaki metni verir; `{ad}` yer tutucularını `degerler` ile doldurur. */
export function m(anahtar: MetinAnahtari, degerler: Record<string, string | number> = {}): string {
  return tr[anahtar].replace(/\{(\w+)\}/g, (butun, ad: string) =>
    ad in degerler ? String(degerler[ad]) : butun,
  );
}

// Çekirdekten gelen değerlerin adları. `Record` her değerin karşılığı olduğunu
// derleme sırasında denetler.
export const KIP_ADI: Record<Kip, MetinAnahtari> = {
  Inceleme: 'kip.Inceleme',
  Siki: 'kip.Siki',
  Proje: 'kip.Proje',
  Serbest: 'kip.Serbest',
};

export const MODEL_PROFILI_ADI: Record<ModelProfili, MetinAnahtari> = {
  Hizli: 'model.Hizli',
  Dengeli: 'model.Dengeli',
  Derin: 'model.Derin',
  Otomatik: 'model.Otomatik',
};

export const ARAC_DURUMU_ADI: Record<AracDurumu | 'Suruyor' | 'Baslamadi', MetinAnahtari> = {
  Suruyor: 'arac.durum.Suruyor',
  Basarili: 'arac.durum.Basarili',
  Basarisiz: 'arac.durum.Basarisiz',
  Reddedildi: 'arac.durum.Reddedildi',
  IptalEdildi: 'arac.durum.IptalEdildi',
  SonucuBilinmiyor: 'arac.durum.SonucuBilinmiyor',
  Baslamadi: 'arac.durum.Baslamadi',
};

export const AKTARIM_DURMA_METNI: Record<AktarimDurmaNedeni, MetinAnahtari> = {
  TuketiciYetismedi: 'aktarim.TuketiciYetismedi',
  OlayCokBuyuk: 'aktarim.OlayCokBuyuk',
  Diger: 'aktarim.Diger',
};
```

- [ ] **Step 7: Yer tutucu ekranları yaz**

`ui/src/ana/Ana.svelte`:

```svelte
<!-- Ana pencerenin yer tutucusu; ekranlar 1c-3b'de gelir. -->
<script lang="ts">
  import { m } from '../dil/tr';
</script>

<main class="yer-tutucu">
  <h1>{m('uygulama.adi')}</h1>
</main>

<style>
  .yer-tutucu {
    display: grid;
    place-items: center;
    height: 100%;
  }
  h1 {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
</style>
```

`ui/src/ana/giris.ts`:

```ts
import { mount } from 'svelte';
import '../stiller';
import Ana from './Ana.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Ana, { target: kok });
```

`ui/src/sartlar/Sartlar.svelte`:

```svelte
<!-- Şart penceresinin yer tutucusu; ekranlar 1c-3b'de gelir. -->
<script lang="ts">
  import { m } from '../dil/tr';
</script>

<main class="yer-tutucu">
  <h1>{m('uygulama.adi')}</h1>
</main>

<style>
  .yer-tutucu {
    display: grid;
    place-items: center;
    height: 100%;
  }
  h1 {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
</style>
```

`ui/src/sartlar/giris.ts`:

```ts
import { mount } from 'svelte';
import '../stiller';
import Sartlar from './Sartlar.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Sartlar, { target: kok });
```

- [ ] **Step 8: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: `svelte-check` 0 hata 0 uyarı; testler geçer; derleme sonunda `Paket denetimi temiz.` `dist/index.html` ve `dist/sartlar.html` yalnızca `src`'li `<script type="module">` taşır.

- [ ] **Step 9: Commit**

```bash
git add ui/vite.config.ts ui/svelte.config.js ui/tsconfig.json ui/index.html ui/sartlar.html ui/scripts ui/src
git commit -m "ui: iskelet, tasarım tokenları, sözleşme tipleri ve dil kataloğu"
git push
```

---

### Task 2: Komut çağrıları

İki pencerenin komutları için ince, tipli `invoke` sarmalayıcıları. Şart penceresinin modülü ana pencerenin komutlarını içe aktarmaz; paket denetimi bunu derlenmiş pakette de sınar.

**Files:**
- Create: `ui/src/ipc/ana.ts`, `ui/src/ipc/sartlar.ts`, `ui/src/ipc/hata.ts`
- Test: `ui/src/ipc/ipc.test.ts`

**Interfaces:**
- Consumes: Görev 1'in `ipc/tipler.ts`'i.
- Produces: `ipc/ana.ts`: `uygulamaDurumu()`, `oturumlariListele()`, `calismaAlaniSec()`, `oturumBaslat(calisma_alani: number, model: string)`, `oturumAc(oturum)`, `oturumuDinle(oturum, dinleyici: (m: KanalMesaji) => void)`, `paketiIsledim(oturum, no)`, `gecmisSayfasi(oturum, once_sira)`, `mesajGonder(oturum, metin)`, `turuIptalEt(oturum)`, `aracOnayi(oturum, kararlar: OnayGirdisi[])`, `modelDegistir(oturum, model)`, `izinKipiDegistir(oturum, kip): Promise<Kip>`, `oturumuKapat(oturum)`, `baglantiAc(adres)`, `anahtarPenceresiniAc()`, `sartPenceresiniAc()`; `ipc/sartlar.ts`: `sartPenceresiDurumu()`, `anahtariKaydet(anahtar)`, `sartlariKabulEt(surum)`, `sartlariReddet()`, `sartMetniniGoster()`, `baglantiAc(adres)`; `ipc/hata.ts`: `komutHatasi(hata: unknown): KomutHatasi`.

- [ ] **Step 1: Testi yaz**

`ui/src/ipc/ipc.test.ts`:

```ts
import { Channel } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import * as ana from './ana';
import { komutHatasi } from './hata';
import * as sartlar from './sartlar';

afterEach(() => clearMocks());

function kaydet(): [string, Record<string, unknown> | undefined][] {
  const cagrilar: [string, Record<string, unknown> | undefined][] = [];
  mockIPC((komut, argumanlar) => {
    cagrilar.push([komut, argumanlar as Record<string, unknown> | undefined]);
    return null;
  });
  return cagrilar;
}

describe('ana pencere komutları (1c tasarımı §6.1)', () => {
  it('komut adları ve argüman adları Rust tarafıyla aynı', async () => {
    const c = kaydet();
    await ana.oturumBaslat(7, 'glm-5.3');
    await ana.oturumAc('o-1');
    await ana.paketiIsledim('o-1', 3);
    await ana.gecmisSayfasi('o-1', 120);
    await ana.mesajGonder('o-1', 'selam');
    await ana.turuIptalEt('o-1');
    await ana.aracOnayi('o-1', [{ cagri: 'c-1', muhur: 'm', karar: { tip: 'Onayla' } }]);
    await ana.modelDegistir('o-1', 'auto');
    await ana.izinKipiDegistir('o-1', 'Proje');
    await ana.oturumuKapat('o-1');
    await ana.baglantiAc('https://ornek.com/');
    expect(c).toEqual([
      ['oturum_baslat', { calisma_alani: 7, model: 'glm-5.3' }],
      ['oturum_ac', { oturum: 'o-1' }],
      ['paketi_isledim', { oturum: 'o-1', no: 3 }],
      ['gecmis_sayfasi', { oturum: 'o-1', once_sira: 120 }],
      ['mesaj_gonder', { oturum: 'o-1', metin: 'selam' }],
      ['turu_iptal_et', { oturum: 'o-1' }],
      ['arac_onayi', { oturum: 'o-1', kararlar: [{ cagri: 'c-1', muhur: 'm', karar: { tip: 'Onayla' } }] }],
      ['model_degistir', { oturum: 'o-1', model: 'auto' }],
      ['izin_kipi_degistir', { oturum: 'o-1', kip: 'Proje' }],
      ['oturumu_kapat', { oturum: 'o-1' }],
      ['baglanti_ac', { adres: 'https://ornek.com/' }],
    ]);
  });

  it('argümansız komutlar', async () => {
    const c = kaydet();
    await ana.uygulamaDurumu();
    await ana.oturumlariListele();
    await ana.calismaAlaniSec();
    await ana.anahtarPenceresiniAc();
    await ana.sartPenceresiniAc();
    expect(c.map(([komut]) => komut)).toEqual([
      'uygulama_durumu',
      'oturumlari_listele',
      'calisma_alani_sec',
      'anahtar_penceresini_ac',
      'sart_penceresini_ac',
    ]);
  });

  it('oturumu dinlerken bir kanal gönderir', async () => {
    const c = kaydet();
    await ana.oturumuDinle('o-1', () => {});
    expect(c[0][0]).toBe('oturumu_dinle');
    expect(c[0][1]?.oturum).toBe('o-1');
    expect(c[0][1]?.kanal).toBeInstanceOf(Channel);
  });
});

describe('şart penceresi komutları (1c tasarımı §6.1)', () => {
  it('komut adları ve argüman adları', async () => {
    const c = kaydet();
    await sartlar.sartPenceresiDurumu();
    await sartlar.anahtariKaydet('evren_llm_deneme');
    await sartlar.sartlariKabulEt(3);
    await sartlar.sartlariReddet();
    await sartlar.sartMetniniGoster();
    await sartlar.baglantiAc('https://ornek.com/');
    expect(c).toEqual([
      ['sart_penceresi_durumu', {}],
      ['anahtari_kaydet', { anahtar: 'evren_llm_deneme' }],
      ['sartlari_kabul_et', { surum: 3 }],
      ['sartlari_reddet', {}],
      ['sart_metnini_goster', {}],
      ['baglanti_ac', { adres: 'https://ornek.com/' }],
    ]);
  });
});

describe('komut hatası', () => {
  it('komutun döndüğü hata aynen gelir', () => {
    expect(komutHatasi({ kod: 'KayitMesgul', mesaj: 'açık' })).toEqual({
      kod: 'KayitMesgul',
      mesaj: 'açık',
    });
  });

  it("Tauri'nin düz metin hatası iç hata sayılır", () => {
    expect(komutHatasi('Command x not allowed by ACL')).toEqual({
      kod: 'Ic',
      mesaj: 'Command x not allowed by ACL',
    });
    expect(komutHatasi({ kod: 'Bilinmeyen', mesaj: 'x' }).kod).toBe('Ic');
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/ipc`
Expected: FAIL — `./ana`, `./sartlar`, `./hata` yok.

- [ ] **Step 3: Sarmalayıcıları yaz**

`ui/src/ipc/hata.ts`:

```ts
import type { HataKodu, KomutHatasi } from './tipler';

const KODLAR: readonly HataKodu[] = [
  'OturumYok',
  'KayitMesgul',
  'SaltOkunur',
  'GecersizGirdi',
  'Evren',
  'Ic',
];

/**
 * `invoke`'un reddettiği değeri `KomutHatasi`'na çevirir. Komutlar hatayı
 * `{ kod, mesaj }` olarak döner; Tauri'nin kendi hataları (ör. yetki reddi)
 * düz metin gelir ve `Ic` sayılır.
 */
export function komutHatasi(hata: unknown): KomutHatasi {
  if (typeof hata === 'object' && hata !== null) {
    const { kod, mesaj } = hata as { kod?: unknown; mesaj?: unknown };
    if (typeof mesaj === 'string' && KODLAR.includes(kod as HataKodu)) {
      return { kod: kod as HataKodu, mesaj };
    }
  }
  return { kod: 'Ic', mesaj: typeof hata === 'string' ? hata : String(hata) };
}
```

`ui/src/ipc/ana.ts`:

```ts
// Ana pencerenin komutları (1c tasarımı §6.1). Argüman adları Rust'taki
// adlardır: komutlar `rename_all = "snake_case"` ile tanımlıdır.
import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  CalismaAlani,
  GecmisSayfasi,
  KanalMesaji,
  Kip,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  UygulamaDurumu,
} from './tipler';

export const uygulamaDurumu = () => invoke<UygulamaDurumu>('uygulama_durumu');

export const oturumlariListele = () => invoke<OturumSatiri[]>('oturumlari_listele');

export const calismaAlaniSec = () => invoke<CalismaAlani | null>('calisma_alani_sec');

export const oturumBaslat = (calisma_alani: number, model: string) =>
  invoke<OturumAcilisi>('oturum_baslat', { calisma_alani, model });

export const oturumAc = (oturum: OturumId) => invoke<OturumAcilisi>('oturum_ac', { oturum });

/** Oturumun olaylarını dinler. Açılan oturum başına bir kez çağrılır. */
export function oturumuDinle(oturum: OturumId, dinleyici: (mesaj: KanalMesaji) => void) {
  const kanal = new Channel<KanalMesaji>(dinleyici);
  return invoke<void>('oturumu_dinle', { oturum, kanal });
}

export const paketiIsledim = (oturum: OturumId, no: number) =>
  invoke<void>('paketi_isledim', { oturum, no });

export const gecmisSayfasi = (oturum: OturumId, once_sira: number) =>
  invoke<GecmisSayfasi>('gecmis_sayfasi', { oturum, once_sira });

export const mesajGonder = (oturum: OturumId, metin: string) =>
  invoke<void>('mesaj_gonder', { oturum, metin });

export const turuIptalEt = (oturum: OturumId) => invoke<void>('turu_iptal_et', { oturum });

export const aracOnayi = (oturum: OturumId, kararlar: OnayGirdisi[]) =>
  invoke<void>('arac_onayi', { oturum, kararlar });

export const modelDegistir = (oturum: OturumId, model: string) =>
  invoke<void>('model_degistir', { oturum, model });

/** Yükseltmede yerel onay sorulur; reddedilirse eski kip döner. */
export const izinKipiDegistir = (oturum: OturumId, kip: Kip) =>
  invoke<Kip>('izin_kipi_degistir', { oturum, kip });

export const oturumuKapat = (oturum: OturumId) => invoke<void>('oturumu_kapat', { oturum });

export const baglantiAc = (adres: string) => invoke<void>('baglanti_ac', { adres });

export const anahtarPenceresiniAc = () => invoke<void>('anahtar_penceresini_ac');

export const sartPenceresiniAc = () => invoke<void>('sart_penceresini_ac');
```

`ui/src/ipc/sartlar.ts`:

```ts
// Şart penceresinin komutları (1c tasarımı §6.1). Bu modül ana pencerenin
// komutlarını içe aktarmaz; şart penceresinin paketi sohbet kodunu taşımaz.
import { invoke } from '@tauri-apps/api/core';
import type { AnahtarSonucu, KabulYaniti, SartMetni, SartPenceresiDurumu } from './tipler';

export const sartPenceresiDurumu = () => invoke<SartPenceresiDurumu>('sart_penceresi_durumu');

/** Anahtar bir kez Rust'a gider; hiçbir komut onu geri döndürmez. */
export const anahtariKaydet = (anahtar: string) =>
  invoke<AnahtarSonucu>('anahtari_kaydet', { anahtar });

export const sartlariKabulEt = (surum: number) =>
  invoke<KabulYaniti>('sartlari_kabul_et', { surum });

export const sartlariReddet = () => invoke<void>('sartlari_reddet');

export const sartMetniniGoster = () => invoke<SartMetni>('sart_metnini_goster');

export const baglantiAc = (adres: string) => invoke<void>('baglanti_ac', { adres });
```

- [ ] **Step 4: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: temiz; `ipc.test.ts` geçer.

- [ ] **Step 5: Commit**

```bash
git add ui/src/ipc
git commit -m "ui: iki pencerenin komut çağrıları"
git push
```

---

### Task 3: Oturum durumu işlevi

Kanal mesajını ve geçmiş sayfasını oturumun ekran durumuna uygulayan saf işlev (1c tasarımı §8.4). Kurallar: bitişik ve aynı turdaki metin parçaları tek yanıtta birleşir; araç çağrısı, onay, şart, hata ve tur sonu akışı keser; paket numarasında boşluk görülünce durum `bosluk` olur ve sonrası uygulanmaz; açılıştaki geçmişin son `sira`sından küçük ya da eşit `sira`lı canlı olay atlanır; eşi olmayan araç sonucu yok sayılır. Dinleyici "işledim"i mesajı uyguladığı anda bildirir, çizimi beklemez.

**Files:**
- Create: `ui/src/durum/oturum.ts`, `ui/src/durum/dinleyici.ts`
- Test: `ui/src/durum/oturum.test.ts`

**Interfaces:**
- Consumes: Görev 1'in tipleri.
- Produces: `durum/oturum.ts`: `Anlik = 'Bos' | 'Calisiyor' | 'OnayBekliyor' | 'SartBekliyor'`; `EkranOgesi` (`KullaniciMesaji`, `Yanit`, `AracCagrisi`, `OnayIstegi`, `SartIstegi`, `Hata`, `TurSonu`, `AktarimDurdu`; hepsi `anahtar`, `sira`, `tur` taşır); `OturumEkrani`; `Girdi` (`Kanal`, `EskiSayfa`, `MesajGonderildi`, `OnayYanitlandi`, `ModelDegisti`, `KipDegisti`); `gecmisOgesi(o: GorunumOgesi): EkranOgesi`; `acilistanEkran(acilis: OturumAcilisi): OturumEkrani`; `uygula(durum, girdi): OturumEkrani`. `durum/dinleyici.ts`: `kanalIsleyicisi(oku, yaz, isledim): (m: KanalMesaji) => void`.

- [ ] **Step 1: Testi yaz**

`ui/src/durum/oturum.test.ts`:

```ts
import { describe, expect, it, vi } from 'vitest';
import type { KanalMesaji, Olay, OturumAcilisi, Zarf } from '../ipc/tipler';
import { kanalIsleyicisi } from './dinleyici';
import { acilistanEkran, uygula, type EkranOgesi, type OturumEkrani } from './oturum';

const OTURUM = 'o-1';
const TUR = 't-1';

function acilis(gecmis: OturumAcilisi['gecmis'] = { ogeler: [], daha_eski_var: false }) {
  return acilistanEkran({
    oturum: OTURUM,
    calisma_alani: 'C:\\proje',
    gecmis,
    ozet: {
      model: 'glm-5.3',
      kip: 'Siki',
      yonlendirilen_model: null,
      son_kredi: null,
      istem: 0,
      tamamlama: 0,
      dusunce: 0,
    },
    salt_okunur: null,
  });
}

let sira = 100;
function zarf(yuk: Olay, tur: string | null = TUR): Zarf<Olay> {
  sira += 1;
  return { sema: 1, oturum: OTURUM, tur, sira, yuk };
}

function paket(no: number, ...yukler: Olay[]): KanalMesaji {
  return { tip: 'Olaylar', no, zarflar: yukler.map((y) => zarf(y)) };
}

function onayGerekiyor(islem: string, cagri: string): Extract<Olay, { tip: 'OnayGerekiyor' }> {
  return {
    tip: 'OnayGerekiyor',
    islem,
    ogeler: [
      {
        cagri,
        arac: 'dosya_yaz',
        aciklama: 'a.rs yazılır',
        hedefler: ['a.rs'],
        diff: null,
        uyarilar: [],
        muhur: 'm',
      },
    ],
  };
}

function tipler(d: OturumEkrani): EkranOgesi['tip'][] {
  return d.ogeler.map((o) => o.tip);
}

describe('kanal mesajı → oturum durumu (1c tasarımı §8.4)', () => {
  it('mesaj, tur ve bitişik metin parçaları tek yanıtta birleşir', () => {
    let d = uygula(acilis(), { tip: 'MesajGonderildi', metin: 'selam' });
    expect(d.anlik).toBe('Calisiyor');
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'TokenParcasi', metin: 'Mer' },
        { tip: 'TokenParcasi', metin: 'haba' },
      ),
    });
    expect(tipler(d)).toEqual(['KullaniciMesaji', 'Yanit']);
    expect(d.ogeler[0]).toMatchObject({ metin: 'selam', tur: TUR });
    expect(d.ogeler[1]).toMatchObject({ metin: 'Merhaba', akista: true });
    expect(d.aktif_tur).toBe(TUR);
    expect(d.beklenen_no).toBe(2);
  });

  it('düşünce ve metin aynı yanıtta ayrı tutulur; araç çağrısı akışı keser', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'DusunceParcasi', metin: 'düşün' },
        { tip: 'TokenParcasi', metin: 'yaz' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-1', arac: 'dosya_oku', aciklama: 'a.rs' },
        { tip: 'TokenParcasi', metin: 'sonra' },
      ),
    });
    expect(tipler(d)).toEqual(['Yanit', 'AracCagrisi', 'Yanit']);
    expect(d.ogeler[0]).toMatchObject({ dusunce: 'düşün', metin: 'yaz', akista: false });
    expect(d.ogeler[1]).toMatchObject({ cagri: 'c-1', durum: 'Suruyor' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, {
        tip: 'AracSonucu',
        cagri: 'c-1',
        arac: 'dosya_oku',
        durum: 'Basarili',
        icerik: 'içerik',
      }),
    });
    expect(d.ogeler[1]).toMatchObject({ durum: 'Basarili', sonuc: 'içerik' });
  });

  it('onay isteği kart olur; yanıtlanınca çalışmaya döner', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        {
          tip: 'OnayGerekiyor',
          islem: 'i-1',
          ogeler: [
            {
              cagri: 'c-1',
              arac: 'dosya_yaz',
              aciklama: 'a.rs yazılır',
              hedefler: ['a.rs'],
              diff: null,
              uyarilar: [],
              muhur: 'm',
            },
          ],
        },
      ),
    });
    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-1', durum: 'Bekliyor' });
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler.at(-1)).toMatchObject({ durum: 'Yanitlandi' });
  });

  it('önceki onayın gecikmiş yanıtı yeni bekleyen isteği temizlemez', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, onayGerekiyor('i-1', 'c-1')),
    });
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, onayGerekiyor('i-2', 'c-2')),
    });
    expect(d.anlik).toBe('OnayBekliyor');

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });

    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-2', durum: 'Bekliyor' });
  });

  it('eski onay kartı yanıtlanırken yeni bekleyen istek durumu korur', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, onayGerekiyor('i-1', 'c-1')),
    });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, onayGerekiyor('i-2', 'c-2')),
    });

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });

    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.find((o) => o.tip === 'OnayIstegi' && o.islem === 'i-1')).toMatchObject({
      durum: 'Yanitlandi',
    });
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-2', durum: 'Bekliyor' });
  });

  it('tur sonu akışı dondurur, bekleyen onayı geçersiz kılar', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'TokenParcasi', metin: 'x' },
        { tip: 'OnayGerekiyor', islem: 'i-1', ogeler: [] },
        { tip: 'Tamamlandi', neden: { tip: 'Iptal' } },
      ),
    });
    expect(tipler(d)).toEqual(['Yanit', 'OnayIstegi', 'TurSonu']);
    expect(d.ogeler[0]).toMatchObject({ akista: false });
    expect(d.ogeler[1]).toMatchObject({ durum: 'Gecersiz' });
    expect(d.anlik).toBe('Bos');
    expect(d.aktif_tur).toBeNull();
  });

  it('şart beklerken gelen olay beklemeyi çözer', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'SartOnayiGerekiyor', surum: 3, metin: '…' }),
    });
    expect(d.anlik).toBe('SartBekliyor');
    d = uygula(d, { tip: 'Kanal', mesaj: paket(2, { tip: 'TokenParcasi', metin: 'devam' }) });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler.find((o) => o.tip === 'SartIstegi')).toMatchObject({ durum: 'Cozuldu' });
  });

  it('kullanım, kredi ve yönlendirilen model durum özetine işlenir', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'IstekBilgisi', istek_id: 'r-1', yonlendirilen_model: 'deepseek-v4-flash' },
        { tip: 'TokenKullanimi', istem: 10, tamamlama: 5, dusunce: 2 },
        { tip: 'TokenKullanimi', istem: 1, tamamlama: 1, dusunce: 0 },
        {
          tip: 'KrediGozlemi',
          gozlem: { kalan: '12.5', tutulan: null, istek_id: 'r-1', zaman_ms: 1 },
        },
      ),
    });
    expect(d.ozet).toMatchObject({
      yonlendirilen_model: 'deepseek-v4-flash',
      istem: 11,
      tamamlama: 6,
      dusunce: 2,
    });
    expect(d.ozet.son_kredi?.kalan).toBe('12.5');
  });

  it('aktarım durunca uyarı öğesi eklenir ve oturum boşa düşer', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' }),
    });
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' });
    expect(d.anlik).toBe('Bos');
  });

  it('paket numarasında boşluk hata olarak işaretlenir; sonrası uygulanmaz', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }) });
    d = uygula(d, { tip: 'Kanal', mesaj: paket(3, { tip: 'TokenParcasi', metin: 'x' }) });
    expect(d.bosluk).toBe(true);
    expect(tipler(d)).toEqual([]);
    const sonra = uygula(d, { tip: 'Kanal', mesaj: paket(4, { tip: 'TokenParcasi', metin: 'y' }) });
    expect(sonra).toBe(d);
  });

  it('kapanış mesajından sonra oturum kapalıdır', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }) });
    d = uygula(d, { tip: 'Kanal', mesaj: { tip: 'Kapandi', no: 2 } });
    expect(d.kapandi).toBe(true);
    expect(d.anlik).toBe('Bos');
  });

  it('eski durum değişmez', () => {
    const once = uygula(acilis(), { tip: 'MesajGonderildi', metin: 'a' });
    const kopya = structuredClone(once);
    uygula(once, { tip: 'Kanal', mesaj: paket(1, { tip: 'TokenParcasi', metin: 'b' }) });
    expect(once).toEqual(kopya);
  });
});

describe('geçmiş ve canlı olaylar (1c tasarımı §5.5)', () => {
  const gecmis: OturumAcilisi['gecmis'] = {
    ogeler: [
      { tip: 'KullaniciMesaji', sira: 10, tur: 't-0', metin: 'önce' },
      { tip: 'Yanit', sira: 12, tur: 't-0', metin: 'yanıt', dusunce: null },
      {
        tip: 'AracCagrisi',
        sira: 12,
        tur: 't-0',
        model_cagri_id: 'm-1',
        arac: 'dosya_yaz',
        argumanlar: '{}',
        durum: 'SonucuBilinmiyor',
        sonuc: null,
      },
      { tip: 'TurSonu', sira: 15, tur: 't-0', neden: { tip: 'Normal' } },
    ],
    daha_eski_var: true,
  };

  it('geçmiş ekran öğelerine çevrilir; anahtarlar benzersiz', () => {
    const d = acilis(gecmis);
    expect(tipler(d)).toEqual(['KullaniciMesaji', 'Yanit', 'AracCagrisi', 'TurSonu']);
    expect(new Set(d.ogeler.map((o) => o.anahtar)).size).toBe(4);
    expect(d.gecmis_son_sira).toBe(15);
    expect(d.daha_eski_var).toBe(true);
  });

  it('geçmişte zaten olan sıradaki canlı olay atlanır', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'Kanal',
      mesaj: {
        tip: 'Olaylar',
        no: 1,
        zarflar: [
          { sema: 1, oturum: OTURUM, tur: null, sira: 14, yuk: { tip: 'Hata', mesaj: 'eski' } },
          { sema: 1, oturum: OTURUM, tur: null, sira: 16, yuk: { tip: 'Hata', mesaj: 'yeni' } },
        ],
      },
    });
    expect(d.ogeler.filter((o) => o.tip === 'Hata')).toMatchObject([{ mesaj: 'yeni' }]);
  });

  it('eşi olmayan araç sonucu yok sayılır', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'AracSonucu',
        cagri: 'bilinmeyen',
        arac: 'dosya_yaz',
        durum: 'SonucuBilinmiyor',
        icerik: '[sonucu bilinmiyor]',
      }),
    });
    expect(d.ogeler).toHaveLength(4);
  });

  it('eski sayfa başa eklenir; tekrar eden öğe eklenmez', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'EskiSayfa',
      sayfa: {
        ogeler: [
          { tip: 'KullaniciMesaji', sira: 2, tur: 't-a', metin: 'en eski' },
          { tip: 'KullaniciMesaji', sira: 10, tur: 't-0', metin: 'önce' },
        ],
        daha_eski_var: false,
      },
    });
    expect(d.ogeler.map((o) => o.sira)).toEqual([2, 10, 12, 12, 15]);
    expect(d.daha_eski_var).toBe(false);
  });
});

describe('kanal dinleyicisi', () => {
  it('"işledim"i uygular uygulamaz bildirir; çizimi beklemez (1c tasarımı §4.3)', () => {
    let durum = acilis();
    const sirasi: string[] = [];
    const isledim = vi.fn((no: number) => sirasi.push(`isledim ${no}`));
    const dinle = kanalIsleyicisi(
      () => durum,
      (d) => {
        sirasi.push('yaz');
        durum = d;
      },
      isledim,
    );
    dinle(paket(1, { tip: 'TurBasladi' }));
    expect(sirasi).toEqual(['yaz', 'isledim 1']);
  });

  it('boşlukta ve kapanışta "işledim" gönderilmez', () => {
    let durum = acilis();
    const isledim = vi.fn();
    const dinle = kanalIsleyicisi(
      () => durum,
      (d) => (durum = d),
      isledim,
    );
    dinle(paket(2, { tip: 'TurBasladi' }));
    expect(isledim).not.toHaveBeenCalled();
    durum = acilis();
    dinle({ tip: 'Kapandi', no: 1 });
    expect(isledim).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/durum`
Expected: FAIL — `./oturum` ve `./dinleyici` yok.

- [ ] **Step 3: İşlevi yaz**

`ui/src/durum/oturum.ts`:

```ts
// Bir oturumun ekran durumu ve onu güncelleyen saf işlev (1c tasarımı §8.4).
// `uygula` DOM'a dokunmaz; eski durumu değiştirmez, yenisini döner. Svelte
// yalnızca bu durumu çizer.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  BitisNedeni,
  CagriId,
  DurumOzeti,
  GecmisSayfasi,
  GorunumOgesi,
  IslemId,
  KanalMesaji,
  Kip,
  Olay,
  OnayOgesi,
  OturumAcilisi,
  OturumId,
  SaltOkunurNedeni,
  TurId,
  Zarf,
} from '../ipc/tipler';

/** Oturumun o anki hâli; oturum listesindeki nokta ve Composer buna bakar. */
export type Anlik = 'Bos' | 'Calisiyor' | 'OnayBekliyor' | 'SartBekliyor';

interface Ortak {
  /** Svelte `each` bloğunun anahtarı; öğe var oldukça değişmez. */
  anahtar: string;
  sira: number;
  tur: TurId | null;
}

export type EkranOgesi =
  | (Ortak & { tip: 'KullaniciMesaji'; metin: string })
  | (Ortak & { tip: 'Yanit'; metin: string; dusunce: string; akista: boolean })
  | (Ortak & {
      tip: 'AracCagrisi';
      /** Kayıttan gelen çağrıda yok. */
      cagri: CagriId | null;
      arac: string;
      aciklama: string | null;
      argumanlar: string | null;
      /** `null`: çağrı hiç başlamadı. */
      durum: AracDurumu | 'Suruyor' | null;
      sonuc: string | null;
    })
  | (Ortak & {
      tip: 'OnayIstegi';
      islem: IslemId;
      ogeler: OnayOgesi[];
      durum: 'Bekliyor' | 'Yanitlandi' | 'Gecersiz';
    })
  | (Ortak & { tip: 'SartIstegi'; surum: number; durum: 'Bekliyor' | 'Cozuldu' })
  | (Ortak & { tip: 'Hata'; mesaj: string })
  | (Ortak & { tip: 'TurSonu'; neden: BitisNedeni })
  | (Ortak & { tip: 'AktarimDurdu'; neden: AktarimDurmaNedeni });

export interface OturumEkrani {
  oturum: OturumId;
  calisma_alani: string | null;
  salt_okunur: SaltOkunurNedeni | null;
  ogeler: EkranOgesi[];
  daha_eski_var: boolean;
  ozet: DurumOzeti;
  anlik: Anlik;
  aktif_tur: TurId | null;
  /** Beklenen paket numarası; abonelik 1'den başlar (1c tasarımı §4.4). */
  beklenen_no: number;
  /**
   * Açılışta okunan geçmişin son `sira`sı. Bundan küçük ya da eşit `sira`lı
   * canlı olayın etkisi geçmişte zaten vardır; atlanır.
   */
  gecmis_son_sira: number;
  /** Olay kuyruğu kapandı; bu abonelikten başka mesaj gelmez. */
  kapandi: boolean;
  /** Paket numarasında boşluk görüldü: ekran oturumu kayıttan yeniden açar. */
  bosluk: boolean;
  /** Canlı yerel öğelerin anahtarı için sayaç. */
  yerel_sayac: number;
}

export type Girdi =
  | { tip: 'Kanal'; mesaj: KanalMesaji }
  | { tip: 'EskiSayfa'; sayfa: GecmisSayfasi }
  | { tip: 'MesajGonderildi'; metin: string }
  | { tip: 'OnayYanitlandi'; islem: IslemId }
  | { tip: 'ModelDegisti'; model: string }
  | { tip: 'KipDegisti'; kip: Kip };

/** Kayıttan gelen bir geçmiş öğesini ekran öğesine çevirir. */
export function gecmisOgesi(o: GorunumOgesi): EkranOgesi {
  switch (o.tip) {
    case 'KullaniciMesaji':
      return { ...o, anahtar: `g-${o.sira}-k` };
    case 'Yanit':
      return {
        tip: 'Yanit',
        anahtar: `g-${o.sira}-y`,
        sira: o.sira,
        tur: o.tur,
        metin: o.metin ?? '',
        dusunce: o.dusunce ?? '',
        akista: false,
      };
    case 'AracCagrisi':
      return {
        tip: 'AracCagrisi',
        anahtar: `g-${o.sira}-a-${o.model_cagri_id}`,
        sira: o.sira,
        tur: o.tur,
        cagri: null,
        arac: o.arac,
        aciklama: null,
        argumanlar: o.argumanlar,
        durum: o.durum,
        sonuc: o.sonuc,
      };
    case 'TurSonu':
      return { ...o, anahtar: `g-${o.sira}-s` };
  }
}

/** `oturum_baslat` ya da `oturum_ac`'ın sonucundan ilk ekran durumu. */
export function acilistanEkran(acilis: OturumAcilisi): OturumEkrani {
  const ogeler = acilis.gecmis.ogeler.map(gecmisOgesi);
  return {
    oturum: acilis.oturum,
    calisma_alani: acilis.calisma_alani,
    salt_okunur: acilis.salt_okunur,
    ogeler,
    daha_eski_var: acilis.gecmis.daha_eski_var,
    ozet: acilis.ozet,
    anlik: 'Bos',
    aktif_tur: null,
    beklenen_no: 1,
    gecmis_son_sira: ogeler.reduce((en, o) => Math.max(en, o.sira), 0),
    kapandi: false,
    bosluk: false,
    yerel_sayac: 0,
  };
}

export function uygula(durum: OturumEkrani, girdi: Girdi): OturumEkrani {
  switch (girdi.tip) {
    case 'Kanal':
      return kanalUygula(durum, girdi.mesaj);
    case 'EskiSayfa': {
      const enEski = durum.ogeler[0]?.sira ?? Number.POSITIVE_INFINITY;
      const eskiler = girdi.sayfa.ogeler.filter((o) => o.sira < enEski).map(gecmisOgesi);
      return {
        ...durum,
        ogeler: [...eskiler, ...durum.ogeler],
        daha_eski_var: girdi.sayfa.daha_eski_var,
      };
    }
    case 'MesajGonderildi': {
      const d = kopya(durum);
      d.ogeler.push({
        tip: 'KullaniciMesaji',
        anahtar: `y-${d.yerel_sayac}`,
        sira: sonSira(d),
        tur: null,
        metin: girdi.metin,
      });
      d.yerel_sayac += 1;
      d.anlik = 'Calisiyor';
      return d;
    }
    case 'OnayYanitlandi': {
      const d = kopya(durum);
      let eslesenIstekGuncellendi = false;
      d.ogeler = d.ogeler.map((o) => {
        if (o.tip === 'OnayIstegi' && o.islem === girdi.islem && o.durum === 'Bekliyor') {
          eslesenIstekGuncellendi = true;
          return { ...o, durum: 'Yanitlandi' };
        }
        return o;
      });
      const bekleyenOnayVar = d.ogeler.some((o) => o.tip === 'OnayIstegi' && o.durum === 'Bekliyor');
      if (eslesenIstekGuncellendi && d.anlik === 'OnayBekliyor' && !bekleyenOnayVar) {
        d.anlik = 'Calisiyor';
      }
      return d;
    }
    case 'ModelDegisti':
      return { ...durum, ozet: { ...durum.ozet, model: girdi.model } };
    case 'KipDegisti':
      return { ...durum, ozet: { ...durum.ozet, kip: girdi.kip } };
  }
}

function kanalUygula(durum: OturumEkrani, mesaj: KanalMesaji): OturumEkrani {
  if (durum.bosluk || durum.kapandi) return durum;
  if (mesaj.no !== durum.beklenen_no) return { ...durum, bosluk: true };
  const d = kopya(durum);
  d.beklenen_no += 1;
  if (mesaj.tip === 'Kapandi') {
    akisiKapat(d);
    bekleyenleriKapat(d);
    d.kapandi = true;
    d.anlik = 'Bos';
    d.aktif_tur = null;
    return d;
  }
  for (const zarf of mesaj.zarflar) olayUygula(d, zarf);
  return d;
}

/** Tek bir olayı çalışma kopyasına uygular; `d` bu paket için yeni bir nesnedir. */
function olayUygula(d: OturumEkrani, z: Zarf<Olay>): void {
  if (z.sira <= d.gecmis_son_sira) return;
  const yuk = z.yuk;
  if (d.anlik === 'SartBekliyor' && yuk.tip !== 'SartOnayiGerekiyor') sartiCoz(d);
  switch (yuk.tip) {
    case 'TurBasladi':
      d.anlik = 'Calisiyor';
      d.aktif_tur = z.tur;
      // Gönderilen mesaj turun kimliğini henüz bilmiyordu.
      for (let i = d.ogeler.length - 1; i >= 0; i--) {
        const o = d.ogeler[i];
        if (o.tip === 'KullaniciMesaji' && o.tur === null) {
          d.ogeler[i] = { ...o, tur: z.tur };
          break;
        }
      }
      return;
    case 'TokenParcasi':
      akisaEkle(d, z, 'metin', yuk.metin);
      return;
    case 'DusunceParcasi':
      akisaEkle(d, z, 'dusunce', yuk.metin);
      return;
    case 'AracCagrisiBasladi':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'AracCagrisi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        cagri: yuk.cagri,
        arac: yuk.arac,
        aciklama: yuk.aciklama,
        argumanlar: null,
        durum: 'Suruyor',
        sonuc: null,
      });
      return;
    case 'AracSonucu': {
      // Eşi olmayan sonuç (kurtarmadaki bilinmeyen çağrı) geçmişte zaten görünür.
      const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === yuk.cagri);
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, durum: yuk.durum, sonuc: yuk.icerik };
      return;
    }
    case 'OnayGerekiyor':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'OnayIstegi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        islem: yuk.islem,
        ogeler: yuk.ogeler,
        durum: 'Bekliyor',
      });
      d.anlik = 'OnayBekliyor';
      return;
    case 'SartOnayiGerekiyor':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'SartIstegi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        surum: yuk.surum,
        durum: 'Bekliyor',
      });
      d.anlik = 'SartBekliyor';
      return;
    case 'KrediGozlemi':
      d.ozet = { ...d.ozet, son_kredi: yuk.gozlem };
      return;
    case 'TokenKullanimi':
      d.ozet = {
        ...d.ozet,
        istem: d.ozet.istem + yuk.istem,
        tamamlama: d.ozet.tamamlama + yuk.tamamlama,
        dusunce: d.ozet.dusunce + yuk.dusunce,
      };
      return;
    case 'IstekBilgisi':
      if (yuk.yonlendirilen_model !== null) {
        d.ozet = { ...d.ozet, yonlendirilen_model: yuk.yonlendirilen_model };
      }
      return;
    case 'Hata':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'Hata',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        mesaj: yuk.mesaj,
      });
      return;
    case 'Tamamlandi':
      akisiKapat(d);
      bekleyenleriKapat(d);
      d.ogeler.push({
        tip: 'TurSonu',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        neden: yuk.neden,
      });
      d.anlik = 'Bos';
      d.aktif_tur = null;
      return;
    case 'AktarimDurdu':
      akisiKapat(d);
      bekleyenleriKapat(d);
      d.ogeler.push({
        tip: 'AktarimDurdu',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        neden: yuk.neden,
      });
      d.anlik = 'Bos';
      d.aktif_tur = null;
      return;
  }
}

/** Bitişik ve aynı turdaki metin parçaları tek yanıtta birleşir. */
function akisaEkle(d: OturumEkrani, z: Zarf<Olay>, alan: 'metin' | 'dusunce', metin: string) {
  const son = d.ogeler.at(-1);
  if (son?.tip === 'Yanit' && son.akista && son.tur === z.tur) {
    d.ogeler[d.ogeler.length - 1] = { ...son, [alan]: son[alan] + metin };
    return;
  }
  d.ogeler.push({
    tip: 'Yanit',
    anahtar: `c-${z.sira}`,
    sira: z.sira,
    tur: z.tur,
    metin: alan === 'metin' ? metin : '',
    dusunce: alan === 'dusunce' ? metin : '',
    akista: true,
  });
}

/** Akan yanıtı dondurur: tamamlanan mesaj bir kez çizilir (1c tasarımı §9.2). */
function akisiKapat(d: OturumEkrani) {
  const son = d.ogeler.at(-1);
  if (son?.tip === 'Yanit' && son.akista) d.ogeler[d.ogeler.length - 1] = { ...son, akista: false };
}

/** Tur bittiğinde karar bekleyen kartlar artık karar almaz. */
function bekleyenleriKapat(d: OturumEkrani) {
  d.ogeler = d.ogeler.map((o) => {
    if (o.tip === 'OnayIstegi' && o.durum === 'Bekliyor') return { ...o, durum: 'Gecersiz' };
    if (o.tip === 'SartIstegi' && o.durum === 'Bekliyor') return { ...o, durum: 'Cozuldu' };
    return o;
  });
}

/** Şart kararı verildi: tur olay üretmeye devam ediyor ya da bitiyor. */
function sartiCoz(d: OturumEkrani) {
  d.ogeler = d.ogeler.map((o) =>
    o.tip === 'SartIstegi' && o.durum === 'Bekliyor' ? { ...o, durum: 'Cozuldu' } : o,
  );
  d.anlik = 'Calisiyor';
}

function sonSira(d: OturumEkrani): number {
  return d.ogeler.reduce((en, o) => Math.max(en, o.sira), d.gecmis_son_sira);
}

function kopya(d: OturumEkrani): OturumEkrani {
  return { ...d, ogeler: [...d.ogeler] };
}
```

`ui/src/durum/dinleyici.ts`:

```ts
import type { KanalMesaji } from '../ipc/tipler';
import { uygula, type OturumEkrani } from './oturum';

/**
 * Kanal dinleyicisi: mesajı duruma uygular ve "işledim"i hemen bildirir.
 * Bildirim çizimi beklemez (1c tasarımı §4.3): küçültülmüş pencere çizim
 * yapmasa da paketleri işlemeyi sürdürür, tur iptal edilmez. Boşluk görülen
 * ya da kapanan abonelikte bildirim gönderilmez.
 */
export function kanalIsleyicisi(
  oku: () => OturumEkrani,
  yaz: (durum: OturumEkrani) => void,
  isledim: (no: number) => void,
): (mesaj: KanalMesaji) => void {
  return (mesaj) => {
    const yeni = uygula(oku(), { tip: 'Kanal', mesaj });
    yaz(yeni);
    if (mesaj.tip === 'Olaylar' && !yeni.bosluk) isledim(mesaj.no);
  };
}
```

- [ ] **Step 4: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: temiz; `oturum.test.ts`'deki 16 test geçer.

- [ ] **Step 5: Commit**

```bash
git add ui/src/durum
git commit -m "ui: kanal mesajını oturum durumuna uygulayan saf işlev"
git push
```

---

### Task 4: Temel ve form bileşenleri

Tasarım sisteminin temel (Badge, Button, Card, Icon, IconButton, IconTile) ve form (Checkbox, Input, Select, Textarea) bileşenleri. React'teki fare durumları (`useState` ile hover/press) CSS `:hover`/`:active` olur; renk tonları sınıf ve CSS değişkeniyle verilir. Checkbox gerçek bir onay kutusunun üstüne çizilir; Select klavyeyle kullanılabilen bir combobox'tır (odak düğmede kalır, `aria-activedescendant`).

**Files:**
- Create: `ui/src/bilesenler/ikonlar.ts`, `Icon.svelte`, `Badge.svelte`, `Button.svelte`, `Card.svelte`, `IconButton.svelte`, `IconTile.svelte`, `Checkbox.svelte`, `Input.svelte`, `Select.svelte`, `Textarea.svelte` (hepsi `ui/src/bilesenler/` altında)
- Test: `ui/src/bilesenler/temel.test.ts`

**Interfaces:**
- Produces: `ikonlar.ts`: `IKONLAR`, `IkonAdi` (tasarım sistemindeki kebab-case adlar; `circle-help` Lucide 1.47'de `CircleQuestionMark`). Bileşenlerin prop'ları tasarım sistemindeki `.d.ts`'lerdeki gibidir; farklar: `style` yerine `class`; `children`/`actions`/`footer` Svelte snippet'idir; `Select` seçenekleri `{ value, label, hint? }` (`Secenek`, modül betiğinden dışa aktarılır) ve zorunlu `label` (erişilebilir ad) alır, değişiklik `onchange(value)`; `Checkbox` değişikliği `onchange(sonraki: boolean)`; `Input` ve `Textarea` `value`'yu `$bindable` olarak taşır; `Textarea` `element` ile metin alanını dışarı bağlar.

- [ ] **Step 1: Testi yaz**

`ui/src/bilesenler/temel.test.ts`:

```ts
import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Button from './Button.svelte';
import Checkbox from './Checkbox.svelte';
import IconButton from './IconButton.svelte';
import Select from './Select.svelte';

const metin = (m: string) => createRawSnippet(() => ({ render: () => `<span>${m}</span>` }));

const MODELLER = [
  { value: 'glm-5.3', label: 'Derin', hint: 'glm-5.3' },
  { value: 'auto', label: 'Otomatik' },
];
const KIPLER = [
  { value: 'Siki', label: 'Sıkı' },
  { value: 'Serbest', label: 'Serbest' },
];

describe('temel bileşenler', () => {
  it('Button tıklanır; yüklenirken ve kapalıyken tıklanmaz', async () => {
    const onclick = vi.fn();
    const { rerender } = render(Button, { props: { onclick, children: metin('Kaydet') } });
    const dugme = screen.getByRole<HTMLButtonElement>('button', { name: 'Kaydet' });
    await fireEvent.click(dugme);
    expect(onclick).toHaveBeenCalledOnce();
    await rerender({ onclick, loading: true, children: metin('Kaydet') });
    expect(dugme.disabled).toBe(true);
    expect(dugme.getAttribute('aria-busy')).toBe('true');
  });

  it('IconButton etiketini erişilebilir ad olarak taşır', () => {
    render(IconButton, { props: { icon: 'x', label: 'Kapat', active: true } });
    const dugme = screen.getByRole('button', { name: 'Kapat' });
    expect(dugme.getAttribute('title')).toBe('Kapat');
    expect(dugme.getAttribute('aria-pressed')).toBe('true');
  });

  it('Checkbox gerçek onay kutusudur; karışık durumu bildirir', async () => {
    const onchange = vi.fn();
    render(Checkbox, { props: { label: 'a.rs', indeterminate: true, onchange } });
    const kutu = screen.getByRole<HTMLInputElement>('checkbox', { name: 'a.rs' });
    expect(kutu.indeterminate).toBe(true);
    await fireEvent.click(kutu);
    expect(onchange).toHaveBeenCalledWith(true);
  });
});

describe('Select', () => {
  it('klavyeyle açılır, okla gezilir, Enter ile seçilir', async () => {
    const onchange = vi.fn();
    render(Select, { props: { label: 'Model', value: 'glm-5.3', options: MODELLER, onchange } });
    const tetik = screen.getByRole('combobox', { name: 'Model' });
    expect(tetik.textContent).toContain('Derin');
    await fireEvent.keyDown(tetik, { key: 'ArrowDown' });
    expect(tetik.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getAllByRole('option')).toHaveLength(2);
    await fireEvent.keyDown(tetik, { key: 'ArrowDown' });
    await fireEvent.keyDown(tetik, { key: 'Enter' });
    expect(onchange).toHaveBeenCalledWith('auto');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('Escape kapatır, seçim değişmez', async () => {
    const onchange = vi.fn();
    render(Select, { props: { label: 'Kip', value: 'Siki', options: KIPLER, onchange } });
    const tetik = screen.getByRole('combobox', { name: 'Kip' });
    await fireEvent.click(tetik);
    await fireEvent.keyDown(tetik, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(onchange).not.toHaveBeenCalled();
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/bilesenler/temel.test.ts`
Expected: FAIL — bileşen dosyaları yok.

- [ ] **Step 3: İkon kaydını ve temel bileşenleri yaz**

`ui/src/bilesenler/ikonlar.ts`:

```ts
// Kalfa'nın kullandığı Lucide ikonları. Yalnızca buradakiler pakete girer;
// yeni bir ikon gerekince buraya eklenir. Adlar tasarım sistemindeki gibi
// kebab-case'tir.
import {
  AtSign,
  Ban,
  Box,
  Check,
  ChevronDown,
  ChevronRight,
  CircleCheck,
  CircleQuestionMark,
  CircleStop,
  CircleX,
  Code,
  Coins,
  Cpu,
  ExternalLink,
  FilePen,
  FileSearch,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  Hexagon,
  Info,
  KeyRound,
  List,
  LoaderCircle,
  Lock,
  MessageSquare,
  Minus,
  Plus,
  RotateCcw,
  Search,
  Send,
  Shield,
  Square,
  TriangleAlert,
  User,
  WifiOff,
  X,
} from '@lucide/svelte';

export const IKONLAR = {
  'at-sign': AtSign,
  ban: Ban,
  box: Box,
  check: Check,
  'chevron-down': ChevronDown,
  'chevron-right': ChevronRight,
  'circle-check': CircleCheck,
  'circle-help': CircleQuestionMark,
  'circle-stop': CircleStop,
  'circle-x': CircleX,
  code: Code,
  coins: Coins,
  cpu: Cpu,
  'external-link': ExternalLink,
  'file-pen': FilePen,
  'file-search': FileSearch,
  'file-text': FileText,
  folder: Folder,
  'folder-open': FolderOpen,
  'folder-plus': FolderPlus,
  hexagon: Hexagon,
  info: Info,
  'key-round': KeyRound,
  list: List,
  'loader-circle': LoaderCircle,
  lock: Lock,
  'message-square': MessageSquare,
  minus: Minus,
  plus: Plus,
  'rotate-ccw': RotateCcw,
  search: Search,
  send: Send,
  shield: Shield,
  square: Square,
  'triangle-alert': TriangleAlert,
  user: User,
  'wifi-off': WifiOff,
  x: X,
} as const;

export type IkonAdi = keyof typeof IKONLAR;
```

`ui/src/bilesenler/Icon.svelte`:

```svelte
<!-- Lucide ikonu. Kalfa kendi glifini çizmez (tasarım sistemi: Icon). -->
<script lang="ts">
  import { IKONLAR, type IkonAdi } from './ikonlar';

  interface Props {
    name: IkonAdi;
    /** 14 satır içi, 16 varsayılan, 18 menü, 20 başlık. */
    size?: number;
    strokeWidth?: number;
    /** Erişilebilir ad; süs ikonlarında verilmez. */
    title?: string;
    class?: string;
  }

  let { name, size = 16, strokeWidth = 1.75, title, class: sinif = '' }: Props = $props();

  const Bilesen = $derived(IKONLAR[name]);
  const siniflar = $derived(sinif ? `ikon ${sinif}` : 'ikon');
</script>

{#if title}
  <Bilesen {size} {strokeWidth} class={siniflar} role="img" aria-label={title} />
{:else}
  <Bilesen {size} {strokeWidth} class={siniflar} aria-hidden="true" />
{/if}
```

`ui/src/bilesenler/Badge.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
    icon?: IkonAdi;
    /** İkon yerine dolu bir durum noktası. */
    dot?: boolean;
    shape?: 'pill' | 'square';
    class?: string;
    children?: Snippet;
  }

  let { tone = 'neutral', icon, dot = false, shape = 'pill', class: sinif, children }: Props =
    $props();
</script>

<span class={['rozet', `ton-${tone}`, { kare: shape === 'square' }, sinif]}>
  {#if dot}<span class="nokta"></span>{/if}
  {#if icon}<Icon name={icon} size={12} />{/if}
  {@render children?.()}
</span>

<style>
  .rozet {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border-radius: var(--radius-pill);
    font: var(--fw-medium) var(--text-micro) / 1 var(--font-sans);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
    white-space: nowrap;
  }
  .kare {
    border-radius: var(--radius-sm);
  }
  .nokta {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--fg);
    flex: none;
  }
  .ton-neutral {
    --fg: var(--text-secondary);
    --bg: color-mix(in oklab, var(--warm-200) 10%, transparent);
    --bd: color-mix(in oklab, var(--warm-200) 22%, transparent);
  }
  .ton-accent {
    --fg: var(--text-accent);
    --bg: var(--surface-selected);
    --bd: color-mix(in oklab, var(--copper-400) 45%, transparent);
  }
  .ton-ok {
    --fg: var(--status-ok);
    --bg: color-mix(in oklab, var(--green-400) 14%, transparent);
    --bd: color-mix(in oklab, var(--green-400) 38%, transparent);
  }
  .ton-warn {
    --fg: var(--status-warn);
    --bg: color-mix(in oklab, var(--amber-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--amber-200) 38%, transparent);
  }
  .ton-danger {
    --fg: var(--status-danger);
    --bg: color-mix(in oklab, var(--red-400) 14%, transparent);
    --bd: color-mix(in oklab, var(--red-400) 38%, transparent);
  }
  .ton-info {
    --fg: var(--status-info);
    --bg: color-mix(in oklab, var(--blue-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--blue-200) 38%, transparent);
  }
</style>
```

`ui/src/bilesenler/Button.svelte`:

```svelte
<!--
  Kalfa'nın eylem düğmesi. İzin kararları approve/caution/danger üçlüsünü
  kullanır; karar hiçbir zaman sıradan bir düğme gibi görünmez.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    variant?: 'primary' | 'secondary' | 'ghost' | 'approve' | 'caution' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    iconLeft?: IkonAdi;
    iconRight?: IkonAdi;
    fullWidth?: boolean;
    loading?: boolean;
    class?: string;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    iconLeft,
    iconRight,
    fullWidth = false,
    loading = false,
    disabled = false,
    type = 'button',
    class: sinif,
    children,
    ...geri
  }: Props = $props();

  const ikonBoyu = $derived(size === 'sm' ? 14 : 16);
</script>

<button
  {...geri}
  {type}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  class={['dugme', `v-${variant}`, `b-${size}`, { tam: fullWidth }, sinif]}
>
  {#if loading}
    <Icon name="loader-circle" size={ikonBoyu} class="donen" />
  {:else if iconLeft}
    <Icon name={iconLeft} size={ikonBoyu} />
  {/if}
  {@render children?.()}
  {#if iconRight}<Icon name={iconRight} size={ikonBoyu} />{/if}
</button>

<style>
  .dugme {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 34px;
    padding: 0 14px;
    font: var(--fw-medium) var(--text-small) / 1 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .dugme:hover:not(:disabled) {
    background: var(--bg-uzerinde);
  }
  .dugme:active:not(:disabled) {
    background: var(--bg-basili);
    transform: translateY(0.5px);
  }
  .dugme:disabled {
    color: var(--text-muted);
    background: transparent;
    border-color: var(--border-subtle);
    cursor: not-allowed;
    opacity: 0.55;
  }
  .tam {
    width: 100%;
  }
  .b-sm {
    height: 28px;
    padding: 0 10px;
    gap: 6px;
    font-size: var(--text-micro);
  }
  .b-lg {
    height: 42px;
    padding: 0 20px;
    font-size: var(--text-body-size);
  }
  .v-primary {
    --fg: var(--action-primary-fg);
    --bg: var(--action-primary);
    --bd: transparent;
    --bg-uzerinde: var(--action-primary-hover);
    --bg-basili: var(--action-primary-press);
  }
  .v-secondary {
    --fg: var(--text-body);
    --bg: transparent;
    --bd: var(--border-default);
    --bg-uzerinde: var(--surface-raised);
    --bg-basili: var(--surface-sunken);
  }
  .v-ghost {
    --fg: var(--text-secondary);
    --bg: transparent;
    --bd: transparent;
    --bg-uzerinde: var(--surface-raised);
    --bg-basili: var(--surface-sunken);
  }
  .v-approve {
    --fg: var(--status-ok);
    --bg: color-mix(in oklab, var(--green-400) 16%, transparent);
    --bd: color-mix(in oklab, var(--green-400) 45%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--green-400) 26%, transparent);
    --bg-basili: color-mix(in oklab, var(--green-400) 34%, transparent);
  }
  .v-caution {
    --fg: var(--status-warn);
    --bg: color-mix(in oklab, var(--amber-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--amber-200) 42%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--amber-200) 24%, transparent);
    --bg-basili: color-mix(in oklab, var(--amber-200) 32%, transparent);
  }
  .v-danger {
    --fg: var(--status-danger);
    --bg: color-mix(in oklab, var(--red-400) 16%, transparent);
    --bd: color-mix(in oklab, var(--red-400) 45%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--red-400) 26%, transparent);
    --bg-basili: color-mix(in oklab, var(--red-400) 34%, transparent);
  }
  :global(.donen) {
    animation: kalfa-don 1s linear infinite;
  }
</style>
```

`ui/src/bilesenler/Card.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    title?: string;
    /** Başlığın altındaki tek aralıklı satır: yol, sayı, süre. */
    subtitle?: string;
    icon?: IkonAdi;
    /** Başlığın sağındaki denetimler. */
    actions?: Snippet;
    tone?: 'default' | 'sunken' | 'accent' | 'danger';
    padded?: boolean;
    selected?: boolean;
    class?: string;
    children?: Snippet;
  }

  let {
    title,
    subtitle,
    icon,
    actions,
    tone = 'default',
    padded = true,
    selected = false,
    class: sinif,
    children,
  }: Props = $props();
</script>

<section class={['kart', `ton-${tone}`, { secili: selected }, sinif]}>
  {#if title || actions}
    <header class={{ ayrik: Boolean(children) }}>
      {#if icon}
        <span class="karo"><Icon name={icon} size={15} /></span>
      {/if}
      <div class="baslik-blogu">
        <div class="baslik">{title}</div>
        {#if subtitle}<div class="alt-baslik">{subtitle}</div>{/if}
      </div>
      {#if actions}<div class="eylemler">{@render actions()}</div>{/if}
    </header>
  {/if}
  {#if children}
    <div class={{ ic: true, bosluklu: padded }}>{@render children()}</div>
  {/if}
</section>

<style>
  .kart {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  .ton-sunken {
    background: var(--surface-sunken);
  }
  .ton-accent,
  .secili {
    border-color: var(--border-accent);
  }
  .ton-danger {
    border-color: color-mix(in oklab, var(--red-400) 40%, transparent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
  }
  header.ayrik {
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .karo {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-secondary);
    flex: none;
  }
  .baslik-blogu {
    min-width: 0;
    flex: 1;
  }
  .baslik {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .alt-baslik {
    font: var(--type-meta);
    color: var(--text-muted);
    margin-top: 3px;
  }
  .eylemler {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .bosluklu {
    padding: var(--pad-card);
  }
</style>
```

`ui/src/bilesenler/IconButton.svelte`:

```svelte
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    icon: IkonAdi;
    /** Zorunlu: aria-label ve ipucu olur. */
    label: string;
    size?: 'sm' | 'md' | 'lg';
    variant?: 'ghost' | 'primary';
    active?: boolean;
    class?: string;
  }

  let {
    icon,
    label,
    size = 'md',
    variant = 'ghost',
    active = false,
    class: sinif,
    ...geri
  }: Props = $props();

  const ikonBoyu = $derived(size === 'sm' ? 14 : size === 'lg' ? 18 : 16);
</script>

<button
  type="button"
  {...geri}
  aria-label={label}
  title={label}
  aria-pressed={active || undefined}
  class={['ikon-dugme', `b-${size}`, `v-${variant}`, { etkin: active }, sinif]}
>
  <Icon name={icon} size={ikonBoyu} />
</button>

<style>
  .ikon-dugme {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: var(--radius-md);
    border: var(--border-width) solid transparent;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .b-sm {
    width: 26px;
    height: 26px;
  }
  .b-lg {
    width: 40px;
    height: 40px;
  }
  .v-ghost:hover:not(:disabled) {
    background: var(--surface-raised);
  }
  .etkin {
    background: var(--surface-selected);
    color: var(--text-accent);
    border-color: var(--border-accent);
  }
  .v-primary {
    background: var(--action-primary);
    color: var(--action-primary-fg);
  }
  .v-primary:hover:not(:disabled) {
    background: var(--action-primary-hover);
  }
  .ikon-dugme:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
```

`ui/src/bilesenler/IconTile.svelte`:

```svelte
<!-- Mesaj yazarının ve günlük satırının simgesi olarak kullanılan kare karo. -->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon: IkonAdi;
    tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
    /** 28 sık satırda, 32 varsayılan, 40 başlıkta. */
    size?: 28 | 32 | 40;
    class?: string;
  }

  let { icon, tone = 'neutral', size = 32, class: sinif }: Props = $props();
</script>

<span class={['karo', `ton-${tone}`, `b-${size}`, sinif]}>
  <Icon name={icon} size={Math.round(size * 0.5)} />
</span>

<style>
  .karo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    border-radius: var(--radius-md);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
  }
  .b-28 {
    width: 28px;
    height: 28px;
  }
  .b-32 {
    width: 32px;
    height: 32px;
  }
  .b-40 {
    width: 40px;
    height: 40px;
  }
  .ton-neutral {
    --fg: var(--text-secondary);
    --bg: var(--surface-sunken);
    --bd: var(--border-subtle);
  }
  .ton-accent {
    --fg: var(--text-accent);
    --bg: var(--surface-selected);
    --bd: color-mix(in oklab, var(--copper-400) 40%, transparent);
  }
  .ton-ok {
    --fg: var(--status-ok);
    --bg: var(--green-tint);
    --bd: color-mix(in oklab, var(--green-400) 35%, transparent);
  }
  .ton-warn {
    --fg: var(--status-warn);
    --bg: var(--amber-tint);
    --bd: color-mix(in oklab, var(--amber-200) 35%, transparent);
  }
  .ton-danger {
    --fg: var(--status-danger);
    --bg: var(--red-tint);
    --bd: color-mix(in oklab, var(--red-400) 35%, transparent);
  }
  .ton-info {
    --fg: var(--status-info);
    --bg: var(--blue-tint);
    --bd: color-mix(in oklab, var(--blue-200) 35%, transparent);
  }
</style>
```

- [ ] **Step 4: Form bileşenlerini yaz**

`ui/src/bilesenler/Checkbox.svelte`:

```svelte
<!-- Gerçek bir onay kutusunun üstüne çizilir: klavye ve ekran okuyucu desteği yerleşiktir. -->
<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    checked?: boolean;
    /** Karışık durum: kısmen seçili dosya listesinin "Tümünü seç" satırı. */
    indeterminate?: boolean;
    label?: string;
    /** Sağdaki tek aralıklı not, ör. "+12 −4". */
    hint?: string;
    disabled?: boolean;
    onchange?: (sonraki: boolean) => void;
    class?: string;
  }

  let {
    checked = false,
    indeterminate = false,
    label,
    hint,
    disabled = false,
    onchange,
    class: sinif,
  }: Props = $props();

  let girdi = $state<HTMLInputElement>();

  $effect(() => {
    if (girdi) girdi.indeterminate = indeterminate;
  });
</script>

<label class={['onay-kutusu', { 'devre-disi': disabled }, sinif]}>
  <input
    bind:this={girdi}
    type="checkbox"
    class="gizli-erisilebilir"
    {checked}
    {disabled}
    onchange={(e) => onchange?.(e.currentTarget.checked)}
  />
  <span class={['kutu', { dolu: checked || indeterminate }]} aria-hidden="true">
    {#if indeterminate}
      <Icon name="minus" size={12} strokeWidth={3} />
    {:else if checked}
      <Icon name="check" size={12} strokeWidth={3} />
    {/if}
  </span>
  {#if label}<span class="etiket">{label}</span>{/if}
  {#if hint}<span class="ipucu">{hint}</span>{/if}
</label>

<style>
  .onay-kutusu {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 9px;
    cursor: pointer;
  }
  .devre-disi {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .kutu {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 17px;
    height: 17px;
    flex: none;
    border-radius: var(--radius-xs);
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-strong);
    color: var(--action-primary-fg);
    transition: var(--transition-control);
  }
  .dolu {
    background: var(--action-primary);
    border-color: var(--action-primary);
  }
  input:focus-visible + .kutu {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .etiket {
    font: var(--type-small);
    color: var(--text-body);
  }
  .ipucu {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
```

`ui/src/bilesenler/Input.svelte`:

```svelte
<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLInputAttributes, 'class' | 'size' | 'type' | 'value'> {
    value?: string;
    type?: 'text' | 'password' | 'search';
    iconLeft?: IkonAdi;
    iconRight?: IkonAdi;
    /** Tek aralıklı değer: anahtar, istek kimliği, yol. */
    mono?: boolean;
    size?: 'md' | 'lg';
    invalid?: boolean;
    class?: string;
  }

  let {
    value = $bindable(''),
    type = 'text',
    iconLeft,
    iconRight,
    mono = false,
    size = 'md',
    invalid = false,
    disabled = false,
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<div class={['girdi', `b-${size}`, { gecersiz: invalid, 'devre-disi': disabled }, sinif]}>
  {#if iconLeft}<span class="ikon-yeri"><Icon name={iconLeft} size={15} /></span>{/if}
  <input
    {...geri}
    {type}
    {value}
    {disabled}
    aria-invalid={invalid || undefined}
    class={{ mono }}
    oninput={(e) => (value = e.currentTarget.value)}
  />
  {#if iconRight}<span class="ikon-yeri"><Icon name={iconRight} size={15} /></span>{/if}
</div>

<style>
  .girdi {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--control-height);
    padding: 0 10px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-md);
    transition: var(--transition-control);
  }
  .b-lg {
    height: var(--control-height-lg);
  }
  .girdi:focus-within {
    border-color: var(--border-accent);
    box-shadow: var(--shadow-focus);
  }
  .gecersiz,
  .gecersiz:focus-within {
    border-color: var(--status-danger);
  }
  .devre-disi {
    opacity: 0.55;
  }
  .ikon-yeri {
    display: inline-flex;
    color: var(--text-muted);
  }
  input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-body);
    font: var(--type-small);
  }
  input.mono {
    font: var(--type-meta);
  }
</style>
```

`ui/src/bilesenler/Select.svelte`:

```svelte
<!--
  Açılır seçim listesi. Odak düğmede kalır; seçenekler aria-activedescendant
  ile gösterilir. Ok tuşları, Enter, Boşluk ve Escape çalışır.
-->
<script lang="ts" module>
  export interface Secenek {
    value: string;
    label: string;
    /** Sağa yaslı tek aralıklı ipucu: model kimliği, kısayol. */
    hint?: string;
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    value: string;
    options: Secenek[];
    /** Ekran okuyucunun okuduğu ad; görünmez. */
    label: string;
    icon?: IkonAdi;
    tone?: 'default' | 'accent';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onchange?: (value: string) => void;
    class?: string;
  }

  let {
    value,
    options,
    label,
    icon,
    tone = 'default',
    size = 'md',
    disabled = false,
    onchange,
    class: sinif,
  }: Props = $props();

  const kimlik = $props.id();
  let acik = $state(false);
  let vurgu = $state(0);
  let kok = $state<HTMLDivElement>();

  const secili = $derived(options.find((o) => o.value === value));

  function ac() {
    if (disabled) return;
    vurgu = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
    acik = true;
  }

  function sec(secenek: Secenek) {
    acik = false;
    if (secenek.value !== value) onchange?.(secenek.value);
  }

  function tus(e: KeyboardEvent) {
    if (!acik) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
        e.preventDefault();
        ac();
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      vurgu = Math.min(options.length - 1, vurgu + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      vurgu = Math.max(0, vurgu - 1);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const secenek = options[vurgu];
      if (secenek) sec(secenek);
    } else if (e.key === 'Escape' || e.key === 'Tab') {
      acik = false;
    }
  }

  function disariBasildi(e: PointerEvent) {
    if (acik && kok && !kok.contains(e.target as Node)) acik = false;
  }
</script>

<svelte:window onpointerdown={disariBasildi} />

<div bind:this={kok} class={['secim', sinif]}>
  <button
    type="button"
    role="combobox"
    class={['tetik', `b-${size}`, `ton-${tone}`, { acik }]}
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={acik}
    aria-controls={`${kimlik}-liste`}
    aria-label={label}
    aria-activedescendant={acik ? `${kimlik}-${vurgu}` : undefined}
    onclick={() => (acik ? (acik = false) : ac())}
    onkeydown={tus}
  >
    {#if icon}<span class="ikon-yeri"><Icon name={icon} size={15} /></span>{/if}
    <span class="deger">{secili?.label ?? value}</span>
    <span class="ok"><Icon name="chevron-down" size={14} /></span>
  </button>
  {#if acik}
    <div id={`${kimlik}-liste`} role="listbox" aria-label={label} class="liste">
      {#each options as secenek, i (secenek.value)}
        <button
          type="button"
          id={`${kimlik}-${i}`}
          role="option"
          tabindex="-1"
          aria-selected={secenek.value === value}
          class={['secenek', { vurgulu: i === vurgu, secili: secenek.value === value }]}
          onpointerdown={(e) => e.preventDefault()}
          onpointermove={() => (vurgu = i)}
          onclick={() => sec(secenek)}
        >
          <span class="etiket">{secenek.label}</span>
          {#if secenek.hint}<span class="ipucu">{secenek.hint}</span>{/if}
          {#if secenek.value === value}<Icon name="check" size={14} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .secim {
    position: relative;
    display: inline-block;
  }
  .tetik {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: var(--control-height);
    padding: 0 10px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-body);
    font: var(--type-small);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .tetik:hover:not(:disabled) {
    background: var(--surface-raised);
  }
  .tetik:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .b-sm {
    height: 28px;
  }
  .b-lg {
    height: var(--control-height-lg);
  }
  .acik {
    border-color: var(--border-accent);
  }
  .ikon-yeri,
  .ok {
    display: inline-flex;
    color: var(--text-muted);
  }
  .ton-accent {
    color: var(--text-accent);
  }
  .ton-accent .ikon-yeri {
    color: var(--text-accent);
  }
  .deger {
    flex: 1;
    text-align: left;
  }
  .liste {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    min-width: 100%;
    z-index: 40;
    padding: 4px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    white-space: nowrap;
  }
  .secenek {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 9px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-body);
    font: var(--type-small);
    text-align: left;
    cursor: pointer;
  }
  .vurgulu {
    background: var(--surface-raised);
  }
  .secili {
    background: var(--surface-selected);
    color: var(--text-accent);
  }
  .etiket {
    flex: 1;
  }
  .ipucu {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
```

`ui/src/bilesenler/Textarea.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLTextareaAttributes, 'class' | 'value'> {
    value?: string;
    rows?: number;
    /** Bakır kenarlık hep açık: Composer kullanır. */
    accent?: boolean;
    /** Metin alanının çerçevesi içinde, metnin altında duran denetimler. */
    footer?: Snippet;
    /** Metin alanı öğesi; odak vermek için bağlanır. */
    element?: HTMLTextAreaElement;
    class?: string;
  }

  let {
    value = $bindable(''),
    rows = 3,
    accent = false,
    disabled = false,
    footer,
    element = $bindable(),
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<div class={['metin-alani', { vurgulu: accent, 'devre-disi': disabled }, sinif]}>
  <textarea bind:this={element} {...geri} {rows} {disabled} bind:value></textarea>
  {#if footer}<div class="alt">{@render footer()}</div>{/if}
</div>

<style>
  .metin-alani {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: 10px 12px;
    transition: var(--transition-control);
  }
  .metin-alani:focus-within,
  .vurgulu {
    border: var(--border-width-accent) solid var(--border-accent);
  }
  .devre-disi {
    opacity: 0.55;
  }
  textarea {
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    resize: none;
    color: var(--text-body);
    font: var(--type-body);
  }
  .alt {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
</style>
```

- [ ] **Step 5: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: temiz; `temel.test.ts` geçer. `svelte-check` erişilebilirlik uyarısı vermez.

- [ ] **Step 6: Commit**

```bash
git add ui/src/bilesenler
git commit -m "ui: temel ve form bileşenleri"
git push
```

---

### Task 5: Güvenli Markdown ve veri bileşenleri

Markdown metni `mdast` sözdizimi ağacına çevrilir ve Svelte bileşenleriyle çizilir (1c tasarımı §9.1). Akan metin ekran karesi başına en çok bir kez çözülür; tamamlanan metin bir kez çözülür (§9.2). Kod blokları akış sırasında renklendirilmez; tamamlanınca Shiki'nin JavaScript motoruyla, tasarım sisteminin sözdizimi renklerinden oluşan bir temayla renklendirilir. Renklendirme modülü yalnızca gerekince yüklenir; şart penceresinin açılış paketine girmez. Veri bileşenleri: CodeBlock, DiffView (onay isteğindeki birleşik diff'i çözen `diff.ts` ile), MetaList, TokenMeter.

**Files:**
- Create: `ui/src/markdown/cozumle.ts`, `ui/src/markdown/adres.ts`, `ui/src/markdown/vurgula.ts`, `ui/src/markdown/Dugum.svelte`, `ui/src/markdown/Markdown.svelte`
- Create: `ui/src/bilesenler/CodeBlock.svelte`, `ui/src/bilesenler/diff.ts`, `ui/src/bilesenler/DiffView.svelte`, `ui/src/bilesenler/MetaList.svelte`, `ui/src/bilesenler/TokenMeter.svelte`
- Test: `ui/src/markdown/markdown.test.ts`, `ui/src/bilesenler/diff.test.ts`, `ui/src/bilesenler/veri.test.ts`

**Interfaces:**
- Consumes: Görev 1'in `m`'i; Görev 4'ün bileşenleri.
- Produces: `markdown/cozumle.ts`: `cozumle(metin): Root`; `markdown/adres.ts`: `guvenliAdres(adres): string | null`; `markdown/vurgula.ts`: `Parca { metin, renk }`, `dilAdi(etiket): Dil | null`, `vurgula(kod, etiket): Promise<Parca[][] | null>`; `Markdown.svelte` prop'ları: `metin`, `akista?`, `baglantiAc: (adres: string) => void`, `belge?`, `class?`. `bilesenler/diff.ts`: `DiffSatiri { kind, old?, neu?, text }`, `DiffParcasi { baslik, satirlar }`, `diffCozumle(metin): DiffParcasi[]`, `diffSayilari(parcalar): { eklenen, silinen }`. `CodeBlock` prop'ları: `code`, `lang?`, `highlight?`, `startLine?`, `showLineNumbers?`, `truncated?`, `maxHeight?` (CSS uzunluğu). `MetaList`: `MetaOgesi` (modül betiğinden). `TokenMeter`: `Dilim` (modül betiğinden).

- [ ] **Step 1: Testleri yaz**

`ui/src/markdown/markdown.test.ts`:

````ts
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { guvenliAdres } from './adres';
import Markdown from './Markdown.svelte';
import { dilAdi, vurgula } from './vurgula';

function ciz(metin: string, akista = false) {
  const baglantiAc = vi.fn();
  const sonuc = render(Markdown, { props: { metin, akista, baglantiAc } });
  return { ...sonuc, baglantiAc };
}

describe('güvenli Markdown (1c tasarımı §9.1, §11.3)', () => {
  it('ham HTML çalışmaz, metin olarak görünür', () => {
    const { container } = ciz(
      '<script>alert(1)</script>\n\nmetin <img src=x onerror="alert(2)"> sonu',
    );
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<script>alert(1)</script>');
    expect(container.textContent).toContain('<img src=x onerror="alert(2)">');
  });

  it('javascript: ve file: bağlantıya dönüşmez, düz metin kalır', () => {
    const { container } = ciz('[tıkla](javascript:alert(1)) ve [dosya](file:///etc/passwd)');
    expect(container.querySelector('button')).toBeNull();
    expect(container.querySelector('a')).toBeNull();
    expect(container.textContent).toContain('tıkla ve dosya');
  });

  it('http bağlantısı düğmedir; tıklanınca uygulamaya adres verilir', async () => {
    const { baglantiAc } = ciz('[EVREN](https://evren.example/yol)');
    const dugme = screen.getByRole('button', { name: 'EVREN' });
    await fireEvent.click(dugme);
    expect(baglantiAc).toHaveBeenCalledWith('https://evren.example/yol');
  });

  it('uzak görsel yüklenmez, yerinde alt metni görünür', () => {
    const { container } = ciz('![build rozeti](https://img.shields.io/badge/a-b-green)');
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('[build rozeti]');
  });

  it('kapanmamış kod bloğu metnin sonuna kadar sürer', () => {
    const { container } = ciz('önce\n\n```rust\nfn main() {\n    let a = 1;', true);
    const kod = container.querySelector('.kod');
    expect(kod?.textContent).toContain('fn main() {');
    expect(kod?.textContent).toContain('let a = 1;');
  });

  it('GFM tablosu hizalamasıyla çizilir', () => {
    const { container } = ciz('| a | b |\n|:--|--:|\n| 1 | 2 |');
    expect(container.querySelectorAll('th')).toHaveLength(2);
    expect(container.querySelector('td.sag')?.textContent).toBe('2');
  });

  it('görev listesi kutuları salt okunurdur', () => {
    const { container } = ciz('- [x] bitti\n- [ ] bekliyor');
    const kutular = container.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');
    expect([...kutular].map((k) => [k.checked, k.disabled])).toEqual([
      [true, true],
      [false, true],
    ]);
  });
});

describe('adres denetimi', () => {
  it('yalnızca mutlak http ve https', () => {
    expect(guvenliAdres('https://a.example/x?y=1')).toBe('https://a.example/x?y=1');
    expect(guvenliAdres('http://a.example')).toBe('http://a.example/');
    for (const kotu of [
      'javascript:alert(1)',
      'JAVASCRIPT:alert(1)',
      'file:///etc/passwd',
      'data:text/html,x',
      '/goreli',
      'goreli.html',
      'mailto:a@b.c',
    ]) {
      expect(guvenliAdres(kotu), kotu).toBeNull();
    }
  });
});

describe('kod renklendirme (1c tasarımı §9.2)', () => {
  it('dil etiketleri ve takma adlar', () => {
    expect(dilAdi('rust')).toBe('rust');
    expect(dilAdi('RS')).toBe('rust');
    expect(dilAdi('sh')).toBe('shellscript');
    expect(dilAdi('brainfuck')).toBeNull();
  });

  it('JavaScript motoruyla satır satır renkli parçalar', async () => {
    const satirlar = await vurgula('fn main() {\n    let a = "x";\n}\n', 'rust');
    expect(satirlar).toHaveLength(3);
    const parcalar = satirlar?.flat() ?? [];
    expect(parcalar.map((p) => p.metin).join('')).toBe('fn main() {    let a = "x";}');
    expect(parcalar.find((p) => p.metin === 'fn')?.renk.toLowerCase()).toBe('#c99bd6');
    expect(parcalar.find((p) => p.metin.includes('x'))?.renk.toLowerCase()).toBe('#8fbf7f');
  });

  it('desteklenmeyen dil renklendirilmez', async () => {
    expect(await vurgula('x', 'brainfuck')).toBeNull();
  });
});
````

`ui/src/bilesenler/diff.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { diffCozumle, diffSayilari } from './diff';

const DIFF = [
  '--- a/src/main.rs',
  '+++ b/src/main.rs',
  '@@ -1,3 +1,4 @@',
  ' fn main() {',
  '-    eski();',
  '+    yeni();',
  '+    --- tire ile başlayan satır',
  ' }',
  '@@ -10,2 +11,2 @@ fn b()',
  '-x',
  '+y',
  '\\ No newline at end of file',
  '',
].join('\n');

describe('birleşik diff', () => {
  it('bloklara ve numaralı satırlara çevrilir', () => {
    const [ilk, ikinci] = diffCozumle(DIFF);
    expect(ilk.baslik).toBe('@@ -1,3 +1,4 @@');
    expect(ilk.satirlar).toEqual([
      { kind: 'ctx', old: 1, neu: 1, text: 'fn main() {' },
      { kind: 'del', old: 2, text: '    eski();' },
      { kind: 'add', neu: 2, text: '    yeni();' },
      { kind: 'add', neu: 3, text: '    --- tire ile başlayan satır' },
      { kind: 'ctx', old: 3, neu: 4, text: '}' },
    ]);
    expect(ikinci.baslik).toBe('@@ -10,2 +11,2 @@ fn b()');
    expect(ikinci.satirlar).toEqual([
      { kind: 'del', old: 10, text: 'x' },
      { kind: 'add', neu: 11, text: 'y' },
    ]);
  });

  it('eklenen ve silinen satırları sayar', () => {
    expect(diffSayilari(diffCozumle(DIFF))).toEqual({ eklenen: 3, silinen: 2 });
  });

  it('yeni dosya', () => {
    const [p] = diffCozumle('--- a/x\n+++ b/x\n@@ -0,0 +1,2 @@\n+a\n+b\n');
    expect(p.satirlar).toEqual([
      { kind: 'add', neu: 1, text: 'a' },
      { kind: 'add', neu: 2, text: 'b' },
    ]);
  });
});
```

`ui/src/bilesenler/veri.test.ts`:

```ts
import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CodeBlock from './CodeBlock.svelte';
import DiffView from './DiffView.svelte';
import MetaList from './MetaList.svelte';
import TokenMeter from './TokenMeter.svelte';

describe('veri bileşenleri', () => {
  it('CodeBlock satır numaralarıyla çizer; kırpıldıysa söyler', () => {
    const { container } = render(CodeBlock, {
      props: { code: 'a\nb\n', startLine: 41, truncated: true },
    });
    expect([...container.querySelectorAll('.no')].map((n) => n.textContent)).toEqual(['41', '42']);
    expect(container.textContent).toContain('Çıktının devamı gösterilmiyor');
  });

  it('DiffView eklenen ve silinen satırları işaretler', () => {
    const { container } = render(DiffView, {
      props: {
        hunkHeader: '@@ -1 +1 @@',
        lines: [
          { kind: 'del', old: 1, text: 'eski' },
          { kind: 'add', neu: 1, text: 'yeni' },
        ],
      },
    });
    expect([...container.querySelectorAll('tr')].map((t) => t.className.split(' ')[0])).toEqual([
      't-del',
      't-add',
    ]);
    expect(container.textContent).toContain('@@ -1 +1 @@');
  });

  it('MetaList etiket ve değerleri tanım listesi olarak yazar', () => {
    const { container } = render(MetaList, {
      props: { items: [{ label: 'Nerede', value: 'src/a.rs', mono: true }] },
    });
    expect(container.querySelector('dt')?.textContent).toBe('Nerede');
    expect(container.querySelector('dd.mono')?.textContent).toBe('src/a.rs');
  });

  it('TokenMeter sayıları Türkçe biçimle yazar', () => {
    const { container } = render(TokenMeter, {
      props: { segments: [{ label: 'İstem', value: 12840 }] },
    });
    expect(container.textContent).toContain('İstem (12.840)');
  });
});
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/markdown src/bilesenler/diff.test.ts src/bilesenler/veri.test.ts`
Expected: FAIL — modüller yok.

- [ ] **Step 3: Markdown çözümleme, adres denetimi ve renklendirme**

`ui/src/markdown/cozumle.ts`:

```ts
import type { Root } from 'mdast';
import { fromMarkdown } from 'mdast-util-from-markdown';
import { gfmFromMarkdown } from 'mdast-util-gfm';
import { gfm } from 'micromark-extension-gfm';

/**
 * Markdown metnini sözdizimi ağacına çevirir (CommonMark + GFM). Ağaç Svelte
 * bileşenleriyle çizilir; ham HTML düğümleri metin olarak kalır (1c tasarımı
 * §9.1). Kapanmamış kod bloğu CommonMark gereği metnin sonuna kadar sürer.
 */
export function cozumle(metin: string): Root {
  return fromMarkdown(metin, { extensions: [gfm()], mdastExtensions: [gfmFromMarkdown()] });
}
```

`ui/src/markdown/adres.ts`:

```ts
/**
 * Bağlantıya dönüşebilecek adres: yalnızca mutlak `http` ve `https`
 * adresleri. `javascript:`, `file:`, `data:` ve göreli adresler `null` döner;
 * bunlar düz metin kalır (1c tasarımı §9.1). Rust da `baglanti_ac`'ta aynı
 * denetimi yapar.
 */
export function guvenliAdres(adres: string): string | null {
  let url: URL;
  try {
    url = new URL(adres);
  } catch {
    return null;
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
  return url.href;
}
```

`ui/src/markdown/vurgula.ts`:

```ts
// Kod renklendirme (1c tasarımı §9.2): tamamlanan mesajdaki kod blokları için.
// Shiki yerel olarak paketlenir, JavaScript düzenli ifade motoruyla çalışır
// (WASM motoru CSP'de `wasm-unsafe-eval` isterdi). Diller ilk kullanımda
// yüklenir. Çıktı HTML değil tokendır; Svelte çizer.
import { createHighlighterCore, type HighlighterCore, type ThemeRegistration } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';

export interface Parca {
  metin: string;
  renk: string;
}

const DUZ = '#98a0a6';

// Renkler tasarım sisteminin sözdizimi tokenlarıdır (colors.css: --code-*).
const TEMA: ThemeRegistration = {
  name: 'kalfa',
  type: 'dark',
  colors: { 'editor.foreground': DUZ, 'editor.background': '#161a1d' },
  tokenColors: [
    { settings: { foreground: DUZ } },
    { scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: '#5c6469' } },
    {
      scope: ['string', 'string.quoted', 'string.template', 'punctuation.definition.string'],
      settings: { foreground: '#8fbf7f' },
    },
    {
      scope: ['constant.numeric', 'constant.language', 'constant.character'],
      settings: { foreground: '#dd9161' },
    },
    {
      scope: [
        'keyword',
        'storage',
        'storage.type',
        'storage.modifier',
        'keyword.control',
        'variable.language',
        'entity.name.tag',
      ],
      settings: { foreground: '#c99bd6' },
    },
    {
      scope: [
        'entity.name.function',
        'support.function',
        'meta.function-call',
        'entity.other.attribute-name',
      ],
      settings: { foreground: '#7fb3e0' },
    },
    {
      scope: [
        'entity.name.type',
        'entity.name.class',
        'entity.name.namespace',
        'support.type',
        'support.class',
      ],
      settings: { foreground: '#6fc7c0' },
    },
    { scope: ['markup.inserted'], settings: { foreground: '#8fd6ac' } },
    { scope: ['markup.deleted'], settings: { foreground: '#e5948c' } },
  ],
};

const DILLER = {
  rust: () => import('@shikijs/langs/rust'),
  typescript: () => import('@shikijs/langs/typescript'),
  javascript: () => import('@shikijs/langs/javascript'),
  json: () => import('@shikijs/langs/json'),
  toml: () => import('@shikijs/langs/toml'),
  yaml: () => import('@shikijs/langs/yaml'),
  shellscript: () => import('@shikijs/langs/shellscript'),
  python: () => import('@shikijs/langs/python'),
  diff: () => import('@shikijs/langs/diff'),
  html: () => import('@shikijs/langs/html'),
  css: () => import('@shikijs/langs/css'),
} as const;

type Dil = keyof typeof DILLER;

const TAKMA_ADLAR: Record<string, Dil> = {
  rs: 'rust',
  ts: 'typescript',
  js: 'javascript',
  jsonc: 'json',
  yml: 'yaml',
  sh: 'shellscript',
  bash: 'shellscript',
  shell: 'shellscript',
  zsh: 'shellscript',
  py: 'python',
};

/** Kod bloğunun dil etiketini desteklenen bir dile çevirir; yoksa `null`. */
export function dilAdi(etiket: string): Dil | null {
  const k = etiket.trim().toLowerCase();
  if (k in DILLER) return k as Dil;
  return TAKMA_ADLAR[k] ?? null;
}

let vurgulayici: Promise<HighlighterCore> | null = null;

function vurgulayiciyiAl(): Promise<HighlighterCore> {
  vurgulayici ??= createHighlighterCore({
    themes: [TEMA],
    langs: [],
    engine: createJavaScriptRegexEngine(),
  });
  return vurgulayici;
}

/** Kodu satır satır renkli parçalara böler; dil desteklenmiyorsa `null`. */
export async function vurgula(kod: string, etiket: string): Promise<Parca[][] | null> {
  const dil = dilAdi(etiket);
  if (!dil) return null;
  const v = await vurgulayiciyiAl();
  if (!v.getLoadedLanguages().includes(dil)) await v.loadLanguage(DILLER[dil]());
  const { tokens } = v.codeToTokens(kod.replace(/\n$/, ''), { lang: dil, theme: 'kalfa' });
  return tokens.map((satir) => satir.map((t) => ({ metin: t.content, renk: t.color ?? DUZ })));
}
```

- [ ] **Step 4: Veri bileşenleri**

`ui/src/bilesenler/CodeBlock.svelte`:

```svelte
<!--
  Kod ve araç çıktısı. `highlight` açıksa ve dil destekleniyorsa Shiki
  tokenlarıyla renklendirilir; renkler `style:` yönergesiyle verilir (CSP
  satır içi stil özniteliğine izin vermez). Renklendirme modülü yalnızca
  gerekince yüklenir.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import type { Parca } from '../markdown/vurgula';

  interface Props {
    code: string;
    /** Markdown kod bloğunun dil etiketi. */
    lang?: string;
    highlight?: boolean;
    /** İlk satırın numarası: araç çıktısı çoğu zaman dosyanın bir dilimidir. */
    startLine?: number;
    showLineNumbers?: boolean;
    /** Çıktı kırpıldıysa "···" devam işareti. */
    truncated?: boolean;
    /** CSS uzunluğu, ör. "320px". */
    maxHeight?: string;
    class?: string;
  }

  let {
    code,
    lang,
    highlight = false,
    startLine = 1,
    showLineNumbers = true,
    truncated = false,
    maxHeight,
    class: sinif,
  }: Props = $props();

  const satirlar = $derived(code.replace(/\n$/, '').split('\n'));
  let renkli: Parca[][] | null = $state.raw(null);

  $effect(() => {
    renkli = null;
    if (!highlight || !lang) return;
    const [kod, dil] = [code, lang];
    let gecersiz = false;
    import('../markdown/vurgula')
      .then(({ vurgula }) => vurgula(kod, dil))
      .then((sonuc) => {
        if (!gecersiz) renkli = sonuc;
      })
      .catch(() => {
        // Renklendirilemeyen kod düz gösterilir.
      });
    return () => {
      gecersiz = true;
    };
  });
</script>

<div class={['kod', sinif]}>
  <div class="kaydirma" style:max-height={maxHeight}>
    <table>
      <tbody>
        {#each satirlar as satir, i (i)}
          <tr>
            {#if showLineNumbers}<td class="no">{startLine + i}</td>{/if}
            <td class="satir"
              >{#if renkli?.[i]}{#each renkli[i] as parca, j (j)}<span style:color={parca.renk}
                    >{parca.metin}</span
                  >{/each}{:else}{satir}{/if}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if truncated}
    <div class="devami" title={m('kod.kirpildi')}>
      <span aria-hidden="true">···</span>
      <span class="gizli-erisilebilir">{m('kod.kirpildi')}</span>
    </div>
  {/if}
</div>

<style>
  .kod {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .kaydirma {
    overflow: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font: var(--type-code);
  }
  .no {
    width: 1px;
    white-space: nowrap;
    text-align: right;
    padding: 0 12px 0 14px;
    color: var(--warm-400);
    user-select: none;
    vertical-align: top;
  }
  .satir {
    padding: 0 14px;
    white-space: pre;
    color: var(--code-punct);
  }
  .no + .satir {
    padding-left: 0;
  }
  .devami {
    text-align: center;
    padding: 4px 0 8px;
    color: var(--text-muted);
    font: var(--type-meta);
  }
</style>
```

`ui/src/bilesenler/diff.ts`:

```ts
// Onay isteğindeki birleşik diff'i (kalfa-tools, `similar`) DiffView'un
// satırlarına çevirir.

export interface DiffSatiri {
  kind: 'add' | 'del' | 'ctx';
  /** Değişiklikten önceki satır numarası. */
  old?: number;
  /** Değişiklikten sonraki satır numarası. */
  neu?: number;
  text: string;
}

export interface DiffParcasi {
  /** "@@ -42,10 +42,18 @@" başlığı. */
  baslik: string;
  satirlar: DiffSatiri[];
}

const BLOK = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/;

export function diffCozumle(metin: string): DiffParcasi[] {
  const parcalar: DiffParcasi[] = [];
  let parca: DiffParcasi | null = null;
  let eski = 0;
  let yeni = 0;
  for (const satir of metin.split('\n')) {
    const blok = BLOK.exec(satir);
    if (blok) {
      parca = { baslik: satir, satirlar: [] };
      parcalar.push(parca);
      eski = Number(blok[1]);
      yeni = Number(blok[2]);
      continue;
    }
    // İlk bloktan önceki "---"/"+++" dosya başlıkları atlanır.
    if (!parca) continue;
    if (satir.startsWith('+')) {
      parca.satirlar.push({ kind: 'add', neu: yeni++, text: satir.slice(1) });
    } else if (satir.startsWith('-')) {
      parca.satirlar.push({ kind: 'del', old: eski++, text: satir.slice(1) });
    } else if (satir.startsWith(' ')) {
      parca.satirlar.push({ kind: 'ctx', old: eski++, neu: yeni++, text: satir.slice(1) });
    }
    // "\ No newline at end of file" ve sondaki boş satır gösterilmez.
  }
  return parcalar;
}

/** Eklenen ve silinen satır sayısı: dosya listesindeki "+12 −4" notu. */
export function diffSayilari(parcalar: DiffParcasi[]): { eklenen: number; silinen: number } {
  let eklenen = 0;
  let silinen = 0;
  for (const p of parcalar) {
    for (const s of p.satirlar) {
      if (s.kind === 'add') eklenen++;
      else if (s.kind === 'del') silinen++;
    }
  }
  return { eklenen, silinen };
}
```

`ui/src/bilesenler/DiffView.svelte`:

```svelte
<script lang="ts">
  import type { DiffSatiri } from './diff';

  interface Props {
    /** Birleşik diff blok başlığı, ör. "@@ -42,10 +42,18 @@ async fn main()". */
    hunkHeader?: string;
    lines: DiffSatiri[];
    class?: string;
  }

  let { hunkHeader, lines, class: sinif }: Props = $props();

  const ISARET = { add: '+', del: '−', ctx: ' ' } as const;
</script>

<div class={['diff', sinif]}>
  {#if hunkHeader}<div class="blok-basligi">{hunkHeader}</div>{/if}
  <table>
    <tbody>
      {#each lines as satir, i (i)}
        <tr class={`t-${satir.kind}`}>
          <td class="no">{satir.old ?? ''}</td>
          <td class="no yeni">{satir.neu ?? ''}</td>
          <td class="isaret">{ISARET[satir.kind]}</td>
          <td class="metin">{satir.text}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .diff {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: auto;
    font: var(--type-code);
  }
  .blok-basligi {
    padding: 6px 14px;
    color: var(--text-muted);
    background: color-mix(in oklab, var(--warm-200) 5%, transparent);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  td {
    color: var(--fg, var(--code-punct));
  }
  .no {
    width: 1px;
    white-space: nowrap;
    text-align: right;
    padding: 0 8px 0 12px;
    color: var(--warm-400);
    user-select: none;
    background: var(--oluk, transparent);
  }
  .no.yeni {
    padding: 0 10px 0 4px;
  }
  .isaret {
    width: 1px;
    padding: 0 8px;
    user-select: none;
  }
  .metin {
    padding: 0 14px 0 0;
    white-space: pre;
  }
  .t-add {
    background: var(--diff-add-bg);
    --fg: var(--diff-add-fg);
    --oluk: var(--diff-add-gutter);
  }
  .t-del {
    background: var(--diff-del-bg);
    --fg: var(--diff-del-fg);
    --oluk: var(--diff-del-gutter);
  }
</style>
```

`ui/src/bilesenler/MetaList.svelte`:

```svelte
<script lang="ts" module>
  export interface MetaOgesi {
    label: string;
    value: string;
    /** Değer tek aralıklı: yol, boyut, kimlik, zaman. */
    mono?: boolean;
    tone?: 'default' | 'accent';
  }
</script>

<script lang="ts">
  interface Props {
    items: MetaOgesi[];
    labelWidth?: number;
    dense?: boolean;
    class?: string;
  }

  let { items, labelWidth = 112, dense = false, class: sinif }: Props = $props();
</script>

<dl class={['meta', { sik: dense }, sinif]} style:grid-template-columns={`${labelWidth}px 1fr`}>
  {#each items as oge, i (i)}
    <dt>{oge.label}</dt>
    <dd class={{ mono: oge.mono, vurgulu: oge.tone === 'accent' }}>{oge.value}</dd>
  {/each}
</dl>

<style>
  .meta {
    display: grid;
    row-gap: 10px;
    column-gap: 12px;
    margin: 0;
  }
  .sik {
    row-gap: 6px;
  }
  dt {
    font: var(--type-small);
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    font: var(--type-small);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .mono {
    font: var(--type-meta);
  }
  .vurgulu {
    color: var(--text-accent);
  }
</style>
```

`ui/src/bilesenler/TokenMeter.svelte`:

```svelte
<script lang="ts" module>
  export interface Dilim {
    label: string;
    value: number;
    /** Herhangi bir renk tokenı; verilmezse bakır. */
    color?: string;
  }
</script>

<script lang="ts">
  interface Props {
    segments: Dilim[];
    /** Bağlam penceresi; verilirse çubuk oranla dolar. */
    total?: number;
    used?: number;
    class?: string;
  }

  let { segments, total, used, class: sinif }: Props = $props();

  const toplam = $derived(segments.reduce((a, d) => a + d.value, 0) || 1);
  const doluluk = $derived(total ? Math.min(1, (used ?? toplam) / total) : 1);
</script>

<div class={['olcer', sinif]}>
  <div class="cubuk">
    <div class="dolgu" style:width={`${doluluk * 100}%`}>
      {#each segments as dilim, i (i)}
        <span
          style:width={`${(dilim.value / toplam) * 100}%`}
          style:background={dilim.color ?? 'var(--copper-400)'}
        ></span>
      {/each}
    </div>
  </div>
  <div class="aciklama">
    {#each segments as dilim, i (i)}
      <span class="kalem">
        <span class="nokta" style:background={dilim.color ?? 'var(--copper-400)'}></span>
        {dilim.label} ({dilim.value.toLocaleString('tr-TR')})
      </span>
    {/each}
  </div>
</div>

<style>
  .cubuk {
    display: flex;
    height: 6px;
    border-radius: var(--radius-pill);
    overflow: hidden;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
  }
  .dolgu {
    display: flex;
    min-width: 2px;
  }
  .aciklama {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin-top: 10px;
  }
  .kalem {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font: var(--type-meta);
    color: var(--text-secondary);
  }
  .nokta {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
</style>
```

- [ ] **Step 5: Markdown çiziciler**

`ui/src/markdown/Dugum.svelte`:

```svelte
<!--
  Markdown sözdizimi ağacının bir düğümü. Hiçbir şey HTML olarak çizilmez:
  ham HTML metin olarak görünür; bağlantı yalnızca http/https ise düğmeye
  dönüşür; görselin yerine alt metni yazılır, hiçbir görsel yüklenmez
  (1c tasarımı §9.1).
-->
<script lang="ts">
  import type { AlignType, Parent, RootContent } from 'mdast';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import { m } from '../dil/tr';
  import { guvenliAdres } from './adres';
  import Dugum from './Dugum.svelte';

  interface Props {
    dugum: RootContent;
    /** Akan mesajda kod blokları renklendirilmez (1c tasarımı §9.2). */
    akista: boolean;
    baglantiAc: (adres: string) => void;
  }

  let { dugum, akista, baglantiAc }: Props = $props();

  function hiza(a: AlignType | undefined) {
    return a === 'right' ? 'sag' : a === 'center' ? 'orta' : undefined;
  }
</script>

{#snippet cocuklar(d: Parent)}
  {#each d.children as cocuk, i (i)}
    <Dugum dugum={cocuk} {akista} {baglantiAc} />
  {/each}
{/snippet}

{#if dugum.type === 'text'}
  {dugum.value}
{:else if dugum.type === 'paragraph'}
  <p>{@render cocuklar(dugum)}</p>
{:else if dugum.type === 'heading'}
  <svelte:element this={`h${dugum.depth}`}>{@render cocuklar(dugum)}</svelte:element>
{:else if dugum.type === 'emphasis'}
  <em>{@render cocuklar(dugum)}</em>
{:else if dugum.type === 'strong'}
  <strong>{@render cocuklar(dugum)}</strong>
{:else if dugum.type === 'delete'}
  <del>{@render cocuklar(dugum)}</del>
{:else if dugum.type === 'inlineCode'}
  <code>{dugum.value}</code>
{:else if dugum.type === 'break'}
  <br />
{:else if dugum.type === 'thematicBreak'}
  <hr />
{:else if dugum.type === 'blockquote'}
  <blockquote>{@render cocuklar(dugum)}</blockquote>
{:else if dugum.type === 'list'}
  {#if dugum.ordered}
    <ol start={dugum.start ?? 1}>{@render cocuklar(dugum)}</ol>
  {:else}
    <ul>{@render cocuklar(dugum)}</ul>
  {/if}
{:else if dugum.type === 'listItem'}
  <li class={{ gorev: typeof dugum.checked === 'boolean' }}>
    {#if typeof dugum.checked === 'boolean'}
      <input type="checkbox" checked={dugum.checked} disabled />
    {/if}
    {@render cocuklar(dugum)}
  </li>
{:else if dugum.type === 'code'}
  <CodeBlock
    code={dugum.value}
    lang={dugum.lang ?? undefined}
    highlight={!akista}
    showLineNumbers={false}
  />
{:else if dugum.type === 'html'}
  <code class="ham-html">{dugum.value}</code>
{:else if dugum.type === 'link'}
  {@const adres = guvenliAdres(dugum.url)}
  {#if adres}
    <button type="button" class="baglanti" title={adres} onclick={() => baglantiAc(adres)}>
      {@render cocuklar(dugum)}
    </button>
  {:else}
    {@render cocuklar(dugum)}
  {/if}
{:else if dugum.type === 'linkReference'}
  {@render cocuklar(dugum)}
{:else if dugum.type === 'image' || dugum.type === 'imageReference'}
  <span class="gorsel">[{dugum.alt || m('markdown.gorsel')}]</span>
{:else if dugum.type === 'table'}
  <div class="tablo">
    <table>
      {#if dugum.children[0]}
        <thead>
          <tr>
            {#each dugum.children[0].children as hucre, j (j)}
              <th class={hiza(dugum.align?.[j])}>{@render cocuklar(hucre)}</th>
            {/each}
          </tr>
        </thead>
      {/if}
      <tbody>
        {#each dugum.children.slice(1) as satir, i (i)}
          <tr>
            {#each satir.children as hucre, j (j)}
              <td class={hiza(dugum.align?.[j])}>{@render cocuklar(hucre)}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else if dugum.type === 'footnoteReference'}
  <sup>[{dugum.label ?? dugum.identifier}]</sup>
{:else if dugum.type === 'footnoteDefinition'}
  <div class="dipnot">[{dugum.label ?? dugum.identifier}] {@render cocuklar(dugum)}</div>
{/if}
```

`ui/src/markdown/Markdown.svelte`:

```svelte
<!--
  Güvenli Markdown gösterimi (1c tasarımı §9.1, §9.2). Sohbette, düşünce
  panelinde, araç çıktısında ve şart metninde aynı çizici kullanılır.
  Akan metin ekran karesi başına en çok bir kez yeniden çözülür; tamamlanan
  metin bir kez çözülür.
-->
<script lang="ts">
  import type { Root } from 'mdast';
  import { untrack } from 'svelte';
  import { cozumle } from './cozumle';
  import Dugum from './Dugum.svelte';

  interface Props {
    metin: string;
    akista?: boolean;
    /** Bağlantı tıklanınca; Rust adresi yerel iletişim kutusunda gösterip açar. */
    baglantiAc: (adres: string) => void;
    /** Şart metni gibi uzun belgeler için serif yazı düzeni. */
    belge?: boolean;
    class?: string;
  }

  let { metin, akista = false, baglantiAc, belge = false, class: sinif }: Props = $props();

  let agac: Root = $state.raw(untrack(() => cozumle(metin)));
  let bekleyen: number | null = null;

  $effect(() => {
    const guncel = metin;
    if (!akista) {
      if (bekleyen !== null) cancelAnimationFrame(bekleyen);
      bekleyen = null;
      agac = cozumle(guncel);
      return;
    }
    if (bekleyen !== null) return;
    bekleyen = requestAnimationFrame(() => {
      bekleyen = null;
      agac = cozumle(untrack(() => metin));
    });
  });

  $effect(() => () => {
    if (bekleyen !== null) cancelAnimationFrame(bekleyen);
  });
</script>

<div class={['markdown', { belge }, sinif]}>
  {#each agac.children as dugum, i (i)}
    <Dugum {dugum} {akista} {baglantiAc} />
  {/each}
</div>

<style>
  .markdown {
    font: var(--type-body);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .belge {
    font: var(--type-doc);
  }
  .markdown > :global(:first-child) {
    margin-top: 0;
  }
  .markdown > :global(:last-child) {
    margin-bottom: 0;
  }
  .markdown :global(p) {
    margin: 0 0 0.75em;
  }
  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3),
  .markdown :global(h4),
  .markdown :global(h5),
  .markdown :global(h6) {
    margin: 1.1em 0 0.5em;
    color: var(--text-title);
    font-weight: var(--fw-semibold);
    line-height: var(--lh-snug);
  }
  .markdown :global(h1) {
    font-size: var(--text-title-lg);
  }
  .markdown :global(h2) {
    font-size: var(--text-title-size);
  }
  .markdown :global(h3),
  .markdown :global(h4),
  .markdown :global(h5),
  .markdown :global(h6) {
    font-size: var(--text-subtitle);
  }
  .markdown :global(ul),
  .markdown :global(ol) {
    margin: 0 0 0.75em;
    padding-left: 1.4em;
  }
  .markdown :global(li > p) {
    margin: 0;
  }
  .markdown :global(li.gorev) {
    list-style: none;
    margin-left: -1.4em;
  }
  .markdown :global(li.gorev > input[type='checkbox']) {
    margin: 0 8px 0 0;
    vertical-align: -2px;
  }
  .markdown :global(li.gorev > p) {
    display: inline;
  }
  .markdown :global(blockquote) {
    margin: 0 0 0.75em;
    padding: 2px 12px;
    border-left: 3px solid var(--border-strong);
    color: var(--text-secondary);
  }
  .markdown :global(code) {
    font: var(--type-meta);
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--surface-sunken);
  }
  .markdown :global(.ham-html) {
    color: var(--text-secondary);
  }
  .markdown :global(hr) {
    border: none;
    border-top: var(--border-width) solid var(--border-subtle);
    margin: 1em 0;
  }
  .markdown :global(.baglanti) {
    display: inline;
    padding: 0;
    border: none;
    border-bottom: 1px solid color-mix(in oklab, var(--text-link) 35%, transparent);
    background: none;
    color: var(--text-link);
    font: inherit;
    cursor: pointer;
  }
  .markdown :global(.baglanti:hover) {
    color: var(--text-link-hover);
    border-bottom-color: currentColor;
  }
  .markdown :global(.gorsel) {
    color: var(--text-muted);
    font-style: italic;
  }
  .markdown :global(.tablo) {
    overflow-x: auto;
    margin: 0 0 0.75em;
  }
  .markdown :global(.tablo table) {
    border-collapse: collapse;
    font: var(--type-small);
  }
  .markdown :global(.tablo th),
  .markdown :global(.tablo td) {
    padding: 5px 10px;
    border: var(--border-width) solid var(--border-subtle);
    text-align: left;
  }
  .markdown :global(.tablo th) {
    color: var(--text-title);
    font-weight: var(--fw-semibold);
  }
  .markdown :global(.tablo .orta) {
    text-align: center;
  }
  .markdown :global(.tablo .sag) {
    text-align: right;
  }
  .markdown :global(.dipnot) {
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .markdown :global(.kod) {
    margin: 0 0 0.75em;
  }
</style>
```

- [ ] **Step 6: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: temiz. `markdown.test.ts`: ham HTML metin olarak kalır; `javascript:` ve `file:` bağlantıya dönüşmez; uzak görsel alt metne döner; kapanmamış kod bloğu; GFM tablosu; Shiki Rust kodunu renklendirir.

- [ ] **Step 7: Commit**

```bash
git add ui/src/markdown ui/src/bilesenler
git commit -m "ui: güvenli Markdown, kod renklendirme ve veri bileşenleri"
git push
```

---

### Task 6: Sohbet ve gezinme bileşenleri, bileşen vitrini

Sohbet (Composer, Message, ThoughtPanel, ToolCallCard) ve gezinme (NavItem, NavRail, NavSection, PanelHeader, SessionRow, StatusBar, StatusItem) bileşenleri. Composer'da Enter gönderir, Shift+Enter yeni satır açar; tur sürerken gönder düğmesinin yerinde durdurma düğmesi durur; model ve izin kipi aynı çerçevededir (1c tasarımı §8.1, §9.5). Tasarım sistemindeki Composer'ın dosya ekleme, bağlam ve kod düğmeleri 1c'de yoktur. Mesaj yazarının varsayılan adı "Siz"dir (tasarım sistemindeki "Sen" değil). Vitrin, bütün bileşenleri örnek verilerle gösteren ve yalnızca geliştirme sunucusunda açılan bir sayfadır; tasarım sistemiyle görsel karşılaştırma için kullanılır (1c tasarımı §11.5).

**Files:**
- Create: `ui/src/bilesenler/Message.svelte`, `ThoughtPanel.svelte`, `ToolCallCard.svelte`, `Composer.svelte`, `NavItem.svelte`, `NavRail.svelte`, `NavSection.svelte`, `PanelHeader.svelte`, `SessionRow.svelte`, `StatusBar.svelte`, `StatusItem.svelte` (hepsi `ui/src/bilesenler/` altında)
- Create: `ui/vitrin.html`, `ui/src/vitrin/giris.ts`, `ui/src/vitrin/Vitrin.svelte`
- Test: `ui/src/bilesenler/sohbet.test.ts`

**Interfaces:**
- Consumes: Görev 4 ve 5'in bileşenleri; Görev 1'in `m`, `KIP_ADI`, `MODEL_PROFILI_ADI`, `KIPLER`.
- Produces: `Composer` prop'ları: `value` (`$bindable`), `models: Secenek[]`, `model`, `modes: Secenek[]`, `mode`, `running?`, `disabled?`, `placeholder?`, `onsend(metin)`, `oncancel()`, `onmodelchange(model)`, `onmodechange(kip)`. `ToolCallCard`: `status: 'running' | 'done' | 'failed' | 'denied' | 'unknown'` ve zorunlu `statusLabel`. `SessionRow`: `status: 'running' | 'waiting' | 'done' | 'failed' | 'partial' | 'idle'`, `statusLabel?`. `NavRail` ve `StatusBar` zorunlu `label` (erişilebilir ad) alır; `ThoughtPanel` `open`'ı `$bindable` taşır ve kapalı başlar.

- [ ] **Step 1: Testi yaz**

`ui/src/bilesenler/sohbet.test.ts`:

```ts
import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Composer from './Composer.svelte';
import SessionRow from './SessionRow.svelte';
import ThoughtPanel from './ThoughtPanel.svelte';
import ToolCallCard from './ToolCallCard.svelte';

const metin = (m: string) => createRawSnippet(() => ({ render: () => `<span>${m}</span>` }));

const MODELLER = [
  { value: 'glm-5.3', label: 'Derin', hint: 'glm-5.3' },
  { value: 'auto', label: 'Otomatik' },
];
const KIPLER = [
  { value: 'Siki', label: 'Sıkı' },
  { value: 'Serbest', label: 'Serbest' },
];

describe('Composer (1c tasarımı §8.1, §9.5)', () => {
  function ciz(ek: Record<string, unknown> = {}) {
    const onsend = vi.fn();
    const oncancel = vi.fn();
    render(Composer, {
      props: { models: MODELLER, model: 'glm-5.3', modes: KIPLER, mode: 'Siki', onsend, oncancel, ...ek },
    });
    return { onsend, oncancel, alan: screen.getByRole<HTMLTextAreaElement>('textbox') };
  }

  it('Enter gönderir ve alanı boşaltır; Shift+Enter göndermez', async () => {
    const { onsend, alan } = ciz();
    await fireEvent.input(alan, { target: { value: 'selam' } });
    await fireEvent.keyDown(alan, { key: 'Enter', shiftKey: true });
    expect(onsend).not.toHaveBeenCalled();
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).toHaveBeenCalledWith('selam');
    expect(alan.value).toBe('');
  });

  it('boş mesaj gönderilmez', async () => {
    const { onsend, alan } = ciz();
    await fireEvent.input(alan, { target: { value: '   ' } });
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).not.toHaveBeenCalled();
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Gönder' }).disabled).toBe(true);
  });

  it('tur sürerken gönder yerine durdur vardır', async () => {
    const { onsend, oncancel, alan } = ciz({ running: true });
    expect(screen.queryByRole('button', { name: 'Gönder' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Turu durdur' }));
    expect(oncancel).toHaveBeenCalledOnce();
    await fireEvent.input(alan, { target: { value: 'x' } });
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).not.toHaveBeenCalled();
  });
});

describe('sohbet ve gezinme bileşenleri', () => {
  it('ThoughtPanel kapalı başlar, düğmeyle açılır', async () => {
    render(ThoughtPanel, { props: { children: metin('iç düşünce') } });
    const dugme = screen.getByRole('button', { name: /Düşünce/ });
    expect(dugme.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByText('iç düşünce')).toBeNull();
    await fireEvent.click(dugme);
    expect(screen.getByText('iç düşünce').textContent).toBe('iç düşünce');
  });

  it('ToolCallCard durumu ekran okuyucuya adıyla okunur', () => {
    render(ToolCallCard, {
      props: { tool: 'Dosya okundu', target: 'a.rs', status: 'unknown', statusLabel: 'Sonucu bilinmiyor' },
    });
    expect(screen.getAllByRole('img', { name: 'Sonucu bilinmiyor' })).toHaveLength(1);
  });

  it('SessionRow durum adını gizli metin olarak taşır', () => {
    render(SessionRow, { props: { title: 'Oturum', status: 'waiting', statusLabel: 'Onay bekliyor' } });
    expect(screen.getAllByRole('button', { name: /Onay bekliyor/ })).toHaveLength(1);
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/bilesenler/sohbet.test.ts`
Expected: FAIL — bileşen dosyaları yok.

- [ ] **Step 3: Sohbet bileşenleri**

`ui/src/bilesenler/Message.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';
  import IconTile from './IconTile.svelte';

  interface Props {
    author?: 'user' | 'kalfa';
    /** Verilmezse "Siz" ya da "Kalfa". */
    name?: string;
    /** Kısa saat, "14:28". */
    time?: string;
    /** Token'lar gelmeye devam ederken yanıp sönen bakır imleç. */
    streaming?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { author = 'kalfa', name, time, streaming = false, class: sinif, children }: Props =
    $props();

  const kullanici = $derived(author === 'user');
</script>

<article class={['mesaj', { kullanici }, sinif]}>
  <IconTile icon={kullanici ? 'user' : 'hexagon'} tone={kullanici ? 'neutral' : 'accent'} />
  <div class="govde">
    <header>
      <span class="ad">{name ?? (kullanici ? m('mesaj.kullanici') : m('mesaj.kalfa'))}</span>
      {#if time}<span class="zaman">{time}</span>{/if}
    </header>
    <div class="icerik">
      {@render children?.()}
      {#if streaming}<span class="imlec" aria-hidden="true"></span>{/if}
    </div>
  </div>
</article>

<style>
  .mesaj {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
  }
  .kullanici {
    background: var(--surface-raised);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
  }
  .govde {
    flex: 1;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 5px;
  }
  .ad {
    font: var(--fw-semibold) var(--text-small) / 1 var(--font-sans);
    color: var(--text-title);
  }
  .zaman {
    margin-left: auto;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .icerik {
    font: var(--type-body);
    color: var(--text-body);
    text-wrap: pretty;
  }
  .imlec {
    display: inline-block;
    width: 8px;
    height: 15px;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--copper-200);
    animation: kalfa-imlec 1s steps(2) infinite;
  }
</style>
```

`ui/src/bilesenler/ThoughtPanel.svelte`:

```svelte
<!-- Modelin düşünce dökümü. Kalfa bunu kapalı gösterir: düşünce sunulur, dayatılmaz. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';
  import Icon from './Icon.svelte';

  interface Props {
    /** Özet satırı; verilmezse katalogdaki genel metin. */
    summary?: string;
    open?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { summary, open = $bindable(false), class: sinif, children }: Props = $props();

  const kimlik = $props.id();
</script>

<section class={['dusunce', sinif]}>
  <button
    type="button"
    aria-expanded={open}
    aria-controls={`${kimlik}-ic`}
    onclick={() => (open = !open)}
  >
    <span class="ok"><Icon name={open ? 'chevron-down' : 'chevron-right'} size={15} /></span>
    <span class="metinler">
      <span class="baslik">{m('dusunce.baslik')}</span>
      <span class="ozet">{summary ?? m('dusunce.ozet')}</span>
    </span>
  </button>
  {#if open}
    <div id={`${kimlik}-ic`} class="ic">{@render children?.()}</div>
  {/if}
</section>

<style>
  .dusunce {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 12px 14px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }
  .ok {
    display: inline-flex;
    color: var(--text-muted);
  }
  .metinler {
    flex: 1;
  }
  .baslik {
    display: block;
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .ozet {
    display: block;
    margin-top: 3px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .ic {
    padding: 12px 14px 14px 39px;
    border-top: var(--border-width) solid var(--border-subtle);
    font: var(--type-small);
    color: var(--text-secondary);
  }
</style>
```

`ui/src/bilesenler/ToolCallCard.svelte`:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import IconTile from './IconTile.svelte';
  import type { IkonAdi } from './ikonlar';

  type Durum = 'running' | 'done' | 'failed' | 'denied' | 'unknown';

  interface Props {
    /** Ne oldu, geçmiş zamanla: "Dosya okundu". */
    tool: string;
    /** Tek aralıklı hedef: yol, desen. */
    target?: string;
    /** Tek aralıklı özet: "412 satır · 18 KB". */
    meta?: string;
    status?: Durum;
    /** Durumun ekran okuyucuya okunan adı. */
    statusLabel: string;
    icon?: IkonAdi;
    class?: string;
    /** Çıktı önizlemesi; çoğunlukla bir CodeBlock. */
    children?: Snippet;
  }

  let {
    tool,
    target,
    meta,
    status = 'done',
    statusLabel,
    icon = 'file-text',
    class: sinif,
    children,
  }: Props = $props();

  const DURUM_IKONU: Record<Durum, IkonAdi> = {
    running: 'loader-circle',
    done: 'circle-check',
    failed: 'triangle-alert',
    denied: 'circle-x',
    unknown: 'circle-help',
  };
</script>

<section class={['arac', sinif]}>
  <header>
    <IconTile {icon} />
    <div class="metinler">
      <div class="ad">{tool}</div>
      {#if target}<div class="hedef">{target}</div>{/if}
      {#if meta}<div class="ozet">{meta}</div>{/if}
    </div>
    <span class={['durum', `d-${status}`]}>
      <Icon
        name={DURUM_IKONU[status]}
        size={19}
        title={statusLabel}
        class={status === 'running' ? 'donen' : undefined}
      />
    </span>
  </header>
  {#if children}<div class="cikti">{@render children()}</div>{/if}
</section>

<style>
  .arac {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
  }
  .metinler {
    flex: 1;
    min-width: 0;
  }
  .ad {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .hedef {
    margin-top: 3px;
    font: var(--type-meta);
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }
  .ozet {
    margin-top: 6px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .durum {
    display: inline-flex;
  }
  .d-running {
    color: var(--status-warn);
  }
  .d-done {
    color: var(--status-ok);
  }
  .d-failed,
  .d-denied {
    color: var(--status-danger);
  }
  .d-unknown {
    color: var(--text-muted);
  }
  .cikti {
    padding: 0 14px 14px;
  }
</style>
```

`ui/src/bilesenler/Composer.svelte`:

```svelte
<!--
  Oturumun altındaki bakır çerçeveli giriş: mesaj, model ve izin kipi tek
  çerçevede; kullanıcı ikisini görmeden gönderemez. Enter gönderir,
  Shift+Enter yeni satır açar (1c tasarımı §9.5). Tur sürerken gönder
  düğmesinin yerinde durdurma düğmesi durur.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import IconButton from './IconButton.svelte';
  import Select, { type Secenek } from './Select.svelte';
  import Textarea from './Textarea.svelte';

  interface Props {
    value?: string;
    models: Secenek[];
    model: string;
    modes: Secenek[];
    mode: string;
    /** Tur sürüyor: gönderme kapalı, durdurma açık. */
    running?: boolean;
    disabled?: boolean;
    placeholder?: string;
    onsend?: (metin: string) => void;
    oncancel?: () => void;
    onmodelchange?: (model: string) => void;
    onmodechange?: (kip: string) => void;
    class?: string;
  }

  let {
    value = $bindable(''),
    models,
    model,
    modes,
    mode,
    running = false,
    disabled = false,
    placeholder = m('composer.yer_tutucu'),
    onsend,
    oncancel,
    onmodelchange,
    onmodechange,
    class: sinif,
  }: Props = $props();

  const gonderilebilir = $derived(!running && !disabled && value.trim().length > 0);

  function gonder() {
    if (!gonderilebilir) return;
    const metin = value;
    value = '';
    onsend?.(metin);
  }

  function tus(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      gonder();
    }
  }
</script>

<Textarea
  bind:value
  accent
  rows={2}
  {placeholder}
  {disabled}
  aria-label={placeholder}
  onkeydown={tus}
  class={sinif}
>
  {#snippet footer()}
    <Select
      size="sm"
      icon="box"
      tone="accent"
      label={m('composer.model')}
      value={model}
      options={models}
      disabled={disabled || running}
      onchange={onmodelchange}
    />
    <Select
      size="sm"
      icon="shield"
      label={m('composer.kip')}
      value={mode}
      options={modes}
      disabled={disabled || running}
      onchange={onmodechange}
    />
    <span class="bosluk"></span>
    {#if running}
      <IconButton icon="square" label={m('composer.durdur')} variant="primary" onclick={oncancel} />
    {:else}
      <IconButton
        icon="send"
        label={m('composer.gonder')}
        variant="primary"
        disabled={!gonderilebilir}
        onclick={gonder}
      />
    {/if}
  {/snippet}
</Textarea>

<style>
  .bosluk {
    flex: 1;
  }
</style>
```

- [ ] **Step 4: Gezinme bileşenleri**

`ui/src/bilesenler/NavItem.svelte`:

```svelte
<!-- Sol menüde ya da iç içe listede tek satır. Etkin satır bakır kenarlık alır. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    icon?: IkonAdi;
    active?: boolean;
    /** Sağdaki tek aralıklı sayı ya da not. */
    badge?: string | number;
    /** Girinti düzeyi; her düzey 14 px ekler. */
    depth?: number;
    class?: string;
    children?: Snippet;
  }

  let {
    icon,
    active = false,
    badge,
    depth = 0,
    class: sinif,
    children,
    ...geri
  }: Props = $props();
</script>

<button
  type="button"
  {...geri}
  aria-current={active ? 'page' : undefined}
  class={['menu-ogesi', { etkin: active }, sinif]}
  style:padding-left={`${10 + depth * 14}px`}
>
  {#if icon}<span class="ikon-yeri"><Icon name={icon} size={18} /></span>{/if}
  <span class="etiket">{@render children?.()}</span>
  {#if badge !== undefined}<span class="rozet">{badge}</span>{/if}
</button>

<style>
  .menu-ogesi {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    height: var(--row-height);
    padding-right: var(--space-6);
    border: 0;
    border-radius: var(--radius-md);
    cursor: pointer;
    font: var(--fw-regular) var(--text-small) / 1 var(--font-sans);
    color: var(--text-secondary);
    background: transparent;
    transition: var(--transition-control);
    text-align: left;
  }
  .menu-ogesi:hover {
    color: var(--text-body);
    background: var(--surface-raised);
  }
  .etkin,
  .etkin:hover {
    font-weight: var(--fw-semibold);
    color: var(--text-title);
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .ikon-yeri {
    display: inline-flex;
  }
  .etkin .ikon-yeri {
    color: var(--text-accent);
  }
  .etiket {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rozet {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
```

`ui/src/bilesenler/NavRail.svelte`:

```svelte
<!-- Masaüstü uygulamasının 213 px'lik sabit sol menüsü. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Ekran okuyucunun okuduğu menü adı. */
    label: string;
    /** Üstteki marka alanı. */
    header?: Snippet;
    /** Alta sabitlenen alan. */
    footer?: Snippet;
    class?: string;
    children?: Snippet;
  }

  let { label, header, footer, class: sinif, children }: Props = $props();
</script>

<nav class={['menu', sinif]} aria-label={label}>
  {#if header}<div class="ust">{@render header()}</div>{/if}
  <div class="liste">{@render children?.()}</div>
  {#if footer}<div class="alt">{@render footer()}</div>{/if}
</nav>

<style>
  .menu {
    display: flex;
    flex-direction: column;
    width: var(--rail-width);
    flex: 0 0 auto;
    height: 100%;
    background: var(--bg-rail);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .ust {
    padding: var(--space-6) var(--space-6) var(--space-4);
  }
  .liste {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .alt {
    padding: var(--space-4) var(--space-6);
    border-top: var(--border-width) solid var(--border-subtle);
  }
</style>
```

`ui/src/bilesenler/NavSection.svelte`:

```svelte
<!-- Menü içinde büyük harfli, aralıklı grup etiketi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    class?: string;
    children?: Snippet;
  }

  let { label, class: sinif, children }: Props = $props();
</script>

<div class={['bolum', sinif]}>
  <div class="etiket">{label}</div>
  {@render children?.()}
</div>

<style>
  .bolum {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-6);
  }
  .etiket {
    font: var(--type-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 0 var(--space-5) var(--space-3);
  }
</style>
```

`ui/src/bilesenler/PanelHeader.svelte`:

```svelte
<!-- Her panelin ya da sütunun üstündeki 44 px'lik başlık çubuğu. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon?: IkonAdi;
    title: string;
    /** Başlığın yanındaki tek aralıklı bilgi: yol, sayı, zaman. */
    subtitle?: string;
    /** Sağa yaslı denetimler. */
    actions?: Snippet;
    class?: string;
  }

  let { icon, title, subtitle, actions, class: sinif }: Props = $props();
</script>

<header class={['panel-basligi', sinif]}>
  {#if icon}<span class="ikon-yeri"><Icon name={icon} size={20} /></span>{/if}
  <div class="metinler">
    <h2 class="baslik">{title}</h2>
    {#if subtitle}<span class="alt-baslik">{subtitle}</span>{/if}
  </div>
  {#if actions}<div class="eylemler">{@render actions()}</div>{/if}
</header>

<style>
  .panel-basligi {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    height: var(--titlebar-height);
    flex: 0 0 auto;
    padding: 0 var(--space-8);
    background: var(--bg-panel);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .ikon-yeri {
    display: inline-flex;
    color: var(--text-muted);
  }
  .metinler {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: var(--space-5);
  }
  .baslik {
    margin: 0;
    font: var(--type-title);
    color: var(--text-title);
    letter-spacing: var(--tracking-tight);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .alt-baslik {
    font: var(--type-meta);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .eylemler {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>
```

`ui/src/bilesenler/SessionRow.svelte`:

```svelte
<!--
  295 px'lik oturum listesinde bir oturum. Nokta rengi durumu gösterir
  (1c tasarımı §8.1): çalışıyor, onay ya da şart bekliyor, tamamlandı,
  hata, yarım kaldı.
-->
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Durum = 'running' | 'waiting' | 'done' | 'failed' | 'partial' | 'idle';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children' | 'title'> {
    title: string;
    /** İki satıra kırpılan kısa açıklama; çoğunlukla çalışma alanı. */
    preview?: string;
    /** Sağa yaslı tek aralıklı zaman. */
    time?: string;
    status?: Durum;
    /** Durumun ekran okuyucuya okunan adı. */
    statusLabel?: string;
    active?: boolean;
    class?: string;
  }

  let {
    title,
    preview,
    time,
    status,
    statusLabel,
    active = false,
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<button
  type="button"
  {...geri}
  aria-current={active ? 'true' : undefined}
  class={['oturum-satiri', { etkin: active }, sinif]}
>
  <span class="ust">
    {#if status}
      <span class={['nokta', `d-${status}`]} aria-hidden="true"></span>
      {#if statusLabel}<span class="gizli-erisilebilir">{statusLabel}</span>{/if}
    {/if}
    <span class="baslik">{title}</span>
    {#if time}<span class="zaman">{time}</span>{/if}
  </span>
  {#if preview}<span class="onizleme">{preview}</span>{/if}
</button>

<style>
  .oturum-satiri {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
    padding: var(--space-5) var(--space-6);
    cursor: pointer;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    transition: var(--transition-control);
  }
  .oturum-satiri:hover {
    background: var(--surface-raised);
  }
  .etkin,
  .etkin:hover {
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .ust {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
  }
  .nokta {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-pill);
    flex: 0 0 auto;
    background: var(--text-muted);
  }
  .d-running {
    background: var(--status-warn);
  }
  .d-waiting {
    background: var(--text-accent);
  }
  .d-done {
    background: var(--status-ok);
  }
  .d-failed {
    background: var(--status-danger);
  }
  .d-partial {
    background: var(--status-info);
  }
  .baslik {
    flex: 1;
    min-width: 0;
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .etkin .baslik {
    color: var(--text-title);
  }
  .zaman {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .onizleme {
    font: var(--fw-regular) var(--text-micro) / 1.45 var(--font-sans);
    color: var(--text-muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
```

`ui/src/bilesenler/StatusBar.svelte`:

```svelte
<!-- 46 px'lik alt çubuk: bakiye, token kullanımı, model, izin kipi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Ekran okuyucunun okuduğu ad. */
    label: string;
    left?: Snippet;
    right?: Snippet;
    class?: string;
  }

  let { label, left, right, class: sinif }: Props = $props();
</script>

<footer class={['durum-cubugu', sinif]} aria-label={label}>
  {@render left?.()}
  <div class="sag">{@render right?.()}</div>
</footer>

<style>
  .durum-cubugu {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: var(--statusbar-height);
    flex: 0 0 auto;
    padding: 0 var(--space-8);
    background: var(--bg-rail);
    border-top: var(--border-width) solid var(--border-subtle);
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .sag {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }
</style>
```

`ui/src/bilesenler/StatusItem.svelte`:

```svelte
<!-- Durum çubuğunda tek bir bilgi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon?: IkonAdi;
    tone?: 'ok' | 'warn' | 'danger' | 'info' | 'accent';
    /** Fareyle üzerine gelince görünen açıklama. */
    title?: string;
    class?: string;
    children?: Snippet;
  }

  let { icon, tone, title, class: sinif, children }: Props = $props();
</script>

<span class={['durum-ogesi', tone && `t-${tone}`, sinif]} {title}>
  {#if icon}<Icon name={icon} size={14} />{/if}
  {@render children?.()}
</span>

<style>
  .durum-ogesi {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .t-ok {
    color: var(--status-ok);
  }
  .t-warn {
    color: var(--status-warn);
  }
  .t-danger {
    color: var(--status-danger);
  }
  .t-info {
    color: var(--status-info);
  }
  .t-accent {
    color: var(--text-accent);
  }
</style>
```

- [ ] **Step 5: Vitrin**

`ui/vitrin.html`:

```html
<!doctype html>
<html lang="tr">
  <head>
    <meta charset="utf-8" />
    <title>Kalfa — bileşen vitrini</title>
  </head>
  <body class="kalfa-root">
    <div id="kok"></div>
    <script type="module" src="/src/vitrin/giris.ts"></script>
  </body>
</html>
```

`ui/src/vitrin/giris.ts`:

```ts
// Bileşen vitrini: yalnızca geliştirme sunucusunda (`npm run dev`,
// http://localhost:5173/vitrin.html). Derlemeye girmez.
import { mount } from 'svelte';
import '../stiller';
import Vitrin from './Vitrin.svelte';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');
mount(Vitrin, { target: kok });
```

`ui/src/vitrin/Vitrin.svelte`:

````svelte
<!--
  Bileşen vitrini: tasarım sistemiyle görsel karşılaştırma için (1c tasarımı
  §11.5). Örnek veriler burada kalır; uygulamaya girmez.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import Composer from '../bilesenler/Composer.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle } from '../bilesenler/diff';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import Input from '../bilesenler/Input.svelte';
  import Message from '../bilesenler/Message.svelte';
  import MetaList from '../bilesenler/MetaList.svelte';
  import NavItem from '../bilesenler/NavItem.svelte';
  import NavRail from '../bilesenler/NavRail.svelte';
  import NavSection from '../bilesenler/NavSection.svelte';
  import PanelHeader from '../bilesenler/PanelHeader.svelte';
  import Select from '../bilesenler/Select.svelte';
  import SessionRow from '../bilesenler/SessionRow.svelte';
  import StatusBar from '../bilesenler/StatusBar.svelte';
  import StatusItem from '../bilesenler/StatusItem.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import ThoughtPanel from '../bilesenler/ThoughtPanel.svelte';
  import TokenMeter from '../bilesenler/TokenMeter.svelte';
  import ToolCallCard from '../bilesenler/ToolCallCard.svelte';
  import { KIP_ADI, MODEL_PROFILI_ADI, m } from '../dil/tr';
  import { KIPLER } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';

  const MARKDOWN = [
    '## Yapılanlar',
    '',
    '`src/main.rs` içinde **iki** değişiklik var:',
    '',
    '- [x] okuma',
    '- [ ] yazma',
    '',
    '| Dosya | Satır |',
    '|:--|--:|',
    '| main.rs | 42 |',
    '',
    '```rust',
    'fn main() {',
    '    println!("merhaba");',
    '}',
    '```',
    '',
    '[EVREN](https://evren.example) · [zararlı](javascript:alert(1)) · <b>ham html</b>',
    '',
    '![rozet](https://img.shields.io/badge/x-y-green)',
  ].join('\n');

  const DIFF = [
    '--- a/src/main.rs',
    '+++ b/src/main.rs',
    '@@ -1,3 +1,4 @@',
    ' fn main() {',
    '-    println!("merhaba");',
    '+    let ad = "Kalfa";',
    '+    println!("merhaba {ad}");',
    ' }',
  ].join('\n');

  const modeller = [
    { value: 'qwen3.8-flash-next', label: m(MODEL_PROFILI_ADI.Hizli), hint: 'qwen3.8-flash-next' },
    { value: 'deepseek-v4-flash', label: m(MODEL_PROFILI_ADI.Dengeli), hint: 'deepseek-v4-flash' },
    { value: 'glm-5.3', label: m(MODEL_PROFILI_ADI.Derin), hint: 'glm-5.3' },
    { value: 'auto', label: m(MODEL_PROFILI_ADI.Otomatik), hint: 'auto' },
  ];
  const kipler = KIPLER.map((k) => ({ value: k, label: m(KIP_ADI[k]) }));

  let model = $state('glm-5.3');
  let kip = $state('Siki');
  let metin = $state('');
  let calisiyor = $state(false);
  let secili = $state(true);
  let girdi = $state('');
  const baglanti = (adres: string) => console.info('baglanti_ac', adres);
</script>

<div class="vitrin">
  <NavRail label="Menü">
    {#snippet header()}<strong class="marka">Kalfa</strong>{/snippet}
    <NavItem icon="message-square" active badge={3}>Oturumlar</NavItem>
    <NavItem icon="key-round">Anahtarı değiştir</NavItem>
    <NavSection label="Çalışma alanları">
      <NavItem icon="folder" depth={1}>harness</NavItem>
    </NavSection>
  </NavRail>

  <div class="liste">
    <PanelHeader icon="list" title="Oturumlar" subtitle="4 kayıt" />
    <SessionRow title="Diff görünümü" preview="C:\projeler\harness" time="14:28" status="running" statusLabel="Çalışıyor" active />
    <SessionRow title="Onay bekleyen" time="14:10" status="waiting" statusLabel="Onay bekliyor" />
    <SessionRow title="Bitti" time="dün" status="done" statusLabel="Tamamlandı" />
    <SessionRow title="Yarım kalan" time="dün" status="partial" statusLabel="Yarım kaldı" />
    <SessionRow title="Hata" time="pzt" status="failed" statusLabel="Hata" />
  </div>

  <main class="icerik">
    <PanelHeader icon="message-square" title="Bileşen vitrini" subtitle="1c-3a">
      {#snippet actions()}
        <IconButton icon="plus" label="Yeni oturum" />
        <IconButton icon="x" label="Oturumu kapat" />
      {/snippet}
    </PanelHeader>

    <div class="kaydirma">
      <section>
        <h3>Düğmeler ve rozetler</h3>
        <div class="satir">
          <Button variant="primary" iconLeft="send">Gönder</Button>
          <Button>İkincil</Button>
          <Button variant="ghost">Hayalet</Button>
          <Button variant="approve" iconLeft="check">Tümünü onayla</Button>
          <Button variant="caution">Seçilenleri onayla</Button>
          <Button variant="danger" iconLeft="x">Reddet</Button>
          <Button loading>Bekleniyor</Button>
          <Button disabled>Kapalı</Button>
        </div>
        <div class="satir">
          <Badge>nötr</Badge>
          <Badge tone="accent" icon="box">glm-5.3</Badge>
          <Badge tone="ok" dot>Bağlı</Badge>
          <Badge tone="warn">Onay bekliyor</Badge>
          <Badge tone="danger" shape="square">Hata</Badge>
          <Badge tone="info">Bilgi</Badge>
          <IconTile icon="file-text" />
          <IconTile icon="hexagon" tone="accent" />
          <IconTile icon="triangle-alert" tone="warn" size={40} />
        </div>
      </section>

      <section>
        <h3>Formlar</h3>
        <div class="satir">
          <Checkbox label="src/main.rs" hint="+2 −1" checked={secili} onchange={(v) => (secili = v)} />
          <Checkbox label="Tümünü seç" indeterminate />
          <Input bind:value={girdi} placeholder="Oturum ara" iconLeft="search" />
          <Input type="password" mono placeholder="evren_llm_…" invalid />
          <Select label="Model" icon="box" tone="accent" value={model} options={modeller} onchange={(v) => (model = v)} />
        </div>
        <Textarea placeholder="Ret gerekçesi" rows={2} />
      </section>

      <section>
        <h3>Sohbet</h3>
        <Message author="user" time="14:28">Diff görünümünü ekler misin?</Message>
        <Message time="14:29" streaming={calisiyor}>
          <Markdown metin={MARKDOWN} akista={calisiyor} baglantiAc={baglanti} />
        </Message>
        <ThoughtPanel>
          <Markdown metin="Önce dosyayı okumalıyım." baglantiAc={baglanti} />
        </ThoughtPanel>
        <ToolCallCard tool="Dosya okundu" target="src/main.rs" meta="3 satır" statusLabel="Tamamlandı">
          <CodeBlock code={'fn main() {\n    println!("merhaba");\n}'} lang="rust" highlight startLine={1} />
        </ToolCallCard>
        <ToolCallCard tool="Dosya yazılıyor" target="src/lib.rs" status="running" statusLabel="Çalışıyor" icon="file-pen" />
        <ToolCallCard tool="Dosya yazılamadı" target="src/lib.rs" status="denied" statusLabel="Reddedildi" icon="file-pen" />
        <ToolCallCard tool="Sonucu bilinmiyor" status="unknown" statusLabel="Sonucu bilinmiyor" />
        <Composer
          bind:value={metin}
          models={modeller}
          {model}
          modes={kipler}
          mode={kip}
          running={calisiyor}
          onsend={() => (calisiyor = true)}
          oncancel={() => (calisiyor = false)}
          onmodelchange={(v) => (model = v)}
          onmodechange={(v) => (kip = v)}
        />
      </section>

      <section>
        <h3>Veri</h3>
        {#each diffCozumle(DIFF) as parca, i (i)}
          <DiffView hunkHeader={parca.baslik} lines={parca.satirlar} />
        {/each}
        <Card title="Onay isteği" subtitle="2 dosya" icon="file-pen" tone="accent">
          <MetaList
            items={[
              { label: 'Ne', value: 'src/main.rs dosyasını düzenler' },
              { label: 'Nerede', value: 'C:\\projeler\\harness\\src\\main.rs', mono: true },
              { label: 'Model', value: 'glm-5.3', tone: 'accent' },
            ]}
          />
        </Card>
        <TokenMeter
          segments={[
            { label: 'İstem', value: 12840 },
            { label: 'Tamamlama', value: 2210, color: 'var(--blue-200)' },
            { label: 'Düşünce', value: 930, color: 'var(--green-200)' },
          ]}
        />
      </section>

      <section class="kalfa-light belge-ornegi">
        <h3>Açık tema: şart metni</h3>
        <Markdown metin={MARKDOWN} belge baglantiAc={baglanti} />
      </section>
    </div>

    <StatusBar label="Durum çubuğu">
      {#snippet left()}
        <StatusItem icon="coins" title="Son gözlenen bakiye">1.250,00 CR · 14:29</StatusItem>
        <StatusItem icon="cpu">glm-5.3</StatusItem>
      {/snippet}
      {#snippet right()}
        <StatusItem icon="shield" tone="accent">Sıkı</StatusItem>
        <StatusItem tone="ok">15.980 token</StatusItem>
      {/snippet}
    </StatusBar>
  </main>
</div>

<style>
  .vitrin {
    display: flex;
    height: 100%;
  }
  .marka {
    font: var(--type-title);
    color: var(--text-title);
  }
  .liste {
    width: var(--list-width);
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 8px;
    background: var(--bg-panel);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .icerik {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .kaydirma {
    flex: 1;
    overflow: auto;
    padding: var(--pad-panel);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 32px;
  }
  h3 {
    margin: 0;
    font: var(--type-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .satir {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  .belge-ornegi {
    padding: 20px;
    border-radius: var(--radius-xl);
    background: var(--bg-app);
    color: var(--text-body);
  }
</style>
````

- [ ] **Step 6: Doğrula**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: temiz; bütün testler (10 dosya, 102 test) geçer. Vitrin derlemeye girmez: `dist/vitrin.html` oluşmaz.

Run: `cd ui && npm run dev` ve tarayıcıda `http://localhost:5173/vitrin.html`
Expected: üç sütun (menü, oturum listesi, içerik) ve alttaki durum çubuğu görünür; etkin oturum satırının başlığı açık renklidir; Markdown mesajındaki Rust kod bloğu renklidir; `javascript:` bağlantısı düz metindir; `<b>` etiketi metin olarak görünür; rozet görseli yerine `[rozet]` yazar. Sunucuyu kapat.

- [ ] **Step 7: Commit**

```bash
git add ui/src/bilesenler ui/src/vitrin ui/vitrin.html
git commit -m "ui: sohbet ve gezinme bileşenleri; bileşen vitrini"
git push
```

---

### Task 7: Son doğrulama ve durum dosyası

**Files:**
- Modify: `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Temiz kurulumla doğrula**

Run:

```bash
cd ui
rm -rf node_modules dist
npm ci && npm run check && npm test && npm run build
git status --short .
```

Expected: `npm ci` hatasız; `svelte-check` 0 hata 0 uyarı; 10 test dosyası, 102 test geçer; `Paket denetimi temiz.`; `git status` `ui/` altında izlenmeyen dosya göstermez (`node_modules` ve `dist` yok sayılır).

- [ ] **Step 2: Durum dosyasını güncelle**

`docs/GELISTIRME_DURUMU.md` içinde "Devam eden aşama — 1c" bölümüne, "Tasarım belgesi" alt başlığından önce şu alt başlığı ekle. Tabloda yalnızca Adım 1'i koştuğun platformun sütununu gerçek çıktılarla doldur; öbür sütunda `(inceleme)` kalsın:

```markdown
### 1c-3a — arayüz temeli (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3a-arayuz-temeli.md`](superpowers/plans/2026-09-23-kalfa-1c-3a-arayuz-temeli.md).
`ui/` altında Vite + Svelte 5 + TypeScript iskeleti; tasarım tokenları ve
paketlenmiş fontlar; Türkçe katalog; komut sözleşmesinin tipleri ve
çağrıları; oturum durumu işlevi; güvenli Markdown ve Shiki; tasarım
sisteminin 1c bileşenleri ve geliştirmede açılan bileşen vitrini. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | (sonuç) | (sonuç) |
| `npm run check` | (sonuç) | (sonuç) |
| `npm test` | (sayılar) | (sayılar) |
| `npm run build` + paket denetimi | (sonuç) | (sonuç) |
```

- [ ] **Step 3: Commit**

```bash
git add docs/GELISTIRME_DURUMU.md
git commit -m "docs: 1c-3a arayüz temeli tamamlandı; doğrulama kaydı"
git push
```

- [ ] **Step 4 (inceleyici): Öbür platformda doğrulama, görsel karşılaştırma, birleştirme**

İnceleyici PR dalını alır; Adım 1'in komutlarını öbür platformda koşar; `npm run dev` ile vitrini açıp tasarım sistemiyle karşılaştırır (tokenlar, üç sütun, bileşenler); tablonun boş sütununu doldurup commit'ler ve inceleme temizse PR'ı birleştirir.

## Plan sonu kontrol listesi

- [ ] `npm ci`, `npm run check` (0 hata, 0 uyarı), `npm test`, `npm run build` iki platformda temiz.
- [ ] Derlenmiş HTML'de satır içi betik, stil özniteliği ve dış adres yok; şart penceresinin açılış paketi sohbet komutlarını içermez (paket denetimi).
- [ ] Kaynakta sabit `style="…"`, `{@html}`, `innerHTML`, `eval` yok (`csp.test.ts`).
- [ ] Markdown: ham HTML metin olarak; yalnızca http/https bağlantı; görsel yerine alt metin; kapanmamış kod bloğu; GFM tablosu (`markdown.test.ts`).
- [ ] Durum işlevi: birleştirme, kritik olaylar, onay isteği, tur sonu, paket boşluğu, geçmişle birleşme (`oturum.test.ts`); "işledim" çizimi beklemez.
- [ ] Kodda kullanılan her metin anahtarı katalogda (`dil.test.ts`); tokenlarda ad çakışması yok (`tokenlar.test.ts`).
- [ ] Komut ve argüman adları spec §6.1'deki gibi (`ipc.test.ts`).
- [ ] Vitrin tasarım sistemiyle karşılaştırıldı.
