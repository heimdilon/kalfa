//! Onay kartındaki birleşik diff (spec §11.4).

use similar::TextDiff;

pub fn birlesik_diff(yol: &str, eski: &str, yeni: &str) -> String {
    TextDiff::from_lines(eski, yeni)
        .unified_diff()
        .header(&format!("a/{yol}"), &format!("b/{yol}"))
        .to_string()
}
