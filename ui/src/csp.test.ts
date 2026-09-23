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
