// Kaydırılan bir kutunun en alta yapışması: kullanıcı en alttayken içerik
// büyüdükçe kutu da aşağı kayar; kullanıcı yukarı kaydırınca bırakılır.
//
// Kutunun konumunu kullanıcıdan başkası da değiştirir: kendi kaydırmamızın
// geç gelen olayı (o arada içerik yeniden büyümüş olabilir), onay sayfası
// kapanınca odağın geri döndüğü düğme, tarayıcının kaydırma çapası. Bunlara
// bakıp yapışmayı bırakmak yeni içeriği görünmez kılardı. Bu yüzden yapışma
// yalnızca kullanıcının yakın zamandaki kaydırma niyetinden
// (`kaydirmaNiyetiMi`) sonra bırakılır; en alta hangi yoldan inilirse inilsin
// yeniden yapışılır.

export interface Kutu {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
}

/** Kullanıcının niyetinden sonra kaydırma olaylarının ona sayıldığı süre. */
const NIYET_SURESI_MS = 1000;

const KAYDIRMA_TUSLARI = new Set(['PageUp', 'PageDown', 'ArrowUp', 'ArrowDown', 'Home', 'End', ' ']);

/**
 * Olay kullanıcının kutuyu kaydırma niyeti mi? Tekerlek ve dokunarak
 * sürükleme evet; işaretçi yalnızca kutunun kendisine (kaydırma çubuğuna)
 * basınca; tuşlardan yalnızca kaydırma tuşları. Kutunun içindeki bir düğmeye
 * tıklamak niyet değildir: tıklamanın ardından içerik değişip kutu kayabilir.
 */
export function kaydirmaNiyetiMi(
  olay: { type: string; target: unknown; key?: string },
  kutu: unknown,
): boolean {
  switch (olay.type) {
    case 'wheel':
    case 'touchmove':
      return true;
    case 'pointerdown':
      return olay.target === kutu;
    case 'keydown':
      return KAYDIRMA_TUSLARI.has(olay.key ?? '');
    default:
      return false;
  }
}

export class AltaYapisma {
  altta = true;
  #niyet = -Infinity;

  /** `esik`: en alta bu kadar pikselden yakınsa en altta sayılır. */
  constructor(
    private readonly esik: number,
    private readonly saat: () => number = () => performance.now(),
  ) {}

  /** Kullanıcı kaydırmaya niyetlendi (`kaydirmaNiyetiMi`). */
  niyet(): void {
    this.#niyet = this.saat();
  }

  /** İçerik büyüdü: en alttaysa kutuyu en alta kaydırır. */
  kaydir(kutu: Kutu): void {
    if (this.altta) kutu.scrollTop = kutu.scrollHeight;
  }

  /** `scroll` olayı. */
  olay(kutu: Kutu): void {
    if (kutu.scrollHeight - kutu.scrollTop - kutu.clientHeight < this.esik) {
      this.altta = true;
    } else if (this.saat() - this.#niyet < NIYET_SURESI_MS) {
      this.altta = false;
    }
  }

  /** Oturum değişince: en alttan başlanır. */
  sifirla(): void {
    this.altta = true;
    this.#niyet = -Infinity;
  }
}
