// Onay isteğindeki birleşik diff'i (kalfa-tools, `similar`) DiffView'un
// satırlarına çevirir.

export interface DiffSatiri {
  kind: 'add' | 'del' | 'ctx';
  /** Değişiklikten önceki satır numarası. */
  old?: number;
  /** Değişiklikten sonraki satır numarası. */
  neu?: number;
  text: string;
}

export interface DiffParcasi {
  /** "@@ -42,10 +42,18 @@" başlığı. */
  baslik: string;
  satirlar: DiffSatiri[];
}

const BLOK = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/;

export function diffCozumle(metin: string): DiffParcasi[] {
  const parcalar: DiffParcasi[] = [];
  let parca: DiffParcasi | null = null;
  let eski = 0;
  let yeni = 0;
  let kalanEski = 0;
  let kalanYeni = 0;
  const satirlar = metin.split('\n');
  for (let i = 0; i < satirlar.length; i++) {
    const satir = satirlar[i];
    const blok = BLOK.exec(satir);
    if (blok) {
      parca = { baslik: satir, satirlar: [] };
      parcalar.push(parca);
      eski = Number(blok[1]);
      kalanEski = Number(blok[2] ?? 1);
      yeni = Number(blok[3]);
      kalanYeni = Number(blok[4] ?? 1);
      continue;
    }
    // Başlıklar yalnız blok dışında atlanır; aynı önekler içerikte de olabilir.
    if (
      kalanEski === 0 && kalanYeni === 0 &&
      satir.startsWith('--- ') && satirlar[i + 1]?.startsWith('+++ ')
    ) {
      parca = null;
      i++;
      continue;
    }
    if (!parca) continue;
    if (satir.startsWith('+')) {
      parca.satirlar.push({ kind: 'add', neu: yeni++, text: satir.slice(1) });
      kalanYeni = Math.max(0, kalanYeni - 1);
    } else if (satir.startsWith('-')) {
      parca.satirlar.push({ kind: 'del', old: eski++, text: satir.slice(1) });
      kalanEski = Math.max(0, kalanEski - 1);
    } else if (satir.startsWith(' ')) {
      parca.satirlar.push({ kind: 'ctx', old: eski++, neu: yeni++, text: satir.slice(1) });
      kalanEski = Math.max(0, kalanEski - 1);
      kalanYeni = Math.max(0, kalanYeni - 1);
    }
    // "\ No newline at end of file" ve sondaki boş satır gösterilmez.
  }
  return parcalar;
}

/** Eklenen ve silinen satır sayısı: dosya listesindeki "+12 −4" notu. */
export function diffSayilari(parcalar: DiffParcasi[]): { eklenen: number; silinen: number } {
  let eklenen = 0;
  let silinen = 0;
  for (const p of parcalar) {
    for (const s of p.satirlar) {
      if (s.kind === 'add') eklenen++;
      else if (s.kind === 'del') silinen++;
    }
  }
  return { eklenen, silinen };
}
