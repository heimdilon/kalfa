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
