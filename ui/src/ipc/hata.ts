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
