//! Görünmez ve yön değiştiren karakterler (güvenlik analizi Bulgu 7).

use kalfa_core::gorunum::{gorunmez_mi, gorunmez_var_mi, gorunur_yap};

/// `ui/src/bilesenler/gorunmez.test.ts` ile aynı temsilî liste.
const GORUNMEZLER: &[u32] = &[
    0x0000, 0x0007, 0x000B, 0x000D, 0x001B, 0x007F, 0x0085, 0x009B, 0x00AD, 0x061C, 0x180E, 0x200B,
    0x200C, 0x200D, 0x200E, 0x200F, 0x202A, 0x202B, 0x202C, 0x202D, 0x202E, 0x2028, 0x2029, 0x2060,
    0x2066, 0x2067, 0x2068, 0x2069, 0xFEFF, 0xFFF9, 0x115F, 0x1160, 0x3164, 0xFFA0, 0xE0001,
    0xE0041,
];

const GORUNURLER: &[char] = &[
    'a', 'ç', 'ğ', 'ı', 'İ', 'ş', 'ö', 'ü', ' ', '\n', '\t', '😀', '→', '中',
];

#[test]
fn temsili_liste_yakalanir() {
    for &kod in GORUNMEZLER {
        let c = char::from_u32(kod).unwrap();
        assert!(gorunmez_mi(c), "U+{kod:04X}");
    }
}

#[test]
fn turkce_emoji_satir_sonu_ve_sekme_yakalanmaz() {
    for &c in GORUNURLER {
        assert!(!gorunmez_mi(c), "{c:?}");
    }
}

#[test]
fn her_denetim_karakteri_yakalanir() {
    for kod in 0..=0x10FFFF_u32 {
        let Some(c) = char::from_u32(kod) else {
            continue;
        };
        if c.is_control() && c != '\n' && c != '\t' {
            assert!(gorunmez_mi(c), "U+{kod:04X}");
        }
    }
}

#[test]
fn kacis_bicimi_ve_crlf() {
    assert_eq!(gorunur_yap("a\u{202E}b"), "a⟨U+202E⟩b");
    assert_eq!(gorunur_yap("x\u{E0041}"), "x⟨U+E0041⟩");
    assert_eq!(gorunur_yap("a\r\nb\rc"), "a\nb⟨U+000D⟩c");
    assert!(!gorunmez_var_mi("satır\r\nsatır\tsekme"));
    assert!(gorunmez_var_mi("a\rb"));
}
