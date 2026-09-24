import { describe, expect, it } from 'vitest';
import { ansiParcala } from './ansi';

describe('ANSI (aşama 2 tasarımı §10.1)', () => {
  it('kaçışsız metin tek parçadır', () => {
    expect(ansiParcala('düz metin')).toEqual([{ metin: 'düz metin', renk: null, kalin: false }]);
  });

  it('renk ve kalınlık çizilir, sıfırlama geri alır', () => {
    expect(ansiParcala('\x1b[1;31merror\x1b[0m: bozuk \x1b[32mok\x1b[39m son')).toEqual([
      { metin: 'error', renk: 'kirmizi', kalin: true },
      { metin: ': bozuk ', renk: null, kalin: false },
      { metin: 'ok', renk: 'yesil', kalin: false },
      { metin: ' son', renk: null, kalin: false },
    ]);
  });

  it('SGR dışındaki kaçışlar atılır', () => {
    expect(ansiParcala('\x1b[2K\x1b[1Gsatır\x1b]0;başlık\x07 sonu\x1b7')).toEqual([
      { metin: 'satır sonu', renk: null, kalin: false },
    ]);
  });

  it('HTML olarak yorumlanacak bir şey üretmez; metin olduğu gibi kalır', () => {
    expect(ansiParcala('<img src=x onerror=alert(1)>')).toEqual([
      { metin: '<img src=x onerror=alert(1)>', renk: null, kalin: false },
    ]);
  });
});
