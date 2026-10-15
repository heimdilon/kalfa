//! Kalfa aşama 2c: Windows yalıtım prototipi (aşama 2 tasarımı §9.5).
//!
//!   kalfa-prototip olc [önek…]   ölçümleri koşar (yönetici gerekmez)
//!   kalfa-prototip olc temizle   deneme ağacını siler
//!   kalfa-prototip calistir      yalıtım kullanıcısı olarak; isteği stdin'den okur
//!   kalfa-prototip ui-dene <h>   yalıtımın içinde P10 denemeleri
//!   kalfa-prototip kap …         AppContainer ek prototipi (kap_olc.rs)
//!   kalfa-prototip g1 olc [kök]  MCP 1.1a G1 ölçümü (g1.rs)

#[cfg(windows)]
mod calistir;
#[cfg(windows)]
mod g1;
#[cfg(windows)]
mod kap;
#[cfg(windows)]
mod kap_olc;
#[cfg(windows)]
mod olc;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod win;

#[cfg(windows)]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let kod = match a.get(1).map(String::as_str) {
        Some("calistir") => calistir::ana(),
        Some("olc") => olc::ana(&a[2..]),
        Some("ui-dene") => ui::dene(&a[2..]),
        Some("kap") => kap_olc::ana(&a[2..]),
        Some("g1") => g1::ana(&a[2..]),
        _ => {
            eprintln!(
                "kullanım: kalfa-prototip olc [önek… | temizle] | kap [hazirla | olc [önek…] | sil]"
            );
            2
        }
    };
    std::process::exit(kod);
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Bu prototip yalnızca Windows'ta çalışır.");
}
