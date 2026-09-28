// Onay yüzeylerinde görünmez ve yön değiştiren karakterler (güvenlik analizi
// Bulgu 7). Rust'taki `kalfa_core::gorunum` ile aynı kural: `\n` ve `\t`
// dışındaki denetim karakterleri, biçim karakterleri, satır ve paragraf
// ayırıcıları ve Hangul dolgu karakterleri `⟨U+XXXX⟩` olarak gösterilir.

const GORUNMEZ = /[\p{Cc}\p{Cf}\p{Zl}\p{Zp}ᅟᅠㅤﾠ]/u;

export interface GorunurParca {
  metin: string;
  /** Görünmez bir karakterin kaçışı. */
  kacis: boolean;
}

export function gorunmezMi(k: string): boolean {
  return k !== '\n' && k !== '\t' && GORUNMEZ.test(k);
}

function kacis(k: string): string {
  const kod = k.codePointAt(0) ?? 0;
  return `⟨U+${kod.toString(16).toUpperCase().padStart(4, '0')}⟩`;
}

/** Metni düz ve kaçışlanmış parçalara böler. `\r\n` yalnızca `\n` olur. */
export function gorunurYap(metin: string): GorunurParca[] {
  const parcalar: GorunurParca[] = [];
  let duz = '';
  const karakterler = [...metin];
  for (let i = 0; i < karakterler.length; i++) {
    const k = karakterler[i] as string;
    if (k === '\r' && karakterler[i + 1] === '\n') continue;
    if (gorunmezMi(k)) {
      if (duz) parcalar.push({ metin: duz, kacis: false });
      duz = '';
      parcalar.push({ metin: kacis(k), kacis: true });
    } else {
      duz += k;
    }
  }
  if (duz) parcalar.push({ metin: duz, kacis: false });
  return parcalar;
}

/** Düz metin olarak kaçışlanmış hâli (etiketler için). */
export function gorunurMetin(metin: string): string {
  return gorunurYap(metin)
    .map((p) => p.metin)
    .join('');
}
