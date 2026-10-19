// Ayar formunun saf sayı doğrulaması. DOM ve ağ yoktur; Rust aynı kuralları
// yeniden denetler.

/** Rust `AZAMI_SINIR` ile aynı: JavaScript'in güvenli tamsayı üst sınırı (2^53 - 1). */
export const AZAMI_SINIR = Number.MAX_SAFE_INTEGER;

/** Etkin bağlam penceresinin en küçük değeri (`kalfa_core::oturum::EN_AZ_ETKIN_PENCERE`). */
export const ETKIN_PENCERE_ALT_SINIRI = 32_000;

/** Metni tam sayıya çevirir; yalnız rakam ve güvenli tamsayı aralığı kabul edilir. */
export function tamSayiCoz(metin: string): number | null {
  const temiz = metin.trim();
  if (!/^\d+$/.test(temiz)) return null;
  const sayi = Number(temiz);
  return Number.isSafeInteger(sayi) ? sayi : null;
}

/** Etkin bağlam penceresini doğrular: 32_000 ile 2^53 - 1 arasında tam sayı. */
export function etkinPencereDogrula(metin: string): number | null {
  const deger = tamSayiCoz(metin);
  if (deger === null || deger < ETKIN_PENCERE_ALT_SINIRI) return null;
  return deger;
}
