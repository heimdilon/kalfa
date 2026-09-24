// Ana pencerede gösterilen değerlerin biçimi: zaman, gün grubu, oturum durumu,
// araç kartı başlığı, sayılar. Saf işlevler; DOM'a dokunmaz.
import { ARAC_DURUMU_ADI, m, type MetinAnahtari } from '../dil/tr';
import type { AracDurumu, OturumDurumu, OturumSatiri } from '../ipc/tipler';
import type { Anlik, EkranOgesi } from '../durum/oturum';

const GUN_MS = 24 * 60 * 60 * 1000;

function gunBasi(ms: number): number {
  const t = new Date(ms);
  return new Date(t.getFullYear(), t.getMonth(), t.getDate()).getTime();
}

/** Kaç takvim günü önce: 0 bugün, 1 dün. */
function gunFarki(ms: number, simdi: number): number {
  return Math.round((gunBasi(simdi) - gunBasi(ms)) / GUN_MS);
}

const SAAT = new Intl.DateTimeFormat('tr-TR', { hour: '2-digit', minute: '2-digit' });
const HAFTA_GUNU = new Intl.DateTimeFormat('tr-TR', { weekday: 'short' });
const GUN_AY = new Intl.DateTimeFormat('tr-TR', { day: 'numeric', month: 'short' });

export function saat(ms: number): string {
  return SAAT.format(ms);
}

/**
 * Oturum satırının sağındaki zaman: bugün saat, dün "Dün", bu hafta gün adı,
 * daha eskisi gün ve ay.
 */
export function zamanEtiketi(ms: number, simdi: number): string {
  const fark = gunFarki(ms, simdi);
  if (fark <= 0) return saat(ms);
  if (fark === 1) return m('zaman.dun');
  if (fark < 7) return HAFTA_GUNU.format(ms);
  return GUN_AY.format(ms);
}

export type GunGrubu = 'bugun' | 'dun' | 'hafta' | 'eski';

export const GUN_GRUBU_ADI: Record<GunGrubu, MetinAnahtari> = {
  bugun: 'liste.grup.bugun',
  dun: 'liste.grup.dun',
  hafta: 'liste.grup.hafta',
  eski: 'liste.grup.eski',
};

export function gunGrubu(ms: number, simdi: number): GunGrubu {
  const fark = gunFarki(ms, simdi);
  if (fark <= 0) return 'bugun';
  if (fark === 1) return 'dun';
  if (fark < 7) return 'hafta';
  return 'eski';
}

/** Satırları sırasını bozmadan gün gruplarına ayırır; boş grup dönmez. */
export function gruplara<T>(
  ogeler: T[],
  zaman: (oge: T) => number,
  simdi: number,
): { grup: GunGrubu; ogeler: T[] }[] {
  const gruplar: { grup: GunGrubu; ogeler: T[] }[] = [];
  for (const oge of ogeler) {
    const grup = gunGrubu(zaman(oge), simdi);
    const son = gruplar.at(-1);
    if (son?.grup === grup) son.ogeler.push(oge);
    else gruplar.push({ grup, ogeler: [oge] });
  }
  return gruplar;
}

export type SatirDurumu = 'running' | 'waiting' | 'done' | 'failed' | 'partial' | 'idle';

/**
 * Oturum satırının durumu (1c tasarımı §8.1). Açık oturumun anlık hâli kayıttaki
 * son durumdan önce gelir: çalışıyor, onay bekliyor, şart bekliyor.
 */
export function satirDurumu(
  durum: OturumDurumu,
  anlik: Anlik | null,
): { status: SatirDurumu; etiket: string } {
  switch (anlik) {
    case 'Calisiyor':
      return { status: 'running', etiket: m('oturum.durum.calisiyor') };
    case 'OnayBekliyor':
      return { status: 'waiting', etiket: m('oturum.durum.onay') };
    case 'SartBekliyor':
      return { status: 'waiting', etiket: m('oturum.durum.sart') };
  }
  switch (durum.tip) {
    case 'Tamamlandi':
      return { status: 'done', etiket: m('oturum.durum.tamamlandi') };
    case 'IptalEdildi':
      return { status: 'idle', etiket: m('oturum.durum.iptal') };
    case 'Hata':
      return { status: 'failed', etiket: m('oturum.durum.hata') };
    case 'YarimKaldi':
      return { status: 'partial', etiket: m('oturum.durum.yarim') };
    case 'Okunamadi':
      return { status: 'failed', etiket: m('oturum.durum.okunamadi') };
    case 'Bos':
      return { status: 'idle', etiket: m('oturum.durum.bos') };
  }
}

/** Listede gösterilen başlık: ilk kullanıcı mesajı ya da dosya adı. */
export function satirBasligi(satir: OturumSatiri): string {
  if (satir.baslik) return satir.baslik;
  if (satir.durum.tip === 'Okunamadi') return satir.dosya.split(/[\\/]/).at(-1) ?? satir.dosya;
  return m('oturum.yeni');
}

type AracOgesi = Extract<EkranOgesi, { tip: 'AracCagrisi' }>;
type KartDurumu = 'running' | 'done' | 'failed' | 'denied' | 'unknown';

/** Her aracın başlığı: sürerken, başarıyla bitince, diğer sonuçlarda. */
const ARAC_BASLIGI: Record<string, [MetinAnahtari, MetinAnahtari, MetinAnahtari]> = {
  dosya_oku: ['arac.dosya_oku.suruyor', 'arac.dosya_oku.bitti', 'arac.dosya_oku.ad'],
  dosya_listele: [
    'arac.dosya_listele.suruyor',
    'arac.dosya_listele.bitti',
    'arac.dosya_listele.ad',
  ],
  icerik_ara: ['arac.icerik_ara.suruyor', 'arac.icerik_ara.bitti', 'arac.icerik_ara.ad'],
  dosya_yaz: ['arac.dosya_yaz.suruyor', 'arac.dosya_yaz.bitti', 'arac.dosya_yaz.ad'],
  dosya_duzenle: [
    'arac.dosya_duzenle.suruyor',
    'arac.dosya_duzenle.bitti',
    'arac.dosya_duzenle.ad',
  ],
  kabuk: ['arac.kabuk.suruyor', 'arac.kabuk.bitti', 'arac.kabuk.ad'],
};

/**
 * Araç kartının başlığı: sürerken şimdiki zamanla, başarıyla bitince geçmiş
 * zamanla ("Dosya okundu"); reddedilen ya da başarısız çağrıda yalın adla.
 */
export function aracBasligi(oge: AracOgesi): string {
  const basliklar = ARAC_BASLIGI[oge.arac];
  if (!basliklar) return oge.arac;
  if (oge.durum === 'Suruyor') return m(basliklar[0]);
  if (oge.durum === 'Basarili') return m(basliklar[1]);
  return m(basliklar[2]);
}

/** Kartın hedefi: canlı çağrıda açıklama, kayıttan gelende argümandaki yol ya da desen. */
export function aracHedefi(oge: AracOgesi): string | undefined {
  if (oge.aciklama) return oge.aciklama;
  if (!oge.argumanlar) return undefined;
  try {
    const a = JSON.parse(oge.argumanlar) as Record<string, unknown>;
    if (typeof a.betik === 'string') return a.betik.split('\n')[0];
    if (typeof a.program === 'string') {
      const argumanlar = Array.isArray(a.argumanlar) ? a.argumanlar : [];
      return [a.program, ...argumanlar].join(' ');
    }
    for (const alan of ['yol', 'desen', 'dizin']) {
      if (typeof a[alan] === 'string') return a[alan];
    }
  } catch {
    // Çözülemeyen argüman gösterilmez.
  }
  return undefined;
}

export function aracKartDurumu(durum: AracDurumu | 'Suruyor' | null): {
  status: KartDurumu;
  etiket: string;
} {
  switch (durum) {
    case 'Suruyor':
      return { status: 'running', etiket: m(ARAC_DURUMU_ADI.Suruyor) };
    case 'Basarili':
      return { status: 'done', etiket: m(ARAC_DURUMU_ADI.Basarili) };
    case 'Basarisiz':
      return { status: 'failed', etiket: m(ARAC_DURUMU_ADI.Basarisiz) };
    case 'Reddedildi':
      return { status: 'denied', etiket: m(ARAC_DURUMU_ADI.Reddedildi) };
    case 'IptalEdildi':
      return { status: 'denied', etiket: m(ARAC_DURUMU_ADI.IptalEdildi) };
    case 'SonucuBilinmiyor':
      return {
        status: 'unknown',
        etiket: m(ARAC_DURUMU_ADI.SonucuBilinmiyor),
      };
    case null:
      return { status: 'unknown', etiket: m(ARAC_DURUMU_ADI.Baslamadi) };
  }
}

/** Kabuk kartının özeti: sonucun ilk satırı ("Çıkış kodu: 0"). */
export function kabukOzeti(oge: AracOgesi): string | undefined {
  if (oge.arac !== 'kabuk' || !oge.sonuc) return undefined;
  // Sonuç `[kaynak: …]` etiketiyle başlar.
  return oge.sonuc.split('\n').find((s) => s && !s.startsWith('[kaynak:'));
}

/** Araç çıktısının kartta gösterilen kısmı. */
export function ciktiOnizlemesi(
  metin: string,
  azamiSatir = 40,
): { kod: string; kirpildi: boolean } {
  const satirlar = metin.replace(/\n$/, '').split('\n');
  if (satirlar.length <= azamiSatir) return { kod: satirlar.join('\n'), kirpildi: false };
  return { kod: satirlar.slice(0, azamiSatir).join('\n'), kirpildi: true };
}

const TAM_SAYI = new Intl.NumberFormat('tr-TR');
const KREDI = new Intl.NumberFormat('tr-TR', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export function sayi(n: number): string {
  return TAM_SAYI.format(n);
}

/** EVREN'in ondalık metni; çözülemezse olduğu gibi. */
export function kredi(ondalik: string): string {
  const n = Number(ondalik);
  return Number.isFinite(n) ? KREDI.format(n) : ondalik;
}
