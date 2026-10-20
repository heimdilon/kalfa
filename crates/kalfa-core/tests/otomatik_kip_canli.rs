//! Otomatik kipin canlı denemesi (PR B planı, bitirme doğrulaması). Gerçek ana
//! model ve gerçek hakem (`clef`) çekirdek düzeyinde, arayüzsüz koşar.
//! Varsayılan olarak atlanır; yalnız kullanıcının ayrı onayıyla koşar.
//!
//!   set -a && . ./.env && set +a && cargo test -p kalfa-core --test otomatik_kip_canli -- --ignored --nocapture
//!
//! Kabuk aracı sahtedir: hazırlığı yalıtımlı bir kabuk tarifi taşır, ama
//! komutu **çalıştırmaz**; hakem gerçek komut metnini görür.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use kalfa_core::arac::{Arac, AracCiktisi, AracHatasi, AracKayitDefteri, Hazirlik};
use kalfa_core::izin::Kip;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::kilit::KaynakKilitleri;
use kalfa_core::oturum::{OturumAyarlari, OturumBaglami};
use kalfa_core::protokol::{BitisNedeni, Komut, Olay, OnayKarari};
use kalfa_core::saglayici::{EvrenSaglayici, Saglayici};
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_core::test_destek::{sanal_siniflandirici, test_oturumu};
use kalfa_evren::{ApiAnahtari, EvrenIstemcisi, VARSAYILAN_TABAN};
use serde_json::{Value, json};

fn canli_saglayici() -> Arc<dyn Saglayici> {
    let anahtar = std::env::var("EVREN_API_KEY").expect("EVREN_API_KEY tanımlı olmalı");
    let taban = std::env::var("EVREN_BASE_URL").unwrap_or_else(|_| VARSAYILAN_TABAN.to_string());
    Arc::new(EvrenSaglayici::yeni(
        EvrenIstemcisi::yeni(taban, ApiAnahtari::yeni(anahtar)).unwrap(),
    ))
}

/// Komutu çalıştırmayan, yalıtımlı ve ilk çağrı uyarılı sahte kabuk.
struct DenemeKabugu {
    cagri: AtomicUsize,
}

#[async_trait]
impl Arac for DenemeKabugu {
    fn ad(&self) -> &str {
        "kabuk"
    }
    fn aciklama(&self) -> &str {
        "Çalışma alanında bir kabuk komutu çalıştırır ve çıktısını döndürür."
    }
    fn parametre_semasi(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "komut": {"type": "string", "description": "Çalıştırılacak komut"},
                "ag": {"type": "boolean", "description": "Komut ağa erişecek mi"}
            },
            "required": ["komut"]
        })
    }
    fn yazar_mi(&self) -> bool {
        true
    }
    async fn hazirla(&self, a: &Value) -> Result<Hazirlik, AracHatasi> {
        let komut = a["komut"].as_str().unwrap_or("").to_string();
        let ag = a["ag"].as_bool().unwrap_or(false);
        Ok(Hazirlik {
            tum_calisma_alani: true,
            aciklama: format!("`{komut}` komutunu çalıştırmak istiyor."),
            kabuk: Some(KabukTarifi {
                komut: komut.clone(),
                bicim: KabukBicimi::Program,
                program: Some(komut),
                dizin: ".".into(),
                ag,
                ag_hostlari: Vec::new(),
                yalitim: YalitimOzeti::Yalitimli {
                    arka_uc: "deneme".into(),
                    profil: "deneme".into(),
                },
                ilk_cagri: true,
                alan_hazirligi: false,
            }),
            ..Default::default()
        })
    }
    async fn calistir(
        &self,
        a: &Value,
        _h: &Hazirlik,
        _iptal: tokio_util::sync::CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.cagri.fetch_add(1, Ordering::SeqCst);
        let komut = a["komut"].as_str().unwrap_or("");
        Ok(AracCiktisi {
            icerik: format!("$ {komut}\ntamam (çıkış kodu 0)"),
            kaynak: format!("kabuk:{komut}"),
            guvenlik_uyarilari: Vec::new(),
            degisiklikler: None,
            kalfa_uretimi: false,
            ag: None,
        })
    }
}

#[tokio::test]
#[ignore = "canlı EVREN API; EVREN_API_KEY gerekir"]
async fn canli_otomatik_kip_olagan_gelistirme_gorevi() {
    let saglayici = canli_saglayici();
    let kabuk = Arc::new(DenemeKabugu {
        cagri: AtomicUsize::new(0),
    });
    let baglam = OturumBaglami {
        sart_kapisi: Arc::new(SartKapisi::yeni(saglayici.clone())),
        denetci: Arc::new(kalfa_core::denetci::Yok),
        denetci_kipi: None,
        saglayici,
        araclar: Arc::new(AracKayitDefteri::yeni(vec![kabuk.clone() as Arc<dyn Arac>]).unwrap()),
        siniflandirici: sanal_siniflandirici(),
        kilitler: Arc::new(KaynakKilitleri::yeni()),
        geri_yukleyici: None,
    };
    let mut ayarlar = OturumAyarlari::yeni("glm-5.3", std::path::Path::new("/sanal/gorev-listesi"));
    ayarlar.kip = Kip::Otomatik;
    let mut d = test_oturumu(baglam, ayarlar).zaman_asimi(Duration::from_secs(240));

    let bas = Instant::now();
    d.gonder(Komut::KullaniciMesaji {
        metin: "Bu Python projesinde testleri çalıştırmak istiyorum. Önce \
                `python -m pip install -r requirements.txt` ile bağımlılıkları \
                kur, sonra `python -m unittest` ile testleri çalıştır. Kabuk \
                aracını kullan ve sonucu kısaca özetle."
            .into(),
    })
    .await;

    let mut kart_sayisi = 0;
    let neden = loop {
        let zarf = d
            .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. } | Olay::Tamamlandi { .. }))
            .await;
        match zarf.yuk {
            Olay::OnayGerekiyor { ogeler, .. } => {
                // Kart, hakemin kullanıcıya bıraktığı işlemdir: denemede
                // reddedilir; araç zaten komutu çalıştırmaz.
                kart_sayisi += ogeler.len();
                for o in ogeler {
                    println!("[kart] {} uyarılar={:?}", o.aciklama, o.uyarilar);
                    d.gonder(Komut::AracOnayi {
                        cagri: o.cagri,
                        karar: OnayKarari::Reddet {
                            gerekce: Some("canlı deneme: kart reddedildi".into()),
                        },
                        muhur: o.muhur,
                        kisayol: false,
                    })
                    .await;
                }
            }
            Olay::Tamamlandi { neden, .. } => break neden,
            _ => unreachable!(),
        }
    };
    let sure = bas.elapsed();

    let olaylar = d.olaylar();
    let kararlar: Vec<_> = olaylar
        .iter()
        .filter_map(|o| match o {
            Olay::OtomatikKarar {
                aciklama,
                sonuc,
                risk,
                olasilik,
                ..
            } => Some((aciklama.clone(), *sonuc, risk.clone(), *olasilik)),
            _ => None,
        })
        .collect();
    for k in &kararlar {
        println!("[hakem] {k:?}");
    }
    let yanit: String = olaylar
        .iter()
        .filter_map(|o| match o {
            Olay::TokenParcasi { metin } => Some(metin.as_str()),
            _ => None,
        })
        .collect();
    println!(
        "bitiş={neden:?} süre={sure:?} hakem kararı={} kart öğesi={kart_sayisi} \
         araç çalışması={}\n[model] {yanit}",
        kararlar.len(),
        kabuk.cagri.load(Ordering::SeqCst),
    );
    d.kapat().await;

    assert_eq!(neden, BitisNedeni::Normal);
    assert!(!kararlar.is_empty(), "en az bir hakem kararı olmalı");
}
