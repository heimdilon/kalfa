use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use kalfa_core::arac::Arac;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::protokol::VeriCikisi;
use kalfa_sandbox::Yalitimsiz;
use kalfa_tools::{
    CalismaAlani, DosyaDuzenle, DosyaListele, DosyaOku, DosyaYaz, IcerikAra, KabukAraci,
    KabukGuveni,
};
use serde_json::json;
use tempfile::TempDir;

#[derive(Default)]
struct SahteGuven;
impl KabukGuveni for SahteGuven {
    fn guvenildi_mi(&self) -> bool {
        true
    }
    fn guvenildi(&self) {}
}

#[tokio::test]
async fn her_arac_veri_cikisini_hazirlikta_verir() {
    let d = TempDir::new().unwrap();
    let s = Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap());
    let alan = CalismaAlani::ac(s).unwrap();

    let oku = DosyaOku::yeni(alan.clone());
    std::fs::write(d.path().join("a.txt"), b"merhaba").unwrap();
    let h_oku = oku.hazirla(&json!({"yol": "a.txt"})).await.unwrap();
    assert_eq!(h_oku.veri_cikisi, Some(VeriCikisi::DosyaIcerigi));

    let ara = IcerikAra::yeni(alan.clone());
    let h_ara = ara.hazirla(&json!({"desen": "merhaba"})).await.unwrap();
    assert_eq!(h_ara.veri_cikisi, Some(VeriCikisi::AramaSonuclari));

    let listele = DosyaListele::yeni(alan.clone());
    let h_listele = listele.hazirla(&json!({"desen": "*"})).await.unwrap();
    assert_eq!(h_listele.veri_cikisi, Some(VeriCikisi::DosyaAdlari));

    let yaz = DosyaYaz::yeni(alan.clone());
    let h_yaz = yaz
        .hazirla(&json!({"yol": "b.txt", "icerik": "selam"}))
        .await
        .unwrap();
    assert_eq!(h_yaz.veri_cikisi, Some(VeriCikisi::YalnizcaYazildi));

    let duzenle = DosyaDuzenle::yeni(alan.clone());
    let h_duzenle = duzenle
        .hazirla(&json!({"yol": "a.txt", "eski_metin": "merhaba", "yeni_metin": "dunya"}))
        .await
        .unwrap();
    assert_eq!(h_duzenle.veri_cikisi, Some(VeriCikisi::YalnizcaYazildi));

    let kabuk = KabukAraci::yeni(
        alan.clone(),
        Arc::new(Yalitimsiz),
        Arc::new(SahteGuven),
        Arc::new(AtomicBool::new(true)),
    );
    let h_kabuk = kabuk
        .hazirla(&json!({"program": "echo", "argumanlar": ["1"]}))
        .await
        .unwrap();
    assert_eq!(h_kabuk.veri_cikisi, Some(VeriCikisi::KomutCiktisi));
}
