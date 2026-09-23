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
