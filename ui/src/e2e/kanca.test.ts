import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { KanalMesaji, Olay } from '../ipc/tipler';
import { damgalar, sar } from './kanca';

let sira = 0;
function paket(no: number, ...yukler: Olay[]): KanalMesaji {
  return {
    tip: 'Olaylar',
    no,
    zarflar: yukler.map((yuk) => ({ sema: 1, oturum: 'o-1', tur: 't-1', sira: ++sira, yuk })),
  };
}

const metin = (m: string): Olay => ({ tip: 'TokenParcasi', metin: m });
const dusunce = (m: string): Olay => ({ tip: 'DusunceParcasi', metin: m });

describe('damgalar', () => {
  it('metin ve düşünce parçalarındaki bütün damgaları okur', () => {
    const m = paket(
      1,
      metin('[[e2e:1:1000.5]][[e2e:2:1010]]'),
      dusunce('önce [[e2e:3:1020.25]] sonra'),
      { tip: 'Hata', mesaj: '[[e2e:4:1030]]' },
    );
    expect(damgalar(m)).toEqual([1000.5, 1010, 1020.25]);
  });

  it('damgasız ve kapanış mesajında boş döner', () => {
    expect(damgalar(paket(1, metin('merhaba')))).toEqual([]);
    expect(damgalar({ tip: 'Kapandi', no: 2 })).toEqual([]);
  });
});

describe('sar', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'Date'] });
    vi.setSystemTime(10_000);
    // jsdom'un çizim döngüsü sahte saatle ilerlemez; kare 16 ms sayılır.
    vi.stubGlobal('requestAnimationFrame', (f: FrameRequestCallback) =>
      setTimeout(() => f(Date.now()), 16),
    );
  });

  afterEach(() => {
    window.__kalfaE2e?.surdur();
    window.__kalfaE2e?.sifirla();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('kancayı kurar ve damgalı parçanın çizimden sonraki gecikmesini ölçer', () => {
    const islenen: number[] = [];
    const dinle = sar((m) => islenen.push(m.no));
    const kanca = window.__kalfaE2e!;

    dinle(paket(1, metin('[[e2e:1:9990]]'), metin('[[e2e:2:9980]]')));
    expect(islenen).toEqual([1]);
    expect(kanca.gecikmeler()).toEqual([]);

    vi.advanceTimersByTime(20);
    expect(kanca.gecikmeler()).toHaveLength(2);
    const [a, b] = kanca.gecikmeler();
    expect(b - a).toBe(10);
    expect(a).toBeGreaterThanOrEqual(10);

    kanca.sifirla();
    expect(kanca.gecikmeler()).toEqual([]);
  });

  it('duraklatılınca mesajları bekletir, sürdürülünce sırayla işler', () => {
    const islenen: number[] = [];
    const dinle = sar((m) => islenen.push(m.no));
    const kanca = window.__kalfaE2e!;

    kanca.duraklat();
    dinle(paket(1, metin('a')));
    dinle(paket(2, metin('b')));
    dinle({ tip: 'Kapandi', no: 3 });
    expect(islenen).toEqual([]);
    expect(kanca.bekleyen()).toBe(3);

    kanca.surdur();
    expect(islenen).toEqual([1, 2, 3]);
    expect(kanca.bekleyen()).toBe(0);

    dinle(paket(4, metin('c')));
    expect(islenen).toEqual([1, 2, 3, 4]);
  });
});
