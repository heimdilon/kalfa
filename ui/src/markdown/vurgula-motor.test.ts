// Shiki'nin JavaScript düzenli ifade motoru, seçilen dillerde TextMate
// dilbilgilerinin asıl motoru Oniguruma ile aynı tokenları verir (1c tasarımı
// §9.2, §13). Oniguruma'nın WASM'ı yalnızca bu testte yüklenir; pakete girmez.
import { createHighlighterCore, type HighlighterCore } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import { createOnigurumaEngine } from 'shiki/engine/oniguruma';
import { beforeAll, describe, expect, it } from 'vitest';
import { DILLER, TEMA } from './vurgula';

type Dil = keyof typeof DILLER;

const ORNEKLER: Record<Dil, string> = {
  rust: String.raw`use std::collections::HashMap;

/// Belge yorumu
#[derive(Debug, Clone)]
pub struct Kayit<'a> { ad: &'a str, sayi: u64 }

impl<'a> Kayit<'a> {
    pub fn yeni(ad: &'a str) -> Self {
        let ham = r#"iç "tırnak" \n"#;
        let bayt = b'x';
        match ad.len() { 0 => panic!("boş: {}", ham), n if n > 3 => {}, _ => () }
        Self { ad, sayi: 0x1F_u64 + 1_000 + bayt as u64 }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> { let mut m: HashMap<&str, i32> = HashMap::new(); m.insert("a", -1); Ok(()) }
`,
  typescript: String.raw`import { type A, b } from './a';
export interface Kutu<T extends object = {}> { deger?: T; readonly ad: string }
enum Renk { Kirmizi = 'k', Mavi = 'm' }
const desen = /^[a-z]+\/(\d{2,})$/giu;
export async function* uret<T>(xs: T[]): AsyncGenerator<T> { for (const x of xs) yield x as T; }
const sablon = ` + '`toplam: ${1 + 2} ${`iç ${b}`}`' + String.raw`;
let n: number | undefined = a?.b ?? 0n;
class C<T> implements Kutu<T> { #gizli = 1; static readonly ad = 'c'; constructor(private readonly x: T) {} }
type Esle<T> = T extends (infer U)[] ? U : never;
`,
  javascript: String.raw`// @ts-check
'use strict';
const nesne = { a: 1, 'b-c': [2, 3], [Symbol.iterator]: function* () {} };
const bolme = 10 / 2 / 5, desen = /a\/b[/]c/g;
label: for (let i = 0; i < 3; i++) { if (i === 1) continue label; }
export default class extends Error { get ad() { return ` + '`x${this.message}`' + String.raw`; } }
const ok = async (x = 1, ...geri) => await import('./m.js');
`,
  json: `{
  "ad": "kalfa",
  "sürüm": 1.5e-3,
  "liste": [true, false, null, {"iç": "\\"kaçış\\" \\u00e7"}],
  "boş": {}
}
`,
  toml: `# yorum
[package]
name = "kalfa"
version = "0.1.0"
edition = '2024'
[dependencies.tauri]
version = "2.11"
features = ["test", 'x']
[[bin]]
name = "a"
zaman = 1979-05-27T07:32:00Z
sayi = +1_000
cok_satir = """
satır
"""
`,
  yaml: `# yorum
ad: kalfa
sürüm: 1.0
liste:
  - bir
  - "iki"
  - 'üç'
nesne: { a: 1, b: [x, y] }
cok: |
  satır bir
  satır iki
bagla: &a
  x: 1
kullan: *a
bos: ~
tarih: 2026-09-23
`,
  // `${…}` şablon dizesinde yorumlanacağı için satır satır.
  shellscript: [
    '#!/usr/bin/env bash',
    'set -euo pipefail',
    'AD="kalfa"; SAYI=$((1 + 2))',
    'for f in *.rs; do echo "dosya: $f ${AD:-yok} $(basename "$f")"; done',
    "if [[ -n \"$AD\" && $SAYI -gt 2 ]]; then printf '%s\\n' 'tek' > /dev/null 2>&1; fi",
    "cat <<'SON'",
    'belge $AD',
    'SON',
    'fonk() { local x=$1; return 0; }',
    'case "$1" in start|basla) fonk a ;; *) exit 1 ;; esac',
    '',
  ].join('\n'),
  python: String.raw`from __future__ import annotations
import re

@dataclass(frozen=True)
class Kayit:
    """Belge dizesi."""
    ad: str
    sayi: int = 0x1F

    def __repr__(self) -> str:
        return f"Kayit({self.ad!r}, {self.sayi:>4}) {{kaçış}}"

async def uret(xs: list[int]) -> AsyncIterator[int]:
    desen = re.compile(r"^\d+(?:\.\d+)?$")
    yield from (x ** 2 for x in xs if x % 2 == 0 and not None)
    lambda a, *b, **c: a
`,
  diff: `diff --git a/a.rs b/a.rs
index 1234567..89abcde 100644
--- a/a.rs
+++ b/a.rs
@@ -1,4 +1,4 @@ fn main
 fn main() {
-    let a = 1;
+    let a = 2;
 }
\\ No newline at end of file
`,
  html: `<!doctype html>
<html lang="tr">
<head>
  <meta charset="utf-8" />
  <title>Kalfa &amp; EVREN</title>
  <style>body { color: #333; margin: 0 auto; }</style>
  <script type="module">const x = 1 < 2 && "</b>"; console.log(x);</script>
</head>
<body class="kalfa-root" data-x='y'>
  <!-- yorum -->
  <a href="https://example.com/?a=1&b=2" onclick="return false">bağlantı</a>
  <input disabled type=checkbox>
</body>
</html>
`,
  css: `@import url("tokenlar.css");
:root { --renk: #b8754f; --bosluk: calc(4px * 2); }
.kalfa-root :focus-visible, a[href^="https"]::after {
  outline: 2px solid color-mix(in srgb, var(--renk) 40%, transparent);
  font: 500 14px/1.4 "Source Sans 3 Variable", sans-serif !important;
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { --renk: rgb(0 0 0 / 50%); } }
@keyframes don { from { transform: rotate(0deg) } to { transform: rotate(360deg) } }
`,
};

type Token = [string, string | undefined];

function tokenlar(v: HighlighterCore, kod: string, dil: Dil): Token[][] {
  return v
    .codeToTokens(kod, { lang: dil, theme: 'kalfa' })
    .tokens.map((satir) => satir.map((t): Token => [t.content, t.color]));
}

describe('Shiki JavaScript motoru', () => {
  let js: HighlighterCore;
  let onig: HighlighterCore;

  beforeAll(async () => {
    const langs = Object.values(DILLER).map((yukle) => yukle());
    js = await createHighlighterCore({
      themes: [TEMA],
      langs,
      engine: createJavaScriptRegexEngine(),
    });
    onig = await createHighlighterCore({
      themes: [TEMA],
      langs,
      engine: createOnigurumaEngine(import('shiki/wasm')),
    });
  });

  it('her dilin bir örneği var', () => {
    expect(Object.keys(ORNEKLER).sort()).toEqual(Object.keys(DILLER).sort());
  });

  it.each(Object.keys(DILLER) as Dil[])('%s: Oniguruma ile aynı tokenlar', (dil) => {
    expect(tokenlar(js, ORNEKLER[dil], dil)).toEqual(tokenlar(onig, ORNEKLER[dil], dil));
  });
});
