// Tur özeti ve geri alma sayfasının metinleri (aşama 3 tasarımı §5.5–§5.11).
// Geri alınabilirliği çekirdek söyler; burada yalnızca gösterilir.
import { m } from '../dil/tr';
import type {
  CakismaNedeni,
  GeriAlinabilirlik,
  GeriAlinamazNedeni,
  GeriAlmaEylemi,
  GeriAlmaSonucu,
  TurOzeti,
} from '../ipc/tipler';
import { sayi } from './bicim';

export interface TurOzetiGorunumu {
  satirlar: string[];
  notlar: string[];
  /** En az bir dosya geri alınabiliyor: "Turu geri al…" gösterilir. */
  geriAlinabilir: boolean;
  /** Araç çağrısı olmayan tur; arayüz çizmez. */
  bos: boolean;
}

export function turOzetiGorunumu(ozet: TurOzeti, geriAlindi: boolean): TurOzetiGorunumu {
  const satirlar: string[] = [];
  for (const d of ozet.dosyalar) {
    const ekler: string[] = [];
    if (d.islem === 'Olusturuldu') ekler.push(m('tur_ozeti.yeni'));
    if (d.sonucu_bilinmiyor) ekler.push(m('tur_ozeti.bilinmiyor'));
    if (d.geri_alinabilir.tip === 'Hayir') ekler.push(m('tur_ozeti.geri_alinamaz'));
    const satir = m('tur_ozeti.dosya', { yol: d.yol, eklenen: d.eklenen, silinen: d.silinen });
    satirlar.push([satir, ...ekler].join(' · '));
  }
  let kabukDegistirdi = false;
  let eksik = false;
  for (const k of ozet.komutlar) {
    const g = k.degisiklikler;
    const komut = k.komut.split('\n')[0] ?? '';
    if (g && (g.toplam > 0 || g.eksik)) {
      kabukDegistirdi = true;
      eksik ||= g.eksik;
      satirlar.push(m('tur_ozeti.komut', { komut, sayi: g.toplam }));
    } else {
      satirlar.push(m('tur_ozeti.komut_degismedi', { komut }));
    }
    if (k.ag) {
      for (const h of k.ag.hostlar) {
        const bayt = boyutMetni(h.giden_bayt + h.gelen_bayt);
        if (h.reddedilenler === h.baglanti_sayisi) {
          satirlar.push(
            m('tur_ozeti.ag_host_ret', {
              host: h.host,
              port: h.port,
              sayi: h.baglanti_sayisi,
            }),
          );
        } else if (h.reddedilenler > 0) {
          satirlar.push(
            m('tur_ozeti.ag_host_kismi_ret', {
              host: h.host,
              port: h.port,
              sayi: h.baglanti_sayisi,
              ret: h.reddedilenler,
              bayt,
            }),
          );
        } else {
          satirlar.push(
            m('tur_ozeti.ag_host', {
              host: h.host,
              port: h.port,
              sayi: h.baglanti_sayisi,
              bayt,
            }),
          );
        }
      }
    }
  }
  const token = ozet.istekler.istem + ozet.istekler.tamamlama;
  if (ozet.istekler.sayi > 0) {
    satirlar.push(m('tur_ozeti.istekler', { sayi: ozet.istekler.sayi, token: sayi(token) }));
  }
  if (ozet.guvenlik_uyarisi > 0) {
    satirlar.push(m('tur_ozeti.guvenlik', { sayi: ozet.guvenlik_uyarisi }));
  }
  const notlar: string[] = [];
  if (kabukDegistirdi) notlar.push(m('tur_ozeti.not_kabuk'));
  if (eksik) notlar.push(m('tur_ozeti.not_eksik'));
  if (geriAlindi) notlar.push(m('tur_ozeti.not_geri_alindi'));
  return {
    satirlar,
    notlar,
    geriAlinabilir: ozet.dosyalar.some((d) => d.geri_alinabilir.tip === 'Evet'),
    bos: ozet.dosyalar.length === 0 && ozet.komutlar.length === 0,
  };
}

export function geriAlinamazMetni(neden: GeriAlinamazNedeni): string {
  return m(`geri_alma.neden.${neden}`);
}

export function cakismaMetni(neden: CakismaNedeni): string {
  if (neden.tip === 'Okunamadi') return m('geri_alma.cakisma.Okunamadi', { hata: neden.hata });
  return m(`geri_alma.cakisma.${neden.tip}`);
}

/** Eylemin kısa adı ve (çakışma ya da geri alınamazlıkta) nedeni. */
export function eylemMetni(eylem: GeriAlmaEylemi): { ad: string; neden: string | null } {
  const ad = m(`geri_alma.eylem.${eylem.tip}`);
  if (eylem.tip === 'Cakisma') return { ad, neden: cakismaMetni(eylem.neden) };
  if (eylem.tip === 'GeriAlinamaz') return { ad, neden: geriAlinamazMetni(eylem.neden) };
  return { ad, neden: null };
}

export function yazilacakMi(eylem: GeriAlmaEylemi): boolean {
  return eylem.tip === 'GeriYukle' || eylem.tip === 'Sil';
}

/** Onay kartının "Geri alınabilir mi" satırı. */
export function geriAlinabilirlikMetni(g: GeriAlinabilirlik): string {
  return g.tip === 'Evet' ? m('onay.geri.Evet') : m(`onay.geri.${g.neden}`);
}

const BOYUT = new Intl.NumberFormat('tr-TR', { maximumFractionDigits: 1 });

/** Bayt sayısı: 0 bayt, 512 bayt, 1,5 KiB, 12,3 MiB, 1,1 GiB. */
export function boyutMetni(bayt: number): string {
  const birimler = ['KiB', 'MiB', 'GiB'];
  if (bayt < 1024) return `${bayt} bayt`;
  let deger = bayt / 1024;
  let i = 0;
  while (deger >= 1024 && i < birimler.length - 1) {
    deger /= 1024;
    i += 1;
  }
  return `${BOYUT.format(deger)} ${birimler[i]}`;
}

/** Konuşmadaki geri alma satırı. */
export function geriAlmaSonucMetni(sonuc: GeriAlmaSonucu[]): string {
  const yapilan = sonuc.filter((s) => !s.hata && yazilacakMi(s.eylem)).length;
  const kalan = sonuc.filter(
    (s) => s.eylem.tip !== 'ZatenOnceki' && (s.hata || !yazilacakMi(s.eylem)),
  ).length;
  const metin = m('geri_alma.yapildi', { sayi: yapilan });
  return kalan > 0 ? `${metin} ${m('geri_alma.dokunulmadi', { sayi: kalan })}` : metin;
}
