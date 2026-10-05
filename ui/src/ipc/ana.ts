// Ana pencerenin komutları (1c tasarımı §6.1). Argüman adları Rust'taki
// adlardır: komutlar `rename_all = "snake_case"` ile tanımlıdır.
import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  CagriId,
  CalismaAlani,
  DenetciDurumu,
  DenetimKipi,
  GecmisSayfasi,
  GunlukSayfasi,
  IslemId,
  KanalMesaji,
  Kip,
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

export const turuIptalEt = (oturum: OturumId) => invoke<void>('turu_iptal_et', { oturum });

export const aracOnayi = (oturum: OturumId, kararlar: OnayGirdisi[]) =>
  invoke<void>('arac_onayi', { oturum, kararlar });

export const modelDegistir = (oturum: OturumId, model: string) =>
  invoke<void>('model_degistir', { oturum, model });

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

/** Önizlemede kullanıcının kararı (aşama 3b). */
export const gonderimKarari = (
  oturum: OturumId,
  islem: IslemId,
  gonderilmeyecekler: CagriId[],
) => invoke<void>('gonderim_karari', { oturum, islem, gonderilmeyecekler });
