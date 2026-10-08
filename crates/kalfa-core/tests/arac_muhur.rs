use std::sync::Arc;

use async_trait::async_trait;
use kalfa_core::arac::*;
use kalfa_core::izin::Kip;
use kalfa_core::kimlik::CagriId;
use kalfa_core::muhur::{akistan_ozetle, muhur_hesapla, ozet_hex};
use kalfa_core::yol::CalismaYolu;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

#[test]
fn akisli_ozet_sinirda_saklar_asimda_tamponu_birakir() {
    for veri in [b"abc".as_slice(), b"abcd", b"abcde"] {
        let sonuc = akistan_ozetle(&mut std::io::Cursor::new(veri), 4, || Ok(())).unwrap();
        assert_eq!(sonuc.ozet, ozet_hex(veri));
        assert_eq!(sonuc.bayt, veri.len() as u64);
        assert_eq!(
            sonuc.icerik.as_deref(),
            if veri.len() <= 4 { Some(veri) } else { None }
        );
    }
    let bos = akistan_ozetle(&mut std::io::empty(), 0, || Ok(())).unwrap();
    assert_eq!(bos.ozet, ozet_hex(b""));
}

#[test]
fn akisli_gorunmez_denetimi_bolunen_utf8_ve_crlf_icin_dogrudur() {
    for karakter in ["\u{200b}", "\u{e0001}"] {
        for geride in [1, 2, 3] {
            let mut veri = vec![b'a'; 64 * 1024 - geride];
            veri.extend_from_slice(karakter.as_bytes());
            let sonuc = akistan_ozetle(&mut std::io::Cursor::new(&veri), 0, || Ok(())).unwrap();
            assert!(sonuc.gorunmez_icerik);
            assert_eq!(sonuc.ozet, ozet_hex(&veri));
        }
    }
    let mut veri = vec![b'a'; 64 * 1024 - 1];
    veri.extend_from_slice(b"\r\nnormal\r\n");
    let sonuc = akistan_ozetle(&mut std::io::Cursor::new(&veri), 0, || Ok(())).unwrap();
    assert!(
        !sonuc.gorunmez_icerik,
        "parça sonunda CRLF görünmez karakter değildir"
    );
    veri.push(b'\r');
    assert!(
        akistan_ozetle(&mut std::io::Cursor::new(veri), 0, || Ok(()))
            .unwrap()
            .gorunmez_icerik
    );
}

#[test]
fn diff_olmadan_gorunmez_icerik_bilgisi_muhre_baglanir() {
    let cagri = CagriId::yeni();
    let mut h = Hazirlik::default();
    let once = muhur_hesapla(cagri, "dosya_yaz", &json!({}), &h);
    h.gorunmez_icerik = true;
    assert_ne!(once, muhur_hesapla(cagri, "dosya_yaz", &json!({}), &h));
}

#[test]
fn akisli_ozet_buyuk_kaynaktan_sabit_parcalar_okur_ve_iptali_isler() {
    struct Kaynak {
        kalan: usize,
        okunan: usize,
    }
    impl std::io::Read for Kaynak {
        fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
            assert!(b.len() <= 64 * 1024);
            let n = b.len().min(self.kalan);
            b[..n].fill(b'a');
            self.kalan -= n;
            self.okunan += n;
            Ok(n)
        }
    }
    let mut kaynak = Kaynak {
        kalan: 16 * 1024 * 1024,
        okunan: 0,
    };
    let sonuc = akistan_ozetle(&mut kaynak, 8 * 1024 * 1024, || Ok(())).unwrap();
    assert!(sonuc.icerik.is_none());
    assert_eq!(sonuc.bayt, 16 * 1024 * 1024);
    let mut denetim = 0;
    kaynak.kalan = 16 * 1024 * 1024;
    kaynak.okunan = 0;
    let hata = akistan_ozetle(&mut kaynak, 0, || {
        denetim += 1;
        if denetim == 4 {
            Err(std::io::ErrorKind::Interrupted.into())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(hata.kind(), std::io::ErrorKind::Interrupted);
    assert_eq!(kaynak.okunan, 3 * 64 * 1024);
}

#[test]
fn okuma_baglanti_sayisi_muhre_baglanir() {
    let cagri = CagriId::yeni();
    let mut h = Hazirlik {
        okunan: vec![y("a.txt")],
        okuma_baglanti_sayilari: vec![(y("a.txt"), 1)],
        ..Default::default()
    };
    let ilk = muhur_hesapla(cagri, "dosya_oku", &json!({"yol":"a.txt"}), &h);
    h.okuma_baglanti_sayilari[0].1 = 2;
    assert_ne!(
        ilk,
        muhur_hesapla(cagri, "dosya_oku", &json!({"yol":"a.txt"}), &h)
    );
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
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: false,
            ag: None,
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
        yazilan: vec![YazmaHedefi::yeni(y("a"), Some("x".into()), 1)],
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

/// Kabukta mühür komuta, dizine, ağa, yalıtım durumuna ve profil özetine
/// bağlıdır (aşama 2 tasarımı §3.3).
#[test]
fn kabuk_muhru_tarifin_her_alanina_baglidir() {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    let c = CagriId::yeni();
    let a = json!({"program": "cargo", "argumanlar": ["test"]});
    let tarif = KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        program: None,
        dizin: ".".into(),
        ag: false,
        ag_hostlari: vec![],
        yalitim: YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "p1".into(),
        },
        ilk_cagri: false,
        alan_hazirligi: false,
    };
    let h = |t: KabukTarifi| Hazirlik {
        tum_calisma_alani: true,
        kabuk: Some(t),
        ..Default::default()
    };
    let temel = muhur_hesapla(c, "kabuk", &a, &h(tarif.clone()));
    assert_eq!(temel, muhur_hesapla(c, "kabuk", &a, &h(tarif.clone())));
    let degisikler = [
        KabukTarifi {
            dizin: "src".into(),
            ..tarif.clone()
        },
        KabukTarifi {
            ag: true,
            ..tarif.clone()
        },
        KabukTarifi {
            ag_hostlari: vec!["github.com:443".into()],
            ..tarif.clone()
        },
        KabukTarifi {
            yalitim: YalitimOzeti::Yalitimli {
                arka_uc: "bwrap".into(),
                profil: "p2".into(),
            },
            ..tarif.clone()
        },
        KabukTarifi {
            yalitim: YalitimOzeti::Yalitimsiz {
                talimat: "yok".into(),
            },
            ..tarif.clone()
        },
        KabukTarifi {
            ilk_cagri: true,
            ..tarif.clone()
        },
        KabukTarifi {
            alan_hazirligi: true,
            ..tarif.clone()
        },
    ];
    for d in degisikler {
        assert_ne!(temel, muhur_hesapla(c, "kabuk", &a, &h(d.clone())), "{d:?}");
    }
}

/// İlk çağrı ve alan hazırlığı tek seferliktir: onaylanan tarifte olup taze
/// tarifte tükenmiş koşul onaylı sayılır; onaylanmayan koşul belirirse
/// düzeltme onu silmez ve mühür tutmaz. Başka hiçbir alan düzeltilmez.
#[test]
fn kabukta_tukenen_tek_seferlik_kosul_onayla_kapsanir() {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    let tarif = |ilk_cagri, alan_hazirligi| KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        program: None,
        dizin: ".".into(),
        ag: false,
        ag_hostlari: vec![],
        yalitim: YalitimOzeti::Yalitimli {
            arka_uc: "appcontainer".into(),
            profil: "p1".into(),
        },
        ilk_cagri,
        alan_hazirligi,
    };
    for onaylanan_ilk in [false, true] {
        for onaylanan_hazirlik in [false, true] {
            let onaylanan = tarif(onaylanan_ilk, onaylanan_hazirlik);
            for taze_ilk in [false, true] {
                for taze_hazirlik in [false, true] {
                    let duzeltilmis = tarif(taze_ilk, taze_hazirlik).onayla_kapsanan(&onaylanan);
                    let kapsanir =
                        (!taze_ilk || onaylanan_ilk) && (!taze_hazirlik || onaylanan_hazirlik);
                    assert_eq!(
                        duzeltilmis == onaylanan,
                        kapsanir,
                        "onaylanan ({onaylanan_ilk}, {onaylanan_hazirlik}),                          taze ({taze_ilk}, {taze_hazirlik})"
                    );
                }
            }
        }
    }
    // Öteki alanlar olduğu gibi kalır.
    let farkli = KabukTarifi {
        dizin: "src".into(),
        ..tarif(false, false)
    };
    assert_ne!(farkli.onayla_kapsanan(&tarif(true, true)).dizin, ".");

    let onaylanan = tarif(false, false);
    assert!(
        onaylanan
            .degisiklik_nedeni(&tarif(false, true))
            .contains("hazırlığı onaydan sonra geri alındı")
    );
    assert!(
        onaylanan
            .degisiklik_nedeni(&KabukTarifi {
                yalitim: YalitimOzeti::Yalitimsiz {
                    talimat: "yok".into()
                },
                ..tarif(false, false)
            })
            .contains("Yalıtım")
    );
    assert!(
        AracHatasi::KomutKosuluDegisti(onaylanan.degisiklik_nedeni(&tarif(true, false)).into())
            .to_string()
            .ends_with("; komut çalıştırılmadı. Gerekirse yeniden dene.")
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

#[test]
fn muhur_yeni_alanlardan_etkilenmez() {
    let c = CagriId::yeni();
    let a = json!({"yol": "a"});
    let yalin = Hazirlik {
        yazilan: vec![YazmaHedefi::yeni(y("a"), Some("x".into()), 1)],
        ..Default::default()
    };
    let mut dolu = yalin.clone();
    let hedef = &mut dolu.yazilan[0];
    hedef.onceki_icerik = Some(Arc::from(&b"eski"[..]));
    hedef.onceki_bayt = 4;
    hedef.sonraki_ozet = Some(ozet_hex(b"yeni"));
    hedef.olusacak_dizinler = vec![y("u")];
    assert_eq!(
        muhur_hesapla(c, "yaz", &a, &yalin),
        muhur_hesapla(c, "yaz", &a, &dolu),
        "önceki içeriği onceki_ozet, sonrakini argümanlar zaten bağlar"
    );
    let mut buyuk = yalin.clone();
    buyuk.yazilan[0].buyuk = true;
    assert_eq!(
        muhur_hesapla(c, "yaz", &a, &yalin),
        muhur_hesapla(c, "yaz", &a, &buyuk)
    );
}

#[test]
fn ag_hostlari_muhre_girer() {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
    let c = CagriId::yeni();
    let a = json!({"program": "cargo", "argumanlar": ["test"]});
    let tarif1 = KabukTarifi {
        komut: "cargo test".into(),
        bicim: KabukBicimi::Program,
        program: None,
        dizin: ".".into(),
        ag: true,
        ag_hostlari: vec![],
        yalitim: YalitimOzeti::Yalitimli {
            arka_uc: "bwrap".into(),
            profil: "p1".into(),
        },
        ilk_cagri: false,
        alan_hazirligi: false,
    };
    let mut tarif2 = tarif1.clone();
    tarif2.ag_hostlari = vec!["github.com:443".into()];

    let h = |t: KabukTarifi| Hazirlik {
        tum_calisma_alani: true,
        kabuk: Some(t),
        ..Default::default()
    };

    assert_ne!(
        muhur_hesapla(c, "kabuk", &a, &h(tarif1)),
        muhur_hesapla(c, "kabuk", &a, &h(tarif2)),
        "tek host eklemek mührü değiştirir"
    );
}

/// Eklenti aracının onayı eklentinin özetine, globlarına ve uçlarına
/// bağlıdır (aşama 4 tasarımı §8.3): onaydan sonra güncellenen eklentinin
/// çağrısı çalışmaz.
#[test]
fn eklenti_tarifi_muhre_baglanir() {
    let cagri = CagriId::yeni();
    let tarif = EklentiTarifi {
        ad: "kalfa-rag".into(),
        surum: "0.1.0".into(),
        ozet: "a".repeat(64),
        globlar: vec!["**/*.rs".into()],
        evren_uclari: vec!["embeddings".into()],
    };
    let h = Hazirlik {
        eklenti: Some(tarif.clone()),
        kilitsiz: true,
        ..Default::default()
    };
    let m = muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &h);
    let degisik = |t: EklentiTarifi| {
        let h = Hazirlik {
            eklenti: Some(t),
            ..h.clone()
        };
        muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &h)
    };
    assert_ne!(
        degisik(EklentiTarifi {
            ozet: "b".repeat(64),
            ..tarif.clone()
        }),
        m
    );
    assert_ne!(
        degisik(EklentiTarifi {
            surum: "0.2.0".into(),
            ..tarif.clone()
        }),
        m
    );
    assert_ne!(
        degisik(EklentiTarifi {
            globlar: vec!["**/*".into()],
            ..tarif.clone()
        }),
        m
    );
    assert_ne!(
        degisik(EklentiTarifi {
            evren_uclari: vec![],
            ..tarif.clone()
        }),
        m
    );
    assert_eq!(degisik(tarif.clone()), m);
    let kilitli = Hazirlik {
        kilitsiz: false,
        ..h.clone()
    };
    assert_ne!(
        muhur_hesapla(cagri, "kalfa_rag__ara", &json!({}), &kilitli),
        m
    );
}
