import { describe, expect, it } from 'vitest';
import { AZAMI_SINIR, etkinPencereDogrula, tamSayiCoz } from './butce';

describe('tamSayiCoz', () => {
  it('yalnız rakam ve güvenli tamsayı aralığını kabul eder', () => {
    expect(tamSayiCoz(' 42 ')).toBe(42);
    expect(tamSayiCoz('0')).toBe(0);
    expect(tamSayiCoz(String(AZAMI_SINIR))).toBe(AZAMI_SINIR);
    expect(tamSayiCoz('')).toBeNull();
    expect(tamSayiCoz('-3')).toBeNull();
    expect(tamSayiCoz('1,5')).toBeNull();
    expect(tamSayiCoz('12a')).toBeNull();
    expect(tamSayiCoz(String(AZAMI_SINIR + 1))).toBeNull();
  });
});

describe('etkinPencereDogrula', () => {
  it('32 000 ile 2^53 - 1 arasındaki tam sayıları kabul eder, 31 999 ve altını reddeder', () => {
    expect(etkinPencereDogrula('31999')).toBeNull();
    expect(etkinPencereDogrula('0')).toBeNull();
    expect(etkinPencereDogrula('32000')).toBe(32_000);
    expect(etkinPencereDogrula('200000')).toBe(200_000);
    expect(etkinPencereDogrula(String(AZAMI_SINIR))).toBe(AZAMI_SINIR);
  });
});
