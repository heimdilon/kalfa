import { describe, expect, it } from 'vitest';
import type { ButceGorunumu } from '../ipc/tipler';
import {
  AYAR_ALT_SINIRI,
  AZAMI_SINIR,
  butceGirdisiDogrula,
  kartAltSiniri,
  kartOnerisi,
  tamSayiCoz,
} from './butce';

function gorunum(ustune: Partial<ButceGorunumu> = {}): ButceGorunumu {
  return {
    sinirlar: { istek: 200, token: 5_000_000 },
    baslatilan: 0,
    bilinen_token: 0,
    bilinmeyen: 0,
    onaylanan_bilinmeyen: 0,
    aktif: 0,
    engel: null,
    ...ustune,
  };
}

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

describe('butceGirdisiDogrula', () => {
  it('ayar formunda 1 ile 2^53 - 1 arasındaki sınırı kabul eder', () => {
    expect(butceGirdisiDogrula('1', '1', AYAR_ALT_SINIRI)).toEqual({
      gecerli: true,
      sinirlar: { istek: 1, token: 1 },
    });
    expect(butceGirdisiDogrula(String(AZAMI_SINIR), String(AZAMI_SINIR), AYAR_ALT_SINIRI)).toEqual({
      gecerli: true,
      sinirlar: { istek: AZAMI_SINIR, token: AZAMI_SINIR },
    });
  });

  it('sıfırı, sınır üstünü ve biçimi bozuk değeri reddeder', () => {
    expect(butceGirdisiDogrula('0', '10', AYAR_ALT_SINIRI)).toEqual({ gecerli: false, neden: 'istek' });
    expect(butceGirdisiDogrula('10', '0', AYAR_ALT_SINIRI)).toEqual({ gecerli: false, neden: 'token' });
    expect(butceGirdisiDogrula(String(AZAMI_SINIR + 1), '10', AYAR_ALT_SINIRI)).toEqual({
      gecerli: false,
      neden: 'bicim',
    });
    expect(butceGirdisiDogrula('abc', '10', AYAR_ALT_SINIRI)).toEqual({ gecerli: false, neden: 'bicim' });
  });

  it('kartta sınır bilinen tüketimin üstünde olmalı; eşitlik reddedilir', () => {
    const g = gorunum({ baslatilan: 200, bilinen_token: 900, engel: 'istek' });
    const alt = kartAltSiniri(g);
    expect(butceGirdisiDogrula('200', '5000000', alt)).toEqual({ gecerli: false, neden: 'istek' });
    expect(butceGirdisiDogrula('201', '900', alt)).toEqual({ gecerli: false, neden: 'token' });
    expect(butceGirdisiDogrula('201', '901', alt)).toEqual({
      gecerli: true,
      sinirlar: { istek: 201, token: 901 },
    });
  });
});

describe('kartOnerisi', () => {
  it('geçerli limitleri olduğu gibi önerir', () => {
    expect(kartOnerisi(gorunum({ baslatilan: 3, bilinen_token: 100 }))).toEqual({
      istek: 200,
      token: 5_000_000,
    });
  });

  it('engel bir limitten geliyorsa o limiti tüketimin üstüne önerir', () => {
    const istekEngeli = gorunum({ baslatilan: 200, engel: 'istek' });
    expect(kartOnerisi(istekEngeli)).toEqual({ istek: 400, token: 5_000_000 });

    const tokenEngeli = gorunum({ bilinen_token: 5_000_000, engel: 'token' });
    expect(kartOnerisi(tokenEngeli)).toEqual({ istek: 200, token: 10_000_000 });
  });

  it('öneriyi güvenli tamsayı sınırında tutar', () => {
    const yuksek = gorunum({
      sinirlar: { istek: 200, token: 5_000_000 },
      bilinen_token: AZAMI_SINIR - 10,
      engel: 'token',
    });
    expect(kartOnerisi(yuksek).token).toBe(AZAMI_SINIR);
  });
});
