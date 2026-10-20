//! Kalfa Yetenek Programı Hat 03 — G2a Süreç ve Terminal Deney Kapısı
//! (`prototip/yetenek-surec`).
//!
//! Kullanım:
//!   cargo run --offline -- olc
//!   cargo run --offline -- cocuk <alt-komut>

mod kilit_deneyi;
#[cfg(target_os = "linux")]
mod linux_deneyi;
#[cfg(windows)]
mod win_deneyi;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("cocuk") => {
            let alt = args.get(2).map(String::as_str).unwrap_or("");
            #[cfg(windows)]
            std::process::exit(win_deneyi::cocuk_komutu(alt));
            #[cfg(target_os = "linux")]
            std::process::exit(linux_deneyi::cocuk_komutu(alt));
            #[cfg(not(any(windows, target_os = "linux")))]
            {
                let _ = alt;
                std::process::exit(2);
            }
        }
        Some("olc") | None => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio çalışma zamanı kurulamadı");
            let sonuc = rt.block_on(async {
                let mut tum_satirlar = kilit_deneyi::calistir().await?;
                #[cfg(windows)]
                {
                    let win_satirlar = win_deneyi::calistir()?;
                    tum_satirlar.extend(win_satirlar);
                }
                #[cfg(target_os = "linux")]
                {
                    let linux_satirlar = linux_deneyi::calistir()?;
                    tum_satirlar.extend(linux_satirlar);
                }
                Ok::<_, String>(tum_satirlar)
            });

            match sonuc {
                Ok(satirlar) => {
                    println!("| Deney | Sonuç | Ayrıntı |");
                    println!("|---|---|---|");
                    let mut hepsi_gecti = true;
                    for s in &satirlar {
                        hepsi_gecti &= s.gecti;
                        println!(
                            "| {} | {} | {} |",
                            s.ad,
                            if s.gecti { "GEÇTİ" } else { "KALDI" },
                            s.ayrinti
                        );
                    }
                    std::process::exit(if hepsi_gecti { 0 } else { 1 });
                }
                Err(hata) => {
                    eprintln!("G2a DENEY HATASI: {hata}");
                    std::process::exit(2);
                }
            }
        }
        Some(diger) => {
            eprintln!("bilinmeyen alt komut: {diger}");
            std::process::exit(2);
        }
    }
}
