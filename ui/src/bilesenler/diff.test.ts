import { describe, expect, it } from 'vitest';
import { diffCozumle, diffSayilari } from './diff';

const DIFF = [
  '--- a/src/main.rs',
  '+++ b/src/main.rs',
  '@@ -1,3 +1,4 @@',
  ' fn main() {',
  '-    eski();',
  '+    yeni();',
  '+    --- tire ile başlayan satır',
  ' }',
  '@@ -10,2 +11,2 @@ fn b()',
  '-x',
  '+y',
  '\\ No newline at end of file',
  '',
].join('\n');

describe('birleşik diff', () => {
  it('bloklara ve numaralı satırlara çevrilir', () => {
    const [ilk, ikinci] = diffCozumle(DIFF);
    expect(ilk.baslik).toBe('@@ -1,3 +1,4 @@');
    expect(ilk.satirlar).toEqual([
      { kind: 'ctx', old: 1, neu: 1, text: 'fn main() {' },
      { kind: 'del', old: 2, text: '    eski();' },
      { kind: 'add', neu: 2, text: '    yeni();' },
      { kind: 'add', neu: 3, text: '    --- tire ile başlayan satır' },
      { kind: 'ctx', old: 3, neu: 4, text: '}' },
    ]);
    expect(ikinci.baslik).toBe('@@ -10,2 +11,2 @@ fn b()');
    expect(ikinci.satirlar).toEqual([
      { kind: 'del', old: 10, text: 'x' },
      { kind: 'add', neu: 11, text: 'y' },
    ]);
  });

  it('eklenen ve silinen satırları sayar', () => {
    expect(diffSayilari(diffCozumle(DIFF))).toEqual({ eklenen: 3, silinen: 2 });
  });

  it('yeni dosya', () => {
    const [p] = diffCozumle('--- a/x\n+++ b/x\n@@ -0,0 +1,2 @@\n+a\n+b\n');
    expect(p.satirlar).toEqual([
      { kind: 'add', neu: 1, text: 'a' },
      { kind: 'add', neu: 2, text: 'b' },
    ]);
  });
});
