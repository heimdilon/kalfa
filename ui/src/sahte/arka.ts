// Rust tarafının bellekteki sahtesi: testler ve geliştirme önizlemesi
// (`onizleme.html`) kullanır; uygulama paketine girmez. Komutlar `mockIPC` ile
// yakalanır; kanal mesajları dinleyen `Channel`'a doğrudan verilir.
import type { Channel } from '@tauri-apps/api/core';
import { mockIPC } from '@tauri-apps/api/mocks';
import { KIPLER } from '../ipc/tipler';
import type {
  CagriId,
  CalismaAlani,
  GecmisSayfasi,
  KanalMesaji,
  Kip,
  KomutHatasi,
  Olay,
  OnayOgesi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  SartMetni,
  SartPenceresiDurumu,
  TurId,
  UygulamaDurumu,
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

export class SahteArka {
  /** Gelen her komut, sırayla. */
  cagrilar: { komut: string; argumanlar: Argumanlar }[] = [];
  uygulama: UygulamaDurumu = {
    baglanti: 'Var',
    anahtar_deposu: 'Kalici',
    modeller: [
      { kimlik: 'qwen3.8-flash-next', profil: 'Hizli' },
      { kimlik: 'deepseek-v4-flash', profil: 'Dengeli' },
      { kimlik: 'glm-5.3', profil: 'Derin' },
      { kimlik: 'auto', profil: 'Otomatik' },
    ],
    varsayilan_model: 'glm-5.3',
    son_oturum: null,
    yalitim: {
      tip: 'Yok',
      neden: 'Desteklenmiyor',
      talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
    },
    platform: 'linux',
  };
  /** Ayarlar'daki araç zinciri dizinleri. */
  aracZinciri: string[] = [];
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
  sartMetni: SartMetni = {
    surum: 1,
    esasli: true,
    metin: '# Kullanım şartları\n\nDeneme metni.',
  };
  /** Bu komutlar verilen hatayla reddedilir. */
  hatalar = new Map<string, KomutHatasi>();
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
        });
        return this.#oturumlar.get(yeni)?.acilis;
      }
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
      default:
        // mesaj_gonder, turu_iptal_et, arac_onayi, model_degistir, paketi_isledim,
        // baglanti_ac, anahtar_penceresini_ac, sart_penceresini_ac
        return null;
    }
  }
}
