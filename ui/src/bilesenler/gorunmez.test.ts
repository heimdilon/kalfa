import { describe, expect, it } from 'vitest';
import { gorunmezMi, gorunurMetin, gorunurYap } from './gorunmez';

// Rust'taki `crates/kalfa-core/tests/gorunum.rs` ile aynı temsilî liste.
const GORUNMEZLER = [
  0x0000, 0x0007, 0x000b, 0x000d, 0x001b, 0x007f, 0x0085, 0x009b, 0x00ad, 0x061c, 0x180e, 0x200b,
  0x200c, 0x200d, 0x200e, 0x200f, 0x202a, 0x202b, 0x202c, 0x202d, 0x202e, 0x2028, 0x2029, 0x2060,
  0x2066, 0x2067, 0x2068, 0x2069, 0xfeff, 0xfff9, 0x115f, 0x1160, 0x3164, 0xffa0, 0xe0001, 0xe0041,
];
const GORUNURLER = ['a', 'ç', 'ğ', 'ı', 'İ', 'ş', 'ö', 'ü', ' ', '\n', '\t', '😀', '→', '中'];

describe('görünmez karakterler (Bulgu 7)', () => {
  it('temsilî listenin tamamı yakalanır', () => {
    for (const kod of GORUNMEZLER) {
      expect(gorunmezMi(String.fromCodePoint(kod)), kod.toString(16)).toBe(true);
    }
  });

  it('Türkçe harfler, emoji, satır sonu ve sekme yakalanmaz', () => {
    for (const k of GORUNURLER) expect(gorunmezMi(k), k).toBe(false);
  });

  it('kaçış biçimi ve CRLF', () => {
    expect(gorunurMetin('a‮b')).toBe('a⟨U+202E⟩b');
    expect(gorunurMetin('x\u{E0041}')).toBe('x⟨U+E0041⟩');
    expect(gorunurMetin('a\r\nb\rc')).toBe('a\nb⟨U+000D⟩c');
    expect(gorunurYap('a​b')).toEqual([
      { metin: 'a', kacis: false },
      { metin: '⟨U+200B⟩', kacis: true },
      { metin: 'b', kacis: false },
    ]);
  });
});
