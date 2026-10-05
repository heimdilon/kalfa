//! Onay kartındaki birleşik diff (spec §11.4).

use similar::TextDiff;

pub fn birlesik_diff(yol: &str, eski: &str, yeni: &str) -> String {
    TextDiff::from_lines(eski, yeni)
        .unified_diff()
        .header(&format!("a/{yol}"), &format!("b/{yol}"))
        .to_string()
}

/// Diff satır tabloları ve çıktı da eski/yeni dosya boyutundan önce sınırlanır.
pub(crate) fn sinirli_diff(yol: &str, eski: &str, yeni: &str) -> Option<String> {
    if eski.len().saturating_add(yeni.len()) > 512 * 1024
        || eski.lines().take(4097).count() > 4096
        || yeni.lines().take(4097).count() > 4096
    {
        None
    } else {
        Some(birlesik_diff(yol, eski, yeni))
    }
}
