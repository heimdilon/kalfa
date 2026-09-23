// Şart penceresinin durumu ve komutları (1c tasarımı §7). Kabul isteği
// yalnızca `kabul` yönteminden, yani "Kabul ediyorum" düğmesinden gider; bu
// sınıf başka hiçbir yerde `sartlari_kabul_et` çağırmaz (üst belge §11.8).
// Yalnızca şart penceresinin komutlarını içe aktarır.
import { m } from '../dil/tr';
import { komutHatasi } from '../ipc/hata';
import * as ipc from '../ipc/sartlar';
import type { DepoTuru, SartPenceresiDurumu } from '../ipc/tipler';

export class SartDurum {
  durum = $state<SartPenceresiDurumu | null>(null);
  /** Komut sürüyor: düğmeler kapalı. */
  mesgul = $state(false);
  hata = $state<string | null>(null);
  /** "Şart metni bu sırada güncellendi" notu. */
  not = $state<string | null>(null);
  /** Anahtarın kaydedildiği yer; bellekteyse kullanıcıya söylenir. */
  depo = $state<DepoTuru | null>(null);

  async yukle(): Promise<void> {
    await this.#calistir(async () => {
      this.durum = await ipc.sartPenceresiDurumu();
    });
  }

  /** Anahtar Rust'a bir kez gider; bu sınıf onu saklamaz. */
  async anahtarKaydet(anahtar: string): Promise<void> {
    await this.#calistir(async () => {
      const sonuc = await ipc.anahtariKaydet(anahtar);
      switch (sonuc.tip) {
        case 'Gecersiz':
          this.hata = m('hata.anahtar_gecersiz');
          return;
        case 'Ulasilamadi':
          this.hata = m('hata.evren_ulasilamiyor');
          return;
        case 'Kaydedildi':
          this.depo = sonuc.depo;
          this.durum = sonuc.sonraki;
      }
    });
  }

  async kabul(surum: number): Promise<void> {
    await this.#calistir(async () => {
      const yanit = await ipc.sartlariKabulEt(surum);
      switch (yanit.tip) {
        case 'Kabul':
          this.durum = { tip: 'Tamam' };
          return;
        case 'YeniMetin':
          this.durum = { tip: 'SartGerekli', metin: yanit.metin };
          this.not = m('hata.sart_guncellendi');
          return;
        case 'Kilitli':
          this.durum = { tip: 'Kilitli' };
      }
    });
  }

  async reddet(): Promise<void> {
    await this.#calistir(async () => {
      await ipc.sartlariReddet();
      this.durum = await ipc.sartPenceresiDurumu();
    });
  }

  async metniGoster(): Promise<void> {
    await this.#calistir(async () => {
      this.durum = await ipc.sartMetniniGoster();
    });
  }

  baglantiAc = (adres: string): void => {
    ipc.baglantiAc(adres).catch((e: unknown) => {
      this.hata = komutHatasi(e).mesaj;
    });
  };

  async #calistir(is: () => Promise<void>): Promise<void> {
    this.mesgul = true;
    this.hata = null;
    this.not = null;
    try {
      await is();
    } catch (e) {
      this.hata = komutHatasi(e).mesaj;
      // Durum Rust'ta değişmiş olabilir (ör. kabul bu arada geçersizleşti).
      this.durum = await ipc.sartPenceresiDurumu().catch(() => this.durum);
    } finally {
      this.mesgul = false;
    }
  }
}
