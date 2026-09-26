// Arayüzün Türkçe metin kataloğu (1c tasarımı §9.4). Kurallar: "siz" dili;
// ünlem ve emoji yok; biten iş "Tamamlandı" ile özetlenir; hata metni ne
// olduğunu ve ne yapılacağını söyler. Değişkenler `{ad}` biçimindedir.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  Kip,
  ModelProfili,
  YalitimYoklukNedeni,
} from '../ipc/tipler';

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
  'uygulama.slogan': 'Daha iyi yazılım, daha aydınlık yarınlar.',
  'uygulama.alt_not': 'Küçük adımlar, büyük işler.',
  'genel.kapat': 'Kapat',
  'zaman.dun': 'Dün',

  'menu.ad': 'Menü',
  'menu.oturumlar': 'Oturumlar',
  'menu.ayarlar': 'Ayarlar',
  'menu.anahtar': 'Anahtarı değiştir',

  'liste.baslik': 'Oturumlar',
  'liste.yeni': 'Yeni oturum',
  'liste.ara': 'Oturumlarda ara…',
  'liste.bos': 'Henüz oturum yok. Yeni bir oturumla başlayın.',
  'liste.sonuc_yok': 'Aramaya uyan oturum yok.',
  'liste.grup.bugun': 'Bugün',
  'liste.grup.dun': 'Dün',
  'liste.grup.hafta': 'Bu hafta',
  'liste.grup.eski': 'Daha eski',

  'oturum.yeni': 'Yeni oturum',
  'oturum.durum.calisiyor': 'Çalışıyor',
  'oturum.durum.onay': 'Onay bekliyor',
  'oturum.durum.sart': 'Şart onayı bekliyor',
  'oturum.durum.tamamlandi': 'Tamamlandı',
  'oturum.durum.iptal': 'İptal edildi',
  'oturum.durum.hata': 'Hatayla bitti',
  'oturum.durum.yarim': 'Yarım kaldı',
  'oturum.durum.okunamadi': 'Okunamadı',
  'oturum.durum.bos': 'Boş',

  'konusma.ad': 'Konuşma',
  'konusma.kapat': 'Oturumu kapat',
  'konusma.yeniden_ac': 'Yeniden aç',
  'konusma.eskiler': 'Daha eski mesajları göster',
  'konusma.bos': 'Bu oturumda henüz mesaj yok.',
  'konusma.kapandi': 'Oturum kapandı. Devam etmek için yeniden açın.',
  'konusma.karsilama.baslik': 'Bir oturum seçin ya da yeni bir oturum başlatın.',
  'konusma.karsilama.metin':
    'Kalfa seçtiğiniz klasörde çalışır. Dosya yazmadan önce onayınızı ister.',

  'arac.dosya_oku.suruyor': 'Dosya okunuyor',
  'arac.dosya_oku.bitti': 'Dosya okundu',
  'arac.dosya_oku.ad': 'Dosya okuma',
  'arac.dosya_listele.suruyor': 'Dizin listeleniyor',
  'arac.dosya_listele.bitti': 'Dizin listelendi',
  'arac.dosya_listele.ad': 'Dizin listeleme',
  'arac.icerik_ara.suruyor': 'İçerik aranıyor',
  'arac.icerik_ara.bitti': 'İçerik arandı',
  'arac.icerik_ara.ad': 'İçerik arama',
  'arac.dosya_yaz.suruyor': 'Dosya yazılıyor',
  'arac.dosya_yaz.bitti': 'Dosya yazıldı',
  'arac.dosya_yaz.ad': 'Dosya yazma',
  'arac.dosya_duzenle.suruyor': 'Dosya düzenleniyor',
  'arac.dosya_duzenle.bitti': 'Dosya düzenlendi',
  'arac.dosya_duzenle.ad': 'Dosya düzenleme',
  'arac.kabuk.suruyor': 'Komut çalışıyor',
  'arac.kabuk.bitti': 'Komut çalıştı',
  'arac.kabuk.ad': 'Komut',

  'terminal.etiket': 'Komut çıktısı',
  'terminal.atilan': 'En eski {sayi} satır gösterilmiyor.',
  'terminal.kesik': 'Satırın devamı gösterilmiyor; model çıktının tamamını alır.',

  'onay.kart.baslik': 'Onay isteği: {sayi} değişiklik',
  'onay.kart.incele': 'İncele',
  'onay.kart.yanitlandi': 'Yanıtlandı',
  'onay.kart.gecersiz': 'Tur bitti; karar gerekmiyor',
  'onay.kart.uyari': '{sayi} uyarı',
  'onay.sayfa.baslik': 'Kalfa {sayi} değişiklik yapmak istiyor.',
  'onay.sayfa.alt':
    'Devam etmeden önce değişiklikleri inceleyin. Seçmediğiniz değişiklikler gerekçenizle reddedilir.',
  'onay.sayfa.tumunu_sec': 'Tümünü seç',
  'onay.sayfa.secili': '{secili} / {toplam} seçili',
  'onay.sayfa.goster': '{ad} değişikliğini göster',
  'onay.sayfa.diff_yok': 'Bu değişiklik için önizleme yok.',
  'onay.sayfa.ne': 'Ne',
  'onay.sayfa.nerede': 'Nerede',
  'onay.sayfa.uyarilar': 'Uyarılar',
  'onay.sayfa.gerekce': 'Reddetme nedeni (isteğe bağlı)',
  'onay.sayfa.gerekce_yer': 'Reddetme nedeninizi buraya yazabilirsiniz…',
  'onay.sayfa.tumunu_onayla': 'Tümünü onayla',
  'onay.sayfa.secilenleri_onayla': 'Seçilenleri onayla',
  'onay.sayfa.reddet': 'Reddet',
  'onay.kabuk.komut': 'Komut',
  'onay.kabuk.betik': 'Betik ({yorumlayici})',
  'onay.kabuk.dizin': 'Dizin',
  'onay.kabuk.ag': 'Ağ',
  'onay.kabuk.ag_kapali': 'Kapalı',
  'onay.kabuk.ag_acik': 'Açık: komut internete erişebilecek',
  'onay.kabuk.yalitim': 'Yalıtım',
  'onay.kabuk.yalitimli': 'Yalıtımlı ({arka_uc})',
  'onay.kabuk.yalitimsiz': 'Yalıtımsız',
  'onay.kabuk.yerel':
    'Yalıtımsız komutlar ve çalışma alanının yalıtım için hazırlığı, onayınızdan sonra yerel bir pencerede bir kez daha sorulur.',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
  'sart.kart.bekliyor':
    'EVREN, şartlar kabul edilene kadar isteği yanıtlamıyor. Tur, şart penceresindeki kararınızı bekliyor.',
  'sart.kart.cozuldu': 'Şart kararı verildi.',
  'sart.kart.ac': 'Şart penceresini aç',

  'hata.kart.baslik': 'Tur hatayla bitti',
  'aktarim.baslik': 'Aktarım durdu',

  'durum.ad': 'Durum çubuğu',
  'durum.hazir': 'Hazır',
  'durum.calisiyor': 'Çalışıyor',
  'durum.baglanti_yok': 'Bağlantı yok',
  'durum.anahtar_bellekte': 'Anahtar bellekte',
  'durum.bakiye': '{kalan} kredi · {saat}',
  'durum.bakiye_ipucu': 'Son gözlenen bakiye ve gözlem zamanı',
  'durum.token': '{sayi} token',
  'durum.token_ipucu': 'İstem {istem} · Tamamlama {tamamlama} · Düşünce {dusunce}',
  'durum.model_ipucu': 'Yönlendirilen model',
  'durum.yalitimli': 'Yalıtımlı',
  'durum.yalitimsiz': 'Yalıtımsız',
  'durum.yalitimli_ipucu': 'Komutlar yalıtımda çalışıyor ({arka_uc})',

  'ayarlar.baslik': 'Ayarlar',
  'ayarlar.yalitim': 'Yalıtım',
  'ayarlar.yalitim.hazir': 'Yalıtımlı: {arka_uc} {surum}',
  'ayarlar.yalitim.hazir_aciklama':
    'Komutlar yalıtımda çalışır. Ev dizininiz ve diğer çalışma alanları görünmez, çalışma alanı dışına yazılamaz, ağ kapalıdır; ağ isteyen komut ayrıca onayınıza sunulur.',
  'ayarlar.yalitim.yok': 'Yalıtımsız: {neden}',
  'ayarlar.yalitim.neden.BwrapYok': 'bubblewrap bulunamadı',
  'ayarlar.yalitim.neden.KullaniciAdAlaniKapali': 'sistem kullanıcı ad alanlarını kısıtlıyor',
  'ayarlar.yalitim.neden.KurulumGerekli': 'yalıtımın kurulması gerekiyor',
  'ayarlar.yalitim.neden.KurulumBozuk': 'yalıtımın kurulumu bozuk',
  'ayarlar.yalitim.neden.Desteklenmiyor': 'bu sistemde kullanılamıyor',
  'ayarlar.yalitim.yeniden_dene': 'Yeniden dene',
  'ayarlar.arac_zinciri': 'Araç zinciri dizinleri',
  'ayarlar.arac_zinciri.aciklama':
    "Yalıtımda salt okunur görünen dizinler. Cargo ve rustup araç zincirleri kendiliğinden görünür; başka bir yere kurulan araçlar (örneğin /opt/node) için dizini buradan ekleyin. Ev dizininiz ve Kalfa'nın veri dizini eklenemez.",
  'ayarlar.arac_zinciri.ekle': 'Dizin ekle',
  'ayarlar.arac_zinciri.kaldir': '{yol} dizinini kaldır',
  'ayarlar.arac_zinciri.bos': 'Eklenmiş dizin yok.',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
  'duyuru.sart': 'EVREN şartlarının onayı bekleniyor.',
  'duyuru.tur_sonu': 'Tur bitti.',

  'anahtar.baslik': 'EVREN API anahtarı',
  'anahtar.aciklama.Yok':
    "Kalfa EVREN'e sizin anahtarınızla bağlanır. Anahtarınızı EVREN portalından alabilirsiniz.",
  'anahtar.aciklama.KayitliGecersiz':
    'Kayıtlı anahtar artık geçerli değil. EVREN portalından yeni anahtarınızı alıp girin.',
  'anahtar.aciklama.Degistir': 'Yeni anahtarınızı girin. Açık oturumlar yeni anahtarı kullanır.',
  'anahtar.etiket': 'Anahtar',
  'anahtar.not':
    'Anahtar işletim sisteminin kimlik deposunda saklanır. Kalfa onu arayüze geri göndermez.',
  'anahtar.kaydet': 'Kaydet',

  'sart.baslik': 'EVREN kullanım şartları',
  'sart.surum': 'Sürüm {surum}',
  'sart.esasli': 'Esaslı değişiklik',
  'sart.esasli_degil': 'Küçük değişiklik',
  'sart.aciklama':
    "Kalfa'yı kullanmak için EVREN'in şartlarını kabul etmeniz gerekir. Metin EVREN'den geldiği gibi gösterilir.",
  'sart.kabul': 'Kabul ediyorum',
  'sart.reddet': 'Kabul etmiyorum',

  'kilitli.baslik': 'Şartlar kabul edilmedi',
  'kilitli.metin':
    "EVREN, şartlar kabul edilmeden hiçbir isteği yanıtlamıyor. Bekleyen turlar iptal edildi; oturumlarınız kayıtlı. Anahtarınız saklı kalır; Kalfa'yı bir sonraki açışınızda şartlar yeniden gösterilir.",
  'kilitli.goster': 'Metni yeniden göster',

  'sartlar.tamam': 'Tamamlandı. Kalfa açılıyor.',
  'sartlar.alt_not': 'Kalfa dikkatli çalışır. Kontrol her zaman sizdedir.',
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

export const YALITIM_NEDENI: Record<YalitimYoklukNedeni, MetinAnahtari> = {
  BwrapYok: 'ayarlar.yalitim.neden.BwrapYok',
  KullaniciAdAlaniKapali: 'ayarlar.yalitim.neden.KullaniciAdAlaniKapali',
  KurulumGerekli: 'ayarlar.yalitim.neden.KurulumGerekli',
  KurulumBozuk: 'ayarlar.yalitim.neden.KurulumBozuk',
  Desteklenmiyor: 'ayarlar.yalitim.neden.Desteklenmiyor',
};
