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
}

export type CiktiAkisi = 'Standart' | 'Hata';

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };

export type Olay =
  | { tip: 'TurBasladi' }
  | { tip: 'TokenParcasi'; metin: string }
  | { tip: 'DusunceParcasi'; metin: string }
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  | { tip: 'AracSonucu'; cagri: CagriId; arac: string; durum: AracDurumu; icerik: string }
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
  | { tip: 'IstekBilgisi'; istek_id: string | null; yonlendirilen_model: string | null };

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
  | { tip: 'TurSonu'; sira: number; tur: TurId | null; neden: BitisNedeni };

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
  | { tip: 'Yok'; neden: YalitimYoklukNedeni; talimat: string };

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
  anahtar_deposu: DepoTuru;
  modeller: ModelSecenegi[];
  varsayilan_model: string;
  son_oturum: OturumId | null;
  yalitim: YalitimDurumu;
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
