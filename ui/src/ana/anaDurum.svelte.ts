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

export type SagBolum = 'ortam' | 'degisiklikler' | 'dosyalar';
export const SAG_BOLUMLER: readonly SagBolum[] = ['ortam', 'degisiklikler', 'dosyalar'];
const SAG_BOLUM_ANAHTARI = 'kalfa.sagBolum';

/**
 * Arayüzün kendi küçük ayarları (açık panel bölümü, panel genişliği) bu
 * makinenin WebView deposunda durur; depo kullanılamazsa varsayılan döner.
 */
export function yerelOku(anahtar: string): string | null {
  try {
    return localStorage.getItem(anahtar);
  } catch {
    return null;
  }
}

export function yerelYaz(anahtar: string, deger: string): void {
  try {
    localStorage.setItem(anahtar, deger);
  } catch {
    // Depo yoksa ayar yalnız bu açılışta geçerlidir.
  }
}

function sagBolumOku(): SagBolum | null {
  const b = yerelOku(SAG_BOLUM_ANAHTARI);
  return SAG_BOLUMLER.find((x) => x === b) ?? null;
}
import type {
  ButceKarari,
  ButceSinirlari,
  DenetciDurumu,
  DusunceEforu,
  CagriId,
  GunlukSinirDurumu,
  IslemId,
  Kip,
  McpBaslatmaKarti,
  McpDurumu,
  McpSunucuSatiri,
  OnarimSonucu,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  TurId,
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
  /** Sağ panelin açık bölümü (masaüstü arayüzü tasarımı §6.1); son seçim hatırlanır. */
  sagBolum = $state<SagBolum | null>(sagBolumOku());
  sagBolmeAcik = $derived(this.sagBolum !== null);
  /** Soldaki kenar çubuğu açık mı (Ctrl+B ile gizlenir). */
  kenarAcik = $state(true);
  /** Windows'ta pencere çerçevesizdir; başlık çubuğunu arayüz çizer (masaüstü arayüzü §5). */
  ozelCerceve = $derived(this.uygulama?.platform === 'windows');
  /** Açık gönderim günlüğü (oturum kimliğiyle). */
  gunlukOturum = $state<OturumId | null>(null);
  gunlukAcik = $derived(this.gunlukOturum !== null);
  /** Ekran okuyucuya okunan son duyuru (`aria-live`, 1c tasarımı §9.5). */
  duyuru = $state('');
  /** Menünün seçtiği görünüm: oturumlar, Eklentiler ya da Ayarlar (aşama 2 tasarımı §10.4). */
  gorunum = $state<'oturumlar' | 'eklentiler' | 'ayarlar'>('oturumlar');
  /** EVREN'in günlük token sınırı doldu; `null`: dolmadı (aşama 4c). */
  gunlukSinir = $state<GunlukSinirDurumu | null>(null);
  /** Yalıtıma açılan araç zinciri dizinleri (Linux ve Windows). */
  aracZinciri = $state<string[]>([]);
  /** Windows: kayıtlı bir dizin kapta okunamıyor; Gelişmiş'te Onar gösterilir. */
  aracOnarimGerekli = $state(false);
  algilaniyor = $state(false);
  /** Son alınan teknik rapor ve panoya kopyalamanın sonucu (§9.8). */
  teknikRapor = $state<string | null>(null);
  kopyalama = $state<'kopyalandi' | 'kopyalanamadi' | null>(null);
  /** "Onar" sürüyor mu ve son sonucu (§9.8). */
  onariliyor = $state(false);
  onarim = $state<OnarimSonucu | null>(null);
  /** Geri alma verisinin toplam boyutu (bayt); Ayarlar açılınca okunur. */
  geriAlmaBoyutu = $state<number | null>(null);
  /** Geri alma isteği gönderiliyor; sayfa düğmeleri kapalı. */
  geriAliniyor = $state(false);
  /** Eski sayfası yüklenen oturumlar; aynı sayfa iki kez istenmez. */
  #eskiYukleniyor = new Set<OturumId>();
  #yenidenAciliyor = new Set<OturumId>();

  /** Denetçi kipi ve ayar dosyasının durumu (3c Görev 8); Ayarlar'da değişir (3e). */
  denetci = $state<DenetciDurumu | null>(null);
  /** Yeni oturumların bütçe varsayılanı; Ayarlar açılınca okunur (5b tasarımı §9). */
  butceVarsayilani = $state<ButceSinirlari | null>(null);
  /** MCP sunucularının son görüntüsü; kanal eski revizyonu atar (1.1a-2). */
  mcp = $state<McpDurumu | null>(null);
  aktif = $derived(this.secili ? (this.ekranlar.get(this.secili) ?? null) : null);

  /** Ayarlar'daki MCP bölümü açılınca güncel görüntüyü okur. */
  async mcpYenile(): Promise<void> {
    const d = await this.#dene(() => ipc.mcpDurumu());
    if (d) this.#mcpYaz(d);
  }

  /** Kanal ve doğrudan okuma aynı yere yazar; eski revizyon yenisini ezmez. */
  #mcpYaz(d: McpDurumu): void {
    if (!this.mcp || d.revizyon > this.mcp.revizyon) this.mcp = d;
  }

  /** Çalışma alanının yanıt bekleyen MCP başlatma kartları (tasarım §5.1). */
  mcpKartlari(alan: string | null): McpBaslatmaKarti[] {
    if (!alan) return [];
    return this.mcp?.baslatma_kartlari.filter((k) => k.alan_kimligi === alan) ?? [];
  }

  /** `mcp__<sunucu>__<araç>` adlı aracın sunucusu; en uzun ad eşleşir. */
  mcpSunucusu(arac: string): McpSunucuSatiri | undefined {
    return this.mcp?.sunucular
      .filter((s) => arac.startsWith(`mcp__${s.ozet.ad}__`))
      .sort((a, b) => b.ozet.ad.length - a.ozet.ad.length)[0];
  }

  /** Kart tek kullanımlıdır; tam erişimde kabulü Rust yerel pencerede ayrıca sorar. */
  async mcpKartiYanitla(kart: McpBaslatmaKarti, kabul: boolean): Promise<void> {
    const verildi = await this.#dene(() =>
      ipc.mcpBaslatmaKartiYanitla(kart.istek_kimligi, kabul),
    );
    if (kabul && verildi === false) this.duyuru = m('mcp.kart.verilmedi', { sunucu: kart.sunucu });
  }

  /** Pencere açılınca: uygulama durumu, liste ve son açık oturum (1c tasarımı §5.6). */
  async baslat(): Promise<void> {
    await this.#dene(async () => {
      this.uygulama = await ipc.uygulamaDurumu();
    });
    void this.#katalogEsitle();
    await this.#dene(async () => {
      this.denetci = await ipc.denetciKipi();
    });
    await this.gunlukSiniriYenile();
    await this.#dene(() => ipc.yalitimiDinle((d) => this.#yalitimYaz(d)));
    await this.#dene(() => ipc.mcpDinle((d) => this.#mcpYaz(d)));
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
      if (!alan) return;
      // Arka uçtaki mevcut katalog yerel IPC ile okunur; yeni ağ isteği beklenmez.
      const guncel = await ipc.uygulamaDurumu();
      const model = guncel.modeller.some((m) => m.kimlik === guncel.varsayilan_model)
        ? guncel.varsayilan_model
        : (guncel.modeller[0]?.kimlik ?? guncel.varsayilan_model);
      this.uygulama = { ...guncel, varsayilan_model: model };
      const acilis = await ipc.oturumBaslat(alan.kimlik, model);
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
    if (oldu !== undefined) {
      this.#uygula(oturum, { tip: 'ModelDegisti', model });
      const secenek = this.uygulama?.modeller.find((m) => m.kimlik === model);
      const mevcutEfor =
        this.ekranlar.get(oturum)?.ozet.efor ?? this.uygulama?.varsayilan_efor ?? 'orta';
      const eforlar = secenek?.eforlar ?? [];
      if (eforlar.length > 0 && !eforlar.includes(mevcutEfor)) {
        const yeniEfor = eforlar.includes('orta') ? 'orta' : eforlar[0];
        this.#uygula(oturum, { tip: 'EforDegisti', efor: yeniEfor });
      }
    }
  }

  async eforSec(efor: DusunceEforu): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    const oldu = await this.#dene(() => ipc.eforDegistir(oturum, efor));
    if (oldu !== undefined) this.#uygula(oturum, { tip: 'EforDegisti', efor });
  }

  async modelleriYenile(): Promise<void> {
    const liste = await this.#dene(() => ipc.modelleriYenile());
    if (liste && this.uygulama) {
      const varsayilan_model =
        liste.length > 0 && !liste.some((m) => m.kimlik === this.uygulama!.varsayilan_model)
          ? liste[0].kimlik
          : this.uygulama.varsayilan_model;
      this.uygulama = { ...this.uygulama, modeller: liste, varsayilan_model };
    }
  }

  async #katalogEsitle(): Promise<void> {
    try {
      const liste = await ipc.modelleriYenile();
      if (this.uygulama && liste.length > 0) {
        const varsayilan_model = liste.some((m) => m.kimlik === this.uygulama!.varsayilan_model)
          ? this.uygulama.varsayilan_model
          : liste[0].kimlik;
        this.uygulama = { ...this.uygulama, modeller: liste, varsayilan_model };
      }
    } catch {
      // Katalog alınamazsa (ör. çevrimdışı) mevcut uygulama durumu korunur.
    }
  }

  /** Bölümü açar; açık bölümün düğmesine yeniden basmak paneli kapatır. */
  sagBolumuDegistir(bolum: SagBolum): void {
    this.sagBolumuAc(this.sagBolum === bolum ? null : bolum);
  }

  sagBolumuAc(bolum: SagBolum | null): void {
    this.sagBolum = bolum;
    yerelYaz(SAG_BOLUM_ANAHTARI, bolum ?? '');
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

  /** Turun geri alma planını ister; sayfa plan olayı gelince açılır (aşama 3 tasarımı §5.6). */
  async geriAlmaOnizle(oturum: OturumId, tur: TurId): Promise<void> {
    await this.#dene(() => ipc.geriAlmaOnizle(oturum, tur));
  }

  /** Açık planı mührüyle yürütür; sayfa `TurGeriAlindi` gelince kapanır. */
  async turuGeriAl(oturum: OturumId): Promise<void> {
    const onizleme = this.ekranlar.get(oturum)?.geri_alma;
    if (!onizleme || this.geriAliniyor) return;
    this.geriAliniyor = true;
    try {
      await this.#dene(() => ipc.turuGeriAl(oturum, onizleme.tur, onizleme.muhur));
    } finally {
      this.geriAliniyor = false;
    }
  }

  geriAlmayiKapat(oturum: OturumId): void {
    this.#uygula(oturum, { tip: 'GeriAlmaKapandi' });
  }

  /**
   * Bütçe kararını gönderir (5b tasarımı §7). Kart, komut kabul edilince
   * yanıtlanır; Rust kararı reddederse kartı yeniden yayar.
   */
  async butceKarari(oturum: OturumId, karar: ButceKarari): Promise<void> {
    const gitti = await this.#dene(() => ipc.butceKarari(oturum, karar));
    if (gitti === undefined) return;
    this.#uygula(oturum, { tip: 'ButceYanitlandi' });
  }

  /** Yeni oturumların bütçe varsayılanını kaydeder; Rust da doğrular (5b §9). */
  async butceVarsayilaniniAyarla(istek: number, token: number): Promise<boolean> {
    const yeni = await this.#dene(() => ipc.oturumButcesiniAyarla(istek, token));
    if (yeni === undefined) return false;
    this.butceVarsayilani = yeni;
    return true;
  }

  /** Oturumu, kaydını ve geri alma verisini siler; onayı Rust yerel pencerede sorar. */
  async oturumuSil(oturum: OturumId): Promise<void> {
    const silindi = await this.#dene(() => ipc.oturumuSil(oturum));
    if (!silindi) return;
    this.ekranlar.delete(oturum);
    if (this.secili === oturum) this.secili = null;
    if (this.onay?.oturum === oturum) this.onay = null;
    await this.listeyiYenile();
  }

  onayiAc(oturum: OturumId, islem: IslemId): void {
    this.onay = { oturum, islem };
  }

  onayiKapat(): void {
    this.onay = null;
  }

  gunluguAc(oturum?: OturumId): void {
    this.gunlukOturum = oturum ?? this.secili;
  }

  gunluguKapat(): void {
    this.gunlukOturum = null;
  }

  async onizlemeAyarla(oturum: OturumId, acik: boolean): Promise<void> {
    const basarili = await this.#dene(() => ipc.onizlemeAyarla(oturum, acik));
    if (basarili !== undefined) {
      this.#uygula(oturum, { tip: 'OnizlemeDegisti', acik });
    }
  }

  /** "Bu oturumda izin ver" kısayolunu kaldırır; liste çekirdekten gelir (§9.1). */
  async kisayoluKaldir(oturum: OturumId, kimlik: number): Promise<void> {
    await this.#dene(() => ipc.kisayoluKaldir(oturum, kimlik));
  }

  async gonderimKarari(
    oturum: OturumId,
    islem: IslemId,
    gonderilmeyecekler: CagriId[],
    hepGonderilecekler: CagriId[],
  ): Promise<void> {
    // Kart karar gider gitmez kapanır: hızlı bir yanıtta tur, IPC dönmeden
    // biter ve hâlâ bekleyen kartı geçersiz sayardı.
    this.#uygula(oturum, { tip: 'OnizlemeYanitlandi', islem });
    const basarili = await this.#dene(() =>
      ipc.gonderimKarari(oturum, islem, gonderilmeyecekler, hepGonderilecekler),
    );
    if (basarili === undefined) {
      this.#uygula(oturum, { tip: 'OnizlemeGonderilemedi', islem });
    }
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

  /** Ayarlar'ı açar; Linux ve Windows'ta araç zinciri dizinlerini okur. */
  async ayarlariAc(): Promise<void> {
    this.gorunum = 'ayarlar';
    this.hata = null;
    this.teknikRapor = null;
    this.kopyalama = null;
    this.onarim = null;
    const boyut = await this.#dene(() => ipc.geriAlmaVerisiBoyutu());
    if (boyut !== undefined) this.geriAlmaBoyutu = boyut;
    const butce = await this.#dene(() => ipc.oturumButcesi());
    if (butce !== undefined) this.butceVarsayilani = butce;
    const platform = this.uygulama?.platform;
    if (platform !== 'linux' && platform !== 'windows') return;
    const dizinler = await this.#dene(() => ipc.aracZinciriDizinleri());
    if (dizinler) this.aracZinciri = dizinler;
    if (platform === 'windows') await this.#aracOnarimGerekliOku();
  }

  async #aracOnarimGerekliOku(): Promise<void> {
    const gerekli = await this.#dene(() => ipc.aracZinciriOnarimGerekli());
    if (gerekli !== undefined) this.aracOnarimGerekli = gerekli;
  }

  oturumlaraDon(): void {
    this.gorunum = 'oturumlar';
    this.hata = null;
  }

  /** Eklentiler ekranını açar; listeyi ekranın kendisi okur (aşama 4 tasarımı §14). */
  eklentileriAc(): void {
    this.gorunum = 'eklentiler';
    this.hata = null;
  }

  /**
   * Günlük sınırın şimdiki durumunu okur: açılışta ve tur bitince. Sınır bir
   * istek reddedilince konur, başarılı bir istekle kalkar (aşama 4c).
   */
  async gunlukSiniriYenile(): Promise<void> {
    const sinir = await this.#dene(() => ipc.gunlukSinir());
    if (sinir !== undefined) this.gunlukSinir = sinir;
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
      if (this.uygulama?.platform === 'windows') await this.#aracOnarimGerekliOku();
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
    if (this.uygulama?.platform === 'windows') await this.#aracOnarimGerekliOku();
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
    const yeniOgeler = yeni.ogeler.slice(onceki.ogeler.length);
    const turBitti = yeniOgeler.some((o) => o.tip === 'TurSonu' || o.tip === 'AktarimDurdu');
    if (turBitti) void this.listeyiYenile();
    // Hatayla biten tur sınıra çarpmış olabilir; normal biten tur sınırı kaldırmış olabilir.
    const sinirDegisti = yeniOgeler.some(
      (o) => o.tip === 'TurSonu' && (o.neden.tip === 'Normal' || o.neden.tip === 'Hata'),
    );
    if (sinirDegisti) void this.gunlukSiniriYenile();
  }

  /** Onay istekleri ve tur sonları duyurulur; akan token'lar duyurulmaz. */
  #duyur(onceki: OturumEkrani, yeni: OturumEkrani): void {
    for (const o of yeni.ogeler.slice(onceki.ogeler.length)) {
      if (o.tip === 'OnayIstegi') this.duyuru = m('duyuru.onay', { sayi: o.ogeler.length });
      else if (o.tip === 'SartIstegi') this.duyuru = m('duyuru.sart');
      else if (o.tip === 'ButceKarari') this.duyuru = m('duyuru.butce');
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
