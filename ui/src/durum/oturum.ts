// Bir oturumun ekran durumu ve onu güncelleyen saf işlev (1c tasarımı §8.4).
// `uygula` DOM'a dokunmaz; eski durumu değiştirmez, yenisini döner. Svelte
// yalnızca bu durumu çizer.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  BitisNedeni,
  CagriId,
  DenetimSonucu,
  DurumOzeti,
  GecmisSayfasi,
  GeriAlmaPlani,
  GeriAlmaSonucu,
  TurOzeti,
  GorunumOgesi,
  IslemId,
  KanalMesaji,
  Kip,
  Kisayol,
  Olay,
  OnayOgesi,
  OnizlemeNedeni,
  OnizlemeOgesi,
  OturumAcilisi,
  OturumId,
  SaltOkunurNedeni,
  TurId,
  Zarf,
} from '../ipc/tipler';
import { BOS_TERMINAL, terminalEkle, type Terminal } from './terminal';

/** Oturumun o anki hâli; oturum listesindeki nokta ve Composer buna bakar. */
export type Anlik = 'Bos' | 'Calisiyor' | 'OnayBekliyor' | 'SartBekliyor';

interface Ortak {
  /** Svelte `each` bloğunun anahtarı; öğe var oldukça değişmez. */
  anahtar: string;
  sira: number;
  tur: TurId | null;
}

export type EkranOgesi =
  | (Ortak & { tip: 'KullaniciMesaji'; metin: string })
  | (Ortak & { tip: 'Yanit'; metin: string; dusunce: string; akista: boolean })
  | (Ortak & {
      tip: 'AracCagrisi';
      /** Kayıttan gelen çağrıda yok. */
      cagri: CagriId | null;
      arac: string;
      aciklama: string | null;
      argumanlar: string | null;
      /** `null`: çağrı hiç başlamadı. */
      durum: AracDurumu | 'Suruyor' | null;
      sonuc: string | null;
      /** Kabuğun canlı çıktısı; kayıttan gelen çağrıda yok. */
      terminal: Terminal | null;
      /** Güvenlik denetçisinin rozeti (3c); eski kayıtta ve kapalı kipte yok. */
      denetim: DenetimRozeti | null;
    })
  | (Ortak & {
      tip: 'OnayIstegi';
      islem: IslemId;
      ogeler: OnayOgesi[];
      durum: 'Bekliyor' | 'Yanitlandi' | 'Gecersiz';
    })
  | (Ortak & { tip: 'SartIstegi'; surum: number; durum: 'Bekliyor' | 'Cozuldu' })
  | (Ortak & { tip: 'Hata'; mesaj: string })
  | (Ortak & { tip: 'GuvenlikUyarisi'; mesaj: string })
  | (Ortak & { tip: 'TurSonu'; neden: BitisNedeni })
  | (Ortak & { tip: 'AktarimDurdu'; neden: AktarimDurmaNedeni })
  | (Ortak & {
      tip: 'GeriAlma';
      geri_alinan_tur: TurId;
      sonuc: GeriAlmaSonucu[];
      yarim: boolean;
    })
  | (Ortak & { tip: 'KalfaNotu'; metin: string })
  | (Ortak & {
      tip: 'BaglamOzetlendi';
      adet: number;
      yer_tutucu: number;
      once: number;
      sonra: number;
    })
  /** `geri_alindi`: sonradan bu tur geri alındı (aşama 3 tasarımı §5.11). */
  | (Ortak & { tip: 'TurOzeti'; ozet: TurOzeti; geri_alindi: boolean })
  /** Gönderim önizlemesi (aşama 3b). */
  | (Ortak & {
      tip: 'GonderimOnizlemesi';
      islem: IslemId;
      ogeler: OnizlemeOgesi[];
      neden: OnizlemeNedeni;
      durum: 'Bekliyor' | 'Yanitlandi' | 'Gecersiz';
    });

/** Açık geri alma sayfası: son gelen plan ve mührü (aşama 3 tasarımı §5.6). */
export interface GeriAlmaOnizlemesi {
  tur: TurId;
  plan: GeriAlmaPlani;
  muhur: string;
}

export interface OturumEkrani {
  oturum: OturumId;
  calisma_alani: string | null;
  salt_okunur: SaltOkunurNedeni | null;
  ogeler: EkranOgesi[];
  daha_eski_var: boolean;
  ozet: DurumOzeti;
  anlik: Anlik;
  aktif_tur: TurId | null;
  /** Beklenen paket numarası; abonelik 1'den başlar (1c tasarımı §4.4). */
  beklenen_no: number;
  /**
   * Açılışta okunan geçmişin son `sira`sı. Bundan küçük ya da eşit `sira`lı
   * canlı olayın etkisi geçmişte zaten vardır; atlanır.
   */
  gecmis_son_sira: number;
  /** Olay kuyruğu kapandı; bu abonelikten başka mesaj gelmez. */
  kapandi: boolean;
  /** Paket numarasında boşluk görüldü: ekran oturumu kayıttan yeniden açar. */
  bosluk: boolean;
  /** Canlı yerel öğelerin anahtarı için sayaç. */
  yerel_sayac: number;
  /** Geri alma sayfası açıksa gösterdiği plan. */
  geri_alma: GeriAlmaOnizlemesi | null;
  /** Bu oturumdaki İstekBilgisi olaylarının sayısı (günlük ekranını yenilemek için). */
  istek_sayaci: number;
  /** "Bu oturumda izin ver" kısayolları (aşama 3 tasarımı §9.1); kayda yazılmaz. */
  kisayollar: Kisayol[];
}

/** Araç kartındaki denetim rozetinin durumu (3c Görev 8). */
export type DenetimRozeti = { tip: 'Suruyor' } | { tip: 'Atlandi' } | DenetimSonucu;

export type Girdi =
  | { tip: 'Kanal'; mesaj: KanalMesaji }
  | { tip: 'EskiSayfa'; sayfa: GecmisSayfasi }
  | { tip: 'MesajGonderildi'; metin: string }
  | { tip: 'OnayYanitlandi'; islem: IslemId }
  | { tip: 'ModelDegisti'; model: string }
  | { tip: 'KipDegisti'; kip: Kip }
  | { tip: 'OnizlemeYanitlandi'; islem: IslemId }
  /** Gönderim kararı arka uca ulaşmadı; kart yeniden karar bekler. */
  | { tip: 'OnizlemeGonderilemedi'; islem: IslemId }
  | { tip: 'OnizlemeDegisti'; acik: boolean }
  /** Geri alma sayfası karar verilmeden kapandı. */
  | { tip: 'GeriAlmaKapandi' };

/** Kayıttan gelen bir geçmiş öğesini ekran öğesine çevirir. */
export function gecmisOgesi(o: GorunumOgesi): EkranOgesi {
  switch (o.tip) {
    case 'KullaniciMesaji':
      return { ...o, anahtar: `g-${o.sira}-k` };
    case 'Yanit':
      return {
        tip: 'Yanit',
        anahtar: `g-${o.sira}-y`,
        sira: o.sira,
        tur: o.tur,
        metin: o.metin ?? '',
        dusunce: o.dusunce ?? '',
        akista: false,
      };
    case 'AracCagrisi':
      return {
        tip: 'AracCagrisi',
        anahtar: `g-${o.sira}-a-${o.model_cagri_id}`,
        sira: o.sira,
        tur: o.tur,
        cagri: null,
        arac: o.arac,
        aciklama: null,
        argumanlar: o.argumanlar,
        durum: o.durum,
        sonuc: o.sonuc,
        terminal: null,
        denetim: o.denetim ?? null,
      };
    case 'TurSonu':
      return { ...o, anahtar: `g-${o.sira}-s` };
    case 'GuvenlikUyarisi':
      return { ...o, anahtar: `g-${o.sira}-g` };
    case 'GeriAlma':
      return { ...o, anahtar: `g-${o.sira}-r` };
    case 'KalfaNotu':
      return { ...o, anahtar: `g-${o.sira}-n` };
    case 'TurOzeti':
      return { ...o, anahtar: `g-${o.sira}-o`, geri_alindi: false };
  }
}

/** Geri alınmış turların özetlerini işaretler. */
function geriAlinanlariIsle(ogeler: EkranOgesi[]): EkranOgesi[] {
  const alinan = new Set(
    ogeler.flatMap((o) => (o.tip === 'GeriAlma' && !o.yarim ? [o.geri_alinan_tur] : [])),
  );
  if (alinan.size === 0) return ogeler;
  return ogeler.map((o) =>
    o.tip === 'TurOzeti' && alinan.has(o.ozet.tur) ? { ...o, geri_alindi: true } : o,
  );
}

/** `oturum_baslat` ya da `oturum_ac`'ın sonucundan ilk ekran durumu. */
export function acilistanEkran(acilis: OturumAcilisi): OturumEkrani {
  const ogeler = geriAlinanlariIsle(acilis.gecmis.ogeler.map(gecmisOgesi));
  return {
    oturum: acilis.oturum,
    calisma_alani: acilis.calisma_alani,
    salt_okunur: acilis.salt_okunur,
    ogeler,
    daha_eski_var: acilis.gecmis.daha_eski_var,
    ozet: acilis.ozet,
    anlik: 'Bos',
    aktif_tur: null,
    beklenen_no: 1,
    gecmis_son_sira: ogeler.reduce((en, o) => Math.max(en, o.sira), 0),
    kapandi: false,
    bosluk: false,
    yerel_sayac: 0,
    geri_alma: null,
    istek_sayaci: 0,
    kisayollar: [],
  };
}

export function uygula(durum: OturumEkrani, girdi: Girdi): OturumEkrani {
  switch (girdi.tip) {
    case 'Kanal':
      return kanalUygula(durum, girdi.mesaj);
    case 'EskiSayfa': {
      const enEski = durum.ogeler[0]?.sira ?? Number.POSITIVE_INFINITY;
      const eskiler = girdi.sayfa.ogeler.filter((o) => o.sira < enEski).map(gecmisOgesi);
      return {
        ...durum,
        ogeler: geriAlinanlariIsle([...eskiler, ...durum.ogeler]),
        daha_eski_var: girdi.sayfa.daha_eski_var,
      };
    }
    case 'MesajGonderildi': {
      const d = kopya(durum);
      d.ogeler.push({
        tip: 'KullaniciMesaji',
        anahtar: `y-${d.yerel_sayac}`,
        sira: sonSira(d),
        tur: null,
        metin: girdi.metin,
      });
      d.yerel_sayac += 1;
      d.anlik = 'Calisiyor';
      return d;
    }
    case 'OnayYanitlandi': {
      const d = kopya(durum);
      let eslesenIstekGuncellendi = false;
      d.ogeler = d.ogeler.map((o) => {
        if (o.tip === 'OnayIstegi' && o.islem === girdi.islem && o.durum === 'Bekliyor') {
          eslesenIstekGuncellendi = true;
          return { ...o, durum: 'Yanitlandi' };
        }
        return o;
      });
      const bekleyenOnayVar = d.ogeler.some(
        (o) => (o.tip === 'OnayIstegi' || o.tip === 'GonderimOnizlemesi') && o.durum === 'Bekliyor',
      );
      if (eslesenIstekGuncellendi && d.anlik === 'OnayBekliyor' && !bekleyenOnayVar) {
        d.anlik = 'Calisiyor';
      }
      return d;
    }
    case 'OnizlemeYanitlandi': {
      const d = kopya(durum);
      let eslesenIstekGuncellendi = false;
      d.ogeler = d.ogeler.map((o) => {
        if (o.tip === 'GonderimOnizlemesi' && o.islem === girdi.islem && o.durum === 'Bekliyor') {
          eslesenIstekGuncellendi = true;
          return { ...o, durum: 'Yanitlandi' };
        }
        return o;
      });
      const bekleyenOnayVar = d.ogeler.some(
        (o) => (o.tip === 'OnayIstegi' || o.tip === 'GonderimOnizlemesi') && o.durum === 'Bekliyor',
      );
      if (eslesenIstekGuncellendi && d.anlik === 'OnayBekliyor' && !bekleyenOnayVar) {
        d.anlik = 'Calisiyor';
      }
      return d;
    }
    case 'OnizlemeGonderilemedi': {
      const d = kopya(durum);
      let geriAlindi = false;
      d.ogeler = d.ogeler.map((o) => {
        if (o.tip === 'GonderimOnizlemesi' && o.islem === girdi.islem && o.durum === 'Yanitlandi') {
          geriAlindi = true;
          return { ...o, durum: 'Bekliyor' };
        }
        return o;
      });
      if (geriAlindi && d.anlik === 'Calisiyor') d.anlik = 'OnayBekliyor';
      return d;
    }
    case 'OnizlemeDegisti':
      return { ...durum, ozet: { ...durum.ozet, onizleme_acik: girdi.acik } };
    case 'ModelDegisti':
      return { ...durum, ozet: { ...durum.ozet, model: girdi.model } };
    case 'KipDegisti':
      return { ...durum, ozet: { ...durum.ozet, kip: girdi.kip } };
    case 'GeriAlmaKapandi':
      return { ...durum, geri_alma: null };
  }
}

function kanalUygula(durum: OturumEkrani, mesaj: KanalMesaji): OturumEkrani {
  if (durum.bosluk || durum.kapandi) return durum;
  if (mesaj.no !== durum.beklenen_no) return { ...durum, bosluk: true };
  const d = kopya(durum);
  d.beklenen_no += 1;
  if (mesaj.tip === 'Kapandi') {
    akisiKapat(d);
    bekleyenleriKapat(d);
    d.kapandi = true;
    d.anlik = 'Bos';
    d.aktif_tur = null;
    return d;
  }
  for (const zarf of mesaj.zarflar) olayUygula(d, zarf);
  return d;
}

/** Tek bir olayı çalışma kopyasına uygular; `d` bu paket için yeni bir nesnedir. */
function olayUygula(d: OturumEkrani, z: Zarf<Olay>): void {
  if (z.sira <= d.gecmis_son_sira) return;
  const yuk = z.yuk;
  if (d.anlik === 'SartBekliyor' && yuk.tip !== 'SartOnayiGerekiyor') sartiCoz(d);
  switch (yuk.tip) {
    case 'TurBasladi':
      d.anlik = 'Calisiyor';
      d.aktif_tur = z.tur;
      // Gönderilen mesaj turun kimliğini henüz bilmiyordu.
      for (let i = d.ogeler.length - 1; i >= 0; i--) {
        const o = d.ogeler[i];
        if (o.tip === 'KullaniciMesaji' && o.tur === null) {
          d.ogeler[i] = { ...o, tur: z.tur };
          break;
        }
      }
      return;
    case 'TokenParcasi':
      akisaEkle(d, z, 'metin', yuk.metin);
      return;
    case 'DusunceParcasi':
      akisaEkle(d, z, 'dusunce', yuk.metin);
      return;
    case 'AracCagrisiBasladi':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'AracCagrisi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        cagri: yuk.cagri,
        arac: yuk.arac,
        aciklama: yuk.aciklama,
        argumanlar: null,
        durum: 'Suruyor',
        sonuc: null,
        terminal: null,
        denetim: null,
      });
      return;
    case 'AracSonucu': {
      // Eşi olmayan sonuç (kurtarmadaki bilinmeyen çağrı) geçmişte zaten görünür.
      const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === yuk.cagri);
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, durum: yuk.durum, sonuc: yuk.icerik };
      return;
    }
    case 'DenetimBasladi':
      denetimiIsle(d, yuk.cagri, { tip: 'Suruyor' });
      return;
    case 'DenetimSonucu':
      denetimiIsle(d, yuk.cagri, yuk.sonuc);
      return;
    case 'DenetimAtlandi':
      denetimiIsle(d, yuk.cagri, { tip: 'Atlandi' });
      return;
    case 'Kisayollar':
      d.kisayollar = yuk.liste;
      return;
    case 'BaglamOzetlendi':
      d.ogeler.push({
        tip: 'BaglamOzetlendi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        adet: yuk.adet,
        yer_tutucu: yuk.yer_tutucu,
        once: yuk.once,
        sonra: yuk.sonra,
      });
      return;
    case 'AracCiktisiParcasi': {
      const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === yuk.cagri);
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') {
        d.ogeler[i] = {
          ...o,
          terminal: terminalEkle(o.terminal ?? BOS_TERMINAL, yuk.akis, yuk.metin, yuk.atlanan_bayt),
        };
      }
      return;
    }
    case 'OnayGerekiyor':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'OnayIstegi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        islem: yuk.islem,
        ogeler: yuk.ogeler,
        durum: 'Bekliyor',
      });
      d.anlik = 'OnayBekliyor';
      return;
    case 'SartOnayiGerekiyor':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'SartIstegi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        surum: yuk.surum,
        durum: 'Bekliyor',
      });
      d.anlik = 'SartBekliyor';
      return;
    case 'KrediGozlemi':
      d.ozet = { ...d.ozet, son_kredi: yuk.gozlem };
      return;
    case 'TokenKullanimi':
      d.ozet = {
        ...d.ozet,
        istem: d.ozet.istem + yuk.istem,
        tamamlama: d.ozet.tamamlama + yuk.tamamlama,
        dusunce: d.ozet.dusunce + yuk.dusunce,
      };
      return;
    case 'IstekBilgisi':
      d.istek_sayaci += 1;
      if (yuk.yonlendirilen_model !== null) {
        d.ozet = { ...d.ozet, yonlendirilen_model: yuk.yonlendirilen_model };
      }
      return;
    case 'GonderimOnizlemesi':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'GonderimOnizlemesi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        islem: yuk.islem,
        ogeler: yuk.ogeler,
        neden: yuk.neden,
        durum: 'Bekliyor',
      });
      d.anlik = 'OnayBekliyor';
      return;
    case 'GuvenlikUyarisi':
      d.ogeler.push({
        tip: 'GuvenlikUyarisi',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        mesaj: yuk.mesaj,
      });
      return;
    case 'Hata':
      akisiKapat(d);
      d.ogeler.push({
        tip: 'Hata',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        mesaj: yuk.mesaj,
      });
      return;
    case 'Tamamlandi':
      akisiKapat(d);
      bekleyenleriKapat(d);
      d.ogeler.push({
        tip: 'TurSonu',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        neden: yuk.neden,
      });
      d.anlik = 'Bos';
      d.aktif_tur = null;
      return;
    case 'TurOzeti':
      d.ogeler.push({
        tip: 'TurOzeti',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        ozet: yuk.ozet,
        geri_alindi: false,
      });
      return;
    case 'GeriAlmaVerisiSilindi':
      d.ogeler = d.ogeler.map((o) =>
        o.tip === 'TurOzeti' && yuk.turlar.includes(o.ozet.tur)
          ? { ...o, ozet: veriSilindi(o.ozet) }
          : o,
      );
      return;
    case 'GeriAlmaPlani':
      d.geri_alma = { tur: yuk.geri_alinan_tur, plan: yuk.plan, muhur: yuk.muhur };
      return;
    case 'TurGeriAlindi':
      d.geri_alma = null;
      d.ogeler = d.ogeler.map((o) =>
        o.tip === 'TurOzeti' && o.ozet.tur === yuk.geri_alinan_tur ? { ...o, geri_alindi: true } : o,
      );
      d.ogeler.push({
        tip: 'GeriAlma',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        geri_alinan_tur: yuk.geri_alinan_tur,
        sonuc: yuk.sonuc,
        yarim: false,
      });
      return;
    case 'AktarimDurdu':
      akisiKapat(d);
      bekleyenleriKapat(d);
      d.ogeler.push({
        tip: 'AktarimDurdu',
        anahtar: `c-${z.sira}`,
        sira: z.sira,
        tur: z.tur,
        neden: yuk.neden,
      });
      d.anlik = 'Bos';
      d.aktif_tur = null;
      return;
  }
}

/**
 * Önceki hâlleri silinen turun değişen dosyaları artık geri alınamaz;
 * oluşturduğu dosyalar silinerek yine geri alınabilir (aşama 3 tasarımı §5.2).
 */
function veriSilindi(ozet: TurOzeti): TurOzeti {
  return {
    ...ozet,
    dosyalar: ozet.dosyalar.map((d) =>
      d.geri_alinabilir.tip === 'Evet' && d.islem === 'Degisti'
        ? { ...d, geri_alinabilir: { tip: 'Hayir', neden: 'VeriSilindi' } }
        : d,
    ),
  };
}

/** Bitişik ve aynı turdaki metin parçaları tek yanıtta birleşir. */
function akisaEkle(d: OturumEkrani, z: Zarf<Olay>, alan: 'metin' | 'dusunce', metin: string) {
  const son = d.ogeler.at(-1);
  if (son?.tip === 'Yanit' && son.akista && son.tur === z.tur) {
    d.ogeler[d.ogeler.length - 1] = { ...son, [alan]: son[alan] + metin };
    return;
  }
  d.ogeler.push({
    tip: 'Yanit',
    anahtar: `c-${z.sira}`,
    sira: z.sira,
    tur: z.tur,
    metin: alan === 'metin' ? metin : '',
    dusunce: alan === 'dusunce' ? metin : '',
    akista: true,
  });
}

/** Akan yanıtı dondurur: tamamlanan mesaj bir kez çizilir (1c tasarımı §9.2). */
function akisiKapat(d: OturumEkrani) {
  const son = d.ogeler.at(-1);
  if (son?.tip === 'Yanit' && son.akista) d.ogeler[d.ogeler.length - 1] = { ...son, akista: false };
}

/** Denetçinin rozeti, çağrının kartına (3c Görev 8). */
function denetimiIsle(d: OturumEkrani, cagri: CagriId, rozet: DenetimRozeti) {
  const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === cagri);
  const o = d.ogeler[i];
  if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, denetim: rozet };
}

/** Tur bittiğinde karar bekleyen kartlar artık karar almaz. */
function bekleyenleriKapat(d: OturumEkrani) {
  d.ogeler = d.ogeler.map((o) => {
    if (o.tip === 'OnayIstegi' && o.durum === 'Bekliyor') return { ...o, durum: 'Gecersiz' };
    if (o.tip === 'GonderimOnizlemesi' && o.durum === 'Bekliyor') return { ...o, durum: 'Gecersiz' };
    if (o.tip === 'SartIstegi' && o.durum === 'Bekliyor') return { ...o, durum: 'Cozuldu' };
    return o;
  });
}

/** Şart kararı verildi: tur olay üretmeye devam ediyor ya da bitiyor. */
function sartiCoz(d: OturumEkrani) {
  d.ogeler = d.ogeler.map((o) =>
    o.tip === 'SartIstegi' && o.durum === 'Bekliyor' ? { ...o, durum: 'Cozuldu' } : o,
  );
  d.anlik = 'Calisiyor';
}

function sonSira(d: OturumEkrani): number {
  return d.ogeler.reduce((en, o) => Math.max(en, o.sira), d.gecmis_son_sira);
}

function kopya(d: OturumEkrani): OturumEkrani {
  return { ...d, ogeler: [...d.ogeler] };
}
