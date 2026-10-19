// Ana pencerenin komutları (1c tasarımı §6.1). Argüman adları Rust'taki
// adlardır: komutlar `rename_all = "snake_case"` ile tanımlıdır.
import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  AgHostu,
  AlanHedefi,
  EklentiSatiri,
  GunlukSinirDurumu,
  KurulumSonucu,
  McpDurumu,
  McpIceAktarmaOnizlemesi,
  McpSunucuAyrintisi,
  McpSunucuFormu,
  McpTaslak,
  SeciliKomutBilgisi,
  OturumEklentisi,
  GizliAyarGorunumu,
  CagriId,
  CalismaAlani,
  CalismaAlaniOzeti,
  DizinIcerigi,
  DosyaIcerigi,
  DenetciDurumu,
  DenetimKipi,
  DusunceEforu,
  GecmisSayfasi,
  GunlukSayfasi,
  IslemId,
  KanalMesaji,
  Kip,
  ModelSecenegi,
  OnarimSonucu,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  TurId,
  UygulamaDurumu,
  YalitimCalismaAlani,
  YalitimDurumu,
} from './tipler';

export const uygulamaDurumu = () => invoke<UygulamaDurumu>('uygulama_durumu');

export const modelleriYenile = () => invoke<ModelSecenegi[]>('modelleri_yenile');

export const oturumlariListele = () => invoke<OturumSatiri[]>('oturumlari_listele');

export const calismaAlaniSec = () => invoke<CalismaAlani | null>('calisma_alani_sec');

export const oturumBaslat = (calisma_alani: number, model: string) =>
  invoke<OturumAcilisi>('oturum_baslat', { calisma_alani, model });

export const oturumAc = (oturum: OturumId) => invoke<OturumAcilisi>('oturum_ac', { oturum });

/** Oturumun olaylarını dinler. Açılan oturum başına bir kez çağrılır. */
export function oturumuDinle(oturum: OturumId, dinleyici: (mesaj: KanalMesaji) => void) {
  const kanal = new Channel<KanalMesaji>(dinleyici);
  return invoke<void>('oturumu_dinle', { oturum, kanal });
}

export const paketiIsledim = (oturum: OturumId, no: number) =>
  invoke<void>('paketi_isledim', { oturum, no });

export const gecmisSayfasi = (oturum: OturumId, once_sira: number) =>
  invoke<GecmisSayfasi>('gecmis_sayfasi', { oturum, once_sira });

export const mesajGonder = (oturum: OturumId, metin: string) =>
  invoke<void>('mesaj_gonder', { oturum, metin });

export const oturumOzetle = (oturum: OturumId, talimat: string | null) =>
  invoke<void>('oturum_ozetle', { oturum, talimat });

export const turuIptalEt = (oturum: OturumId) => invoke<void>('turu_iptal_et', { oturum });

export const aracOnayi = (oturum: OturumId, kararlar: OnayGirdisi[]) =>
  invoke<void>('arac_onayi', { oturum, kararlar });

export const modelDegistir = (oturum: OturumId, model: string) =>
  invoke<void>('model_degistir', { oturum, model });

export const eforDegistir = (oturum: OturumId, efor: DusunceEforu) =>
  invoke<void>('efor_degistir', { oturum, efor });

/** Yükseltmede yerel onay sorulur; reddedilirse eski kip döner. */
export const izinKipiDegistir = (oturum: OturumId, kip: Kip) =>
  invoke<Kip>('izin_kipi_degistir', { oturum, kip });

export const oturumuKapat = (oturum: OturumId) => invoke<void>('oturumu_kapat', { oturum });

/** Turun geri alma planını ister; plan `GeriAlmaPlani` olayıyla gelir (aşama 3 tasarımı §5.6). */
export const geriAlmaOnizle = (oturum: OturumId, tur: TurId) =>
  invoke<void>('geri_alma_onizle', { oturum, tur });

/** Önizlenen planı mührüyle yürütür; sonuç `TurGeriAlindi` olayıyla gelir. */
export const turuGeriAl = (oturum: OturumId, tur: TurId, muhur: string) =>
  invoke<void>('turu_geri_al', { oturum, tur, muhur });

/** Rust yerel pencerede sorar; vazgeçilirse `false` (aşama 3 tasarımı §5.10). */
export const oturumuSil = (oturum: OturumId) => invoke<boolean>('oturumu_sil', { oturum });

/** Bütün oturumların geri alma verisinin toplam boyutu (bayt). */
export const geriAlmaVerisiBoyutu = () => invoke<number>('geri_alma_verisi_boyutu');

export const baglantiAc = (adres: string) => invoke<void>('baglanti_ac', { adres });

export const anahtarPenceresiniAc = () => invoke<void>('anahtar_penceresini_ac');

export const sartPenceresiniAc = () => invoke<void>('sart_penceresini_ac');

/**
 * Yalıtım durumunu dinler (aşama 2 tasarımı §10.5): şimdiki durum hemen,
 * sonraki değişimler geldikçe yazılır. Pencere açılınca bir kez çağrılır.
 */
export function yalitimiDinle(dinleyici: (durum: YalitimDurumu) => void) {
  const kanal = new Channel<YalitimDurumu>(dinleyici);
  return invoke<void>('yalitimi_dinle', { kanal });
}

export const yalitimiAlgila = () => invoke<YalitimDurumu>('yalitimi_algila');

export const aracZinciriDizinleri = () => invoke<string[]>('arac_zinciri_dizinleri');

/** Dizini Rust yerel klasör seçiciyle sorar; vazgeçilirse liste değişmez. */
export const aracZinciriDiziniEkle = () => invoke<string[]>('arac_zinciri_dizini_ekle');

export const aracZinciriDiziniKaldir = (yol: string) =>
  invoke<string[]>('arac_zinciri_dizini_kaldir', { yol });

/** Kurulu sürümde kayıtlı bir araç zinciri dizini kapta okunamıyor mu (Onar gerekir). */
export const aracZinciriOnarimGerekli = () => invoke<boolean>('arac_zinciri_onarim_gerekli');

/** Yalıtımsız çalıştırma anahtarı; açmayı Rust yerel pencerede sorar. Yeni değeri döner. */
export const yalitimsiziAyarla = (acik: boolean) =>
  invoke<boolean>('yalitimsizi_ayarla', { acik });

/** Denetçi kipi ve ayar dosyasının durumu (3c Görev 7). */
export const denetciKipi = () => invoke<DenetciDurumu>('denetci_kipi');

/** Denetçi kipini değiştirir; düşürmek yerel pencerede onaylanır. */
export const denetciKipiniAyarla = (kip: DenetimKipi) =>
  invoke<DenetciDurumu>('denetci_kipini_ayarla', { kip });

/** Oturumdaki bir "bu oturumda izin ver" kısayolunu kaldırır (3e). */
export const kisayoluKaldir = (oturum: OturumId, kimlik: number) =>
  invoke<void>('kisayolu_kaldir', { oturum, kimlik });

/** Ayarlar > "Gizli dosyalar" (3e). */
export const gizliAyarlari = () => invoke<GizliAyarGorunumu>('gizli_ayarlari');

/** Ek gizli kalıp; eklemek onaysızdır. */
export const ekGizliKalipEkle = (kalip: string) =>
  invoke<GizliAyarGorunumu>('ek_gizli_kalip_ekle', { kalip });

/** Ek gizli kalıbı kaldırır; Rust yerel pencerede sorar. */
export const ekGizliKalipKaldir = (kalip: string) =>
  invoke<GizliAyarGorunumu>('ek_gizli_kalip_kaldir', { kalip });

/** Gizli sayılmayanlara ekler; Rust yerel pencerede sorar. */
export const gizliSayilmayanEkle = (kalip: string) =>
  invoke<GizliAyarGorunumu>('gizli_sayilmayan_ekle', { kalip });

/** Gizli sayılmayanlardan kaldırır; onaysızdır. */
export const gizliSayilmayanKaldir = (kalip: string) =>
  invoke<GizliAyarGorunumu>('gizli_sayilmayan_kaldir', { kalip });

/** Kullanıcının ağ izin listesi (3d, §8.3). */
export const agIzinListesi = () => invoke<AgHostu[]>('ag_izin_listesi');

/** Ağ izin listesine host ekler; Rust yerel pencerede sorar. */
export const agHostuEkle = (host: string, ozelAg: boolean) =>
  invoke<AgHostu[]>('ag_hostu_ekle', { host, ozel_ag: ozelAg });

/** Ağ izin listesinden host kaldırır; onaysızdır. */
export const agHostuKaldir = (host: string) => invoke<AgHostu[]>('ag_hostu_kaldir', { host });

/** BT için teknik rapor (aşama 2 tasarımı §9.8), düz metin. */
export const yalitimTeknikRaporu = () => invoke<string>('yalitim_teknik_raporu');

/** "Onar": Rust yerel pencerede sorar, sonra kurulu yardımcıyı UAC ile çalıştırır. */
export const yalitimiOnar = () => invoke<OnarimSonucu>('yalitimi_onar');

/** Windows yalıtımı için hazırlanmış çalışma alanlarını kimlikleriyle listeler. */
export const yalitimCalismaAlanlari = () =>
  invoke<YalitimCalismaAlani[]>('yalitim_calisma_alanlari');

/** Yerel onayı Rust sorar; arayüz yalnızca çalışma alanı kimliğini gönderir. */
export const yalitimCalismaAlaniniKaldir = (kimlik: string) =>
  invoke<boolean>('yalitim_calisma_alanini_kaldir', { kimlik });

/** Gönderim günlüğü sayfası (aşama 3b). */
export const gonderimGunlugu = (oturum: OturumId, once_sira?: number | null) =>
  invoke<GunlukSayfasi>('gonderim_gunlugu', { oturum, once_sira: once_sira ?? null });

/** Gönderilen tek bir öğenin tam metni (aşama 3b). */
export const gonderimOgesi = (oturum: OturumId, sira: number) =>
  invoke<string>('gonderim_ogesi', { oturum, sira });

/** Oturumun önizleme ayarını değiştirir (aşama 3b). */
export const onizlemeAyarla = (oturum: OturumId, acik: boolean) =>
  invoke<void>('onizleme_ayarla', { oturum, acik });

/**
 * Önizlemede kullanıcının kararı (aşama 3b). `hep_gonderilecekler`: eklenti
 * isteğinde "Bu araç çağrısında hep gönder" (aşama 4 tasarımı §9.4).
 */
export const gonderimKarari = (
  oturum: OturumId,
  islem: IslemId,
  gonderilmeyecekler: CagriId[],
  hep_gonderilecekler: CagriId[],
) =>
  invoke<void>('gonderim_karari', {
    oturum,
    islem,
    gonderilmeyecekler,
    hep_gonderilecekler,
  });

// ── Etkin bağlam penceresi ──────────────────────────────────────────────────

/** Yeni oturumların etkin bağlam penceresi (Ayarlar). */
export const etkinPencere = () => invoke<number>('etkin_pencere');

/** Yeni oturumların etkin bağlam penceresini değiştirir; Rust da doğrular. */
export const etkinPencereyiAyarla = (deger: number) =>
  invoke<number>('etkin_pencereyi_ayarla', { deger });

// ── Eklentiler (aşama 4 tasarımı §13, §14) ──────────────────────────────────

export const eklentileriListele = () => invoke<EklentiSatiri[]>('eklentileri_listele');

/** Klasör seçici ve onay yerel penceredir; arayüz yol göndermez. */
export const eklentiKlasordenKur = () => invoke<KurulumSonucu>('eklenti_klasorden_kur');

/** Yerel pencerede onaylanır; vazgeçilirse `false`. */
export const eklentiKaldir = (ad: string) => invoke<boolean>('eklenti_kaldir', { ad });

export const eklentiEtkinAyarla = (ad: string, etkin: boolean) =>
  invoke<void>('eklenti_etkin_ayarla', { ad, etkin });

export const eklentiAlaniAyarla = (ad: string, hedef: AlanHedefi, acik: boolean) =>
  invoke<void>('eklenti_alani_ayarla', { ad, hedef, acik });

export const gonderimIzniniKaldir = (ad: string, kimlik: string) =>
  invoke<void>('gonderim_iznini_kaldir', { ad, kimlik });

/** Yerel pencerede onaylanır; vazgeçilirse `false`. */
export const eklentiVerisiniSil = (ad: string, kimlik: string) =>
  invoke<boolean>('eklenti_verisini_sil', { ad, kimlik });

export const oturumEklentileri = (oturum: OturumId) =>
  invoke<OturumEklentisi[]>('oturum_eklentileri', { oturum });

/** Önizlemede bekleyen eklenti isteğinin gövdesi ("Metni göster"). */
export const eklentiIstegi = (oturum: OturumId, islem: IslemId) =>
  invoke<string>('eklenti_istegi', { oturum, islem });

export const gunlukSinir = () => invoke<GunlukSinirDurumu | null>('gunluk_sinir');

// ── MCP sunucuları (1.1a-2 Görev 10) ───────────────────────────────────────

/**
 * MCP durum görüntüsünü dinler: şimdiki görüntü hemen, sonrakiler değiştikçe
 * gelir. Revizyonu son görülenden büyük olmayan görüntü atılır; eski bir
 * görüntü yenisinin üstüne yazılmaz.
 */
export function mcpDinle(dinleyici: (durum: McpDurumu) => void) {
  let son = 0;
  const kanal = new Channel<McpDurumu>((durum) => {
    if (durum.revizyon <= son) return;
    son = durum.revizyon;
    dinleyici(durum);
  });
  return invoke<void>('mcp_dinle', { kanal });
}

export const mcpDurumu = () => invoke<McpDurumu>('mcp_durumu');

export const mcpSunucuAyrintisi = (ad: string) =>
  invoke<McpSunucuAyrintisi>('mcp_sunucu_ayrintisi', { ad });

/** Yerel dosya seçici; arayüze yalnız dosya adı ve opak kimlik döner. */
export const mcpKomutSec = () => invoke<SeciliKomutBilgisi | null>('mcp_komut_sec');

export const mcpTaslakOlustur = (eskiAd: string | null, form: McpSunucuFormu) =>
  invoke<McpTaslak>('mcp_taslak_olustur', { eski_ad: eskiAd, form });

export const mcpTaslakGuncelle = (
  taslakKimligi: string,
  eskiAd: string | null,
  form: McpSunucuFormu,
) => invoke<number>('mcp_taslak_guncelle', { taslak_kimligi: taslakKimligi, eski_ad: eskiAd, form });

/** Yerel pencerede onaylanır; vazgeçilirse `false`. */
export const mcpTaslakKaydet = (taslakKimligi: string, kusak: number, oturum: OturumId | null) =>
  invoke<boolean>('mcp_taslak_kaydet', { taslak_kimligi: taslakKimligi, kusak, oturum });

/** JSON arka uçta ayrıştırılır; sunucular kapalı ve yalıtımlı eklenir. */
export const mcpIceAktarOnizle = (json: string) =>
  invoke<McpIceAktarmaOnizlemesi>('mcp_ice_aktar_onizle', { json });

export const mcpIceAktarGizlilik = (
  taslakKimligi: string,
  sunucu: string,
  ortam: string,
  gizli: boolean,
) =>
  invoke<number>('mcp_ice_aktar_gizlilik', {
    taslak_kimligi: taslakKimligi,
    sunucu,
    ortam,
    gizli,
  });

/** Yerel pencerede onaylanır; vazgeçilirse `false`. */
export const mcpIceAktarKaydet = (taslakKimligi: string, kusak: number, oturum: OturumId | null) =>
  invoke<boolean>('mcp_ice_aktar_kaydet', { taslak_kimligi: taslakKimligi, kusak, oturum });

export const mcpEtkinAyarla = (ad: string, etkin: boolean) =>
  invoke<void>('mcp_etkin_ayarla', { ad, etkin });

/** Yerel pencerede onaylanır; vazgeçilirse `false`. */
export const mcpSil = (ad: string) => invoke<boolean>('mcp_sil', { ad });

export const mcpYenidenBaslat = (oturum: OturumId, ad: string) =>
  invoke<void>('mcp_yeniden_baslat', { oturum, ad });

/** Tam erişimde arka uç ayrıca yerel pencere onayı ister. */
export const mcpBaslatmaKartiYanitla = (istekKimligi: string, kabul: boolean) =>
  invoke<boolean>('mcp_baslatma_karti_yanitla', { istek_kimligi: istekKimligi, kabul });

export const mcpBaslatmaIzniGeriAl = (alanKimligi: string, ad: string) =>
  invoke<void>('mcp_baslatma_izni_geri_al', { alan_kimligi: alanKimligi, ad });

// Başlık çubuğu (masaüstü arayüzü tasarımı §5.2): yalnız çağıran pencereye uygulanır.
export const pencereKucult = () => invoke<void>('pencere_kucult');
/** Pencerenin yeni hâli: büyük mü. */
export const pencereBuyutDegistir = () => invoke<boolean>('pencere_buyut_degistir');
export const pencereKapat = () => invoke<void>('pencere_kapat');
export const pencereSurukle = () => invoke<void>('pencere_surukle');

// Sağ panel (masaüstü arayüzü tasarımı §6.2, §6.3): yol gönderilmez, kimlik gider.
export const calismaAlaniOzeti = (oturum: OturumId) =>
  invoke<CalismaAlaniOzeti>('calisma_alani_ozeti', { oturum });
export const calismaAlaniDizini = (oturum: OturumId, dizin: number | null) =>
  invoke<DizinIcerigi>('calisma_alani_dizini', { oturum, dizin });
export const calismaAlaniDosyasi = (oturum: OturumId, kimlik: number) =>
  invoke<DosyaIcerigi>('calisma_alani_dosyasi', { oturum, kimlik });
