import { describe, expect, it } from 'vitest';
import { egikKomutAyristir } from './egikKomut';

describe('egikKomutAyristir', () => {
  it('yalnız / ile başlayan metni komut ve argüman olarak ayrıştırır', () => {
    expect(egikKomutAyristir('/compact')).toEqual({ komut: 'compact', arguman: '' });
    expect(egikKomutAyristir('/compact  odak: testler ')).toEqual({
      komut: 'compact',
      arguman: 'odak: testler',
    });
    expect(egikKomutAyristir('merhaba /compact')).toBeNull();
  });
});
