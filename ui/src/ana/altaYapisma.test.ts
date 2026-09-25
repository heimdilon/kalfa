import { describe, expect, it } from 'vitest';
import { AltaYapisma, type Kutu, kaydirmaNiyetiMi } from './altaYapisma';

/** Tarayıcı gibi `scrollTop`'u içerik sınırına kıstıran kutu. */
function kutu(scrollHeight: number, clientHeight = 500): Kutu {
  let top = 0;
  return {
    get scrollTop() {
      return top;
    },
    set scrollTop(deger: number) {
      top = Math.max(0, Math.min(deger, this.scrollHeight - this.clientHeight));
    },
    scrollHeight,
    clientHeight,
  };
}

function kur() {
  let simdi = 10_000;
  const y = new AltaYapisma(48, () => simdi);
  return { y, ilerle: (ms: number) => (simdi += ms) };
}

describe('alta yapışma', () => {
  it('kullanıcıdan gelmeyen kaydırma yapışmayı bırakmaz', () => {
    const { y } = kur();
    const k = kutu(1000);
    y.kaydir(k);
    expect(k.scrollTop).toBe(500);
    // Kendi kaydırmamızın olayı gelmeden içerik büyüdü (yeni onay kartı).
    k.scrollHeight = 1200;
    y.olay(k);
    expect(y.altta).toBe(true);
    // Onay sayfası kapanınca odak yukarıdaki düğmeye döndü.
    k.scrollTop = 300;
    y.olay(k);
    expect(y.altta).toBe(true);
    y.kaydir(k);
    expect(k.scrollTop).toBe(700);
  });

  it('kullanıcı yukarı kaydırınca bırakır; en alta inince yeniden yapışır', () => {
    const { y, ilerle } = kur();
    const k = kutu(1000);
    y.kaydir(k);
    y.niyet();
    ilerle(50);
    k.scrollTop = 200;
    y.olay(k);
    expect(y.altta).toBe(false);
    k.scrollHeight = 1300;
    y.kaydir(k);
    expect(k.scrollTop).toBe(200);
    // Niyetin süresi geçse de en alta inmek yeniden yapıştırır.
    ilerle(5000);
    k.scrollTop = 790;
    y.olay(k);
    expect(y.altta).toBe(true);
  });

  it('kaydırma niyeti: tekerlek, sürükleme, kaydırma çubuğu ve kaydırma tuşları', () => {
    const kutu = {};
    const dugme = {};
    expect(kaydirmaNiyetiMi({ type: 'wheel', target: dugme }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'touchmove', target: dugme }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'pointerdown', target: kutu }, kutu)).toBe(true);
    // Kutudaki bir düğmeye tıklamak niyet değildir.
    expect(kaydirmaNiyetiMi({ type: 'pointerdown', target: dugme }, kutu)).toBe(false);
    expect(kaydirmaNiyetiMi({ type: 'keydown', target: kutu, key: 'PageUp' }, kutu)).toBe(true);
    expect(kaydirmaNiyetiMi({ type: 'keydown', target: kutu, key: 'Enter' }, kutu)).toBe(false);
    expect(kaydirmaNiyetiMi({ type: 'touchstart', target: kutu }, kutu)).toBe(false);
  });

  it('niyetin süresi geçince gelen kaydırma bırakmaz; sıfırlama en alttan başlatır', () => {
    const { y, ilerle } = kur();
    const k = kutu(1000);
    y.niyet();
    ilerle(1500);
    k.scrollTop = 100;
    y.olay(k);
    expect(y.altta).toBe(true);
    y.niyet();
    y.olay(k);
    expect(y.altta).toBe(false);
    y.sifirla();
    expect(y.altta).toBe(true);
    y.olay(k);
    expect(y.altta).toBe(true);
  });
});
