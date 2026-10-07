//! Kabuk uygulamada (aşama 2 tasarımı §4.2, §5.2, §10.3): yalıtımsız komut
//! yalnızca yerel pencerede onaylanır; ilk çağrı çalışma alanına kaydedilir.

use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::izin::{ILK_KABUK_UYARISI, Kip, YALITIMSIZ_UYARISI};
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari, OnayOgesi};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_sandbox::YalitimDurumu;
use kalfa_uygulama::sozlesme::OnayGirdisi;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

struct Ortam {
    _veri: TempDir,
    _alan: TempDir,
    t: TestUygulamasi,
}

fn echo(metin: &str) -> serde_json::Value {
    if cfg!(windows) {
        json!({"program": "cmd", "argumanlar": ["/c", format!("echo {metin}")]})
    } else {
        json!({"program": "sh", "argumanlar": ["-c", format!("echo {metin}")]})
    }
}

async fn ortam(senaryo: Vec<SahteYanit>) -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a"));
    t.uygulama.baslat().await;
    t.iletisim.klasor_ver(alan.path());
    Ortam {
        _veri: veri,
        _alan: alan,
        t,
    }
}

impl Ortam {
    async fn yeni_oturum(&self) -> (OturumId, UnboundedReceiver<KanalMesaji>) {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        let oturum = self
            .t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum;
        let (kanal, rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();
        (oturum, rx)
    }

    async fn bekle(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
        kosul: impl Fn(&Olay) -> bool,
    ) -> Vec<Olay> {
        let mut gorulen = Vec::new();
        loop {
            let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
                .await
                .expect("olay gelmedi")
                .expect("kanal kapandı");
            let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
                panic!("kanal kapandı");
            };
            self.t.uygulama.paketi_isledim(oturum, no).unwrap();
            for z in zarflar {
                let bulundu = kosul(&z.yuk);
                gorulen.push(z.yuk);
                if bulundu {
                    return gorulen;
                }
            }
        }
    }

    async fn onay_ogeleri(
        &self,
        oturum: OturumId,
        rx: &mut UnboundedReceiver<KanalMesaji>,
    ) -> Vec<OnayOgesi> {
        match self
            .bekle(oturum, rx, |o| matches!(o, Olay::OnayGerekiyor { .. }))
            .await
            .pop()
        {
            Some(Olay::OnayGerekiyor { ogeler, .. }) => ogeler,
            _ => unreachable!(),
        }
    }
}

fn onayla(o: &OnayOgesi) -> OnayGirdisi {
    OnayGirdisi {
        cagri: o.cagri,
        muhur: o.muhur.clone(),
        karar: OnayKarari::Onayla,
        kisayol: false,
    }
}

fn sonuc(olaylar: &[Olay]) -> (AracDurumu, String) {
    olaylar
        .iter()
        .find_map(|o| match o {
            Olay::AracSonucu { durum, icerik, .. } => Some((*durum, icerik.clone())),
            _ => None,
        })
        .expect("araç sonucu")
}

#[tokio::test]
async fn yalitim_durumu_uygulama_durumunda_gorunur() {
    let o = ortam(Vec::new()).await;
    assert!(matches!(
        o.t.uygulama.uygulama_durumu().yalitim,
        YalitimDurumu::Yok { .. }
    ));
}

#[tokio::test]
async fn yalitimsiz_komut_yerel_pencerede_onaylaninca_calisir() {
    let o = ortam(vec![
        SahteYanit::arac("kabuk", echo("kabuktan-merhaba")),
        SahteYanit::metin("tamam"),
        SahteYanit::arac("kabuk", echo("ikinci")),
        SahteYanit::metin("bitti"),
    ])
    .await;
    let (oturum, mut rx) = o.yeni_oturum().await;
    // Serbest kipte bile yalıtımsız komut onay ister.
    o.t.iletisim.onay_ver(true);
    o.t.uygulama
        .izin_kipi_degistir(oturum, Kip::Serbest)
        .await
        .unwrap();
    o.t.uygulama
        .mesaj_gonder(oturum, "çalıştır".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert!(ogeler[0].yerel_onay);
    assert!(ogeler[0].uyarilar.contains(&YALITIMSIZ_UYARISI.to_string()));
    assert!(ogeler[0].uyarilar.contains(&ILK_KABUK_UYARISI.to_string()));

    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let sorulan = o.t.iletisim.sorulanlar().pop().unwrap();
    assert!(
        sorulan.starts_with("Komutu yalıtımsız çalıştır: "),
        "{sorulan}"
    );
    assert!(
        sorulan.contains("echo kabuktan-merhaba"),
        "komut çekirdekten okunur: {sorulan}"
    );

    let olaylar = o
        .bekle(oturum, &mut rx, |o| matches!(o, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = sonuc(&olaylar);
    assert_eq!(durum, AracDurumu::Basarili, "{icerik}");
    assert!(icerik.contains("kabuktan-merhaba"));
    assert!(
        olaylar
            .iter()
            .any(|o| matches!(o, Olay::AracCiktisiParcasi { metin, .. } if metin.contains("kabuktan-merhaba"))),
        "çıktı canlı akar"
    );

    // Aynı çalışma alanındaki sonraki çağrıda ilk çağrı uyarısı yoktur;
    // yalıtımsız olduğu için yine yerel onay ister.
    o.t.uygulama
        .mesaj_gonder(oturum, "yine".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    assert!(ogeler[0].yerel_onay);
    assert!(!ogeler[0].uyarilar.contains(&ILK_KABUK_UYARISI.to_string()));
}

#[tokio::test]
async fn yerel_pencerede_vazgecilirse_komut_calismaz() {
    let o = ortam(vec![
        SahteYanit::arac("kabuk", echo("calismamali")),
        SahteYanit::metin("peki"),
    ])
    .await;
    let (oturum, mut rx) = o.yeni_oturum().await;
    o.t.uygulama
        .mesaj_gonder(oturum, "çalıştır".into())
        .await
        .unwrap();
    let ogeler = o.onay_ogeleri(oturum, &mut rx).await;
    o.t.iletisim.onay_ver(false);
    o.t.uygulama
        .arac_onayi(oturum, vec![onayla(&ogeler[0])])
        .await
        .unwrap();
    let olaylar = o
        .bekle(oturum, &mut rx, |o| matches!(o, Olay::Tamamlandi { .. }))
        .await;
    let (durum, icerik) = sonuc(&olaylar);
    assert_eq!(durum, AracDurumu::Reddedildi);
    assert!(icerik.contains("yerel pencerede vazgeçti"), "{icerik}");
}

/// Aşama 2 tasarımı §9.8: yalıtım yokken ve yalıtımsız çalıştırma kapalıyken
/// kabuk modele tanımlanmaz; çağrılırsa çalışmaz.
#[tokio::test]
async fn yalitimsiz_anahtari_kapaliyken_kabuk_sunulmaz() {
    let o = ortam(vec![
        SahteYanit::arac("kabuk", echo("calismamali")),
        SahteYanit::metin("peki"),
    ])
    .await;
    assert!(!o.t.uygulama.yalitimsizi_ayarla(false).await.unwrap());
    let (oturum, mut rx) = o.yeni_oturum().await;
    o.t.uygulama
        .mesaj_gonder(oturum, "çalıştır".into())
        .await
        .unwrap();
    let olaylar = o
        .bekle(oturum, &mut rx, |o| matches!(o, Olay::AracSonucu { .. }))
        .await;
    let (durum, icerik) = sonuc(&olaylar);
    assert_eq!(durum, AracDurumu::Basarisiz);
    assert!(icerik.contains("şu an kullanılamıyor"), "{icerik}");
    let araclar: Vec<String> = o.t.saglayici.istekler()[0]
        .tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert!(!araclar.iter().any(|a| a == "kabuk"), "{araclar:?}");
}
