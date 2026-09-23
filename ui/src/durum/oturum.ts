// Bir oturumun ekran durumu ve onu güncelleyen saf işlev (1c tasarımı §8.4).
// `uygula` DOM'a dokunmaz; eski durumu değiştirmez, yenisini döner. Svelte
// yalnızca bu durumu çizer.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  BitisNedeni,
  CagriId,
  DurumOzeti,
  GecmisSayfasi,
  GorunumOgesi,
  IslemId,
  KanalMesaji,
  Kip,
  Olay,
  OnayOgesi,
  OturumAcilisi,
  OturumId,
  SaltOkunurNedeni,
  TurId,
  Zarf,
} from '../ipc/tipler';

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
    })
  | (Ortak & {
      tip: 'OnayIstegi';
      islem: IslemId;
      ogeler: OnayOgesi[];
      durum: 'Bekliyor' | 'Yanitlandi' | 'Gecersiz';
    })
  | (Ortak & { tip: 'SartIstegi'; surum: number; durum: 'Bekliyor' | 'Cozuldu' })
  | (Ortak & { tip: 'Hata'; mesaj: string })
  | (Ortak & { tip: 'TurSonu'; neden: BitisNedeni })
  | (Ortak & { tip: 'AktarimDurdu'; neden: AktarimDurmaNedeni });

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
}

export type Girdi =
  | { tip: 'Kanal'; mesaj: KanalMesaji }
  | { tip: 'EskiSayfa'; sayfa: GecmisSayfasi }
  | { tip: 'MesajGonderildi'; metin: string }
  | { tip: 'OnayYanitlandi'; islem: IslemId }
  | { tip: 'ModelDegisti'; model: string }
  | { tip: 'KipDegisti'; kip: Kip };

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
      };
    case 'TurSonu':
      return { ...o, anahtar: `g-${o.sira}-s` };
  }
}

/** `oturum_baslat` ya da `oturum_ac`'ın sonucundan ilk ekran durumu. */
export function acilistanEkran(acilis: OturumAcilisi): OturumEkrani {
  const ogeler = acilis.gecmis.ogeler.map(gecmisOgesi);
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
        ogeler: [...eskiler, ...durum.ogeler],
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
      const bekleyenOnayVar = d.ogeler.some((o) => o.tip === 'OnayIstegi' && o.durum === 'Bekliyor');
      if (eslesenIstekGuncellendi && d.anlik === 'OnayBekliyor' && !bekleyenOnayVar) {
        d.anlik = 'Calisiyor';
      }
      return d;
    }
    case 'ModelDegisti':
      return { ...durum, ozet: { ...durum.ozet, model: girdi.model } };
    case 'KipDegisti':
      return { ...durum, ozet: { ...durum.ozet, kip: girdi.kip } };
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
      });
      return;
    case 'AracSonucu': {
      // Eşi olmayan sonuç (kurtarmadaki bilinmeyen çağrı) geçmişte zaten görünür.
      const i = d.ogeler.findLastIndex((o) => o.tip === 'AracCagrisi' && o.cagri === yuk.cagri);
      const o = d.ogeler[i];
      if (o?.tip === 'AracCagrisi') d.ogeler[i] = { ...o, durum: yuk.durum, sonuc: yuk.icerik };
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
      if (yuk.yonlendirilen_model !== null) {
        d.ozet = { ...d.ozet, yonlendirilen_model: yuk.yonlendirilen_model };
      }
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

/** Tur bittiğinde karar bekleyen kartlar artık karar almaz. */
function bekleyenleriKapat(d: OturumEkrani) {
  d.ogeler = d.ogeler.map((o) => {
    if (o.tip === 'OnayIstegi' && o.durum === 'Bekliyor') return { ...o, durum: 'Gecersiz' };
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
