// Bütçe kartı ve ayar formunun saf mantığı (5b tasarımı §7, §9): girdi
// doğrulama, kartın ilk önerisi ve sayaç metinleri. DOM ve ağ yoktur; Rust
// aynı kuralları yeniden denetler.
import type { ButceGorunumu, ButceSinirlari } from '../ipc/tipler';

/** Rust `AZAMI_SINIR` ile aynı: JavaScript'in güvenli tamsayı üst sınırı (2^53 - 1). */
export const AZAMI_SINIR = Number.MAX_SAFE_INTEGER;

/** Metni tam sayıya çevirir; yalnız rakam ve güvenli tamsayı aralığı kabul edilir. */
export function tamSayiCoz(metin: string): number | null {
  const temiz = metin.trim();
  if (!/^\d+$/.test(temiz)) return null;
  const sayi = Number(temiz);
  return Number.isSafeInteger(sayi) ? sayi : null;
}

export type ButceHataNedeni = 'bicim' | 'istek' | 'token';

export type ButceGirdisi =
  | { gecerli: true; sinirlar: ButceSinirlari }
  | { gecerli: false; neden: ButceHataNedeni };

/**
 * İki sınır girdisini doğrular. `alt` her sınırın kesin alt sınırıdır: kartta
 * bilinen tüketim (sınır onun üstünde olmalı), ayar formunda 0 (sınır 1'den
 * başlar). Değer `alt`'a eşit ya da altındaysa reddedilir.
 */
export function butceGirdisiDogrula(
  istekMetni: string,
  tokenMetni: string,
  alt: { istek: number; token: number },
): ButceGirdisi {
  const istek = tamSayiCoz(istekMetni);
  const token = tamSayiCoz(tokenMetni);
  if (istek === null || token === null) return { gecerli: false, neden: 'bicim' };
  if (istek <= alt.istek) return { gecerli: false, neden: 'istek' };
  if (token <= alt.token) return { gecerli: false, neden: 'token' };
  return { gecerli: true, sinirlar: { istek, token } };
}

/**
 * Kartın ilk önerisi: mevcut limitler. Limit tüketimin altında ya da eşitse
 * (engel o limitten geliyorsa) öneri tüketimin üstüne, bir limit kadar eklenir.
 */
export function kartOnerisi(g: ButceGorunumu): ButceSinirlari {
  const istek =
    g.sinirlar.istek > g.baslatilan
      ? g.sinirlar.istek
      : Math.min(AZAMI_SINIR, g.baslatilan + g.sinirlar.istek);
  const token =
    g.sinirlar.token > g.bilinen_token
      ? g.sinirlar.token
      : Math.min(AZAMI_SINIR, g.bilinen_token + g.sinirlar.token);
  return { istek, token };
}

/** Kartın girdileri için alt sınır: yeni limit bilinen tüketimin üstünde olmalı. */
export function kartAltSiniri(g: ButceGorunumu): { istek: number; token: number } {
  return { istek: g.baslatilan, token: g.bilinen_token };
}

/** Ayar formu için alt sınır: sınır 1 ile 2^53 - 1 arasında olmalı. */
export const AYAR_ALT_SINIRI = { istek: 0, token: 0 };
