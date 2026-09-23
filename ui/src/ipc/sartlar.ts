// Şart penceresinin komutları (1c tasarımı §6.1). Bu modül ana pencerenin
// komutlarını içe aktarmaz; şart penceresinin paketi sohbet kodunu taşımaz.
import { invoke } from '@tauri-apps/api/core';
import type { AnahtarSonucu, KabulYaniti, SartPenceresiDurumu } from './tipler';

export const sartPenceresiDurumu = () => invoke<SartPenceresiDurumu>('sart_penceresi_durumu');

/** Anahtar bir kez Rust'a gider; hiçbir komut onu geri döndürmez. */
export const anahtariKaydet = (anahtar: string) =>
  invoke<AnahtarSonucu>('anahtari_kaydet', { anahtar });

export const sartlariKabulEt = (surum: number) =>
  invoke<KabulYaniti>('sartlari_kabul_et', { surum });

export const sartlariReddet = () => invoke<void>('sartlari_reddet');

export const sartMetniniGoster = () => invoke<SartPenceresiDurumu>('sart_metnini_goster');

export const baglantiAc = (adres: string) => invoke<void>('baglanti_ac', { adres });
