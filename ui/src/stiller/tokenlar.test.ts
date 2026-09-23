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
