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

export type Kip = 'Inceleme' | 'Siki' | 'Proje' | 'Serbest' | 'Otomatik';

/** Kiplerin sırası: sağa her geçiş yükseltmedir (1c tasarımı §6). */
export const KIPLER: readonly Kip[] = ['Inceleme', 'Siki', 'Proje', 'Serbest', 'Otomatik'];

export type OtomatikSonuc = 'Izin' | 'Ret' | 'Kullaniciya';

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
  /** MCP aracında sunucu, araç, erişim ve ağ (1.1a tasarımı §9). */
  mcp?: McpOnayKarti;
}

/** MCP onay kartının özeti; komut, ortam ve yol taşımaz. */
export interface McpOnayKarti {
  sunucu: string;
  arac: string;
  erisim: 'Yalitimli' | 'TamErisim';
  ag_hostlari: string[];
  ag_sinirsiz: boolean;
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
  | { tip: 'Kabuk'; program: string; dizin: string }
  /** Yalıtımlı MCP aracı; özet alanları arayüzde gösterilmez. */
  | { tip: 'Mcp'; sunucu: string; arac: string };

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
  diff?: string | null;
}

/** Yazan çağrının bir dosyadaki değişikliği; araç kartında gösterilir. */
export interface DosyaDegisikligi {
  yol: string;
  islem: 'Olusturuldu' | 'Degisti';
  eklenen: number;
  silinen: number;
  /** Gizli dosyada, çok dosyalı yazmada ve 64 KiB üstünde yok. */
  diff?: string | null;
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

export type PlanAdimDurumu = 'Bekliyor' | 'Suruyor' | 'Tamam';

export interface PlanAdimi {
  kimlik: string;
  baslik: string;
  durum: PlanAdimDurumu;
}

export interface OturumPlani {
  guncellendigi_tur?: TurId | null;
  adimlar: PlanAdimi[];
}

export type ProjeTalimatKapsami =
  | { tur: 'kok' }
  | { tur: 'alt_dizin'; deger: { dizin: string } };

export interface ProjeTalimatKaynagi {
  goreli_yol: string;
  kapsam: ProjeTalimatKapsami;
  bayt: number;
  ozet: string;
}

export type ProjeTalimatUyariTuru =
  | { tur: 'dosya_boyutu_asildi'; sinir: number; gercek: number }
  | { tur: 'toplam_boyut_asildi'; sinir: number; gercek: number }
  | { tur: 'kaynak_sayisi_asildi'; sinir: number }
  | { tur: 'gecersiz_utf8' }
  | { tur: 'sembolik_baglanti_reddedildi' }
  | { tur: 'gizli_veya_korumali_reddedildi' }
  | { tur: 'okunamadi'; neden: string };

export interface ProjeTalimatUyarisi {
  goreli_yol: string;
  uyari: ProjeTalimatUyariTuru;
}

export interface ProjeTalimatDegisimi {
  goreli_yol: string;
  onceki_ozet?: string | null;
  yeni_ozet?: string | null;
}

export interface ProjeTalimatAnlikGoruntusu {
  tur: number;
  kaynaklar: ProjeTalimatKaynagi[];
  uyarilar?: ProjeTalimatUyarisi[];
  degisen_kaynaklar?: ProjeTalimatDegisimi[];
  eksik_baglam?: boolean;
}

export interface SoruSecenegi {
  kimlik: string;
  etiket: string;
  aciklama?: string | null;
}

export interface KullaniciSorusu {
  soru_kimligi: string;
  tur: TurId;
  cagri: CagriId;
  baslik: string;
  secenekler: SoruSecenegi[];
  serbest_yanit_izni: boolean;
}

export type KullaniciSorusuYaniti =
  | { tur: 'Secenek'; secenek_kimligi: string }
  | { tur: 'SerbestMetin'; metin: string };

export type SoruSonucu =
  | {
      durum: 'Yanitlandi';
      yanit: KullaniciSorusuYaniti;
      secilen_etiket?: string | null;
    }
  | { durum: 'KullaniciMesajiylaIptal'; yeni_mesaj: string }
  | { durum: 'IptalEdildi' };

export type OnayKarari = { tip: 'Onayla' } | { tip: 'Reddet'; gerekce: string | null };

export type Olay =
  | { tip: 'TurBasladi' }
  | { tip: 'TokenParcasi'; metin: string }
  | { tip: 'DusunceParcasi'; metin: string }
  | { tip: 'AracCagrisiBasladi'; cagri: CagriId; arac: string; aciklama: string }
  /** Yazan çağrının değişiklikleri; `AracCagrisiBasladi`'dan hemen sonra. */
  | { tip: 'DosyaDegisiklikleri'; cagri: CagriId; dosyalar: DosyaDegisikligi[] }
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
  /** Bütün konuşma geçmişi özetlendi (compact). */
  | {
      tip: 'OturumOzetlendi';
      tetik: OzetTetigi;
      once: number;
      sonra: number;
      talimat?: string | null;
      ozet: string;
    }
  /** İstek öncesi tahmini bağlam ve otomatik özetleme eşiği. */
  | { tip: 'BaglamDurumu'; tahmin: number; esik: number }
  /** Bütçe sayaçlarının güncel görünümü. */
  | { tip: 'ButceDurumu'; durum: ButceGorunumu }
  /** Otomatik kip hakeminin kararı. */
  | {
      tip: 'OtomatikKarar';
      cagri: CagriId;
      arac: string;
      aciklama: string;
      sonuc: OtomatikSonuc;
      risk?: string | null;
      olasilik?: number | null;
    }
  | { tip: 'ProjeTalimatlari'; goruntu: ProjeTalimatAnlikGoruntusu }
  | { tip: 'PlanGuncellendi'; plan: OturumPlani }
  | { tip: 'KullaniciSorusuGerekiyor'; soru: KullaniciSorusu }
  | {
      tip: 'KullaniciSorusuCozuldu';
      soru_kimligi: string;
      cagri: CagriId;
      sonuc: SoruSonucu;
    };

export type OzetTetigi = 'Otomatik' | 'Zorunlu' | 'Elle';

/** Oturum bütçesinin görünümü: yalnız sayaçlar, sınır ve engel yoktur. */
export interface ButceGorunumu {
  /** Başlatılan denemeler. */
  baslatilan: number;
  /** Kullanımı bilinen denemelerin token toplamı. */
  bilinen_token: number;
  /** Kullanımı bilinmeyen tamamlanmış denemeler. */
  bilinmeyen: number;
  /** Sonuçlanmamış (aktif) deneme sayısı. */
  aktif: number;
}

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
      degisiklikler?: DosyaDegisikligi[];
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
  | { tip: 'TurOzeti'; sira: number; tur: TurId | null; ozet: TurOzeti }
  | {
      tip: 'OturumOzeti';
      sira: number;
      tur: TurId | null;
      yontem: OzetTetigi;
      once: number;
      sonra: number;
      talimat?: string | null;
      ozet: string;
    }
  | {
      tip: 'OtomatikKarar';
      sira: number;
      tur: TurId | null;
      cagri: CagriId;
      arac: string;
      aciklama: string;
      sonuc: OtomatikSonuc;
      risk?: string | null;
      olasilik?: number | null;
    }
  | {
      tip: 'KullaniciSorusu';
      sira: number;
      tur: TurId | null;
      soru: KullaniciSorusu;
      sonuc?: SoruSonucu | null;
    };

export interface GecmisSayfasi {
  ogeler: GorunumOgesi[];
  daha_eski_var: boolean;
}

export type DusunceEforu = 'kapali' | 'dusuk' | 'orta' | 'yuksek';

export const EFORLAR: readonly DusunceEforu[] = ['kapali', 'dusuk', 'orta', 'yuksek'];

export interface DurumOzeti {
  model: string | null;
  efor?: DusunceEforu | null;
  kip: Kip | null;
  yonlendirilen_model: string | null;
  son_kredi: KrediGozlemi | null;
  istem: number;
  tamamlama: number;
  dusunce: number;
  onizleme_acik?: boolean;
  denetci_kipi?: DenetimKipi;
  plan?: OturumPlani | null;
  proje_talimatlari?: ProjeTalimatAnlikGoruntusu | null;
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
  eforlar?: DusunceEforu[];
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

/** Sağ panelin dosya görünümü (masaüstü arayüzü tasarımı §6.3); kimlikler oturuma bağlıdır. */
export interface DosyaGirdisi {
  kimlik: number;
  ad: string;
  tur: 'Dosya' | 'Dizin' | 'Baglanti';
  boyut: number | null;
}

export interface DizinIcerigi {
  girdiler: DosyaGirdisi[];
  /** Dizinde 2000'den çok girdi var. */
  kirpildi: boolean;
}

export type DosyaIcerigi =
  | { tip: 'Metin'; metin: string; kirpildi: boolean }
  | { tip: 'Ikili' }
  | { tip: 'Gizli' }
  | { tip: 'CokAdli' };

export type GitDurumu = { tip: 'Dal'; ad: string } | { tip: 'AyrikHead' } | { tip: 'GitDegil' };

export interface CalismaAlaniOzeti {
  kisa_yol: string;
  git: GitDurumu;
  /** Bütün kayıttan: geri alınmamış turlarda değişen dosyalar. */
  ciktilar: { yol: string; eklenen: number; silinen: number }[];
  /** Bağlam yönetiminin özetlediği araç çıktısı sayısı (5a). */
  ozetlenen: number;
}

export interface UygulamaDurumu {
  baglanti: 'Var' | 'Yok';
  anahtar_deposu: DepoTuru;
  modeller: ModelSecenegi[];
  varsayilan_model: string;
  varsayilan_efor?: DusunceEforu;
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
  /** MCP başlatma kartı oturuma bu kimlikle bağlanır (1.1a tasarımı §5.1). */
  calisma_alani_kimligi: string | null;
  gecmis: GecmisSayfasi;
  ozet: DurumOzeti;
  salt_okunur: SaltOkunurNedeni | null;
  /** Bütçe görünümü; salt okunur oturumda ve açık oturumda `null` (5b tasarımı §9). */
  butce: ButceGorunumu | null;
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

// ── MCP sunucuları (1.1a-2 Görev 10) ───────────────────────────────────────
// Arayüz yol göndermez: komut yerel seçiciyle seçilir (`kimlik`), çalışma
// alanı oturum ya da alan kimliğiyle gelir. Gizli değer yalnız formda gider,
// geri hiç gelmez.

export type McpKipi = 'yalitimli' | 'tam_erisim';

export type McpKomut =
  | { tur: 'program_adi'; ad: string; argumanlar?: string[] }
  | { tur: 'secili_yurutulebilir'; kimlik: string; argumanlar?: string[] };

export interface McpAgKurali {
  host: string;
  port: number;
  ozel_ag?: boolean;
}

export type McpOrtamFormGirdisi =
  | { tur: 'duz'; ad: string; deger: string }
  | { tur: 'yeni_gizli'; ad: string; deger: string }
  | { tur: 'korunan_gizli'; ad: string; kimlik: string };

export interface McpSunucuFormu {
  ad: string;
  etkin?: boolean;
  komut: McpKomut;
  ortam?: McpOrtamFormGirdisi[];
  kip?: McpKipi;
  ag_izinleri?: McpAgKurali[];
  baslatma_zamani_asimi_sn: number;
  cagri_zamani_asimi_sn: number;
}

export interface McpSunucuOzeti {
  ad: string;
  etkin: boolean;
  kip: McpKipi;
  gizli_ortam_sayisi: number;
}

export type McpDurumEtiketi = 'durdu' | 'basliyor' | 'calisiyor' | 'bosta_kapandi' | 'hata';

/** `kusak` süreç kuşağıdır; eski kuşağın olayı yenisinin üstüne yazılmaz. */
export interface McpAlanDurumu {
  alan_kimligi: string;
  durum: McpDurumEtiketi;
  kusak: number | null;
  hata: string | null;
  izin: McpKipi | null;
  arac_sayisi: number;
}

export interface McpSunucuSatiri {
  ozet: McpSunucuOzeti;
  alanlar: McpAlanDurumu[];
}

/** Model dışı başlatma kartı (tasarım §5.1); model açamaz, yanıtlayamaz. */
export interface McpBaslatmaKarti {
  istek_kimligi: string;
  alan_kimligi: string;
  sunucu: string;
  kip: McpKipi;
  /** `kip`'in arka uçtaki görünen adı. */
  erisim_kipi: string;
  komut_ozeti: string;
  ag_listesi: string[];
  uyari: string;
  wde_kodlu: boolean;
}

/** Kanal görüntüsü; `revizyon` artar, arayüz eskisini atar. */
export interface McpDurumu {
  revizyon: number;
  sunucular: McpSunucuSatiri[];
  baslatma_kartlari: McpBaslatmaKarti[];
}

export interface McpOrtamDegiskeni {
  ad: string;
  gizli: boolean;
  deger_acik?: string;
  gizli_var: boolean;
  gizli_kimlik?: string;
}

export interface McpSunucuAyari {
  ad: string;
  etkin: boolean;
  komut: McpKomut;
  ortam: McpOrtamDegiskeni[];
  kip: McpKipi;
  ag_izinleri: McpAgKurali[];
  baslatma_zamani_asimi_sn: number;
  cagri_zamani_asimi_sn: number;
}

export interface McpAracOzeti {
  birlesik_ad: string;
  uzak_ad: string;
  aciklama: string;
  d_ozeti_hex: string;
  denetim_metni: string;
  denetim_sonucu: DenetimSonucu | null;
  sunulur: boolean;
  uyari: string | null;
  onbellekten: boolean;
}

export interface McpAlanAyrintisi {
  alan_kimligi: string;
  araclar: McpAracOzeti[];
  /** Kapatılan araçlar yalnız sıra numarasıyla. */
  reddedilenler: string[];
  /** Maskeli son stderr satırları. */
  stderr: string[];
}

export interface McpSunucuAyrintisi {
  ayrinti: McpSunucuAyari;
  alanlar: McpAlanAyrintisi[];
}

export interface SeciliKomutBilgisi {
  kimlik: string;
  gorunen_ad: string;
}

export interface McpTaslak {
  taslak_kimligi: string;
  kusak: number;
}

export interface McpIceAktarmaSunucusu {
  ad: string;
  etkin: boolean;
  kip: McpKipi;
  komut_gorunumu: string;
  secili_komut_kimligi?: string;
  argumanlar: string[];
  ortam: McpOrtamDegiskeni[];
}

export interface McpIceAktarmaOnizlemesi {
  taslak_kimligi: string;
  kusak: number;
  sunucular: McpIceAktarmaSunucusu[];
  atlanan_url_sunuculari: { ad: string; neden: string }[];
}

// ── Arka plan süreçleri ve terminaller (Hat 03) ───────────────────────────

export type IsKipi =
  | { tur: 'borulu' }
  | { tur: 'terminal'; boyut: { satir: number; sutun: number } };

export type IsDurumu =
  | { durum: 'calisiyor' }
  | { durum: 'durduruluyor' }
  | { durum: 'bitti'; cikis_kodu: number | null; sure_ms: number; zaman_asimi: boolean }
  | { durum: 'sonucu_bilinmiyor'; neden: string };

export interface IsOzeti {
  is_id: string;
  oturum: OturumId;
  gosterim: string;
  kip: IsKipi;
  durum: IsDurumu;
  toplam_bayt: number;
  tasti: boolean;
  uyarilar: string[];
}

export interface IsOkumaSonucu {
  is_id: string;
  gosterim: string;
  kip: IsKipi;
  durum: IsDurumu;
  cikti: string;
  yeni_imlec: number;
  toplam_bayt: number;
  tasti: boolean;
  uyarilar: string[];
}
