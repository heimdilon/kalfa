// Terminal satırındaki ANSI kaçışları (aşama 2 tasarımı §10.1). Yalnızca SGR
// (renk ve kalınlık) çizilir; imleç hareketi, başlık ve diğer kaçışlar atılır.
// Sonuç metin parçalarıdır; görünüm onları metin düğümü olarak ekler, HTML
// olarak yorumlanmaz.

export type Renk = 'kirmizi' | 'yesil' | 'sari' | 'mavi' | 'soluk';

export interface AnsiParcasi {
  metin: string;
  renk: Renk | null;
  kalin: boolean;
}

// CSI (ESC [ ... son harf), OSC (ESC ] ... BEL ya da ESC \) ve tek karakterli
// kaçışlar (ESC 7, ESC M).
const KACIS = /\x1b\[([0-?]*)[ -/]*([@-~])|\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)|\x1b[0-~]/g;

const RENK: Record<number, Renk> = {
  31: 'kirmizi',
  91: 'kirmizi',
  32: 'yesil',
  92: 'yesil',
  33: 'sari',
  93: 'sari',
  34: 'mavi',
  94: 'mavi',
  35: 'mavi',
  95: 'mavi',
  36: 'mavi',
  96: 'mavi',
  90: 'soluk',
};

export function ansiParcala(satir: string): AnsiParcasi[] {
  const parcalar: AnsiParcasi[] = [];
  let renk: Renk | null = null;
  let kalin = false;
  let son = 0;
  const ekle = (metin: string) => {
    if (!metin) return;
    const onceki = parcalar.at(-1);
    if (onceki && onceki.renk === renk && onceki.kalin === kalin) onceki.metin += metin;
    else parcalar.push({ metin, renk, kalin });
  };
  for (const e of satir.matchAll(KACIS)) {
    ekle(satir.slice(son, e.index));
    son = e.index + e[0].length;
    if (e[2] !== 'm') continue;
    const kodlar = (e[1] ?? '').split(';').map((k) => (k === '' ? 0 : Number(k)));
    for (const kod of kodlar) {
      if (kod === 0) {
        renk = null;
        kalin = false;
      } else if (kod === 1) kalin = true;
      else if (kod === 22) kalin = false;
      else if (kod === 39) renk = null;
      else if (kod in RENK) renk = RENK[kod];
      else if ((kod >= 30 && kod <= 37) || (kod >= 90 && kod <= 97)) renk = null;
    }
  }
  ekle(satir.slice(son));
  return parcalar;
}
