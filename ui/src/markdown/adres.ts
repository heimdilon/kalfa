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
