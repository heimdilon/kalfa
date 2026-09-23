// Sürüm derlemesinde Windows'ta ayrıca bir konsol penceresi açılmaz.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    kalfa_kabuk::calistir();
}
