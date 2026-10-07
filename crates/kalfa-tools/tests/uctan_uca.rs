use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::AracHatasi;
use kalfa_core::arac::AracKayitDefteri;
use kalfa_core::izin::{GIZLI_UYARISI, KORUMALI_UYARISI, Kip, Siniflandirici};
use kalfa_core::kayit::OturumKaydi;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{OturumAyarlari, OturumBaglami, baslat};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici};
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_core::test_destek::*;
use kalfa_evren::tipler::Role;
use kalfa_evren::{ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
use kalfa_tools::{CalismaAlani, varsayilan_araclar};
use serde_json::json;
use tempfile::TempDir;

struct Ortam {
    alan: TempDir,
    kayitlar: TempDir,
    siniflandirici: Arc<Siniflandirici>,
}

/// Oturum kaydı çalışma alanının dışında tutulur (spec §6.6).
fn ortam() -> Ortam {
    let alan = tempfile::tempdir().unwrap();
    let kayitlar = tempfile::tempdir().unwrap();
    let siniflandirici = Arc::new(Siniflandirici::yeni(alan.path(), &[]).unwrap());
    Ortam {
        alan,
        kayitlar,
        siniflandirici,
    }
}

fn gercek_araclarla(o: &Ortam, s: &Arc<SahteSaglayici>, kip: Kip, kayitli: bool) -> Surucu {
    let calisma_alani = CalismaAlani::ac(o.siniflandirici.clone()).unwrap();
    let mut ayar = OturumAyarlari::yeni("glm-5.3", o.alan.path());
    ayar.kip = kip;
    if kayitli {
        ayar.kayit_yolu = Some(o.kayitlar.path().join("oturum.jsonl"));
    }
    test_oturumu(
        test_baglami(
            s.clone(),
            varsayilan_araclar(calisma_alani),
            o.siniflandirici.clone(),
        ),
        ayar,
    )
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn gizli_dosyanin_normal_sabit_baglantisi_her_kipte_okuma_onayi_ister() {
    for kip in [Kip::Inceleme, Kip::Siki, Kip::Proje, Kip::Serbest] {
        for onayla in [false, true] {
            let o = ortam();
            std::fs::write(o.alan.path().join(".env"), "SIR=yalniz_onayla\n").unwrap();
            std::fs::hard_link(o.alan.path().join(".env"), o.alan.path().join("normal.txt"))
                .unwrap();
            let s = SahteSaglayici::yeni(vec![
                SahteYanit::arac("dosya_oku", json!({"yol": "normal.txt"})),
                SahteYanit::metin("bitti"),
            ]);
            let mut d = gercek_araclarla(&o, &s, kip, false);
            d.gonder(mesaj("oku")).await;
            let ogeler = onay_bekle(&mut d).await;
            assert_eq!(s.istekler().len(), 1);
            assert!(ogeler[0].uyarilar.iter().any(|u| u == GIZLI_UYARISI));
            assert!(ogeler[0].diff.is_none());
            d.gonder(Komut::AracOnayi {
                cagri: ogeler[0].cagri,
                karar: if onayla {
                    OnayKarari::Onayla
                } else {
                    OnayKarari::Reddet { gerekce: None }
                },
                muhur: ogeler[0].muhur.clone(),
                kisayol: false,
            })
            .await;
            assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
            assert_eq!(
                arac_yanitlari(&s, 1)[0].contains("SIR=yalniz_onayla"),
                onayla
            );
        }
    }
}

#[tokio::test]
async fn diff_gosterilmeyen_gorunmez_icerik_yine_onay_ister() {
    for kip in [Kip::Proje, Kip::Serbest] {
        for kayitli in [false, true] {
            for duzenle in [false, true] {
                let o = ortam();
                let (ad, args) = if duzenle {
                    std::fs::write(o.alan.path().join("a.txt"), "a\n".repeat(4097)).unwrap();
                    (
                        "dosya_duzenle",
                        json!({"yol":"a.txt", "eski_metin":"a", "yeni_metin":"\u{200b}", "tumu":true}),
                    )
                } else {
                    (
                        "dosya_yaz",
                        json!({"yol":"a.txt", "icerik":"\u{200b}\n".repeat(4097)}),
                    )
                };
                let s = SahteSaglayici::yeni(vec![
                    SahteYanit::arac(ad, args),
                    SahteYanit::metin("bitti"),
                ]);
                let mut d = gercek_araclarla(&o, &s, kip, kayitli);
                d.gonder(mesaj("yaz")).await;
                let ogeler = onay_bekle(&mut d).await;
                assert!(ogeler[0].diff.is_none());
                assert!(
                    ogeler[0]
                        .uyarilar
                        .iter()
                        .any(|u| u == kalfa_core::gorunum::GORUNMEZ_UYARISI)
                );
                d.gonder(Komut::AracOnayi {
                    cagri: ogeler[0].cagri,
                    karar: OnayKarari::Reddet { gerekce: None },
                    muhur: ogeler[0].muhur.clone(),
                    kisayol: false,
                })
                .await;
                assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
                if duzenle {
                    assert_eq!(
                        std::fs::read_to_string(o.alan.path().join("a.txt")).unwrap(),
                        "a\n".repeat(4097)
                    );
                } else {
                    assert!(!o.alan.path().join("a.txt").exists());
                }
            }
        }
    }
}

fn arac_yanitlari(s: &SahteSaglayici, istek: usize) -> Vec<String> {
    s.istekler()[istek]
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .map(|m| m.content.clone().unwrap_or_default())
        .collect()
}

fn arac_cagrilari_tek_yanitli(s: &SahteSaglayici) {
    for (istek_no, istek) in s.istekler().iter().enumerate() {
        let cagrilar: Vec<&str> = istek
            .messages
            .iter()
            .flat_map(|m| m.tool_calls.iter().flatten())
            .map(|tc| tc.id.as_str())
            .collect();
        let yanitlar: Vec<&str> = istek
            .messages
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert_eq!(
            cagrilar.len(),
            yanitlar.len(),
            "istek {istek_no}: her araç çağrısı için bir araç yanıtı olmalı"
        );
        for cagri in cagrilar {
            assert_eq!(
                yanitlar.iter().filter(|yanit| **yanit == cagri).count(),
                1,
                "istek {istek_no}: {cagri} çağrısı tam olarak bir araç yanıtına bağlanmalı"
            );
        }
    }
}

#[tokio::test]
async fn gizli_duzenlemenin_icerik_hatalari_onaydan_once_modele_gitmez() {
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        for (veri, eski) in [
            (b"SIR=iki iki\n".as_slice(), "yok"),
            (b"SIR=iki iki\n".as_slice(), "iki"),
            (b"SIR=\0".as_slice(), "yok"),
            (&[0xff][..], "yok"),
        ] {
            let o = ortam();
            std::fs::write(o.alan.path().join(".env"), veri).unwrap();
            let s = SahteSaglayici::yeni(vec![
                SahteYanit::arac(
                    "dosya_duzenle",
                    json!({"yol": ".env", "eski_metin": eski, "yeni_metin": "yeni"}),
                ),
                SahteYanit::metin("bitti"),
            ]);
            let mut d = gercek_araclarla(&o, &s, kip, false);
            d.gonder(mesaj("düzenle")).await;
            let ogeler = onay_bekle(&mut d).await;
            assert_eq!(
                s.istekler().len(),
                1,
                "{kip:?}: hazırlık sonucu gönderilemez"
            );
            assert!(ogeler[0].uyarilar.iter().any(|u| u == GIZLI_UYARISI));
            d.gonder(Komut::AracOnayi {
                cagri: ogeler[0].cagri,
                karar: OnayKarari::Reddet { gerekce: None },
                muhur: ogeler[0].muhur.clone(),
                kisayol: false,
            })
            .await;
            assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
            let yanitlar = arac_yanitlari(&s, 1);
            assert!(yanitlar[0].contains("[kullanıcı reddetti]"));
            for yasak in ["bulunamadı", "kez geçiyor", "ikili", "UTF-8", "SIR="] {
                assert!(!yanitlar[0].contains(yasak), "{yanitlar:?}");
            }
            assert_eq!(std::fs::read(o.alan.path().join(".env")).unwrap(), veri);
        }
    }
}

#[tokio::test]
async fn onayli_gizli_duzenleme_calisir_ama_degisen_icerigi_aciklamaz() {
    for degistir in [false, true] {
        let o = ortam();
        let dosya = o.alan.path().join(".env");
        std::fs::write(&dosya, "SIR=eski\n").unwrap();
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac(
                "dosya_duzenle",
                json!({"yol": ".env", "eski_metin": "eski", "yeni_metin": "yeni"}),
            ),
            SahteYanit::metin("bitti"),
        ]);
        let mut d = gercek_araclarla(&o, &s, Kip::Proje, false);
        d.gonder(mesaj("düzenle")).await;
        let ogeler = onay_bekle(&mut d).await;
        assert_eq!(ogeler[0].hedefler, [".env"]);
        assert!(ogeler[0].diff.as_deref().unwrap().contains("+SIR=yeni"));
        if degistir {
            std::fs::write(&dosya, [0xff]).unwrap();
        }
        d.gonder(Komut::AracOnayi {
            cagri: ogeler[0].cagri,
            karar: OnayKarari::Onayla,
            muhur: ogeler[0].muhur.clone(),
            kisayol: false,
        })
        .await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        let yanitlar = arac_yanitlari(&s, 1);
        if degistir {
            assert!(yanitlar[0].contains("Dosya onaydan sonra değişti"));
            assert!(!yanitlar[0].contains("UTF-8"));
            assert_eq!(std::fs::read(&dosya).unwrap(), [0xff]);
        } else {
            assert_eq!(std::fs::read_to_string(&dosya).unwrap(), "SIR=yeni\n");
        }
    }
}

/// Boş düzenleme de bir okuma yetkisidir: eşleşme sayısı gizli içeriği anlatır.
#[tokio::test]
async fn gizli_bos_duzenleme_her_kipte_onay_ister() {
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        for tumu in [false, true] {
            for onayla in [false, true] {
                let o = ortam();
                std::fs::write(o.alan.path().join(".env"), "SIR=iki iki\n").unwrap();
                let s = SahteSaglayici::yeni(vec![
                    SahteYanit::arac(
                        "dosya_duzenle",
                        json!({
                        "yol": ".env", "eski_metin": "iki", "yeni_metin": "iki", "tumu": tumu
                        }),
                    ),
                    SahteYanit::metin("bitti"),
                ]);
                let mut d = gercek_araclarla(&o, &s, kip, false);
                d.gonder(mesaj("dene")).await;
                let ogeler = onay_bekle(&mut d).await;
                assert_eq!(s.istekler().len(), 1);
                assert!(ogeler[0].uyarilar.iter().any(|u| u == GIZLI_UYARISI));
                assert!(!ogeler[0].aciklama.contains("2 kez"));
                d.gonder(Komut::AracOnayi {
                    cagri: ogeler[0].cagri,
                    karar: if onayla {
                        OnayKarari::Onayla
                    } else {
                        OnayKarari::Reddet { gerekce: None }
                    },
                    muhur: ogeler[0].muhur.clone(),
                    kisayol: false,
                })
                .await;
                assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
                let yanit = &arac_yanitlari(&s, 1)[0];
                assert_eq!(yanit.contains("2 kez"), onayla && !tumu, "{yanit}");
                if onayla && tumu {
                    assert!(!yanit.contains("[hata]"), "{yanit}");
                }
                assert_eq!(
                    std::fs::read_to_string(o.alan.path().join(".env")).unwrap(),
                    "SIR=iki iki\n"
                );
            }
        }
    }
}

/// İçerik okuyan düzenleme ile kör yazma farklı yetkilerdir (K8).
#[tokio::test]
async fn gizli_kor_yazma_proje_ve_serbest_kipte_okuma_onayi_istemez() {
    for kip in [Kip::Proje, Kip::Serbest] {
        let o = ortam();
        std::fs::write(o.alan.path().join(".env"), "BENZERSIZ_ESKI_SIR=1\n").unwrap();
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac("dosya_yaz", json!({"yol": ".env", "icerik": "YENI=2\n"})),
            SahteYanit::metin("bitti"),
        ]);
        let mut d = gercek_araclarla(&o, &s, kip, false);
        d.gonder(mesaj("yaz")).await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        assert!(
            !d.gecmis
                .iter()
                .any(|o| matches!(o.yuk, Olay::OnayGerekiyor { .. }))
        );
        assert!(!arac_yanitlari(&s, 1)[0].contains("BENZERSIZ_ESKI_SIR"));
        assert_eq!(
            std::fs::read_to_string(o.alan.path().join(".env")).unwrap(),
            "YENI=2\n"
        );
    }
}

#[tokio::test]
async fn ui_siz_tam_dongu_gercek_araclarla() {
    let o = ortam();
    let dosya = o.alan.path().join("selam.txt");
    std::fs::write(&dosya, "merhaba\n").unwrap();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac("dosya_oku", json!({"yol": "selam.txt"})),
        SahteYanit::arac(
            "dosya_duzenle",
            json!({"yol": "selam.txt", "eski_metin": "merhaba", "yeni_metin": "merhaba dünya"}),
        ),
        SahteYanit::metin("Dosyayı güncelledim."),
    ]);
    let mut d = gercek_araclarla(&o, &s, Kip::Siki, true);
    d.gonder(mesaj("selam.txt'yi düzelt")).await;

    // Okuma Sıkı kipte de serbest; düzenleme onay ister.
    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 1);
    assert_eq!(ogeler[0].arac, "dosya_duzenle");
    assert!(
        ogeler[0]
            .diff
            .as_deref()
            .unwrap()
            .contains("+merhaba dünya")
    );
    assert_eq!(
        std::fs::read_to_string(&dosya).unwrap(),
        "merhaba\n",
        "onaydan önce dosya değişmez"
    );

    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(std::fs::read_to_string(&dosya).unwrap(), "merhaba dünya\n");
    assert_eq!(d.metin(), "Dosyayı güncelledim.");

    // Model, okuma sonucunu kaynak etiketiyle gördü.
    assert!(
        arac_yanitlari(&s, 1)[0]
            .starts_with("<arac_sonucu kaynak=\"dosya:selam.txt:1-1\" sinir=\"")
    );

    // Kayıttan kurtarma aynı bağlamı verir.
    let durum =
        kurtar(&OturumKaydi::oku(&o.kayitlar.path().join("oturum.jsonl")).unwrap()).unwrap();
    assert!(durum.bilinmeyenler.is_empty() && durum.tamamlayicilar.is_empty());
    assert_eq!(
        durum.mesajlar.len(),
        6,
        "kullanıcı, 2×(asistan + araç), asistan"
    );
    arac_cagrilari_tek_yanitli(&s);
}

#[tokio::test]
async fn disari_cikma_ve_gizli_dosya_uctan_uca() {
    let o = ortam();
    std::fs::write(o.alan.path().join(".env"), "SIR=1\n").unwrap();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("dosya_oku", json!({"yol": "../dis.txt"})),
            ("dosya_oku", json!({"yol": ".env"})),
        ]),
        SahteYanit::metin("anlaşıldı"),
    ]);
    let mut d = gercek_araclarla(&o, &s, Kip::Serbest, false);
    d.gonder(mesaj("oku")).await;

    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(
        ogeler.len(),
        1,
        "dışarı çıkan yol planlamada reddedilir; yalnızca gizli dosya onaya gelir"
    );
    assert_eq!(ogeler[0].hedefler, vec![".env"]);
    assert_eq!(ogeler[0].uyarilar, vec![GIZLI_UYARISI]);
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Reddet { gerekce: None },
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);

    let yanitlar = arac_yanitlari(&s, 1);
    assert_eq!(yanitlar.len(), 2);
    assert!(
        yanitlar.iter().any(|y| y.contains("üst dizine çıkılamaz")),
        "{yanitlar:?}"
    );
    assert!(
        yanitlar
            .iter()
            .any(|y| y.contains("[kullanıcı reddetti] gerekçe belirtilmedi"))
    );
    assert!(
        !yanitlar.iter().any(|y| y.contains("SIR=1")),
        "reddedilen gizli dosya modele gitmez"
    );
    arac_cagrilari_tek_yanitli(&s);
}

#[tokio::test]
async fn gizli_dizin_cocugunun_okumasi_her_kipte_onay_ister() {
    for kip in [Kip::Inceleme, Kip::Siki, Kip::Proje, Kip::Serbest] {
        let o = ortam();
        std::fs::create_dir(o.alan.path().join("credentials")).unwrap();
        std::fs::write(
            o.alan.path().join("credentials/token.txt"),
            "BENZERSIZ_SIR=123",
        )
        .unwrap();
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac("dosya_oku", json!({"yol":"credentials/token.txt"})),
            SahteYanit::metin("bitti"),
        ]);
        let mut d = gercek_araclarla(&o, &s, kip, false);
        d.gonder(mesaj("oku")).await;
        let ogeler = onay_bekle(&mut d).await;
        assert_eq!(s.istekler().len(), 1);
        assert!(ogeler[0].uyarilar.iter().any(|u| u == GIZLI_UYARISI));
        d.gonder(Komut::AracOnayi {
            cagri: ogeler[0].cagri,
            karar: OnayKarari::Reddet { gerekce: None },
            muhur: ogeler[0].muhur.clone(),
            kisayol: false,
        })
        .await;
        assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
        assert!(!arac_yanitlari(&s, 1)[0].contains("BENZERSIZ_SIR"));
    }
}

#[tokio::test]
async fn baglanti_takma_adlari_serbest_kipte_bile_yetki_atlatamaz() {
    let o = ortam();
    std::fs::write(o.alan.path().join(".env"), "BENZERSIZ_SIR=123").unwrap();
    std::fs::create_dir_all(o.alan.path().join(".git/hooks")).unwrap();
    std::fs::write(o.alan.path().join(".git/hooks/pre-commit"), "eski").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(".env", o.alan.path().join("normal.txt")).unwrap();
        std::os::unix::fs::symlink(".git", o.alan.path().join("normal")).unwrap();
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(".env", o.alan.path().join("normal.txt")).unwrap();
        std::os::windows::fs::symlink_dir(".git", o.alan.path().join("normal")).unwrap();
    }
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("dosya_oku", json!({"yol": "normal.txt"})),
            (
                "dosya_yaz",
                json!({"yol": "normal/hooks/pre-commit", "icerik": "yeni"}),
            ),
        ]),
        SahteYanit::metin("anlaşıldı"),
    ]);
    let mut d = gercek_araclarla(&o, &s, Kip::Serbest, false);
    d.gonder(mesaj("dene")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let yanitlar = arac_yanitlari(&s, 1);
    assert_eq!(yanitlar.len(), 2);
    assert!(yanitlar.iter().all(|y| y.contains("[hata]")));
    assert!(!yanitlar.iter().any(|y| y.contains("BENZERSIZ_SIR")));
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join(".git/hooks/pre-commit")).unwrap(),
        "eski"
    );
    arac_cagrilari_tek_yanitli(&s);
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_evren_ile_dosya_okuyup_yanitlar() {
    let o = ortam();
    std::fs::write(o.alan.path().join("not.txt"), "Proje kodu: ELMA-4721\n").unwrap();
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    let istemci = EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap();
    let calisma_alani = CalismaAlani::ac(o.siniflandirici.clone()).unwrap();
    let saglayici: Arc<dyn Saglayici> = Arc::new(EvrenSaglayici::yeni(istemci));
    let baglam = OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(varsayilan_araclar(calisma_alani)).unwrap()),
        siniflandirici: o.siniflandirici.clone(),
        kilitler: Arc::new(kalfa_core::kilit::KaynakKilitleri::yeni()),
        geri_yukleyici: None,
        denetci: Arc::new(kalfa_core::denetci::Yok),
        denetci_kipi: None,
    };
    let mut ayar = OturumAyarlari::yeni("qwen3.8-flash-next", o.alan.path());
    ayar.kip = Kip::Proje;
    let (tutamac, olaylar) = baslat(ayar, baglam).unwrap();
    let mut d = Surucu::yeni(tutamac, olaylar).zaman_asimi(Duration::from_secs(180));
    d.gonder(mesaj(
        "not.txt dosyasındaki proje kodu nedir? dosya_oku aracını kullan ve yalnızca kodu yaz.",
    ))
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(d.gecmis.iter().any(|z| {
        matches!(
            &z.yuk,
            Olay::AracSonucu {
                arac,
                durum: AracDurumu::Basarili,
                ..
            } if arac == "dosya_oku"
        )
    }));
    assert!(d.metin().contains("4721"), "yanıt: {}", d.metin());
}

// Aşama 3a: gerçek araçlarla yakalama (aşama 3 tasarımı §5.3).

#[tokio::test]
async fn gercek_araclarla_onceki_hal_depoda_ve_kayitta() {
    use kalfa_core::geri_alma::{OncekiHal, OncekiHalDeposu};
    use kalfa_core::kayit::KayitOgesi;
    use kalfa_core::muhur::ozet_hex;

    let o = ortam();
    std::fs::write(o.alan.path().join("x.txt"), "eski\n").unwrap();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("dosya_yaz", json!({"yol": "x.txt", "icerik": "yeni\n"})),
            ("dosya_yaz", json!({"yol": "a/b/y.txt", "icerik": "y\n"})),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let depo_dizini = o.kayitlar.path().join("geri-alma/oturum");
    let calisma_alani = CalismaAlani::ac(o.siniflandirici.clone()).unwrap();
    let mut ayar = OturumAyarlari::yeni("glm-5.3", o.alan.path());
    ayar.kip = Kip::Proje;
    ayar.kayit_yolu = Some(o.kayitlar.path().join("oturum.jsonl"));
    ayar.geri_alma_dizini = Some(depo_dizini.clone());
    let mut d = test_oturumu(
        test_baglami(
            s.clone(),
            varsayilan_araclar(calisma_alani),
            o.siniflandirici.clone(),
        ),
        ayar,
    );
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;

    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("x.txt")).unwrap(),
        "yeni\n"
    );
    let kayit = OturumKaydi::oku(&o.kayitlar.path().join("oturum.jsonl")).unwrap();
    let mut yedekler: Vec<_> = kayit
        .into_iter()
        .flat_map(|z| match z.yuk {
            KayitOgesi::AracBasladi { yedekler, .. } => yedekler,
            _ => Vec::new(),
        })
        .collect();
    yedekler.sort_by(|a, b| a.yol.cmp(&b.yol));
    assert_eq!(yedekler.len(), 2, "{yedekler:?}");

    let yeni = &yedekler[0];
    assert_eq!(yeni.yol.as_str(), "a/b/y.txt");
    assert_eq!(yeni.onceki, OncekiHal::Yoktu);
    let dizinler: Vec<&str> = yeni.olusan_dizinler.iter().map(|y| y.as_str()).collect();
    assert_eq!(dizinler, ["a", "a/b"]);

    let degisen = &yedekler[1];
    let eski = ozet_hex(b"eski\n");
    assert_eq!(degisen.onceki, OncekiHal::Saklandi { ozet: eski.clone() });
    assert_eq!(degisen.sonraki, Some(ozet_hex(b"yeni\n")));
    assert_eq!((degisen.eklenen, degisen.silinen), (1, 1));
    let depo = OncekiHalDeposu::ac(&depo_dizini).unwrap();
    assert_eq!(depo.oku(&eski).unwrap(), b"eski\n");
}

/// Aşama 3a: Kalfa kapatılıp açıldıktan sonra tur gerçek araçlarla geri
/// alınır (aşama 3 tasarımı §3 madde 1).
#[tokio::test]
async fn kayittan_acilan_oturumda_geri_alma() {
    use kalfa_core::geri_alma::{GeriAlmaEylemi, GeriYukleyici};
    use kalfa_core::oturum::devam_et;

    let o = ortam();
    std::fs::write(o.alan.path().join("x.txt"), "eski\n").unwrap();
    let kayit = o.kayitlar.path().join("oturum.jsonl");
    let depo = o.kayitlar.path().join("geri-alma/oturum");
    let ayar = |kip| {
        let mut a = OturumAyarlari::yeni("glm-5.3", o.alan.path());
        a.kip = kip;
        a.kayit_yolu = Some(kayit.clone());
        a.geri_alma_dizini = Some(depo.clone());
        a
    };
    let baglam = |s: Arc<SahteSaglayici>| {
        let alan = CalismaAlani::ac(o.siniflandirici.clone()).unwrap();
        let mut b = test_baglami(
            s,
            varsayilan_araclar(alan.clone()),
            o.siniflandirici.clone(),
        );
        b.geri_yukleyici = Some(alan as Arc<dyn GeriYukleyici>);
        b
    };

    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            ("dosya_yaz", json!({"yol": "x.txt", "icerik": "yeni\n"})),
            ("dosya_yaz", json!({"yol": "a/b/y.txt", "icerik": "y\n"})),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = test_oturumu(baglam(s), ayar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    let tur = d
        .gecmis
        .iter()
        .find(|z| matches!(z.yuk, Olay::TurBasladi))
        .and_then(|z| z.tur)
        .unwrap();
    d.kapat().await;

    let (t, alici, _) = devam_et(
        &kayit,
        ayar(Kip::Proje),
        baglam(SahteSaglayici::yeni(vec![])),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, alici);
    d.gonder(Komut::GeriAlmaOnizle { tur }).await;
    let (plan, muhur) = match d
        .bekle(|o| matches!(o, Olay::GeriAlmaPlani { .. }))
        .await
        .yuk
    {
        Olay::GeriAlmaPlani { plan, muhur, .. } => (plan, muhur),
        _ => unreachable!(),
    };
    let eylemler: Vec<(&str, &GeriAlmaEylemi)> = plan
        .ogeler
        .iter()
        .map(|o| (o.yol.as_str(), &o.eylem))
        .collect();
    assert!(
        matches!(
            eylemler.as_slice(),
            [
                ("a/b/y.txt", GeriAlmaEylemi::Sil),
                ("x.txt", GeriAlmaEylemi::GeriYukle { .. })
            ]
        ),
        "{eylemler:?}"
    );
    d.gonder(Komut::TuruGeriAl { tur, muhur }).await;
    d.bekle(|o| matches!(o, Olay::TurGeriAlindi { .. })).await;
    d.kapat().await;

    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("x.txt")).unwrap(),
        "eski\n"
    );
    assert!(!o.alan.path().join("a/b/y.txt").exists());
    assert!(
        !o.alan.path().join("a").exists(),
        "turun oluşturduğu boş dizinler de kaldırılır"
    );
}

fn git(kok: &Path, args: &[&str]) {
    let durum = Command::new("git")
        .arg("-C")
        .arg(kok)
        .args(args)
        .status()
        .expect("git kurulu olmalı");
    assert!(durum.success(), "git {args:?}");
}

/// Git korumaları bir yanıtın planlamasında bir kez çözülür (ilk yazan
/// hazırlıkta); aynı yanıttaki sonraki yazmalar da o çözümle kanca dizinini
/// korumalı görür.
#[tokio::test]
async fn ayni_yanittaki_yazmalar_tek_cozumle_de_kanca_dizinini_korur() {
    // `.husky` değil: o zaten yapılandırma sınıfındadır ve onay isterdi.
    let o = ortam();
    git(o.alan.path(), &["init", "-q"]);
    git(o.alan.path(), &["config", "core.hooksPath", "kancalar"]);
    std::fs::create_dir(o.alan.path().join("kancalar")).unwrap();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            (
                "dosya_yaz",
                json!({"yol": "a.txt", "icerik": "a
"}),
            ),
            (
                "dosya_yaz",
                json!({"yol": "kancalar/pre-commit", "icerik": "echo
"}),
            ),
            (
                "dosya_yaz",
                json!({"yol": "b.txt", "icerik": "b
"}),
            ),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = gercek_araclarla(&o, &s, Kip::Serbest, false);
    d.gonder(mesaj("yaz")).await;

    let ogeler = onay_bekle(&mut d).await;
    assert_eq!(ogeler.len(), 1, "yalnızca kanca dizinine yazma onaya gelir");
    assert_eq!(ogeler[0].hedefler, vec!["kancalar/pre-commit"]);
    assert_eq!(ogeler[0].uyarilar, vec![KORUMALI_UYARISI]);
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    for yol in ["a.txt", "kancalar/pre-commit", "b.txt"] {
        assert!(o.alan.path().join(yol).exists(), "{yol}");
    }
}

/// Onay beklenirken `core.hooksPath` hedefe çevrilirse eski onay yazdırmaz:
/// yürütme korumaları planlamadakinden bağımsız, kilit altında yeniden
/// çözer (spec §7.3).
#[tokio::test]
async fn onay_beklerken_kanca_dizini_olan_hedef_eski_onayla_yazilmaz() {
    let o = ortam();
    git(o.alan.path(), &["init", "-q"]);
    std::fs::create_dir(o.alan.path().join("kancalar")).unwrap();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(
            "dosya_yaz",
            json!({"yol": "kancalar/pre-commit", "icerik": "echo
"}),
        ),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = gercek_araclarla(&o, &s, Kip::Siki, false);
    d.gonder(mesaj("yaz")).await;

    let ogeler = onay_bekle(&mut d).await;
    assert!(ogeler[0].uyarilar.is_empty(), "planlamada korumalı değil");
    git(o.alan.path(), &["config", "core.hooksPath", "kancalar"]);
    d.gonder(Komut::AracOnayi {
        cagri: ogeler[0].cagri,
        karar: OnayKarari::Onayla,
        muhur: ogeler[0].muhur.clone(),
        kisayol: false,
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert!(!o.alan.path().join("kancalar/pre-commit").exists());
    assert_eq!(
        arac_yanitlari(&s, 1),
        vec![format!("[çalıştırılmadı] {}", AracHatasi::PolitikaDegisti)]
    );
    let durum = d.gecmis.iter().find_map(|z| match &z.yuk {
        Olay::AracSonucu { durum, .. } => Some(*durum),
        _ => None,
    });
    assert_eq!(durum, Some(AracDurumu::IptalEdildi));
}
