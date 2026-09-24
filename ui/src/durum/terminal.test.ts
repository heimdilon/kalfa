import { describe, expect, it } from 'vitest';
import {
  AZAMI_KARAKTER,
  AZAMI_SATIR,
  BOS_TERMINAL,
  SATIR_SINIRI,
  terminalEkle,
  type Terminal,
} from './terminal';

function metinler(t: Terminal): string[] {
  return t.satirlar.map((s) => `${s.akis}:${s.metin}`);
}

describe('terminal durumu (aşama 2 tasarımı §10.1)', () => {
  it('satır sonuyla bitmeyen parça aynı akışın sonraki parçasıyla birleşir', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'mer', 0);
    t = terminalEkle(t, 'Standart', 'haba\ndün', 0);
    t = terminalEkle(t, 'Standart', 'ya\n', 0);
    expect(metinler(t)).toEqual(['Standart:merhaba', 'Standart:dünya']);
    expect(t.acik).toBe(false);
  });

  it('farklı akış yeni satır açar', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    t = terminalEkle(t, 'Hata', 'uyarı\n', 0);
    t = terminalEkle(t, 'Standart', 'b\n', 0);
    expect(metinler(t)).toEqual(['Standart:a', 'Hata:uyarı', 'Standart:b']);
  });

  it('satır başı satırın üzerine yazar', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', '%10', 0);
    t = terminalEkle(t, 'Standart', '\r%50', 0);
    t = terminalEkle(t, 'Standart', '\r%100\nbitti\n', 0);
    expect(metinler(t)).toEqual(['Standart:%100', 'Standart:bitti']);
  });

  it('boş satırlar korunur', () => {
    const t = terminalEkle(BOS_TERMINAL, 'Standart', 'a\n\nb\n', 0);
    expect(metinler(t)).toEqual(['Standart:a', 'Standart:', 'Standart:b']);
  });

  it('atılan bayt bir bilgi satırıyla bildirilir ve toplanır', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    t = terminalEkle(t, 'Standart', 'b\n', 1200);
    t = terminalEkle(t, 'Standart', '', 30);
    expect(metinler(t)).toEqual([
      'Standart:a',
      'Bilgi:… 1200 bayt gösterilmedi …',
      'Standart:b',
      'Bilgi:… 30 bayt gösterilmedi …',
    ]);
    expect(t.gosterilmeyen_bayt).toBe(1230);
  });

  it('satır sınırı aşılınca en eskiler atılır ve sayılır', () => {
    let t = BOS_TERMINAL;
    for (let i = 0; i < AZAMI_SATIR + 10; i++) t = terminalEkle(t, 'Standart', `${i}\n`, 0);
    expect(t.satirlar).toHaveLength(AZAMI_SATIR);
    expect(t.atilan_satir).toBe(10);
    expect(t.satirlar[0].metin).toBe('10');
  });

  it('uzun satır kesilir; devamı eklenmez', () => {
    let t = terminalEkle(BOS_TERMINAL, 'Standart', 'x'.repeat(SATIR_SINIRI + 10), 0);
    t = terminalEkle(t, 'Standart', 'yyy\nsonraki\n', 0);
    expect(t.satirlar[0]).toEqual({ akis: 'Standart', metin: 'x'.repeat(SATIR_SINIRI), kesik: true });
    expect(t.satirlar[1].metin).toBe('sonraki');
    expect(t.karakter).toBe(SATIR_SINIRI + 'sonraki'.length);
  });

  it('karakter sınırı aşılınca en eski satırlar atılır', () => {
    let t = BOS_TERMINAL;
    const satir = 'a'.repeat(1000);
    for (let i = 0; i < AZAMI_KARAKTER / 1000 + 20; i++) {
      t = terminalEkle(t, 'Standart', `${satir}\n`, 0);
    }
    expect(t.karakter).toBeLessThanOrEqual(AZAMI_KARAKTER);
    expect(t.atilan_satir).toBe(20);
    expect(t.karakter).toBe(t.satirlar.reduce((n, s) => n + s.metin.length, 0));
  });

  it('eski durumu değiştirmez', () => {
    const once = terminalEkle(BOS_TERMINAL, 'Standart', 'a', 0);
    terminalEkle(once, 'Standart', 'b', 0);
    expect(metinler(once)).toEqual(['Standart:a']);
  });
});
