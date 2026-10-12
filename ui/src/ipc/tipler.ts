// Komut sözleşmesinin tipleri (1c tasarımı §6.1). JSON biçimi Rust'taki serde
// tanımlarından gelir: etiketli enum'lar `tip` alanı taşır, birim değerli
// enum'lar düz metindir, `u64` alanları sayıdır. Rust tarafı değişirse burası
// da aynı commit'te değişir.

/** UUID metni. */
export type OturumId = string;
export type TurId = string;
export type CagriId = string;
export type IslemId = string;

// ── kalfa-core ────────────────────────────────────────────────────────────

export type Kip = 'Inceleme' | 'Siki' | 'Proje' | 'Serbest';

/** Kiplerin sırası: sağa her geçiş yükseltmedir (1c tasarımı §6). */
export const KIPLER: readonly Kip[] = ['Inceleme', 'Siki', 'Proje', 'Serbest'];

export type AracDurumu =
  | 'Basarili'
  | 'Basarisiz'
  | 'Reddedildi'
  | 'IptalEdildi'
  | 'SonucuBilinmiyor';

export type BitisNedeni =
  | { tip: 'Normal' }
  | { tip: 'Iptal' }
  | { tip: 'Sinir'; aciklama: string }
  | { tip: 'Hata' };

export type AktarimDurmaNedeni = 'TuketiciYetismedi' | 'OlayCokBuyuk' | 'Diger';

export interface KrediGozlemi {
  /** Ondalık sayı metni. */
  kalan: string | null;
  tutulan: string | null;
  istek_id: string | null;
  /** Unix epoch milisaniyesi. */
  zaman_ms: number;
}

export type KabukBicimi = { tip: 'Program' } | { tip: 'Betik'; yorumlayici: string };

export type YalitimOzeti =
  | { tip: 'Yalitimli'; arka_uc: string; profil: string }
  | { tip: 'Yalitimsiz'; talimat: string };

/** Kabuk çağrısının onay kartında gösterilenler (aşama 2 tasarımı §10.2). */
export interface KabukTarifi {
  komut: string;
  bicim: KabukBicimi;
  /** Program biçiminde programın adı; kabuk kısayolu bununla eşleşir (3e). */
  program?: string;
  dizin: string;
  ag: boolean;
  ag_hostlari?: string[];
  yalitim: YalitimOzeti;
  ilk_cagri: boolean;
  /** Çalışma alanı yalıtım için hazırlanacak (Windows, aşama 2 tasarımı §9.4). */
  alan_hazirligi: boolean;
}

export interface OnayOgesi {
  cagri: CagriId;
  arac: string;
  aciklama: string;
  hedefler: string[];
  diff: string | null;
  uyarilar: string[];
  muhur: string;
  kabuk: KabukTarifi | null;
  /** Onayı Rust yerel pencerede ayrıca sorar (aşama 2 tasarımı §5.2). */
  yerel_onay: boolean;
  /** Kabuk öğesinde: oturumda onaysız değişenler (güvenlik analizi Bulgu 3). */
  onaysiz_degisiklikler?: OnaysizDegisiklikler;
  /** Geri alınabilir mi (aşama 3 tasarımı §5.11); yalnızca okuyan öğede yok. */
  geri_alinabilir?: GeriAlinabilirlik;
  /** Onay kartında "Veri çıkışı" satırı (aşama 3b, tasarım §9.3). */
  veri_cikisi?: VeriCikisi;
  /** "Bu oturumda izin ver" teklifi (aşama 3 tasarımı §9.1); yalnızca uygun öğede. */
  kisayol?: KisayolTeklifi;
  /** Eklenti aracında eklentinin tarifi; ilk gönderim kartı buna bakar (aşama 4 §8.4). */
  eklenti?: EklentiTarifi;
}

/** Eklenti aracının kimliği ve yetenekleri (aşama 4 tasarımı §8.1). */
export interface EklentiTarifi {
  ad: string;
  surum: string;
  /** `.wasm`'ın SHA-256'sı. */
  ozet: string;
  globlar: string[];
  evren_uclari: string[];
  /** Bu çalışma alanında EVREN'e ilk gönderim; onay yerel penceredir. */
  ilk_gonderim: boolean;
  dosya_sayisi: number;
  bayt: number;
}

export type KisayolTeklifi =
  | { tip: 'Dosya'; arac: string; dizin: string }
  | { tip: 'Kabuk'; program: string; dizin: string };

/** Oturumdaki bir kısayol (§9.1). */
export interface Kisayol {
  kimlik: number;
  teklif: KisayolTeklifi;
}

export type VeriCikisi =
  | 'DosyaIcerigi'
  | 'KomutCiktisi'
  | 'AramaSonuclari'
  | 'DosyaAdlari'
  | 'YalnizcaYazildi'
  /** Eklentinin ürettiği metin (aşama 4 tasarımı §8.1). */
  | 'EklentiCiktisi';

export interface OnaysizDegisiklikler {
  yollar: string[];
  /** Onaysız bir komut da çalıştı; değiştirdiği dosyalar bilinmiyor. */
  komut_calisti: boolean;
  derleme_dosyalari: string[];
}

export type CiktiAkisi = 'Standart' | 'Hata';

/** Geri almada bir dosyaya ne olacağı (aşama 3 tasarımı §5.7). */
export type GeriAlmaEylemi =
  | { tip: 'GeriYukle'; ozet: string }
  | { tip: 'Sil' }
  | { tip: 'ZatenOnceki' }
  | { tip: 'Cakisma'; neden: CakismaNedeni }
  | { tip: 'GeriAlinamaz'; neden: GeriAlinamazNedeni };

export type CakismaNedeni =
  | { tip: 'SonrakiTur'; tur: TurId }
  | { tip: 'Komut' }
  | { tip: 'BasarisizCagri' }
  | { tip: 'Elle' }
  | { tip: 'HardLink' }
  | { tip: 'Okunamadi'; hata: string };

export type GeriAlinamazNedeni = 'Buyuk' | 'VeriSilindi' | 'Kayitsiz' | 'Kabuk';

/** Değişikliğin geri alınıp alınamayacağı; bilgiyi çekirdek verir. */
export type GeriAlinabilirlik = { tip: 'Evet' } | { tip: 'Hayir'; neden: GeriAlinamazNedeni };

export interface DosyaOzeti {
  yol: string;
  islem: 'Olusturuldu' | 'Degisti';
  eklenen: number;
  silinen: number;
  sonucu_bilinmiyor: boolean;
  geri_alinabilir: GeriAlinabilirlik;
}

export type DegisiklikTuru = 'Eklendi' | 'Degisti' | 'Silindi';

/** Bir komutun çalışırken değişen dosyaları; geri alınamaz (K2). */
export interface DosyaDegisiklikleri {
  /** En çok 200 dosya; bağımlılık dizinlerinin dışındakiler önce. */
  dosyalar: { yol: string; tur: DegisiklikTuru }[];
  /** Üst düzey dizinlerin sayıları. */
  dizinler?: { dizin: string; eklenen: number; degisen: number; silinen: number }[];
  /** Bütün değişikliklerin sayısı; `dosyalar`'dan büyükse liste kesildi. */
  toplam: number;
  /** Tarama sınırı aşıldı; liste eksik olabilir. */
  eksik: boolean;
}

export interface AgHostOzeti {
  host: string;
  port: number;
  baglanti_sayisi: number;
  giden_bayt: number;
  gelen_bayt: number;
  reddedilenler: number;
}

export interface AgKomutOzeti {
  hostlar: AgHostOzeti[];
  toplam_giden: number;
  toplam_gelen: number;
  reddedilen_sayisi: number;
}

export interface KomutOzeti {
  cagri: CagriId;
  komut: string;
  durum: AracDurumu;
  degisiklikler?: DosyaDegisiklikleri;
  ag?: AgKomutOzeti;
}

export interface IstekOzeti {
  sayi: number;
  istem: number;
  tamamlama: number;
  dusunce: number;
}

/** Tur özeti (aşama 3 tasarımı §5.5). */
export interface TurOzeti {
  tur: TurId;
  dosyalar: DosyaOzeti[];
  komutlar: KomutOzeti[];
  istekler: IstekOzeti;
  guvenlik_uyarisi: number;
}

export interface PlanOgesi {
  yol: string;
  eylem: GeriAlmaEylemi;
  simdiki: string | null;
  uyarilar: string[];
  diff: string | null;
}

export interface GeriAlmaPlani {
  ogeler: PlanOgesi[];
  /** Turun dosya değiştiren komutları; geri alınmaz, yalnızca listelenir. */
  komutlar?: KomutOzeti[];
  veri_silindi: boolean;
}

export interface GeriAlmaSonucu {
  yol: string;
  eylem: GeriAlmaEylemi;
  /** Yürütmede yazılamadıysa nedeni. */
  hata?: string;
}

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };

export type Olay =
  | { tip: 'TurBasladi' }
  | { tip: 'TokenParcasi'; metin: string }
  | { tip: 'DusunceParcasi'; metin: string }
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  | { tip: 'AracSonucu'; cagri: CagriId; arac: string; durum: AracDurumu; icerik: string }
  /** Aracın bildirdiği güvenlik uyarısı; `AracSonucu`'ndan sonra gelir. */
  | { tip: 'GuvenlikUyarisi'; cagri: CagriId; mesaj: string }
  | {
      tip: 'AracCiktisiParcasi';
      cagri: CagriId;
      akis: CiktiAkisi;
      metin: string;
      /** Bu parçadan önce ekran yetişemediği için atılan baytlar. */
      atlanan_bayt: number;
    }
  | { tip: 'OnayGerekiyor'; islem: IslemId; ogeler: OnayOgesi[] }
  | { tip: 'KrediGozlemi'; gozlem: KrediGozlemi }
  | { tip: 'TokenKullanimi'; istem: number; tamamlama: number; dusunce: number }
  | { tip: 'SartOnayiGerekiyor'; surum: number; metin: string }
  | { tip: 'Hata'; mesaj: string }
  | { tip: 'Tamamlandi'; neden: BitisNedeni }
  | { tip: 'AktarimDurdu'; neden: AktarimDurmaNedeni }
  | {
      tip: 'IstekBilgisi';
      istek_id: string | null;
      yonlendirilen_model: string | null;
      amac?: string;
      basarili?: boolean;
    }
  /** Geri alma önizlemesi (aşama 3 tasarımı §5.6). */
  | { tip: 'GeriAlmaPlani'; geri_alinan_tur: TurId; plan: GeriAlmaPlani; muhur: string }
  | { tip: 'TurGeriAlindi'; geri_alinan_tur: TurId; sonuc: GeriAlmaSonucu[] }
  /** Depo sınırı yüzünden bu turların önceki hâlleri silindi (§5.2). */
  | { tip: 'GeriAlmaVerisiSilindi'; turlar: TurId[] }
  /** Turun özeti; `Tamamlandi`'dan sonra gelir. */
  | { tip: 'TurOzeti'; ozet: TurOzeti }
  /** Gönderim öncesi önizleme (aşama 3b). */
  | { tip: 'GonderimOnizlemesi'; islem: IslemId; ogeler: OnizlemeOgesi[]; neden: OnizlemeNedeni }
  /** Denetçi başladı (aşama 3c). */
  | { tip: 'DenetimBasladi'; cagri: CagriId }
  /** Denetçi sonucu (aşama 3c). */
  | { tip: 'DenetimSonucu'; cagri: CagriId; sonuc: DenetimSonucu }
  | { tip: 'DenetimAtlandi'; cagri: CagriId }
  | { tip: 'Kisayollar'; liste: Kisayol[] }
  /** Eski araç çıktıları özetlendi (5a tasarımı §12); geçmişte gösterilmez. */
  | { tip: 'BaglamOzetlendi'; adet: number; yer_tutucu: number; once: number; sonra: number };

export type OnizlemeNedeni = 'Onizleme' | 'Denetim' | 'Eklenti';

export type DenetimKipi = 'Kapali' | 'Uyar' | 'Kati';

/** Denetçi kipi ve `ayarlar.json`'ın durumu (3c Görev 7). */
export interface DenetciDurumu {
  kip: DenetimKipi;
  ayar_hatasi?: string;
}

/** Kullanıcının ağ izin listesindeki bir host (aşama 3 tasarımı §8.3). */
export interface AgHostu {
  host: string;
  port: number;
  ozel_ag: boolean;
}

/** Ayarlar > "Gizli dosyalar" (3e). Varsayılanlar salt okunurdur. */
export interface GizliAyarGorunumu {
  varsayilanlar: string[];
  ek_kaliplar: string[];
  gizli_sayilmayanlar: string[];
  ayar_hatasi?: string;
}

export type DenetimSonucu =
  | { tip: 'Guvenli' }
  | { tip: 'Supheli'; kategori: string; gerekce: string }
  | { tip: 'Bilinmiyor'; neden: string };

export interface OnizlemeOgesi {
  cagri: CagriId;
  kaynak: string;
  bayt: number;
  denetim?: string;
  /** Eklenti isteğinin özeti; yalnızca `neden: 'Eklenti'` öğesinde (aşama 4). */
  eklenti?: EklentiIstekOzeti;
}

export interface Zarf<T> {
  sema: number;
  oturum: OturumId;
  tur: TurId | null;
  sira: number;
  yuk: T;
}

export type KanalMesaji =
  | { tip: 'Olaylar'; no: number; zarflar: Zarf<Olay>[] }
  | { tip: 'Kapandi'; no: number };

export type GorunumOgesi =
  | { tip: 'KullaniciMesaji'; sira: number; tur: TurId | null; metin: string }
  | {
      tip: 'Yanit';
      sira: number;
      tur: TurId | null;
      metin: string | null;
      dusunce: string | null;
    }
  | {
      tip: 'AracCagrisi';
      sira: number;
      tur: TurId | null;
      model_cagri_id: string;
      arac: string;
      argumanlar: string;
      /** `null`: sonuç kaydı yok, çağrı hiç başlamadı. */
      durum: AracDurumu | null;
      sonuc: string | null;
      denetim?: DenetimSonucu | null;
    }
  | { tip: 'TurSonu'; sira: number; tur: TurId | null; neden: BitisNedeni }
  | { tip: 'GuvenlikUyarisi'; sira: number; tur: TurId | null; mesaj: string }
  | {
      tip: 'GeriAlma';
      sira: number;
      tur: TurId | null;
      geri_alinan_tur: TurId;
      sonuc: GeriAlmaSonucu[];
      /** Başladı kaydı var, bitti kaydı yok. */
      yarim: boolean;
    }
  /** Geri almanın modele bildirimi; kullanıcı balonu değildir. */
  | { tip: 'KalfaNotu'; sira: number; tur: TurId | null; metin: string }
  | { tip: 'TurOzeti'; sira: number; tur: TurId | null; ozet: TurOzeti };

export interface GecmisSayfasi {
  ogeler: GorunumOgesi[];
  daha_eski_var: boolean;
}

export interface DurumOzeti {
  model: string | null;
  kip: Kip | null;
  yonlendirilen_model: string | null;
  son_kredi: KrediGozlemi | null;
  istem: number;
  tamamlama: number;
  dusunce: number;
  onizleme_acik?: boolean;
  denetci_kipi?: DenetimKipi;
}

export type OturumDurumu =
  | { tip: 'Bos' }
  | { tip: 'Tamamlandi' }
  | { tip: 'IptalEdildi' }
  | { tip: 'Hata' }
  | { tip: 'YarimKaldi' }
  | { tip: 'Okunamadi'; neden: string };

export interface SartMetni {
  surum: number;
  /** EVREN'in `is_material` bilgisi. */
  esasli: boolean;
  metin: string;
}

// ── kalfa-uygulama ────────────────────────────────────────────────────────

export type ModelProfili = 'Hizli' | 'Dengeli' | 'Derin' | 'Otomatik';

export interface ModelSecenegi {
  kimlik: string;
  profil: ModelProfili;
}

export type DepoTuru = 'Kalici' | 'Bellekte';

export type YalitimYoklukNedeni =
  | 'BwrapYok'
  | 'KullaniciAdAlaniKapali'
  | 'KurulumGerekli'
  | 'KurulumBozuk'
  | 'Desteklenmiyor';

export type YalitimDurumu =
  | { tip: 'Hazir'; arka_uc: string; surum: string }
  | {
      tip: 'Yok';
      neden: YalitimYoklukNedeni;
      /** Geliştiriciye ya da BT'ye yönelik talimat. */
      talimat: string;
      /** BT için teknik ayrıntı: düşen denetim ve hata (§9.8). */
      ayrinti: string;
    };

/** "Onar"ın sonucu (aşama 2 tasarımı §9.8). */
export type OnarimSonucu =
  | { tip: 'Tamam'; yalitim: YalitimDurumu }
  | { tip: 'Vazgecildi' }
  | { tip: 'Basarisiz'; mesaj: string }
  | { tip: 'KuruluDegil'; talimat: string };

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
  anahtar_deposu: DepoTuru;
  modeller: ModelSecenegi[];
  varsayilan_model: string;
  son_oturum: OturumId | null;
  yalitim: YalitimDurumu;
  /** Yalıtım yokken komutlar yalıtımsız çalışabilir mi (aşama 2 tasarımı §9.8). */
  yalitimsiz_izinli: boolean;
  /** `linux`, `windows`, `macos`: araç zinciri dizinleri yalnızca Linux'ta. */
  platform: string;
}

export interface OturumSatiri {
  oturum: OturumId | null;
  dosya: string;
  baslik: string | null;
  calisma_alani: string | null;
  son_degisiklik_ms: number;
  durum: OturumDurumu;
}

export interface CalismaAlani {
  kimlik: number;
  yol: string;
}

/** Windows yalıtımı için hazırlanmış çalışma alanı. */
export interface YalitimCalismaAlani {
  kimlik: string;
  yol: string;
}

export type SaltOkunurNedeni =
  | { tip: 'CalismaAlaniYok'; yol: string }
  | { tip: 'CalismaAlaniKayitsiz' };

export interface OturumAcilisi {
  oturum: OturumId;
  calisma_alani: string | null;
  gecmis: GecmisSayfasi;
  ozet: DurumOzeti;
  salt_okunur: SaltOkunurNedeni | null;
}

export interface OnayGirdisi {
  cagri: CagriId;
  muhur: string;
  karar: OnayKarari;
  /** Kartın önerdiği kısayolu kabul et (§9.1); yoksa `false`. */
  kisayol?: boolean;
}

export type AnahtarNedeni = 'Yok' | 'KayitliGecersiz' | 'Degistir';

export type SartPenceresiDurumu =
  | { tip: 'AnahtarGerekli'; neden: AnahtarNedeni }
  | { tip: 'SartGerekli'; metin: SartMetni }
  | { tip: 'Kilitli' }
  | { tip: 'Tamam' };

export type AnahtarSonucu =
  | { tip: 'Gecersiz' }
  | { tip: 'Ulasilamadi' }
  | { tip: 'Kaydedildi'; depo: DepoTuru; sonraki: SartPenceresiDurumu };

export type KabulYaniti =
  | { tip: 'Kabul' }
  | { tip: 'YeniMetin'; metin: SartMetni }
  | { tip: 'Kilitli' };

export type HataKodu =
  | 'OturumYok'
  | 'KayitMesgul'
  | 'SaltOkunur'
  | 'GecersizGirdi'
  | 'Evren'
  | 'Ic';

export interface KomutHatasi {
  kod: HataKodu;
  mesaj: string;
}

export interface GonderilenOge {
  kaynak: string;
  sira?: number | null;
  bayt: number;
}

export type IstekAsamasi = 'Baglanti' | 'Yanit';

export type IstekSonucu =
  | { tip: 'Basarili' }
  | { tip: 'Hata'; asama: IstekAsamasi; mesaj: string }
  /** Ağa çıkmadı: kullanıcı göndermedi ya da eklenti sınırı doldu (aşama 4 §9.4, §9.5). */
  | { tip: 'Gonderilmedi'; neden: string };

/** Eklentilere açık EVREN ucu (aşama 4 tasarımı §9.1). */
export type EkUc = 'Gomme' | 'Siralama';

/**
 * Eklentinin EVREN isteğinin özeti (aşama 4 tasarımı §9.3). Metnin kendisi
 * kayda yazılmaz; içeriğin kaynağı `dosyalar` listesiyle anlatılır.
 */
export interface EklentiIstekOzeti {
  eklenti: string;
  uc: EkUc;
  model: string;
  metin_sayisi: number;
  bayt: number;
  /** Metinlerin SHA-256 özeti, onaltılık. */
  ozet: string;
  /** En çok 1 000 dosya yolu. */
  dosyalar: string[];
  dosya_fazlasi: number;
}

export interface GunlukToken {
  istem: number;
  tamamlama: number;
  dusunce: number;
}

export interface GunlukSatiri {
  sira: number;
  tur?: TurId | null;
  zaman?: number | null;
  amac: string;
  model?: string | null;
  yonlendirilen_model?: string | null;
  istek_id?: string | null;
  token?: GunlukToken | null;
  sonuc: IstekSonucu;
  yeni: GonderilenOge[];
  onceki: number;
  /** Eklenti isteğinin özeti; yalnızca eklenti satırlarında (aşama 4). */
  eklenti?: EklentiIstekOzeti | null;
}

export interface GunlukSayfasi {
  satirlar: GunlukSatiri[];
  daha_eski_var: boolean;
}


// ── Eklentiler (aşama 4 tasarımı §13, §14) ───────────────────────────────

export type EklentiDurumu =
  | { tip: 'Yuklendi' }
  | { tip: 'DevreDisi' }
  | { tip: 'Yuklenemedi'; neden: string };

/** Modelin gördüğü araç; `ad` örneğin `kalfa_rag__anlamsal_ara`. */
export interface EklentiAraci {
  ad: string;
  aciklama: string;
}

/** Bir eklentinin bir çalışma alanındaki durumu. `yol` yalnızca gösterilir. */
export interface EklentiAlani {
  kimlik: string;
  yol: string;
  acik: boolean;
  gonderim_izni_ms: number | null;
  depo_bayt: number;
}

export interface EklentiSatiri {
  ad: string;
  surum: string;
  aciklama: string;
  yerlesik: boolean;
  durum: EklentiDurumu;
  sha256: string;
  kaynak: string | null;
  kurulum_ms: number | null;
  yetenek: string | null;
  araclar: EklentiAraci[];
  alanlar: EklentiAlani[];
}

export type KurulumSonucu =
  | {
      tip: 'Kuruldu';
      ad: string;
      surum: string;
      guncelleme: boolean;
      izinler_silindi: boolean;
    }
  | { tip: 'Vazgecildi' }
  | { tip: 'Basarisiz'; mesaj: string };

/** Arayüz yol göndermez: açık oturumun alanı ya da kayıttaki kimlik. */
export type AlanHedefi = { tip: 'Oturum'; oturum: OturumId } | { tip: 'Kimlik'; kimlik: string };

export interface OturumEklentisi {
  ad: string;
  yerlesik: boolean;
  acik: boolean;
  kullanilamaz: boolean;
  gonderim_izni_ms: number | null;
}

/** EVREN'in günlük token sınırı doldu (aşama 4c). */
export interface GunlukSinirDurumu {
  sifirlanma: string | null;
}
