//! Kalfa aşama 3d: Linux ağ köprüsü prototipi (aşama 3 tasarımı §8.5, G5).
//!
//!   kalfa-kopru-prototip olc [--cikti <dosya>] [önek…]
//!       ölçümleri koşar; önek verilirse yalnızca kodu o önekle başlayanlar
//!   kalfa-kopru-prototip kopru --kanal <FD> [--lo-ac] [--seccomp] -- <program> …
//!       köprü: ağ ad alanında dinler, bağlantıları kanaldan vekile verir
//!   kalfa-kopru-prototip sina <deneme> …
//!       ağ ad alanının içinden denemeler

#[cfg(target_os = "linux")]
mod ag;
#[cfg(target_os = "linux")]
mod kanal;
#[cfg(target_os = "linux")]
mod kopru;
#[cfg(target_os = "linux")]
mod olc;
#[cfg(target_os = "linux")]
mod seccomp;
#[cfg(target_os = "linux")]
mod sina;
#[cfg(target_os = "linux")]
mod tls;
#[cfg(target_os = "linux")]
mod vekil;

#[cfg(target_os = "linux")]
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let kod = match a.get(1).map(String::as_str) {
        Some("olc") => olc::ana(&a[2..]),
        Some("kopru") => kopru::ana(&a[2..]),
        Some("sina") => sina::ana(&a[2..]),
        _ => {
            eprintln!("kullanım: kalfa-kopru-prototip olc [--cikti <dosya>] [önek…]");
            2
        }
    };
    std::process::exit(kod);
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Bu prototip yalnızca Linux'ta çalışır.");
}
