fn main() -> Result<(), String> {
    for boyut in [1024, 4096] {
        for adet in [1, 16, 256] {
            for yol in ["sdk-base64", "sdk-float", "ham-crate", "ham-elle"] {
                let argumanlar = format!(r#"{{"adet":{adet},"boyut":{boyut},"yol":"{yol}"}}"#);
                println!("{}", kalfa_g6_olcum::olcum::olc(&argumanlar, "yerel")?);
            }
        }
    }
    Ok(())
}
