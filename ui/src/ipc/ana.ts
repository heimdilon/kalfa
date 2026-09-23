// Ana pencerenin komutları (1c tasarımı §6.1). Argüman adları Rust'taki
// adlardır: komutlar `rename_all = "snake_case"` ile tanımlıdır.
import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  CalismaAlani,
  GecmisSayfasi,
  KanalMesaji,
  Kip,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  UygulamaDurumu,
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

export const baglantiAc = (adres: string) => invoke<void>('baglanti_ac', { adres });

export const anahtarPenceresiniAc = () => invoke<void>('anahtar_penceresini_ac');

export const sartPenceresiniAc = () => invoke<void>('sart_penceresini_ac');
