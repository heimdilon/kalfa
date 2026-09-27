// Ana pencerenin durumu ve komutları (1c tasarımı §5, §8). Ekranlar yalnızca
// bu sınıfın alanlarını çizer ve yöntemlerini çağırır. Her açık oturumun ekran
// durumu `durum/oturum.ts`'deki saf işlevle güncellenir.
import { SvelteMap } from 'svelte/reactivity';
import { m } from '../dil/tr';
import { acilistanEkran, uygula, type Girdi, type OturumEkrani } from '../durum/oturum';
import { kanalIsleyicisi } from '../durum/dinleyici';
import { sar as e2eSar } from '../e2e/kanca';
import * as ipc from '../ipc/ana';
import { komutHatasi } from '../ipc/hata';
import type {
  IslemId,
  Kip,
  OnarimSonucu,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  UygulamaDurumu,
  YalitimDurumu,
} from '../ipc/tipler';

export class AnaDurum {
  uygulama = $state<UygulamaDurumu | null>(null);
  liste = $state<OturumSatiri[]>([]);
  /** Açık oturumların ekranları; oturum onay beklerken diğerine geçilebilir. */
  ekranlar = new SvelteMap<OturumId, OturumEkrani>();
  secili = $state<OturumId | null>(null);
  /** Konuşmanın üstünde gösterilen son komut hatası. */
  hata = $state<string | null>(null);
  /** Açık onay sayfası. */
  onay = $state<{ oturum: OturumId; islem: IslemId } | null>(null);
  /** Ekran okuyucuya okunan son duyuru (`aria-live`, 1c tasarımı §9.5). */
  duyuru = $state('');
  /** Menünün seçtiği görünüm: oturumlar ya da Ayarlar (aşama 2 tasarımı §10.4). */
  gorunum = $state<'oturumlar' | 'ayarlar'>('oturumlar');
  /** Yalıtıma açılan araç zinciri dizinleri (Linux). */
  aracZinciri = $state<string[]>([]);
  algilaniyor = $state(false);
  /** Son alınan teknik rapor ve panoya kopyalamanın sonucu (§9.8). */
  teknikRapor = $state<string | null>(null);
  kopyalama = $state<'kopyalandi' | 'kopyalanamadi' | null>(null);
  /** "Onar" sürüyor mu ve son sonucu (§9.8). */
  onariliyor = $state(false);
  onarim = $state<OnarimSonucu | null>(null);
  /** Eski sayfası yüklenen oturumlar; aynı sayfa iki kez istenmez. */
  #eskiYukleniyor = new Set<OturumId>();
  #yenidenAciliyor = new Set<OturumId>();

  aktif = $derived(this.secili ? (this.ekranlar.get(this.secili) ?? null) : null);

  /** Pencere açılınca: uygulama durumu, liste ve son açık oturum (1c tasarımı §5.6). */
  async baslat(): Promise<void> {
    await this.#dene(async () => {
      this.uygulama = await ipc.uygulamaDurumu();
    });
    await this.#dene(() => ipc.yalitimiDinle((d) => this.#yalitimYaz(d)));
    await this.listeyiYenile();
    const son = this.uygulama?.son_oturum;
    if (son) await this.oturumSec(son);
  }

  async listeyiYenile(): Promise<void> {
    await this.#dene(async () => {
      this.liste = await ipc.oturumlariListele();
    });
  }

  async oturumSec(oturum: OturumId): Promise<void> {
    if (!this.ekranlar.has(oturum)) {
      const acildi = await this.#dene(() => this.#ac(oturum));
      if (acildi === undefined) return;
    }
    this.secili = oturum;
    this.gorunum = 'oturumlar';
    this.hata = null;
  }

  /** Klasör seçtirir ve varsayılan modelle yeni oturum başlatır. */
  async yeniOturum(): Promise<void> {
    await this.#dene(async () => {
      const alan = await ipc.calismaAlaniSec();
      if (!alan || !this.uygulama) return;
      const acilis = await ipc.oturumBaslat(alan.kimlik, this.uygulama.varsayilan_model);
      await this.#kur(acilis);
      this.secili = acilis.oturum;
      this.gorunum = 'oturumlar';
      this.hata = null;
      await this.listeyiYenile();
    });
  }

  async gonder(metin: string): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    this.#uygula(oturum, { tip: 'MesajGonderildi', metin });
    await this.#dene(() => ipc.mesajGonder(oturum, metin));
  }

  async iptal(): Promise<void> {
    const oturum = this.secili;
    if (oturum) await this.#dene(() => ipc.turuIptalEt(oturum));
  }

  async onayla(oturum: OturumId, islem: IslemId, kararlar: OnayGirdisi[]): Promise<void> {
    const gitti = await this.#dene(() => ipc.aracOnayi(oturum, kararlar));
    if (gitti === undefined) return;
    this.#uygula(oturum, { tip: 'OnayYanitlandi', islem });
    this.onay = null;
  }

  async modelSec(model: string): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    const oldu = await this.#dene(() => ipc.modelDegistir(oturum, model));
    if (oldu !== undefined) this.#uygula(oturum, { tip: 'ModelDegisti', model });
  }

  /** Yükseltmede Rust yerel onay sorar; reddedilirse eski kip döner (1c tasarımı §6). */
  async kipSec(kip: Kip): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    const yeni = await this.#dene(() => ipc.izinKipiDegistir(oturum, kip));
    if (yeni) this.#uygula(oturum, { tip: 'KipDegisti', kip: yeni });
  }

  /** Yukarı kaydırınca ya da düğmeyle: bir önceki sayfa (1c tasarımı §9.3). */
  async eskileriYukle(oturum: OturumId): Promise<void> {
    const ekran = this.ekranlar.get(oturum);
    const enEski = ekran?.ogeler[0]?.sira;
    if (!ekran?.daha_eski_var || enEski === undefined || this.#eskiYukleniyor.has(oturum)) return;
    this.#eskiYukleniyor.add(oturum);
    try {
      const sayfa = await this.#dene(() => ipc.gecmisSayfasi(oturum, enEski));
      if (sayfa) this.#uygula(oturum, { tip: 'EskiSayfa', sayfa });
    } finally {
      this.#eskiYukleniyor.delete(oturum);
    }
  }

  async kapat(oturum: OturumId): Promise<void> {
    await this.#dene(() => ipc.oturumuKapat(oturum));
    this.ekranlar.delete(oturum);
    if (this.secili === oturum) this.secili = null;
    if (this.onay?.oturum === oturum) this.onay = null;
    await this.listeyiYenile();
  }

  /** Oturumu kapatıp kayıttan yeniden açar: boşlukta, aktarım durunca, kapanınca. */
  async yenidenAc(oturum: OturumId): Promise<void> {
    if (this.#yenidenAciliyor.has(oturum)) return;
    this.#yenidenAciliyor.add(oturum);
    try {
      await ipc.oturumuKapat(oturum).catch(() => undefined);
      this.ekranlar.delete(oturum);
      await this.#dene(() => this.#ac(oturum));
    } finally {
      this.#yenidenAciliyor.delete(oturum);
    }
  }

  onayiAc(oturum: OturumId, islem: IslemId): void {
    this.onay = { oturum, islem };
  }

  onayiKapat(): void {
    this.onay = null;
  }

  baglantiAc = (adres: string): void => {
    void this.#dene(() => ipc.baglantiAc(adres));
  };

  anahtariDegistir(): void {
    void this.#dene(() => ipc.anahtarPenceresiniAc());
  }

  sartPenceresiniGoster(): void {
    void this.#dene(() => ipc.sartPenceresiniAc());
  }

  /** Ayarlar'ı açar; Linux'ta araç zinciri dizinlerini okur. */
  async ayarlariAc(): Promise<void> {
    this.gorunum = 'ayarlar';
    this.hata = null;
    this.teknikRapor = null;
    this.kopyalama = null;
    this.onarim = null;
    if (this.uygulama?.platform !== 'linux') return;
    const dizinler = await this.#dene(() => ipc.aracZinciriDizinleri());
    if (dizinler) this.aracZinciri = dizinler;
  }

  oturumlaraDon(): void {
    this.gorunum = 'oturumlar';
    this.hata = null;
  }

  /** Algılamayı yeniden çalıştırır (aşama 2 tasarımı §8.3). */
  async yalitimiAlgila(): Promise<void> {
    this.algilaniyor = true;
    try {
      const yeni = await this.#dene(() => ipc.yalitimiAlgila());
      if (yeni) this.#yalitimYaz(yeni);
    } finally {
      this.algilaniyor = false;
    }
  }

  async aracZinciriEkle(): Promise<void> {
    const dizinler = await this.#dene(() => ipc.aracZinciriDiziniEkle());
    if (dizinler) this.aracZinciri = dizinler;
  }

  /**
   * BT için teknik raporu panoya kopyalar (aşama 2 tasarımı §9.8). Pano
   * kullanılamazsa rapor Ayarlar'da açık gösterilir; kullanıcı seçip kopyalar.
   */
  async teknikRaporuKopyala(): Promise<void> {
    const rapor = await this.#dene(() => ipc.yalitimTeknikRaporu());
    if (rapor === undefined) return;
    this.teknikRapor = rapor;
    try {
      await navigator.clipboard.writeText(rapor);
      this.kopyalama = 'kopyalandi';
    } catch {
      this.kopyalama = 'kopyalanamadi';
    }
  }

  /** "Onar" (aşama 2 tasarımı §9.8): onay ve UAC Rust'ta; sonuç Ayarlar'da. */
  async yalitimiOnar(): Promise<void> {
    this.onariliyor = true;
    this.onarim = null;
    try {
      const sonuc = await this.#dene(() => ipc.yalitimiOnar());
      if (!sonuc) return;
      this.onarim = sonuc;
      if (sonuc.tip === 'Tamam') this.#yalitimYaz(sonuc.yalitim);
    } finally {
      this.onariliyor = false;
    }
  }

  /** Yalıtımsız çalıştırma anahtarı (aşama 2 tasarımı §9.8). */
  async yalitimsiziAyarla(acik: boolean): Promise<void> {
    const yeni = await this.#dene(() => ipc.yalitimsiziAyarla(acik));
    if (yeni !== undefined && this.uygulama) this.uygulama.yalitimsiz_izinli = yeni;
  }

  async aracZinciriKaldir(yol: string): Promise<void> {
    const dizinler = await this.#dene(() => ipc.aracZinciriDiziniKaldir(yol));
    if (dizinler) this.aracZinciri = dizinler;
  }

  #yalitimYaz(yeni: YalitimDurumu): void {
    if (this.uygulama) this.uygulama.yalitim = yeni;
  }

  async #ac(oturum: OturumId): Promise<void> {
    await this.#kur(await ipc.oturumAc(oturum));
  }

  /** Ekranı kurar; yazılabilir oturumun olaylarını dinlemeye başlar. */
  async #kur(acilis: OturumAcilisi): Promise<void> {
    const oturum = acilis.oturum;
    this.ekranlar.set(oturum, acilistanEkran(acilis));
    if (acilis.salt_okunur) return;
    const isleyici = kanalIsleyicisi(
      () => this.ekranlar.get(oturum) ?? acilistanEkran(acilis),
      (yeni) => this.#yaz(oturum, yeni),
      (no) => {
        ipc.paketiIsledim(oturum, no).catch(() => undefined);
      },
    );
    // Uçtan uca test derlemesinde dinleyici ölçülür ve duraklatılabilir
    // (1c tasarımı §11.4); sürüm derlemesinde bu dal yoktur.
    await ipc.oturumuDinle(oturum, import.meta.env.MODE === 'e2e' ? e2eSar(isleyici) : isleyici);
  }

  #uygula(oturum: OturumId, girdi: Girdi): void {
    const ekran = this.ekranlar.get(oturum);
    if (ekran) this.#yaz(oturum, uygula(ekran, girdi));
  }

  #yaz(oturum: OturumId, yeni: OturumEkrani): void {
    const onceki = this.ekranlar.get(oturum);
    if (!onceki) return;
    this.ekranlar.set(oturum, yeni);
    this.#duyur(onceki, yeni);
    if (yeni.bosluk && !onceki.bosluk) {
      // Paket numarasında boşluk: oturum kayıttan yeniden açılır (1c tasarımı §4.4).
      queueMicrotask(() => void this.yenidenAc(oturum));
    }
    const turBitti = yeni.ogeler
      .slice(onceki.ogeler.length)
      .some((o) => o.tip === 'TurSonu' || o.tip === 'AktarimDurdu');
    if (turBitti) void this.listeyiYenile();
  }

  /** Onay istekleri ve tur sonları duyurulur; akan token'lar duyurulmaz. */
  #duyur(onceki: OturumEkrani, yeni: OturumEkrani): void {
    for (const o of yeni.ogeler.slice(onceki.ogeler.length)) {
      if (o.tip === 'OnayIstegi') this.duyuru = m('duyuru.onay', { sayi: o.ogeler.length });
      else if (o.tip === 'SartIstegi') this.duyuru = m('duyuru.sart');
      else if (o.tip === 'TurSonu') this.duyuru = m('duyuru.tur_sonu');
    }
  }

  /** Komutu çalıştırır; hata olursa konuşmanın üstünde gösterir ve `undefined` döner. */
  async #dene<T>(is: () => Promise<T>): Promise<T | undefined> {
    try {
      const sonuc = await is();
      return sonuc === undefined ? (null as T) : sonuc;
    } catch (e) {
      this.hata = komutHatasi(e).mesaj;
      return undefined;
    }
  }
}
