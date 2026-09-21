use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::*;
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::CagriId;
use kalfa_core::muhur::{muhur_hesapla, ozet_hex};
use kalfa_core::yol::CalismaYolu;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

struct Deneme {
    ad: &'static str,
    yazar: bool,
    sema: Value,
}

impl Deneme {
    fn yazan() -> Self {
        Self {
            ad: "yazan",
            yazar: true,
            sema: json!({
                "type": "object",
                "properties": {"yol": {"type": "string"}, "adet": {"type": "integer", "minimum": 1}},
                "required": ["yol"],
                "additionalProperties": false
            }),
        }
    }

    fn okuyan() -> Self {
        Self {
            ad: "okuyan",
            yazar: false,
            sema: json!({"type": "object"}),
        }
    }
}

#[async_trait]
impl Arac for Deneme {
    fn ad(&self) -> &str {
        self.ad
    }
    fn aciklama(&self) -> &str {
        "deneme aracı"
    }
    fn parametre_semasi(&self) -> Value {
        self.sema.clone()
    }
    fn yazar_mi(&self) -> bool {
        self.yazar
    }
    async fn hazirla(&self, _a: &Value) -> Result<Hazirlik, AracHatasi> {
        Ok(Hazirlik::default())
    }
    async fn calistir(
        &self,
        _a: &Value,
        _h: &Hazirlik,
        _i: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Ok(AracCiktisi {
            icerik: "ok".into(),
            kaynak: "deneme".into(),
        })
    }
}

fn defter() -> AracKayitDefteri {
    AracKayitDefteri::yeni(vec![Arc::new(Deneme::yazan()), Arc::new(Deneme::okuyan())]).unwrap()
}

#[test]
fn argumanlar_semaya_gore_dogrulanir() {
    let d = defter();
    assert_eq!(
        d.dogrula("yazan", r#"{"yol":"a"}"#).unwrap(),
        json!({"yol": "a"})
    );
    let hata = d.dogrula("yazan", r#"{"yol":"a","adet":0}"#).unwrap_err();
    assert!(hata.contains("şemaya uymuyor"), "{hata}");
    assert!(d.dogrula("yazan", r#"{"yol":"a","fazla":1}"#).is_err());
    assert!(
        d.dogrula("yazan", "{}")
            .unwrap_err()
            .contains("şemaya uymuyor")
    );
    assert!(
        d.dogrula("yazan", "{bozuk")
            .unwrap_err()
            .contains("geçerli JSON değil")
    );
    assert!(
        d.dogrula("yok", "{}")
            .unwrap_err()
            .contains("adlı bir araç yok")
    );
}

#[test]
fn bos_arguman_metni_bos_nesne_sayilir() {
    assert_eq!(defter().dogrula("okuyan", "  ").unwrap(), json!({}));
}

#[test]
fn inceleme_kipinde_yazan_arac_tanimlanmaz() {
    let d = defter();
    let adlar = |kip| {
        d.tanimlar(kip)
            .into_iter()
            .map(|t| t.function.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(adlar(Kip::Siki), vec!["yazan", "okuyan"]);
    assert_eq!(adlar(Kip::Inceleme), vec!["okuyan"]);
    assert!(d.bul("okuyan").is_some());
    assert!(d.bul("yok").is_none());
}

#[test]
fn yinelenen_ve_gecersiz_sema_reddedilir() {
    let r = AracKayitDefteri::yeni(vec![Arc::new(Deneme::okuyan()), Arc::new(Deneme::okuyan())]);
    assert!(matches!(r, Err(KayitHatasi::Yinelenen(ad)) if ad == "okuyan"));
    let bozuk = Deneme {
        ad: "bozuk",
        yazar: false,
        sema: json!({"type": 12}),
    };
    assert!(matches!(
        AracKayitDefteri::yeni(vec![Arc::new(bozuk)]),
        Err(KayitHatasi::GecersizSema { .. })
    ));
}

#[test]
fn muhur_islemin_kendisine_baglidir() {
    let c = CagriId::yeni();
    let a = json!({"yol": "a"});
    let h1 = Hazirlik {
        yazilan: vec![YazmaHedefi {
            yol: y("a"),
            onceki_ozet: Some("x".into()),
            baglanti_sayisi: 1,
        }],
        ..Default::default()
    };
    let m = muhur_hesapla(c, "yaz", &a, &h1);
    assert_eq!(m, muhur_hesapla(c, "yaz", &a, &h1.clone()), "deterministik");
    assert_eq!(m.0.len(), 64);

    let mut h2 = h1.clone();
    h2.yazilan[0].onceki_ozet = Some("y".into());
    assert_ne!(
        m,
        muhur_hesapla(c, "yaz", &a, &h2),
        "dosya değişince mühür değişir"
    );
    let mut h3 = h1.clone();
    h3.yazilan[0].onceki_ozet = None;
    assert_ne!(
        m,
        muhur_hesapla(c, "yaz", &a, &h3),
        "dosya yokken ve varken farklı"
    );
    let mut bagli = h1.clone();
    bagli.yazilan[0].baglanti_sayisi += 1;
    assert_ne!(
        m,
        muhur_hesapla(c, "yaz", &a, &bagli),
        "hard link sayısı değişince mühür değişir"
    );
    assert_ne!(
        m,
        muhur_hesapla(c, "yaz", &json!({"yol": "b"}), &h1),
        "argüman"
    );
    assert_ne!(
        m,
        muhur_hesapla(CagriId::yeni(), "yaz", &a, &h1),
        "başka çağrının mührü"
    );
    assert_ne!(m, muhur_hesapla(c, "oku", &a, &h1), "araç adı");

    let mut h4 = h1.clone();
    h4.aciklama = "farklı açıklama".into();
    h4.diff = Some("+x".into());
    assert_eq!(
        m,
        muhur_hesapla(c, "yaz", &a, &h4),
        "türetilmiş alanlar mühre girmez"
    );
}

#[test]
fn ozet_hex_sha256() {
    assert_eq!(
        ozet_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn io_hatasi_arac_hatasina_donusur() {
    let h: AracHatasi = std::io::Error::other("disk dolu").into();
    assert_eq!(h, AracHatasi::GirisCikis("disk dolu".into()));
    assert_eq!(
        AracHatasi::OnayEskidi.to_string(),
        "Dosya onaydan sonra değişti; işlem yapılmadı."
    );
    assert!(
        AracHatasi::PolitikaDegisti
            .to_string()
            .contains("İzin kipi")
    );
}
