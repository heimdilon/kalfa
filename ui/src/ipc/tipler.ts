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
  dizin: string;
  ag: boolean;
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
}

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

export interface KomutOzeti {
  cagri: CagriId;
  komut: string;
  durum: AracDurumu;
  degisiklikler?: DosyaDegisiklikleri;
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
  | { tip: 'IstekBilgisi'; istek_id: string | null; yonlendirilen_model: string | null }
  /** Geri alma önizlemesi (aşama 3 tasarımı §5.6). */
  | { tip: 'GeriAlmaPlani'; geri_alinan_tur: TurId; plan: GeriAlmaPlani; muhur: string }
  | { tip: 'TurGeriAlindi'; geri_alinan_tur: TurId; sonuc: GeriAlmaSonucu[] }
  /** Depo sınırı yüzünden bu turların önceki hâlleri silindi (§5.2). */
  | { tip: 'GeriAlmaVerisiSilindi'; turlar: TurId[] }
  /** Turun özeti; `Tamamlandi`'dan sonra gelir. */
  | { tip: 'TurOzeti'; ozet: TurOzeti };

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
