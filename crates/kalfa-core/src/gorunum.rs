//! Onay yüzeylerinde görünmez ve yön değiştiren karakterler (güvenlik analizi
//! Bulgu 7). "Trojan Source" türü bir komut, yol ya da diff kullanıcıya
//! gördüğünden farklı bir işlemi onaylatabilir. Bu karakterler onay
//! yüzeylerinde `⟨U+XXXX⟩` olarak gösterilir ve işlem her kipte onay ister.

pub const GORUNMEZ_UYARISI: &str = "Bu işlemin metninde görünmeyen ya da yazı yönünü \
değiştiren karakterler var; bunlar ⟨U+…⟩ olarak gösteriliyor.";

/// Unicode 16.0 `Cf` (biçim) karakterleri; `DerivedGeneralCategory.txt`'ten.
const BICIM_ARALIKLARI: &[(u32, u32)] = &[
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// Boş görünen Hangul dolgu karakterleri.
const HANGUL_DOLGU: &[char] = &['\u{115F}', '\u{1160}', '\u{3164}', '\u{FFA0}'];

/// Görünmez ya da yön değiştiren karakter mi? `\n` ve `\t` dışındaki denetim
/// karakterleri (`Cc`), biçim karakterleri (`Cf`), satır ve paragraf
/// ayırıcıları (`Zl`, `Zp`) ve Hangul dolgu karakterleri.
pub fn gorunmez_mi(c: char) -> bool {
    if c == '\n' || c == '\t' {
        return false;
    }
    if c.is_control() || c == '\u{2028}' || c == '\u{2029}' || HANGUL_DOLGU.contains(&c) {
        return true;
    }
    let k = u32::from(c);
    BICIM_ARALIKLARI
        .iter()
        .any(|&(bas, son)| (bas..=son).contains(&k))
}

/// Metinde görünmez karakter var mı? `\r\n` satır sonu sayılır.
pub fn gorunmez_var_mi(metin: &str) -> bool {
    let mut karakterler = metin.chars().peekable();
    while let Some(c) = karakterler.next() {
        if c == '\r' && karakterler.peek() == Some(&'\n') {
            continue;
        }
        if gorunmez_mi(c) {
            return true;
        }
    }
    false
}

/// Görünmez karakterleri `⟨U+XXXX⟩` biçiminde yazar. `\r\n` yalnızca `\n`
/// olur; tek başına `\r` kaçışlanır.
pub fn gorunur_yap(metin: &str) -> String {
    let mut sonuc = String::with_capacity(metin.len());
    let mut karakterler = metin.chars().peekable();
    while let Some(c) = karakterler.next() {
        if c == '\r' && karakterler.peek() == Some(&'\n') {
            continue;
        }
        if gorunmez_mi(c) {
            sonuc.push_str(&format!("⟨U+{:04X}⟩", u32::from(c)));
        } else {
            sonuc.push(c);
        }
    }
    sonuc
}
