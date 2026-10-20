// Ana pencerede gösterilen değerlerin biçimi: zaman, gün grubu, oturum durumu,
// araç kartı başlığı, sayılar. Saf işlevler; DOM'a dokunmaz.
import { ARAC_DURUMU_ADI, m, type MetinAnahtari } from '../dil/tr';
import type { AracDurumu, GunlukSinirDurumu, OturumDurumu, OturumSatiri } from '../ipc/tipler';
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
    case 'SoruBekliyor':
      return { status: 'waiting', etiket: m('oturum.durum.soru') };
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

/** Akan düşüncede gösterilen en çok karakter. */
export const CANLI_DUSUNCE = 2_000;

/**
 * Akan düşüncenin son kısmı. Düşünce milyonlarca karaktere çıkabilir; akarken
 * her karede tamamını çözmek arayüzü geri bıraktırır. Kesilmişse başına "…"
 * konur; UTF-16 vekil çifti bölünmez.
 */
export function dusunceKuyrugu(metin: string): string {
  if (metin.length <= CANLI_DUSUNCE) return metin;
  let bas = metin.length - CANLI_DUSUNCE;
  const kod = metin.charCodeAt(bas);
  if (kod >= 0xdc00 && kod <= 0xdfff) bas += 1;
  return `…${metin.slice(bas)}`;
}

/** Çalışma alanının kısa adı: yolun son klasörü; yol yoksa "kalfa". */
export function projeAdi(yol: string | null): string {
  if (!yol) return 'kalfa';
  const parcalar = yol.replace(/\\/g, '/').split('/').filter(Boolean);
  return parcalar.at(-1) ?? yol;
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
  plan_guncelle: [
    'arac.plan_guncelle.suruyor',
    'arac.plan_guncelle.bitti',
    'arac.plan_guncelle.ad',
  ],
  kullaniciya_sor: [
    'arac.kullaniciya_sor.suruyor',
    'arac.kullaniciya_sor.bitti',
    'arac.kullaniciya_sor.ad',
  ],
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
    for (const alan of ['yol', 'desen', 'dizin', 'baslik']) {
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

/**
 * Modele giden sonuçtan etiketi ayıklar: `<arac_sonucu … sinir="…">` sarmalı
 * (güvenlik analizi Bulgu 5) ya da eski kayıtlardaki `[kaynak: …]` satırı.
 * Sarmalın dışındaki `[güvenlik]` satırları kalır.
 */
export function sonucGovdesi(sonuc: string): string {
  const satirlar = sonuc.split('\n');
  const ilk = satirlar[0] ?? '';
  if (ilk.startsWith('<arac_sonucu ')) {
    const sinir = /sinir="([0-9a-f]+)">$/.exec(ilk)?.[1];
    const kapanis = satirlar.lastIndexOf(`</arac_sonucu sinir="${sinir}">`);
    if (sinir && kapanis > 0) {
      return [...satirlar.slice(1, kapanis), ...satirlar.slice(kapanis + 1)].join('\n');
    }
  }
  if (ilk.startsWith('[kaynak:')) return satirlar.slice(1).join('\n');
  return sonuc;
}

/** Kabuk kartının özeti: sonucun ilk satırı ("Çıkış kodu: 0"). */
export function kabukOzeti(oge: AracOgesi): string | undefined {
  if (oge.arac !== 'kabuk' || !oge.sonuc) return undefined;
  return sonucGovdesi(oge.sonuc)
    .split('\n')
    .find((s) => s);
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

/** Bağlam özetinin notu (5a tasarımı §12). */
export function baglamNotu(o: {
  adet: number;
  yer_tutucu: number;
  once: number;
  sonra: number;
}): string {
  const metin = m('baglam.ozetlendi', {
    toplam: o.adet + o.yer_tutucu,
    once: sayi(o.once),
    sonra: sayi(o.sonra),
  });
  return o.yer_tutucu > 0 ? `${metin} ${m('baglam.yer_tutucu', { adet: o.yer_tutucu })}` : metin;
}

/** EVREN'in ondalık metni; çözülemezse olduğu gibi. */
export function kredi(ondalik: string): string {
  const n = Number(ondalik);
  return Number.isFinite(n) ? KREDI.format(n) : ondalik;
}

const GUN_AY_UZUN = new Intl.DateTimeFormat('tr-TR', { day: 'numeric', month: 'long' });
const TARIH = new Intl.DateTimeFormat('tr-TR', {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
});

/** "20 Ekim": gönderim izninin verildiği gün. */
export function gunAy(ms: number): string {
  return GUN_AY_UZUN.format(ms);
}

/** "20 Ekim 2026": eklentinin kurulum tarihi. */
export function tarih(ms: number): string {
  return TARIH.format(ms);
}

// Rust'ın `{:.1}` biçimi tam yarıda çifte yuvarlar; kart ikisini yan yana gösterir.
const MB = new Intl.NumberFormat('tr-TR', {
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
  roundingMode: 'halfEven',
});

/** Ondalık MB, Rust'taki ilk gönderim metniyle aynı biçimde: `6,0 MB`. */
export function mbMetni(bayt: number): string {
  return `${MB.format(bayt / 1_000_000)} MB`;
}

/**
 * Metni ters tırnaklı kod parçalarına böler (`a \`b\` c` → düz, kod, düz).
 * Yalnızca ters tırnak çözülür; başka işaret ya da HTML yorumlanmaz. Kapanmayan
 * ters tırnak olduğu gibi kalır.
 */
export function kodParcalari(metin: string): { metin: string; kod: boolean }[] {
  const parcalar = metin.split('`');
  if (parcalar.length % 2 === 0) {
    const son = parcalar.pop() ?? '';
    parcalar[parcalar.length - 1] += `\`${son}`;
  }
  return parcalar.map((p, i) => ({ metin: p, kod: i % 2 === 1 })).filter((p) => p.metin !== '');
}

const ZAMAN_DAMGASI = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/;

/**
 * Durum çubuğundaki günlük sınır metni (aşama 4c): sıfırlanma anı yerel
 * saatle; zaman damgası geçersizse yalnızca "Günlük sınır doldu".
 */
export function gunlukSinirMetni(sinir: GunlukSinirDurumu): string {
  const ms =
    sinir.sifirlanma && ZAMAN_DAMGASI.test(sinir.sifirlanma)
      ? Date.parse(sinir.sifirlanma)
      : Number.NaN;
  return Number.isNaN(ms)
    ? m('durum.gunluk_sinir')
    : m('durum.gunluk_sinir_sifirlanma', { saat: saat(ms) });
}
