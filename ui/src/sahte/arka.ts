// Rust tarafının bellekteki sahtesi: testler ve geliştirme önizlemesi
// (`onizleme.html`) kullanır; uygulama paketine girmez. Komutlar `mockIPC` ile
// yakalanır; kanal mesajları dinleyen `Channel`'a doğrudan verilir.
import type { Channel } from '@tauri-apps/api/core';
import { mockIPC } from '@tauri-apps/api/mocks';
import { KIPLER } from '../ipc/tipler';
import type {
  AgHostu,
  AlanHedefi,
  EklentiSatiri,
  GizliAyarGorunumu,
  DenetciDurumu,
  DenetimKipi,
  CagriId,
  CalismaAlani,
  GecmisSayfasi,
  GeriAlmaPlani,
  GeriAlmaSonucu,
  GunlukSayfasi,
  GunlukSinirDurumu,
  IslemId,
  KanalMesaji,
  Kip,
  KomutHatasi,
  KurulumSonucu,
  Olay,
  OnarimSonucu,
  OnayOgesi,
  OturumAcilisi,
  OturumEklentisi,
  OturumId,
  OturumSatiri,
  SartMetni,
  SartPenceresiDurumu,
  TurId,
  UygulamaDurumu,
  YalitimCalismaAlani,
  YalitimDurumu,
} from '../ipc/tipler';

type Argumanlar = Record<string, unknown>;

export const GECERSIZ_ANAHTAR = 'evren_llm_gecersiz';
export const AGSIZ_ANAHTAR = 'evren_llm_agsiz';

interface SahteOturum {
  acilis: OturumAcilisi;
  satir: OturumSatiri;
  eskiSayfa: GecmisSayfasi;
}

export function bosOzet(model = 'glm-5.3', kip: Kip = 'Siki'): OturumAcilisi['ozet'] {
  return {
    model,
    kip,
    yonlendirilen_model: null,
    son_kredi: null,
    istem: 0,
    tamamlama: 0,
    dusunce: 0,
  };
}

/** Onay isteğindeki bir çağrı; verilmeyen alanlar boş. */
export function onayOgesi(ek: Partial<OnayOgesi> & { cagri: CagriId }): OnayOgesi {
  return {
    arac: 'dosya_yaz',
    aciklama: '',
    hedefler: [],
    diff: null,
    uyarilar: [],
    muhur: `muhur-${ek.cagri}`,
    kabuk: null,
    yerel_onay: false,
    ...ek,
  };
}

/**
 * Eklentiler ekranının örnek verisi (aşama 4d): Kalfa'yla gelen `kalfa-rag`
 * (bir çalışma alanında açık, gönderim izni verilmiş), klasörden kurulmuş bir
 * eklenti ve yüklenemeyen bir eklenti. Her çağrıda yeni kopya döner.
 */
export function ornekEklentiler(): EklentiSatiri[] {
  return [
    {
      ad: 'kalfa-rag',
      surum: '0.1.0',
      aciklama: 'Çalışma alanında Türkçe anlamsal arama.',
      yerlesik: true,
      durum: { tip: 'Yuklendi' },
      sha256: '9f2c4e7a1b8d03a65c4f7e2b90d1a3c8e5f6b7a4d2c1908e3f4a5b6c7d8e9f01',
      kaynak: null,
      kurulum_ms: null,
      yetenek:
        "Bu eklenti `**/*.md`, `**/*.rs` ile eşleşen dosyaları okuyacak ve bu dosyalardan seçtiği parçaları gömme ve yeniden sıralama için EVREN'e gönderecek. Gizli dosyaları okuyamaz. Dosya yazamaz, komut çalıştıramaz, internete doğrudan erişemez. Kendi verisi için çalışma alanı başına en fazla 200 MB yer kullanacak.",
      araclar: [
        {
          ad: 'kalfa_rag__anlamsal_ara',
          aciklama: 'Çalışma alanındaki dosyalarda anlama göre arar ve kaynaklı sonuç verir.',
        },
      ],
      alanlar: [
        {
          kimlik: 'alan-1',
          yol: 'C:\\projeler\\kalfa-api',
          acik: true,
          gonderim_izni_ms: new Date(2026, 9, 20, 10, 5).getTime(),
          depo_bayt: 6_291_456,
        },
      ],
    },
    {
      ad: 'ornek-notlar',
      surum: '0.2.0',
      aciklama: 'Not dosyalarını özetler.',
      yerlesik: false,
      durum: { tip: 'Yuklendi' },
      sha256: '1d0e8f7c6b5a49382716f5e4d3c2b1a0f9e8d7c6b5a4938271605f4e3d2c1b0a',
      kaynak: 'C:\\Users\\kullanici\\eklentiler\\ornek-notlar',
      kurulum_ms: new Date(2026, 9, 18, 14, 30).getTime(),
      yetenek:
        'Bu eklenti çalışma alanındaki dosyaları okumayacak ve EVREN\'e veri göndermeyecek. Gizli dosyaları okuyamaz. Dosya yazamaz, komut çalıştıramaz, internete doğrudan erişemez. Kendi verisini saklamayacak.',
      araclar: [{ ad: 'ornek_notlar__ozetle', aciklama: 'Verilen metni kısaca özetler.' }],
      alanlar: [],
    },
    {
      ad: 'bozuk-eklenti',
      surum: '1.0.0',
      aciklama: 'Kurulumdan sonra bileşeni değişmiş bir eklenti.',
      yerlesik: false,
      durum: {
        tip: 'Yuklenemedi',
        neden: 'Eklentinin .wasm dosyası kurulumdan sonra değişmiş; eklentiyi yeniden kurun.',
      },
      sha256: '',
      kaynak: 'C:\\Users\\kullanici\\eklentiler\\bozuk-eklenti',
      kurulum_ms: new Date(2026, 9, 10, 9, 0).getTime(),
      yetenek: null,
      araclar: [],
      alanlar: [],
    },
  ];
}

export class SahteArka {
  /** Gelen her komut, sırayla. */
  cagrilar: { komut: string; argumanlar: Argumanlar }[] = [];
  uygulama: UygulamaDurumu = {
    baglanti: 'Var',
    anahtar_deposu: 'Kalici',
    modeller: [
      { kimlik: 'qwen3.8-flash-next', profil: 'Hizli', eforlar: ['kapali', 'dusuk', 'orta', 'yuksek'] },
      { kimlik: 'deepseek-v4-flash', profil: 'Dengeli', eforlar: ['kapali', 'dusuk', 'orta', 'yuksek'] },
      { kimlik: 'glm-5.3', profil: 'Derin', eforlar: ['kapali', 'dusuk', 'orta', 'yuksek'] },
      { kimlik: 'auto', profil: 'Otomatik', eforlar: ['kapali', 'dusuk', 'orta', 'yuksek'] },
    ],
    varsayilan_model: 'glm-5.3',
    varsayilan_efor: 'orta',
    son_oturum: null,
    yalitim: {
      tip: 'Yok',
      neden: 'Desteklenmiyor',
      talimat: "Bu platformda Kalfa'nın yalıtımı yok.",
      ayrinti: 'denetim=platform; yalıtım arka ucu yok',
    },
    yalitimsiz_izinli: false,
    platform: 'linux',
  };
  /** `yalitimsizi_ayarla`'nın yerel penceresinin yanıtı. */
  yalitimsizOnayi = true;
  /** `yalitimi_onar`'ın sonucu; `Tamam` ise yeni durum da yazılır. */
  onarimSonucu: OnarimSonucu = {
    tip: 'Tamam',
    yalitim: { tip: 'Hazir', arka_uc: 'windows', surum: 'AppContainer' },
  };
  /** Ayarlar'daki araç zinciri dizinleri. */
  aracZinciri: string[] = [];
  /** Kayıtlı bir araç zinciri dizini kapta okunamıyor; başarılı Onar temizler. */
  aracOnarimGerekli = false;
  /** Ayarlar'daki Windows yalıtım çalışma alanları. */
  hazirAlanlar: YalitimCalismaAlani[] = [];
  /** `yalitim_calisma_alanini_kaldir` için yerel onay yanıtı. */
  yalitimKaldirmaOnayi = true;
  /** `arac_zinciri_dizini_ekle`'nin klasör seçicisinin döneceği dizin; `null` vazgeçmektir. */
  aracKlasoru: string | null = '/opt/node';
  /** `yalitimi_algila` bu durumu bulur; `null` ise durum değişmez. */
  sonrakiYalitim: YalitimDurumu | null = null;
  #yalitimKanali: Channel<YalitimDurumu> | null = null;
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
  klasor: CalismaAlani | null = { kimlik: 1, yol: 'C:\\projeler\\kalfa' };
  /** İzin kipi yükseltmesinde yerel iletişim kutusunun yanıtı. */
  yukseltmeOnayi = true;
  /** Şart penceresinin durumu ve gösterilecek metin. */
  sart: SartPenceresiDurumu = { tip: 'Tamam' };
  denetci: DenetciDurumu = { kip: 'Kapali' };
  /** Ayarlar > "Gizli dosyalar" (3e). */
  gizli: GizliAyarGorunumu = {
    varsayilanlar: ['.env', '.env.*', '*.pem', 'credentials*'],
    ek_kaliplar: [],
    gizli_sayilmayanlar: [],
  };
  agListesi: AgHostu[] = [];
  /** Gizliliği azaltan ayarların yerel penceresinin yanıtı (3e). */
  gizliOnayi = true;
  sartMetni: SartMetni = {
    surum: 1,
    esasli: true,
    metin: '# Kullanım şartları\n\nDeneme metni.',
  };
  /** Bu komutlar verilen hatayla reddedilir. */
  hatalar = new Map<string, KomutHatasi>();
  /** `oturumu_sil`'in yerel penceresinin yanıtı. */
  silmeOnayi = true;
  /** `geri_alma_onizle`'nin yayacağı plan; `null` ise olay gelmez. */
  geriAlmaPlani: GeriAlmaPlani | null = null;
  /** `geri_alma_verisi_boyutu`'nun yanıtı. */
  geriAlmaBoyutu = 0;
  /** Gönderim günlüğü sayfaları; yoksa boş döner. */
  gonderimGunlukleri = new Map<OturumId, GunlukSayfasi>();
  /** `gonderim_ogesi` metinleri; anahtar: `${oturum}:${sira}`. */
  gonderimOgeleri = new Map<string, string>();
  /** Eklentiler ekranının listesi (aşama 4d); örnek veri `ornekEklentiler()`'dedir. */
  eklentiler: EklentiSatiri[] = ornekEklentiler();
  /** `eklenti_klasorden_kur`'un sonucu (klasör seçici ve yerel onay penceresi). */
  kurulumSonucu: KurulumSonucu = { tip: 'Vazgecildi' };
  /** `eklenti_kaldir` ve `eklenti_verisini_sil` için yerel onay yanıtı. */
  eklentiOnayi = true;
  /**
   * "Bu oturumdaki izinler" panelinin eklentileri. Varsayılan boştur: panel
   * ancak bir test ya da senaryo eklenti verirse görünür.
   */
  oturumEklentileri: OturumEklentisi[] = [];
  /** `eklenti_istegi` gövdeleri; anahtar: işlem kimliği. */
  eklentiIstekleri = new Map<IslemId, string>();
  /** `gunluk_sinir`'in yanıtı; `null`: sınır dolmadı. */
  gunlukSinir: GunlukSinirDurumu | null = null;
  /** `oturum_butcesi`'nin yanıtı: yeni oturumların bütçe varsayılanı (5b §9). */
  butceVarsayilani: { istek: number; token: number } = { istek: 200, token: 5_000_000 };
  /** Komutun yerine geçen davranış (önizlemenin senaryosu kullanır). */
  ozel = new Map<string, (a: Argumanlar) => unknown>();

  #oturumlar = new Map<OturumId, SahteOturum>();
  #kanallar = new Map<OturumId, Channel<KanalMesaji>>();
  #no = new Map<OturumId, number>();
  #sira = new Map<OturumId, number>();
  #sayac = 0;

  kur(): this {
    mockIPC((komut, argumanlar) => this.#isle(komut, (argumanlar ?? {}) as Argumanlar));
    return this;
  }

  /** Kayıtlı bir oturum ekler (listede görünür, açılabilir). */
  oturumEkle(
    acilis: Partial<OturumAcilisi> & { oturum: OturumId },
    satir: Partial<OturumSatiri> = {},
    eskiSayfa: GecmisSayfasi = { ogeler: [], daha_eski_var: false },
  ): OturumId {
    const tam: OturumAcilisi = {
      calisma_alani: 'C:\\projeler\\kalfa',
      gecmis: { ogeler: [], daha_eski_var: false },
      ozet: bosOzet(),
      salt_okunur: null,
      butce: null,
      ...acilis,
    };
    this.#oturumlar.set(tam.oturum, {
      acilis: tam,
      satir: {
        oturum: tam.oturum,
        dosya: `C:\\veri\\oturumlar\\${tam.oturum}.jsonl`,
        baslik: null,
        calisma_alani: tam.calisma_alani,
        son_degisiklik_ms: Date.now(),
        durum: { tip: 'Bos' },
        ...satir,
      },
      eskiSayfa,
    });
    this.#sira.set(tam.oturum, Math.max(0, ...tam.gecmis.ogeler.map((o) => o.sira)));
    return tam.oturum;
  }

  /** Listede yalnızca satırı olan (okunamayan) kayıt. */
  satirEkle(satir: OturumSatiri): void {
    this.#oturumlar.set(satir.dosya, {
      acilis: {
        oturum: satir.dosya,
        calisma_alani: null,
        gecmis: { ogeler: [], daha_eski_var: false },
        ozet: bosOzet(),
        salt_okunur: null,
        butce: null,
      },
      satir,
      eskiSayfa: { ogeler: [], daha_eski_var: false },
    });
  }

  dinleniyor(oturum: OturumId): boolean {
    return this.#kanallar.has(oturum);
  }

  /** Dinlenen oturuma olayları tek pakette gönderir. */
  olaylar(oturum: OturumId, olaylar: Olay[], tur: TurId | null = 'tur-1'): void {
    const sira = this.#sira.get(oturum) ?? 0;
    const zarflar = olaylar.map((yuk, i) => ({ sema: 1, oturum, tur, sira: sira + i + 1, yuk }));
    this.#sira.set(oturum, sira + olaylar.length);
    this.paket(oturum, { tip: 'Olaylar', no: this.#sonrakiNo(oturum), zarflar });
  }

  /** Numarayı elle veren ham paket (boşluk testleri). */
  paket(oturum: OturumId, mesaj: KanalMesaji): void {
    const kanal = this.#kanallar.get(oturum);
    if (!kanal) throw new Error(`${oturum} dinlenmiyor`);
    kanal.onmessage(mesaj);
  }

  /** Sıradaki paket numarasını harcamadan verir. */
  siradakiNo(oturum: OturumId): number {
    return this.#no.get(oturum) ?? 1;
  }

  /** Yalıtım durumu değişir ve dinleyen pencereye yazılır. */
  yalitimDurumu(yeni: YalitimDurumu): void {
    this.uygulama.yalitim = yeni;
    this.#yalitimKanali?.onmessage(yeni);
  }

  komutlar(ad: string): Argumanlar[] {
    return this.cagrilar.filter((c) => c.komut === ad).map((c) => c.argumanlar);
  }

  #sonrakiNo(oturum: OturumId): number {
    const no = this.siradakiNo(oturum);
    this.#no.set(oturum, no + 1);
    return no;
  }

  #eklenti(ad: unknown): EklentiSatiri | undefined {
    return this.eklentiler.find((e) => e.ad === ad);
  }

  /** Kurulan eklenti listeye girer; güncellemenin kapsamı değiştiyse izinler silinir. */
  #eklentiKuruldu(sonuc: Extract<KurulumSonucu, { tip: 'Kuruldu' }>): void {
    const mevcut = this.#eklenti(sonuc.ad);
    if (mevcut) {
      mevcut.surum = sonuc.surum;
      mevcut.durum = { tip: 'Yuklendi' };
      if (sonuc.izinler_silindi) {
        for (const alan of mevcut.alanlar) alan.gonderim_izni_ms = null;
      }
      return;
    }
    this.eklentiler = [
      ...this.eklentiler,
      {
        ad: sonuc.ad,
        surum: sonuc.surum,
        aciklama: 'Klasörden kurulan eklenti.',
        yerlesik: false,
        durum: { tip: 'Yuklendi' },
        sha256: 'c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00c0ffee00',
        kaynak: `C:\\Users\\kullanici\\eklentiler\\${sonuc.ad}`,
        kurulum_ms: Date.now(),
        yetenek: null,
        araclar: [],
        alanlar: [],
      },
    ];
  }

  #isle(komut: string, a: Argumanlar): unknown {
    this.cagrilar.push({ komut, argumanlar: a });
    const hata = this.hatalar.get(komut);
    if (hata) throw hata;
    const ozel = this.ozel.get(komut);
    if (ozel) return ozel(a);
    const oturum = a.oturum as OturumId;
    switch (komut) {
      case 'uygulama_durumu':
        return this.uygulama;
      case 'modelleri_yenile':
        return this.uygulama.modeller;
      case 'efor_degistir': {
        const o = this.#oturumlar.get(oturum);
        if (o) o.acilis.ozet.efor = a.efor as NonNullable<OturumAcilisi['ozet']['efor']>;
        return null;
      }
      case 'denetci_kipi':
        return this.denetci;
      case 'denetci_kipini_ayarla':
        this.denetci = { kip: a.kip as DenetimKipi };
        return this.denetci;
      case 'gizli_ayarlari':
        return this.gizli;
      case 'ek_gizli_kalip_ekle': {
        const k = String(a.kalip);
        if (!this.gizli.ek_kaliplar.includes(k)) this.gizli.ek_kaliplar = [...this.gizli.ek_kaliplar, k];
        return this.gizli;
      }
      case 'ek_gizli_kalip_kaldir':
        if (this.gizliOnayi) {
          this.gizli.ek_kaliplar = this.gizli.ek_kaliplar.filter((k) => k !== a.kalip);
        }
        return this.gizli;
      case 'gizli_sayilmayan_ekle': {
        const k = String(a.kalip);
        if (this.gizliOnayi && !this.gizli.gizli_sayilmayanlar.includes(k)) {
          this.gizli.gizli_sayilmayanlar = [...this.gizli.gizli_sayilmayanlar, k];
        }
        return this.gizli;
      }
      case 'gizli_sayilmayan_kaldir':
        this.gizli.gizli_sayilmayanlar = this.gizli.gizli_sayilmayanlar.filter(
          (k) => k !== a.kalip,
        );
        return this.gizli;
      case 'ag_izin_listesi':
        return this.agListesi;
      case 'ag_hostu_ekle': {
        if (this.gizliOnayi) {
          const [host, port] = String(a.host).split(':');
          this.agListesi = [
            ...this.agListesi,
            { host, port: port ? Number(port) : 443, ozel_ag: Boolean(a.ozel_ag) },
          ];
        }
        return this.agListesi;
      }
      case 'ag_hostu_kaldir': {
        const [host, port] = String(a.host).split(':');
        const p = port ? Number(port) : 443;
        this.agListesi = this.agListesi.filter((h) => !(h.host === host && h.port === p));
        return this.agListesi;
      }
      case 'oturumlari_listele':
        return [...this.#oturumlar.values()]
          .map((o) => o.satir)
          .sort((x, y) => y.son_degisiklik_ms - x.son_degisiklik_ms);
      case 'calisma_alani_sec':
        return this.klasor;
      case 'oturum_baslat': {
        this.#sayac += 1;
        const yeni = `oturum-yeni-${this.#sayac}`;
        this.oturumEkle({
          oturum: yeni,
          calisma_alani: this.klasor?.yol ?? null,
          ozet: bosOzet(a.model as string),
          butce: {
            sinirlar: this.butceVarsayilani,
            baslatilan: 0,
            bilinen_token: 0,
            bilinmeyen: 0,
            onaylanan_bilinmeyen: 0,
            aktif: 0,
            engel: null,
          },
        });
        return this.#oturumlar.get(yeni)?.acilis;
      }
      case 'oturum_butcesi':
        return this.butceVarsayilani;
      case 'oturum_butcesini_ayarla': {
        this.butceVarsayilani = { istek: a.istek as number, token: a.token as number };
        return this.butceVarsayilani;
      }
      case 'butce_karari':
        return undefined;
      case 'oturum_ac': {
        const o = this.#oturumlar.get(oturum);
        if (!o) throw { kod: 'OturumYok', mesaj: 'Bu oturumun kaydı bulunamadı.' };
        return o.acilis;
      }
      case 'oturumu_dinle':
        this.#kanallar.set(oturum, a.kanal as Channel<KanalMesaji>);
        this.#no.set(oturum, 1);
        return null;
      case 'gecmis_sayfasi':
        return this.#oturumlar.get(oturum)?.eskiSayfa;
      case 'izin_kipi_degistir': {
        const o = this.#oturumlar.get(oturum);
        const eski = o?.acilis.ozet.kip ?? 'Siki';
        const yeni = a.kip as Kip;
        const yukseltme = KIPLER.indexOf(yeni) > KIPLER.indexOf(eski);
        const sonuc = yukseltme && !this.yukseltmeOnayi ? eski : yeni;
        if (o) o.acilis.ozet.kip = sonuc;
        return sonuc;
      }
      case 'oturumu_kapat':
        this.#kanallar.delete(oturum);
        return null;
      case 'geri_alma_onizle':
        if (this.geriAlmaPlani && this.#kanallar.has(oturum)) {
          const plan = this.geriAlmaPlani;
          const tur = a.tur as TurId;
          queueMicrotask(() =>
            this.olaylar(
              oturum,
              [{ tip: 'GeriAlmaPlani', geri_alinan_tur: tur, plan, muhur: `muhur-${tur}` }],
              null,
            ),
          );
        }
        return null;
      case 'turu_geri_al':
        if (this.geriAlmaPlani && this.#kanallar.has(oturum)) {
          const sonuc: GeriAlmaSonucu[] = this.geriAlmaPlani.ogeler.map((o) => ({
            yol: o.yol,
            eylem: o.eylem,
          }));
          const tur = a.tur as TurId;
          queueMicrotask(() =>
            this.olaylar(oturum, [{ tip: 'TurGeriAlindi', geri_alinan_tur: tur, sonuc }], null),
          );
        }
        return null;
      case 'oturumu_sil':
        if (!this.silmeOnayi) return false;
        this.#oturumlar.delete(oturum);
        this.#kanallar.delete(oturum);
        return true;
      case 'geri_alma_verisi_boyutu':
        return this.geriAlmaBoyutu;
      case 'sart_penceresi_durumu':
        return this.sart;
      case 'anahtari_kaydet': {
        const anahtar = String(a.anahtar).trim();
        if (!anahtar || anahtar === GECERSIZ_ANAHTAR) return { tip: 'Gecersiz' };
        if (anahtar === AGSIZ_ANAHTAR) return { tip: 'Ulasilamadi' };
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return { tip: 'Kaydedildi', depo: this.uygulama.anahtar_deposu, sonraki: this.sart };
      }
      case 'sartlari_kabul_et': {
        if (this.sart.tip === 'Kilitli') return { tip: 'Kilitli' };
        if (this.sart.tip !== 'SartGerekli') {
          throw { kod: 'GecersizGirdi', mesaj: 'Onay bekleyen şart metni yok.' };
        }
        if (a.surum !== this.sartMetni.surum) {
          this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
          return { tip: 'YeniMetin', metin: this.sartMetni };
        }
        this.sart = { tip: 'Tamam' };
        return { tip: 'Kabul' };
      }
      case 'sartlari_reddet':
        if (this.sart.tip === 'SartGerekli') this.sart = { tip: 'Kilitli' };
        return null;
      case 'sart_metnini_goster':
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return this.sart;
      case 'yalitimi_dinle':
        this.#yalitimKanali = a.kanal as Channel<YalitimDurumu>;
        this.#yalitimKanali.onmessage(this.uygulama.yalitim);
        return null;
      case 'yalitimi_algila':
        if (this.sonrakiYalitim) this.yalitimDurumu(this.sonrakiYalitim);
        this.sonrakiYalitim = null;
        return this.uygulama.yalitim;
      case 'arac_zinciri_dizinleri':
        return [...this.aracZinciri];
      case 'arac_zinciri_dizini_ekle':
        if (this.aracKlasoru && !this.aracZinciri.includes(this.aracKlasoru)) {
          this.aracZinciri.push(this.aracKlasoru);
        }
        return [...this.aracZinciri];
      case 'arac_zinciri_dizini_kaldir':
        this.aracZinciri = this.aracZinciri.filter((d) => d !== a.yol);
        return [...this.aracZinciri];
      case 'yalitim_calisma_alanlari':
        return [...this.hazirAlanlar];
      case 'yalitim_calisma_alanini_kaldir':
        if (!this.yalitimKaldirmaOnayi) return false;
        if (!this.hazirAlanlar.some((alan) => alan.kimlik === a.kimlik)) return false;
        this.hazirAlanlar = this.hazirAlanlar.filter((alan) => alan.kimlik !== a.kimlik);
        return true;
      case 'arac_zinciri_onarim_gerekli':
        return this.aracOnarimGerekli;
      case 'yalitimi_onar':
        if (this.onarimSonucu.tip === 'Tamam') {
          this.yalitimDurumu(this.onarimSonucu.yalitim);
          this.aracOnarimGerekli = false;
        }
        return this.onarimSonucu;
      case 'yalitim_teknik_raporu':
        return `Kalfa yalıtım raporu\nDurum: ${this.uygulama.yalitim.tip}\n`;
      case 'yalitimsizi_ayarla':
        if (!a.acik || this.yalitimsizOnayi) this.uygulama.yalitimsiz_izinli = Boolean(a.acik);
        return this.uygulama.yalitimsiz_izinli;
      case 'gonderim_gunlugu': {
        const sayfa = this.gonderimGunlukleri.get(oturum);
        return sayfa ?? { satirlar: [], daha_eski_var: false };
      }
      case 'gonderim_ogesi': {
        return this.gonderimOgeleri.get(`${oturum}:${a.sira}`) ?? `Öğe metni (${a.sira})`;
      }
      case 'onizleme_ayarla': {
        const o = this.#oturumlar.get(oturum);
        if (o) o.acilis.ozet.onizleme_acik = Boolean(a.acik);
        return null;
      }
      case 'gonderim_karari':
        return null;
      case 'eklentileri_listele':
        return structuredClone(this.eklentiler);
      case 'eklenti_klasorden_kur': {
        const sonuc = this.kurulumSonucu;
        if (sonuc.tip === 'Kuruldu') this.#eklentiKuruldu(sonuc);
        return sonuc;
      }
      case 'eklenti_kaldir': {
        const e = this.eklentiler.find((x) => x.ad === a.ad && !x.yerlesik);
        if (!this.eklentiOnayi || !e) return false;
        this.eklentiler = this.eklentiler.filter((x) => x.ad !== a.ad || x.yerlesik);
        return true;
      }
      case 'eklenti_etkin_ayarla': {
        const e = this.#eklenti(a.ad);
        if (e) e.durum = { tip: a.etkin ? 'Yuklendi' : 'DevreDisi' };
        return null;
      }
      case 'eklenti_alani_ayarla': {
        const hedef = a.hedef as AlanHedefi;
        const acik = Boolean(a.acik);
        // Kapatmak gönderim iznini de kaldırır (aşama 4 tasarımı §13.3).
        const alan =
          hedef.tip === 'Kimlik'
            ? this.#eklenti(a.ad)?.alanlar.find((x) => x.kimlik === hedef.kimlik)
            : this.oturumEklentileri.find((x) => x.ad === a.ad);
        if (alan) {
          alan.acik = acik;
          if (!acik) alan.gonderim_izni_ms = null;
        }
        return null;
      }
      case 'gonderim_iznini_kaldir': {
        const alan = this.#eklenti(a.ad)?.alanlar.find((x) => x.kimlik === a.kimlik);
        if (alan) alan.gonderim_izni_ms = null;
        return null;
      }
      case 'eklenti_verisini_sil': {
        const alan = this.#eklenti(a.ad)?.alanlar.find((x) => x.kimlik === a.kimlik);
        if (!this.eklentiOnayi || !alan) return false;
        alan.depo_bayt = 0;
        return true;
      }
      case 'oturum_eklentileri':
        return structuredClone(this.oturumEklentileri);
      case 'eklenti_istegi':
        return (
          this.eklentiIstekleri.get(a.islem as IslemId) ??
          '{"model":"qwen3-embedding-8b","input":["fn main() {}"]}'
        );
      case 'gunluk_sinir':
        return this.gunlukSinir;
      default:
        // mesaj_gonder, turu_iptal_et, arac_onayi, model_degistir, paketi_isledim,
        // baglanti_ac, anahtar_penceresini_ac, sart_penceresini_ac
        return null;
    }
  }
}
