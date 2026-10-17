//! MCP arayüz uçları ve yerel onay sınırı (1.1a-2 Görev 10): kayıt ve silme
//! yerel onaysız olmaz; başlatma kartı model dışında arayüze ulaşır, tek
//! kullanımlıdır ve tam erişimde arayüzün "kabul"üne güvenilmez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_core::kimlik::OturumId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::arayuz::McpYazici;
use kalfa_uygulama::mcp::ayarlar::McpKipi;
use kalfa_uygulama::mcp::komutlar::{McpDurumuDto, McpSunucuFormu};
use kalfa_uygulama::sozlesme::OturumAcilisi;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::json;
use tempfile::TempDir;

struct Ortam {
    _veri: TempDir,
    _alan: TempDir,
    exe: PathBuf,
    t: TestUygulamasi,
}

#[derive(Clone, Default)]
struct KayitliYazici(Arc<Mutex<Vec<McpDurumuDto>>>);

impl McpYazici for KayitliYazici {
    fn gonder(&self, durum: &McpDurumuDto) -> Result<(), String> {
        self.0.lock().unwrap().push(durum.clone());
        Ok(())
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
    let exe = veri.path().join(if cfg!(windows) {
        "sunucu.exe"
    } else {
        "sunucu"
    });
    std::fs::write(&exe, b"sahte").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    Ortam {
        _veri: veri,
        _alan: alan,
        exe,
        t,
    }
}

impl Ortam {
    async fn oturum(&self) -> OturumId {
        self.acilis().await.oturum
    }

    async fn acilis(&self) -> OturumAcilisi {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        self.t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
    }

    /// Yerel seçiciyle komut seçip formu arayüzün göndereceği JSON'dan kurar.
    async fn form(&self, ad: &str, kip: &str) -> McpSunucuFormu {
        self.t.iletisim.dosya_ver(&self.exe);
        let secili = self.t.uygulama.mcp_komut_sec().await.unwrap().unwrap();
        serde_json::from_value(json!({
            "ad": ad,
            "etkin": true,
            "komut": {"tur": "secili_yurutulebilir", "kimlik": secili.kimlik},
            "kip": kip,
            "baslatma_zamani_asimi_sn": 5,
            "cagri_zamani_asimi_sn": 5,
        }))
        .unwrap()
    }

    fn sunucu_adlari(&self) -> Vec<String> {
        self.t
            .uygulama
            .mcp_durumu()
            .sunucular
            .into_iter()
            .map(|s| s.ozet.ad)
            .collect()
    }
}

fn dosya_adi(yol: &Path) -> String {
    yol.file_name().unwrap().to_string_lossy().into_owned()
}

#[tokio::test]
async fn kayit_ve_silme_yerel_onaysiz_olmaz() {
    let o = ortam(Vec::new()).await;
    let oturum = o.oturum().await;
    let form = o.form("yerel", "yalitimli").await;
    let taslak = o.t.uygulama.mcp_taslak_olustur(None, form).unwrap();

    o.t.iletisim.onay_ver(false);
    let kaydedildi =
        o.t.uygulama
            .mcp_taslak_kaydet(taslak.taslak_kimligi.clone(), taslak.kusak, Some(oturum))
            .await
            .unwrap();
    assert!(!kaydedildi, "yerel ret kaydetmez");
    assert!(o.sunucu_adlari().is_empty());
    let sorulan = o.t.iletisim.sorulanlar();
    assert!(
        sorulan.last().unwrap().contains(&dosya_adi(&o.exe)),
        "yerel pencere çözülmüş komutu gösterir"
    );

    o.t.iletisim.onay_ver(true);
    assert!(
        o.t.uygulama
            .mcp_taslak_kaydet(taslak.taslak_kimligi, taslak.kusak, Some(oturum))
            .await
            .unwrap()
    );
    assert_eq!(o.sunucu_adlari(), vec!["yerel"]);

    o.t.iletisim.onay_ver(false);
    assert!(!o.t.uygulama.mcp_sil("yerel".into()).await.unwrap());
    assert_eq!(o.sunucu_adlari(), vec!["yerel"], "yerel ret silmez");
    o.t.iletisim.onay_ver(true);
    assert!(o.t.uygulama.mcp_sil("yerel".into()).await.unwrap());
    assert!(o.sunucu_adlari().is_empty());
}

#[tokio::test]
async fn baslatma_karti_arayuze_ulasir_tek_kullanimlik_ve_tam_erisim_yerel_onay_ister() {
    let o = ortam(vec![SahteYanit::metin("tamam")]).await;
    let acilis = o.acilis().await;
    let oturum = acilis.oturum;
    o.t.iletisim.onay_ver(true);
    let form = o.form("tam", "tam_erisim").await;
    let taslak = o.t.uygulama.mcp_taslak_olustur(None, form).unwrap();
    assert!(
        o.t.uygulama
            .mcp_taslak_kaydet(taslak.taslak_kimligi, taslak.kusak, Some(oturum))
            .await
            .unwrap()
    );

    let yazici = KayitliYazici::default();
    o.t.uygulama.mcp_dinle(Box::new(yazici.clone()));
    let (kanal, mut rx) = SahteKanal::yeni();
    o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();

    // Model turunun başında planlayıcı başlatmayı dener; izin olmadığı için
    // süreç başlamaz ve kart model dışında arayüze yazılır.
    o.t.uygulama
        .mesaj_gonder(oturum, "merhaba".into())
        .await
        .unwrap();
    let kart = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(kart) = yazici
                .0
                .lock()
                .unwrap()
                .iter()
                .flat_map(|d| d.baslatma_kartlari.clone())
                .next()
            {
                return kart;
            }
            while rx.try_recv().is_ok() {}
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("başlatma kartı kanala yazılmalı");
    assert_eq!(kart.sunucu, "tam");
    assert_eq!(kart.kip, McpKipi::TamErisim);
    assert_eq!(
        acilis.calisma_alani_kimligi.as_deref(),
        Some(kart.alan_kimligi.as_str()),
        "arayüz kartı oturumun alan kimliğiyle eşler"
    );
    let revizyonlar: Vec<u64> = yazici
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|d| d.revizyon)
        .collect();
    assert!(
        revizyonlar.windows(2).all(|c| c[0] < c[1]),
        "görüntü revizyonları artar: {revizyonlar:?}"
    );

    // Arayüz "kabul" dese de tam erişim yerel pencere onayı olmadan verilmez.
    o.t.iletisim.onay_ver(false);
    let sonuc =
        o.t.uygulama
            .mcp_baslatma_karti_yanitla(kart.istek_kimligi.clone(), true)
            .await;
    assert!(sonuc.is_err(), "yerel ret: {sonuc:?}");
    let durum = o.t.uygulama.mcp_durumu();
    assert!(
        durum
            .sunucular
            .iter()
            .flat_map(|s| &s.alanlar)
            .all(|a| a.izin.is_none()),
        "izin verilmedi"
    );

    // Kart tek kullanımlıdır: aynı yanıt ikinci kez işlenmez.
    o.t.iletisim.onay_ver(true);
    assert!(
        o.t.uygulama
            .mcp_baslatma_karti_yanitla(kart.istek_kimligi, true)
            .await
            .is_err()
    );
}
