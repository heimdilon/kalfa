// Uçtan uca test kancası (1c tasarımı §11.4). Yalnızca `npm run build:e2e`
// (`vite build --mode e2e`) derlemesine girer: `import.meta.env.MODE` sürüm
// derlemesinde sabit olduğu için çağıran dal ve bu modül pakete girmez (paket
// denetimi `__kalfaE2e`'yi arar). Modülün üst düzeyinde yan etki yoktur.
//
// - Gecikme: sahte sağlayıcının `[[e2e:<no>:<ms>]]` damgalı parçaları duruma
//   uygulandıktan sonraki ilk çizimde (`requestAnimationFrame` ardından)
//   `Date.now() - ms` kaydedilir. İki saat aynı makinededir.
// - Duraklatma: kanal mesajları duruma uygulanmadan bekletilir, "işledim"
//   gitmez. Sürdürülünce geliş sırasıyla işlenir.
import type { KanalMesaji } from '../ipc/tipler';

const DAMGA = /\[\[e2e:\d+:(\d+(?:\.\d+)?)\]\]/g;

export interface E2eKancasi {
  duraklat(): void;
  surdur(): void;
  /** Ölçülen gecikmeler (ms), ölçüm sırasıyla. */
  gecikmeler(): number[];
  sifirla(): void;
  /** Duraklatıldığı için bekleyen kanal mesajı sayısı. */
  bekleyen(): number;
}

declare global {
  interface Window {
    __kalfaE2e?: E2eKancasi;
  }
}

let durakli = false;
const bekleyenler: Array<() => void> = [];
const olculen: number[] = [];

/** Mesajdaki metin ve düşünce parçalarının damga zamanları. */
export function damgalar(mesaj: KanalMesaji): number[] {
  if (mesaj.tip !== 'Olaylar') return [];
  const zamanlar: number[] = [];
  for (const zarf of mesaj.zarflar) {
    const yuk = zarf.yuk;
    if (yuk.tip !== 'TokenParcasi' && yuk.tip !== 'DusunceParcasi') continue;
    for (const eslesme of yuk.metin.matchAll(DAMGA)) zamanlar.push(Number(eslesme[1]));
  }
  return zamanlar;
}

/** `window.__kalfaE2e`'yi kurar; tekrar çağrılırsa aynı durumu kullanır. */
export function kancayiKur(): E2eKancasi {
  const kanca: E2eKancasi = {
    duraklat: () => {
      durakli = true;
    },
    surdur: () => {
      durakli = false;
      for (const isle of bekleyenler.splice(0)) isle();
    },
    gecikmeler: () => [...olculen],
    sifirla: () => {
      olculen.length = 0;
    },
    bekleyen: () => bekleyenler.length,
  };
  window.__kalfaE2e = kanca;
  return kanca;
}

/** Kanal dinleyicisini ölçen ve duraklatılabilen bir dinleyiciyle sarar. */
export function sar(isleyici: (mesaj: KanalMesaji) => void): (mesaj: KanalMesaji) => void {
  kancayiKur();
  const isle = (mesaj: KanalMesaji) => {
    isleyici(mesaj);
    const zamanlar = damgalar(mesaj);
    if (zamanlar.length === 0) return;
    requestAnimationFrame(() =>
      setTimeout(() => {
        const simdi = Date.now();
        for (const t of zamanlar) olculen.push(simdi - t);
      }, 0),
    );
  };
  return (mesaj) => {
    if (durakli) bekleyenler.push(() => isle(mesaj));
    else isle(mesaj);
  };
}
