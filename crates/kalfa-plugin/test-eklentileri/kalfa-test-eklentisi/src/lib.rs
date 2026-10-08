//! `kalfa-plugin`'in testlerinin eklentisi (aşama 4 tasarımı §16.1). Her araç
//! bir davranışı sınar; testler aynı `.wasm`'ı farklı manifestlerle yükler.

use kalfa_plugin_sdk::serde_json::Value;
use kalfa_plugin_sdk::{AracTanimi, EvrenUcu, Guest, Hata, Ilerleme, Seviye, Sonuc, host};

struct TestEklentisi;

const ARACLAR: &[&str] = &[
    "yanki", "dongu", "bellek", "sisirme", "dosya", "yetkisiz", "depo", "iptal", "panik",
];

fn metin(a: &Value, alan: &str) -> String {
    a[alan].as_str().unwrap_or_default().to_string()
}

fn hata_metni(h: &Hata) -> String {
    match h {
        Hata::IzinYok(m) => format!("izin-yok: {m}"),
        Hata::Bulunamadi(m) => format!("bulunamadi: {m}"),
        Hata::IptalEdildi => "iptal-edildi".into(),
        Hata::SinirAsildi(m) => format!("sinir-asildi: {m}"),
        Hata::Evren(m) => format!("evren: {m}"),
        Hata::Diger(m) => format!("diger: {m}"),
    }
}

fn sonuc<T>(r: Result<T, Hata>, f: impl FnOnce(T) -> String) -> String {
    match r {
        Ok(d) => f(d),
        Err(h) => hata_metni(&h),
    }
}

impl Guest for TestEklentisi {
    fn araclari_listele() -> Vec<AracTanimi> {
        ARACLAR
            .iter()
            .map(|ad| AracTanimi {
                ad: ad.to_string(),
                aciklama: format!("Test aracı: {ad}"),
                parametre_semasi: r#"{"type":"object"}"#.into(),
            })
            .collect()
    }

    fn calistir(ad: String, argumanlar: String) -> Result<Sonuc, Hata> {
        let a: Value = kalfa_plugin_sdk::argumanlar(&argumanlar)?;
        let icerik = match ad.as_str() {
            "yanki" => {
                host::gunluk(Seviye::Bilgi, "yanki çağrıldı");
                host::ilerleme_bildir(&Ilerleme {
                    tamamlanan: 1,
                    toplam: Some(1),
                    mesaj: Some("bitti".into()),
                });
                metin(&a, "metin")
            }
            // CPU bütçesi: epoch kesmesi bunu durdurmalı.
            "dongu" => {
                let mut x: u64 = 1;
                loop {
                    x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if x == 0 {
                        break format!("{x}");
                    }
                }
            }
            // Bellek sınırı: 1 GiB istenir, sınır 512 MiB.
            "bellek" => {
                let mut parcalar: Vec<Vec<u8>> = Vec::new();
                for _ in 0..1024 {
                    parcalar.push(vec![1u8; 1 << 20]);
                }
                format!("{} MiB ayrıldı", parcalar.len())
            }
            // Sonuç 64 KiB'ta kırpılmalı; günlük 200 satırda kesilmeli.
            "sisirme" => {
                for i in 0..300 {
                    host::gunluk(Seviye::Ayrinti, &format!("satır {i}"));
                }
                "x".repeat(200 * 1024)
            }
            "dosya" => match metin(&a, "islem").as_str() {
                "oku" => sonuc(host::dosya_oku(&metin(&a, "yol")), |m| m),
                "listele" => sonuc(host::dosya_listele(&metin(&a, "desen")), |l| {
                    l.iter()
                        .map(|d| format!("{}\t{}\t{}", d.yol, d.boyut, d.degisme))
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
                // WASI dosya sistemi: ön açık dizin yokken her yol açılamamalı.
                "std" => match std::fs::read_to_string(metin(&a, "yol")) {
                    Ok(m) => format!("OKUNDU: {m}"),
                    Err(e) => format!("okunamadı: {e}"),
                },
                diger => return Err(Hata::Diger(format!("bilinmeyen işlem: {diger}"))),
            },
            "yetkisiz" => {
                let evren = sonuc(host::evren_cagir(EvrenUcu::Embeddings, "{}"), |y| y);
                let depo = sonuc(host::depo_yaz("a", &[1]), |()| "yazıldı".into());
                format!("evren: {evren}\ndepo: {depo}")
            }
            "depo" => {
                let anahtar = metin(&a, "anahtar");
                match metin(&a, "islem").as_str() {
                    "oku" => sonuc(host::depo_oku(&anahtar), |d| match d {
                        Some(b) => format!("değer: {}", String::from_utf8_lossy(&b)),
                        None => "yok".into(),
                    }),
                    "yaz" => {
                        let deger = match a["boyut"].as_u64() {
                            Some(n) => vec![b'd'; n as usize],
                            None => metin(&a, "deger").into_bytes(),
                        };
                        sonuc(host::depo_yaz(&anahtar, &deger), |()| "yazıldı".into())
                    }
                    "sil" => sonuc(host::depo_sil(&anahtar), |()| "silindi".into()),
                    diger => return Err(Hata::Diger(format!("bilinmeyen işlem: {diger}"))),
                }
            }
            // `inatci`: iptali yok sayar; host 2 sn sonra bırakmalı.
            "iptal" => {
                let inatci = a["inatci"].as_bool().unwrap_or(false);
                let mut x: u64 = 1;
                loop {
                    for _ in 0..100_000 {
                        x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                    }
                    if !inatci && host::iptal_istendi() {
                        return Err(Hata::IptalEdildi);
                    }
                    if x == 0 {
                        break format!("{x}");
                    }
                }
            }
            "panik" => {
                eprintln!("stderr satırı");
                panic!("bilerek panik");
            }
            _ => return Err(Hata::Bulunamadi(ad)),
        };
        Ok(Sonuc { icerik })
    }
}

kalfa_plugin_sdk::kalfa_eklentisi!(TestEklentisi);
