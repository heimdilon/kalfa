import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import { guvenliAdres } from './adres';
import Markdown from './Markdown.svelte';
import { dilAdi, vurgula } from './vurgula';

function ciz(metin: string, akista = false) {
  const baglantiAc = vi.fn();
  const sonuc = render(Markdown, { props: { metin, akista, baglantiAc } });
  return { ...sonuc, baglantiAc };
}

describe('güvenli Markdown (1c tasarımı §9.1, §11.3)', () => {
  it('ham HTML çalışmaz, metin olarak görünür', () => {
    const { container } = ciz(
      '<script>alert(1)</script>\n\nmetin <img src=x onerror="alert(2)"> sonu',
    );
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<script>alert(1)</script>');
    expect(container.textContent).toContain('<img src=x onerror="alert(2)">');
  });

  it('javascript: ve file: bağlantıya dönüşmez, düz metin kalır', () => {
    const { container } = ciz('[tıkla](javascript:alert(1)) ve [dosya](file:///etc/passwd)');
    expect(container.querySelector('button')).toBeNull();
    expect(container.querySelector('a')).toBeNull();
    expect(container.textContent).toContain('tıkla ve dosya');
  });

  it('http bağlantısı düğmedir; tıklanınca uygulamaya adres verilir', async () => {
    const { baglantiAc } = ciz('[EVREN](https://evren.example/yol)');
    const dugme = screen.getByRole('button', { name: 'EVREN' });
    await fireEvent.click(dugme);
    expect(baglantiAc).toHaveBeenCalledWith('https://evren.example/yol');
  });

  it('uzak görsel yüklenmez, yerinde alt metni görünür', () => {
    const { container } = ciz('![build rozeti](https://img.shields.io/badge/a-b-green)');
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('[build rozeti]');
  });

  it('kapanmamış kod bloğu metnin sonuna kadar sürer', () => {
    const { container } = ciz('önce\n\n```rust\nfn main() {\n    let a = 1;', true);
    const kod = container.querySelector('.kod');
    expect(kod?.textContent).toContain('fn main() {');
    expect(kod?.textContent).toContain('let a = 1;');
  });

  it('GFM tablosu hizalamasıyla çizilir', () => {
    const { container } = ciz('| a | b |\n|:--|--:|\n| 1 | 2 |');
    expect(container.querySelectorAll('th')).toHaveLength(2);
    expect(container.querySelector('td.sag')?.textContent).toBe('2');
  });

  it('görev listesi kutuları salt okunurdur', () => {
    const { container } = ciz('- [x] bitti\n- [ ] bekliyor');
    const kutular = container.querySelectorAll<HTMLInputElement>('input[type="checkbox"]');
    expect([...kutular].map((k) => [k.checked, k.disabled])).toEqual([
      [true, true],
      [false, true],
    ]);
  });
});

describe('adres denetimi', () => {
  it('yalnızca mutlak http ve https', () => {
    expect(guvenliAdres('https://a.example/x?y=1')).toBe('https://a.example/x?y=1');
    expect(guvenliAdres('http://a.example')).toBe('http://a.example/');
    for (const kotu of [
      'javascript:alert(1)',
      'JAVASCRIPT:alert(1)',
      'file:///etc/passwd',
      'data:text/html,x',
      '/goreli',
      'goreli.html',
      'mailto:a@b.c',
    ]) {
      expect(guvenliAdres(kotu), kotu).toBeNull();
    }
  });
});

describe('kod renklendirme (1c tasarımı §9.2)', () => {
  it('dil etiketleri ve takma adlar', () => {
    expect(dilAdi('rust')).toBe('rust');
    expect(dilAdi('RS')).toBe('rust');
    expect(dilAdi('sh')).toBe('shellscript');
    expect(dilAdi('brainfuck')).toBeNull();
  });

  it('JavaScript motoruyla satır satır renkli parçalar', async () => {
    const satirlar = await vurgula('fn main() {\n    let a = "x";\n}\n', 'rust');
    expect(satirlar).toHaveLength(3);
    const parcalar = satirlar?.flat() ?? [];
    expect(parcalar.map((p) => p.metin).join('')).toBe('fn main() {    let a = "x";}');
    expect(parcalar.find((p) => p.metin === 'fn')?.renk.toLowerCase()).toBe('#c99bd6');
    expect(parcalar.find((p) => p.metin.includes('x'))?.renk.toLowerCase()).toBe('#8fbf7f');
  });

  it('desteklenmeyen dil renklendirilmez', async () => {
    expect(await vurgula('x', 'brainfuck')).toBeNull();
  });
});
