// Kabuk çağrısının canlı çıktısı (aşama 2 tasarımı §10.1). Saf işlev; eski
// durumu değiştirmez. Görünüm sınırlıdır: en çok `AZAMI_SATIR` satır ve
// `AZAMI_KARAKTER` karakter tutar, fazlası baştan atılır ve sayılır; tek satır
// `SATIR_SINIRI` karakterde kesilir. Megabaytlarca metni yeniden yerleştirmek
// yazılım çizimli webview'ı kilitler (Linux'ta ölçüldü); tam çıktı modele
// gider, ekran yalnızca izlemek içindir.
import type { CiktiAkisi } from '../ipc/tipler';

export const AZAMI_SATIR = 5000;
export const AZAMI_KARAKTER = 200_000;
export const SATIR_SINIRI = 4000;

export interface TerminalSatiri {
  /** `Bilgi`: Kalfa'nın eklediği satır ("… gösterilmedi …"). */
  akis: CiktiAkisi | 'Bilgi';
  metin: string;
  /** Satır `SATIR_SINIRI`'nda kesildi; devamı gösterilmiyor. */
  kesik?: boolean;
}

export interface Terminal {
  satirlar: TerminalSatiri[];
  /** Son satır henüz satır sonuyla bitmedi; aynı akışın metni ona eklenir. */
  acik: boolean;
  /** Satırlardaki toplam karakter. */
  karakter: number;
  /** Sınır yüzünden baştan atılan satır sayısı. */
  atilan_satir: number;
  /** Ekran yetişemediği için gösterilmeyen bayt sayısı. */
  gosterilmeyen_bayt: number;
}

export const BOS_TERMINAL: Terminal = {
  satirlar: [],
  acik: false,
  karakter: 0,
  atilan_satir: 0,
  gosterilmeyen_bayt: 0,
};

function satir(akis: TerminalSatiri['akis'], metin: string): TerminalSatiri {
  return metin.length > SATIR_SINIRI
    ? { akis, metin: metin.slice(0, SATIR_SINIRI), kesik: true }
    : { akis, metin };
}

export function terminalEkle(
  t: Terminal,
  akis: CiktiAkisi,
  metin: string,
  atlananBayt: number,
): Terminal {
  const satirlar = [...t.satirlar];
  let karakter = t.karakter;
  let acik = t.acik;
  const ekle = (s: TerminalSatiri) => {
    satirlar.push(s);
    karakter += s.metin.length;
  };
  if (atlananBayt > 0) {
    ekle({ akis: 'Bilgi', metin: `… ${atlananBayt} bayt gösterilmedi …` });
    acik = false;
  }
  const parcalar = metin.split('\n');
  parcalar.forEach((parca, i) => {
    const son = satirlar.at(-1);
    // Satır başı (`\r`) satırın üzerine yazar: ilerleme çubuğunun son hâli kalır.
    const cr = parca.lastIndexOf('\r');
    const yazilan = cr >= 0 ? parca.slice(cr + 1) : parca;
    if (i === 0 && acik && son?.akis === akis) {
      const yeni = son.kesik && cr < 0 ? son : satir(akis, cr >= 0 ? yazilan : son.metin + yazilan);
      karakter += yeni.metin.length - son.metin.length;
      satirlar[satirlar.length - 1] = yeni;
    } else if (i < parcalar.length - 1 || yazilan !== '') {
      ekle(satir(akis, yazilan));
    }
  });
  if (metin !== '') acik = !metin.endsWith('\n');
  let fazla = 0;
  while (
    satirlar.length - fazla > 1 &&
    (satirlar.length - fazla > AZAMI_SATIR || karakter > AZAMI_KARAKTER)
  ) {
    karakter -= satirlar[fazla].metin.length;
    fazla += 1;
  }
  return {
    satirlar: fazla > 0 ? satirlar.slice(fazla) : satirlar,
    acik,
    karakter,
    atilan_satir: t.atilan_satir + fazla,
    gosterilmeyen_bayt: t.gosterilmeyen_bayt + atlananBayt,
  };
}
