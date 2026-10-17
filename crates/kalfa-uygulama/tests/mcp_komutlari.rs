//! MCP arayüz uçları ve yerel onay sınırı (1.1a-2 Görev 10): kayıt ve silme
//! yerel onaysız olmaz; başlatma kartı model dışında arayüze ulaşır, tek
//! kullanımlıdır ve tam erişimde arayüzün "kabul"üne güvenilmez.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::arayuz::McpYazici;
use kalfa_uygulama::mcp::ayarlar::McpKipi;
use kalfa_uygulama::mcp::komutlar::{McpDurumuDto, McpSunucuFormu};
use kalfa_uygulama::sozlesme::{OnayGirdisi, OturumAcilisi};
use kalfa_uygulama::test_destek::{
    SahteKanal, TestUygulamasi, test_uygulamasi, test_uygulamasi_yalitimla,
};
use kalfa_uygulama::yalitim::YalitimKaynagi;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

struct Ortam {
    _veri: TempDir,
    alan: TempDir,
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
    ortam_ile(senaryo, None).await
}

/// `yalitim` verilmezse kabuk yalıtımsızdır ve etkileşimli süreç başlatmaz;
/// gerçek MCP süreci için platformun yalıtıcısı verilir.
async fn ortam_ile(senaryo: Vec<SahteYanit>, yalitim: Option<YalitimKaynagi>) -> Ortam {
    let veri = tempfile::tempdir().unwrap();
    let alan = tempfile::tempdir().unwrap();
    let saglayici = SahteSaglayici::yeni(senaryo);
    saglayici.sart_kabul(1).await.unwrap();
    let t = match yalitim {
        None => test_uygulamasi(veri.path(), saglayici, Some("evren_llm_a")),
        Some(y) => test_uygulamasi_yalitimla(veri.path(), saglayici, Some("evren_llm_a"), y),
    };
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
        alan,
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

/// `target/debug` altındaki gerçek MCP test sunucusu (workspace testleri derler).
fn gercek_sunucu() -> PathBuf {
    let ad = if cfg!(windows) {
        "kalfa-mcp-test-sunucusu.exe"
    } else {
        "kalfa-mcp-test-sunucusu"
    };
    let yol = std::env::current_exe()
        .unwrap()
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join(ad);
    assert!(
        yol.is_file(),
        "workspace testlerinde koşmalı; test sunucusu yok: {}",
        yol.display()
    );
    yol
}

/// Koşul tutan olaya kadar paketleri işler ve görülenleri döner.
async fn bekle(
    o: &Ortam,
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
        o.t.uygulama.paketi_isledim(oturum, no).unwrap();
        for z in zarflar {
            let bulundu = kosul(&z.yuk);
            gorulen.push(z.yuk);
            if bulundu {
                return gorulen;
            }
        }
    }
}

/// Bir tur: yanıtlar senaryoya eklenir, onay istenirse onaylanır. Turun
/// olaylarını döner.
async fn tur(
    o: &Ortam,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    yanitlar: Vec<SahteYanit>,
) -> Vec<Olay> {
    for y in yanitlar {
        o.t.saglayici.senaryoya_ekle(y);
    }
    o.t.uygulama
        .mesaj_gonder(oturum, "devam".into())
        .await
        .unwrap();
    let mut tum = Vec::new();
    loop {
        let son = bekle(o, oturum, rx, |olay| {
            matches!(olay, Olay::Tamamlandi { .. } | Olay::OnayGerekiyor { .. })
        })
        .await;
        tum.extend(son.iter().cloned());
        match son.last() {
            Some(Olay::OnayGerekiyor { ogeler, .. }) => {
                let kararlar = ogeler
                    .iter()
                    .map(|g| OnayGirdisi {
                        cagri: g.cagri,
                        muhur: g.muhur.clone(),
                        karar: OnayKarari::Onayla,
                        kisayol: false,
                    })
                    .collect();
                o.t.uygulama.arac_onayi(oturum, kararlar).await.unwrap();
            }
            _ => return tum,
        }
    }
}

/// Plan Görev 12/3: §4.3'ün kabul edilmiş kalan riski, EVREN'siz sahte
/// sağlayıcıyla ölçülür. Sunucu başladıktan sonra çalışma alanına gelen
/// gizli dosyanın içeriği (kasadaki bilinen sır değil) önce araç sonucuyla,
/// sonra ayrı olarak araç açıklamasıyla sağlayıcıya ulaşır. Yeni süreçte
/// aynı dosyanın okunamadığını işletim sistemi sınırı ölçer
/// (`kalfa-mcp/tests/yalitim.rs`); süreç burada tam erişimlidir.
#[tokio::test]
async fn sonradan_gelen_gizli_icerik_sonuc_ve_tanim_yoluyla_saglayiciya_ulasir() {
    const CANARY: &str = "SONRADAN-GELEN-GIZLI-58b2";
    let o = ortam_ile(Vec::new(), Some(YalitimKaynagi::Platform)).await;
    let acilis = o.acilis().await;
    let oturum = acilis.oturum;
    o.t.iletisim.onay_ver(true);
    o.t.iletisim.dosya_ver(&gercek_sunucu());
    let secili = o.t.uygulama.mcp_komut_sec().await.unwrap().unwrap();
    let form: McpSunucuFormu = serde_json::from_value(json!({
        "ad": "risk",
        "etkin": true,
        "komut": {"tur": "secili_yurutulebilir", "kimlik": secili.kimlik, "argumanlar": ["sinama"]},
        "kip": "tam_erisim",
        "baslatma_zamani_asimi_sn": 20,
        "cagri_zamani_asimi_sn": 20,
    }))
    .unwrap();
    let taslak = o.t.uygulama.mcp_taslak_olustur(None, form).unwrap();
    assert!(
        o.t.uygulama
            .mcp_taslak_kaydet(taslak.taslak_kimligi, taslak.kusak, Some(oturum))
            .await
            .unwrap()
    );
    let (kanal, mut rx) = SahteKanal::yeni();
    o.t.uygulama.oturumu_dinle(oturum, kanal).unwrap();

    // İlk tur başlatma kartını üretir; kart yerel onayla kabul edilir.
    tur(&o, oturum, &mut rx, vec![SahteYanit::metin("1")]).await;
    let kart =
        o.t.uygulama
            .mcp_durumu()
            .baslatma_kartlari
            .pop()
            .expect("başlatma kartı");
    assert!(
        o.t.uygulama
            .mcp_baslatma_karti_yanitla(kart.istek_kimligi, true)
            .await
            .unwrap()
    );
    // Keşif bir turun içinde yayımlanır.
    for _ in 0..10 {
        let bekle = SahteYanit::Gecikmeli(
            Duration::from_millis(1500),
            Box::new(SahteYanit::metin("bekle")),
        );
        tur(&o, oturum, &mut rx, vec![bekle]).await;
        if o.t
            .uygulama
            .mcp_durumu()
            .sunucular
            .iter()
            .flat_map(|s| &s.alanlar)
            .any(|a| a.arac_sayisi == 1)
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let saglayicida = |kosul: &dyn Fn(&serde_json::Value) -> bool| {
        o.t.saglayici
            .istekler()
            .iter()
            .any(|i| kosul(&serde_json::to_value(i).unwrap()))
    };

    // Sonuç yolu: çağrı başlamadan önceki koruma taraması dosyayı görmez;
    // dosya çağrının içinde (sunucu `CALL` yazdıktan sonra) gelir.
    let env = o.alan.path().join(".env");
    let yazici = {
        let (uygulama, env) = (Arc::clone(&o.t.uygulama), env.clone());
        tokio::spawn(async move {
            stderr_bekle(&uygulama, "CALL 1").await;
            std::fs::write(&env, CANARY).unwrap();
        })
    };
    tur(
        &o,
        oturum,
        &mut rx,
        vec![
            SahteYanit::arac("mcp__risk__dene", json!({"islem":"bekleyip_oku","yol":env})),
            SahteYanit::metin("okundu"),
        ],
    )
    .await;
    yazici.await.unwrap();
    assert!(
        saglayicida(&|i| i["messages"].to_string().contains(CANARY)),
        "çağrı içinde gelen içerik araç sonucuyla sağlayıcıya ulaşır (kabul edilen kalan risk)"
    );

    // Tanım yolu, denetim: gizli olmayan dosyanın içeriği açıklamaya konunca
    // liste değişir ve sonraki turların `tools` alanına ulaşır.
    let notlar = o.alan.path().join("notlar.txt");
    izleme_kur(&o, oturum, &mut rx, &notlar).await;
    std::fs::write(&notlar, "KONTROL-ACIKLAMA").unwrap();
    assert!(
        tools_bekle(&o, oturum, &mut rx, "KONTROL-ACIKLAMA").await,
        "açıklama değişikliği sağlayıcının tools alanına ulaşmalı"
    );

    // Tanım yolu, gizli dosya: dosya iki çağrı arasında gelir. Sonraki turun
    // başlatma doğrulaması korumayı yeniden tarar ve süreci yeniden başlatır;
    // eski sürecin açıklaması modele ulaşmaz.
    let ikinci = o.alan.path().join(".env.local");
    izleme_kur(&o, oturum, &mut rx, &ikinci).await;
    let kusak = |o: &Ortam| {
        o.t.uygulama.mcp_durumu().sunucular[0].alanlar[0]
            .kusak
            .unwrap_or_default()
    };
    let onceki = kusak(&o);
    std::fs::write(&ikinci, format!("{CANARY}-tanim")).unwrap();
    assert!(
        !tools_bekle(&o, oturum, &mut rx, &format!("{CANARY}-tanim")).await,
        "turlar arası gelen gizli dosya tanım yoluyla modele ulaşmamalı"
    );
    assert!(
        kusak(&o) > onceki,
        "koruma değişince süreç yeniden başlamalı"
    );
}

/// Sunucuda izleme kurulana kadar (süreç yeniden başlamış olabilir) dener.
async fn izleme_kur(
    o: &Ortam,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    yol: &Path,
) {
    for _ in 0..10 {
        let cagri = SahteYanit::arac(
            "mcp__risk__dene",
            json!({"islem":"izle_aciklamaya_koy","yol":yol}),
        );
        let olaylar = tur(
            o,
            oturum,
            rx,
            vec![
                SahteYanit::Gecikmeli(Duration::from_millis(1500), Box::new(cagri)),
                SahteYanit::metin("izleniyor"),
            ],
        )
        .await;
        let kuruldu = olaylar.iter().any(|olay| {
            matches!(olay, Olay::AracSonucu { durum: AracDurumu::Basarili, icerik, .. } if icerik.contains("TAMAM:"))
        });
        if kuruldu {
            return;
        }
    }
    panic!("izleme kurulamadı");
}

/// En çok 15 uzun tur boyunca sağlayıcının `tools` alanında metni arar.
async fn tools_bekle(
    o: &Ortam,
    oturum: OturumId,
    rx: &mut UnboundedReceiver<KanalMesaji>,
    metin: &str,
) -> bool {
    for _ in 0..15 {
        let liste = SahteYanit::Gecikmeli(
            Duration::from_millis(1500),
            Box::new(SahteYanit::metin("liste")),
        );
        tur(o, oturum, rx, vec![liste]).await;
        let bulundu = o.t.saglayici.istekler().iter().any(|i| {
            serde_json::to_value(i).unwrap()["tools"]
                .to_string()
                .contains(metin)
        });
        if bulundu {
            return true;
        }
    }
    false
}

/// Sunucunun maskeli stderr halkasında satır görünene kadar bekler.
async fn stderr_bekle(uygulama: &kalfa_uygulama::Uygulama, satir: &str) {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Ok(a) = uygulama.mcp_sunucu_ayrintisi("risk")
                && a.alanlar
                    .iter()
                    .any(|al| al.stderr.iter().any(|s| s == satir))
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("sunucu stderr'inde {satir} görülmedi"));
}
