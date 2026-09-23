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
