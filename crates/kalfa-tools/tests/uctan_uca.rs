use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::AracKayitDefteri;
use kalfa_core::izin::{GIZLI_UYARISI, Kip, Siniflandirici};
use kalfa_core::kayit::OturumKaydi;
use kalfa_core::kurtarma::kurtar;
use kalfa_core::oturum::{OturumAyarlari, OturumBaglami, baslat};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use kalfa_core::saglayici::EvrenSaglayici;
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
    })
    .await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(std::fs::read_to_string(&dosya).unwrap(), "merhaba dünya\n");
    assert_eq!(d.metin(), "Dosyayı güncelledim.");

    // Model, okuma sonucunu kaynak etiketiyle gördü.
    assert!(arac_yanitlari(&s, 1)[0].starts_with("[kaynak: dosya:selam.txt:1-1]"));

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
    let baglam = OturumBaglami {
        saglayici: Arc::new(EvrenSaglayici::yeni(istemci)),
        araclar: Arc::new(AracKayitDefteri::yeni(varsayilan_araclar(calisma_alani)).unwrap()),
        siniflandirici: o.siniflandirici.clone(),
        kilitler: Arc::new(kalfa_core::kilit::KaynakKilitleri::yeni()),
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
