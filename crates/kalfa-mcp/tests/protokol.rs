use kalfa_mcp::istemci::{McpHatasi, McpIstemcisi};
use kalfa_mcp::tasima::{CERCEVE, SinirliTasima};
use std::fmt::Debug;
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::process::ChildStderr as ProcessStderr;
use tokio::process::Command as ProcessCommand;

async fn istemci_baslat(kip: &str) -> (McpIstemcisi, Child, BufReader<ProcessStderr>) {
    let mut cocuk = ProcessCommand::new(env!("CARGO_BIN_EXE_kalfa-mcp-test-sunucusu"))
        .arg(kip)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let okuyucu = cocuk.stdout.take().unwrap();
    let yazici = cocuk.stdin.take().unwrap();
    let gunluk = BufReader::new(cocuk.stderr.take().unwrap());
    let (tasima, hata) = SinirliTasima::yeni(okuyucu, yazici);
    let istemci = tokio::time::timeout(Duration::from_secs(10), McpIstemcisi::yeni(tasima, hata))
        .await
        .unwrap()
        .unwrap();
    (istemci, cocuk, gunluk)
}

async fn istemci_kapat(istemci: &mut McpIstemcisi, cocuk: &mut Child) {
    istemci.kapat().await;
    let sonuc = tokio::time::timeout(Duration::from_secs(3), cocuk.wait()).await;
    if sonuc.is_err() {
        let _ = cocuk.kill().await;
        let _ = cocuk.wait().await;
        panic!("yerel test sunucusu stdio kapanışından sonra bitmedi");
    }
}

async fn gunluk_satiri(gunluk: &mut BufReader<ProcessStderr>) -> String {
    let mut satir = String::new();
    tokio::time::timeout(Duration::from_secs(3), gunluk.read_line(&mut satir))
        .await
        .unwrap()
        .unwrap();
    satir.trim().to_owned()
}

fn bos_argumanlar() -> rmcp::model::JsonObject {
    serde_json::from_value(serde_json::json!({})).unwrap()
}

#[tokio::test]
async fn tasima_4_mib_json_cercevesini_ayristirmadan_sinirlar() {
    let (mut yazici, okuyucu) = tokio::io::duplex(CERCEVE + 16);
    let (okuyucu, yazici_arka) = tokio::io::split(okuyucu);
    let (_tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    let mut ham = vec![b' '; CERCEVE + 1];
    ham.push(b'\n');
    yazici.write_all(&ham).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(1), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(hata.borrow().is_some());
}

#[tokio::test]
async fn yarim_json_kapanista_kontrollu_hata_verir() {
    let (mut yazici, okuyucu) = tokio::io::duplex(64);
    let (okuyucu, yazici_arka) = tokio::io::split(okuyucu);
    let (_tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    yazici
        .write_all(br#"{"jsonrpc":"2.0","id":4"#)
        .await
        .unwrap();
    drop(yazici);
    tokio::time::timeout(std::time::Duration::from_secs(1), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(hata.borrow().is_some());
}

#[tokio::test]
async fn gercek_istemci_eski_ve_yeni_donemleri_baglar_ve_cache_kullanmaz() {
    for kip in ["legacy", "modern"] {
        let (mut istemci, mut cocuk, mut gunluk) = istemci_baslat(kip).await;
        for _ in 0..2 {
            let araclar = istemci.araclari_listele().await.unwrap();
            assert_eq!(araclar.len(), 1);
            assert_eq!(araclar[0].name, "ornek");
        }
        assert_eq!(gunluk_satiri(&mut gunluk).await, "LIST");
        assert_eq!(gunluk_satiri(&mut gunluk).await, "LIST");
        istemci_kapat(&mut istemci, &mut cocuk).await;
    }
}

#[tokio::test]
async fn gercek_istemci_sayfalar_ve_kismi_limit_asimini_yayimlamaz() {
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("pagination").await;
    let araclar = istemci.araclari_listele().await.unwrap();
    assert_eq!(araclar.len(), 2);
    istemci_kapat(&mut istemci, &mut cocuk).await;

    for (kip, beklenen) in [
        ("many-pages", "sayfa"),
        ("129-tools", "arac"),
        ("big-definition", "bayt"),
    ] {
        let (mut istemci, mut cocuk, _gunluk) = istemci_baslat(kip).await;
        let hata = istemci.araclari_listele().await.unwrap_err();
        match (beklenen, hata) {
            ("sayfa", McpHatasi::AracListesiUzun)
            | ("arac", McpHatasi::AracSayisi)
            | ("bayt", McpHatasi::TanimBoyutu) => {}
            _ => panic!("test sunucusu beklenen araç listesi sınırını aşmalı"),
        }
        istemci_kapat(&mut istemci, &mut cocuk).await;
    }
}

#[tokio::test]
async fn bos_abonelik_acki_otuz_saniyelik_yoklamaya_gecer() {
    let (mut istemci, mut cocuk, mut gunluk) = istemci_baslat("empty-ack").await;
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CANCEL");
    let durum = istemci.yenileme_durumu();
    let simdi = std::time::Instant::now();
    assert!(!durum.yenileme_gerekli(simdi + Duration::from_secs(10)));
    assert!(durum.yenileme_gerekli(simdi + Duration::from_secs(30)));
    assert_eq!(istemci.araclari_listele().await.unwrap().len(), 1);
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn arac_listesi_degisim_bildirimi_gercek_istemcide_bes_saniyede_yenilenir() {
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("tool-change").await;
    let baslangic = std::time::Instant::now();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !istemci
            .yenileme_durumu()
            .yenileme_gerekli(baslangic + Duration::from_secs(5))
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn server_roots_sampling_ve_elicitation_istekleri_red_alir() {
    let (mut istemci, mut cocuk, mut gunluk) = istemci_baslat("server-requests").await;
    assert_eq!(istemci.araclari_listele().await.unwrap().len(), 1);
    assert_eq!(gunluk_satiri(&mut gunluk).await, "LIST");
    let mut cevaplar = Vec::new();
    for _ in 0..4 {
        cevaplar.push(gunluk_satiri(&mut gunluk).await);
    }
    cevaplar.sort();
    assert_eq!(
        cevaplar,
        ["ELICITATION RED", "PING OK", "ROOTS RED", "SAMPLING RED"]
    );
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn gec_yanit_atilir_ve_kuyruktaki_iptal_edilmis_cagri_gonderilmez() {
    let (istemci, mut cocuk, mut gunluk) = istemci_baslat("late-response").await;
    let istemci = Arc::new(istemci);
    let cagri_istemcisi = istemci.clone();
    let cagri =
        tokio::spawn(async move { cagri_istemcisi.arac_cagir("ornek", bos_argumanlar()).await });
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CALL 1");
    cagri.abort();
    let _ = cagri.await;
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CANCEL");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(istemci.araclari_listele().await.unwrap().len(), 1);

    let mut istemci = Arc::try_unwrap(istemci).ok().unwrap();
    istemci_kapat(&mut istemci, &mut cocuk).await;

    let (istemci, mut cocuk, mut gunluk) = istemci_baslat("ignore-cancel").await;
    let mut istemci = Arc::new(istemci);
    let ilk_istemci = istemci.clone();
    let ilk = tokio::spawn(async move { ilk_istemci.arac_cagir("ornek", bos_argumanlar()).await });
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CALL 1");

    let ikinci_istemci = istemci.clone();
    let ikinci =
        tokio::spawn(async move { ikinci_istemci.arac_cagir("ornek", bos_argumanlar()).await });
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    ikinci.abort();
    let _ = ikinci.await;
    ilk.abort();
    let _ = ilk.await;
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CANCEL");
    assert!(
        tokio::time::timeout(
            Duration::from_millis(300),
            gunluk.read_line(&mut String::new())
        )
        .await
        .is_err(),
        "sahibi beklemeyi bırakmış kuyruk çağrısı sunucuya ulaşmamalı"
    );
    istemci_kapat(Arc::get_mut(&mut istemci).unwrap(), &mut cocuk).await;
}

#[tokio::test]
async fn gecikmeli_cocuk_yazisi_gercek_istemciye_ulasir() {
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("delayed-child-write").await;
    let sonuc = tokio::time::timeout(
        Duration::from_secs(3),
        istemci.arac_cagir("ornek", bos_argumanlar()),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(format!("{sonuc:?}").contains("gecikmeli çocuk yazdı"));
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn uzak_rpc_hata_govdesi_debug_ve_display_icerisinde_yoktur() {
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("secret-error").await;
    let hata = istemci
        .arac_cagir("ornek", bos_argumanlar())
        .await
        .unwrap_err();
    let gorunen = format!("{hata:?} {hata}");
    assert!(!gorunen.contains("gizli-yansi"));
    assert_eq!(
        hata.uzak_rpc().map(|(kod, metin, _)| (kod, metin)),
        Some((-32000, "gizli-yansi"))
    );
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[derive(Clone)]
struct KayitYakalayici(Arc<Mutex<Vec<String>>>);

impl tracing::Subscriber for KayitYakalayici {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn max_level_hint(&self) -> Option<tracing::metadata::LevelFilter> {
        Some(tracing::metadata::LevelFilter::TRACE)
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut alanlar = String::new();
        event.record(&mut |alan: &tracing::field::Field, deger: &dyn Debug| {
            use std::fmt::Write as _;
            let _ = write!(alanlar, "{}={deger:?};", alan.name());
        });
        if let Ok(mut olaylar) = self.0.lock() {
            olaylar.push(alanlar);
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[tokio::test]
async fn debug_acik_kuresel_subscriber_sdk_yukunu_gormez() {
    static OLAYLAR: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();
    let olaylar = OLAYLAR
        .get_or_init(|| {
            let olaylar = Arc::new(Mutex::new(Vec::new()));
            tracing::subscriber::set_global_default(KayitYakalayici(olaylar.clone())).unwrap();
            olaylar
        })
        .clone();
    olaylar.lock().unwrap().clear();
    tracing::trace!("uygulama-kaydi-acik");

    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("secrets").await;
    assert_eq!(
        istemci.araclari_listele().await.unwrap()[0]
            .description
            .as_deref(),
        Some("gizli-yansi")
    );
    let sonuc = istemci.arac_cagir("ornek", bos_argumanlar()).await.unwrap();
    assert!(format!("{sonuc:?}").contains("gizli-yansi"));
    istemci_kapat(&mut istemci, &mut cocuk).await;

    let kayit = olaylar.lock().unwrap().join("\n");
    assert!(
        kayit.contains("uygulama-kaydi-acik"),
        "yakalama katmanı TRACE olaylarını gerçekten görmeli"
    );
    assert!(
        !kayit.contains("gizli-yansi"),
        "SDK ham olayları global TRACE subscriber'a çıkmamalı"
    );
}

#[tokio::test]
async fn ek_girdi_isteyen_yanit_araci_yeniden_cagirmaz() {
    let (mut istemci, mut cocuk, mut gunluk) = istemci_baslat("input-required").await;
    assert!(matches!(
        istemci.arac_cagir("ornek", bos_argumanlar()).await,
        Err(McpHatasi::DesteklenmeyenCevap)
    ));
    assert_eq!(gunluk_satiri(&mut gunluk).await, "CALL 1");
    assert_eq!(istemci.araclari_listele().await.unwrap().len(), 1);
    assert_eq!(
        gunluk_satiri(&mut gunluk).await,
        "LIST",
        "ikinci tools/call gönderilmemeli"
    );
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn aboneligin_son_yaniti_yoklama_yolunu_acar() {
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("subscription-end").await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while !istemci
            .yenileme_durumu()
            .yenileme_gerekli(std::time::Instant::now() + Duration::from_secs(30))
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(istemci.araclari_listele().await.unwrap().len(), 1);
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

#[tokio::test]
async fn hatali_ve_yinelenen_ack_gercek_istemciyi_durdurur() {
    use kalfa_mcp::tasima::TasimaHatasi;
    for kip in ["wrong-ack", "double-ack"] {
        let mut cocuk = ProcessCommand::new(env!("CARGO_BIN_EXE_kalfa-mcp-test-sunucusu"))
            .arg(kip)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let (tasima, mut hata) =
            SinirliTasima::yeni(cocuk.stdout.take().unwrap(), cocuk.stdin.take().unwrap());
        let sonuc = McpIstemcisi::yeni(tasima, hata.clone()).await;
        if hata.borrow().is_none() {
            tokio::time::timeout(Duration::from_secs(3), hata.changed())
                .await
                .unwrap()
                .unwrap();
        }
        let beklenen = if kip == "wrong-ack" {
            TasimaHatasi::BeklenmeyenAbonelikAck
        } else {
            TasimaHatasi::YinelenenAbonelikAck
        };
        assert_eq!(*hata.borrow(), Some(beklenen));
        if let Ok(mut istemci) = sonuc {
            assert!(istemci.araclari_listele().await.is_err());
            istemci.kapat().await;
        }
        tokio::time::timeout(Duration::from_secs(3), cocuk.wait())
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn iptalin_uc_sunucu_davranisinda_istemci_kapanir() {
    for kip in ["cancel-response", "silent-cancel", "ignore-cancel"] {
        let (istemci, mut cocuk, mut gunluk) = istemci_baslat(kip).await;
        let istemci = Arc::new(istemci);
        let cagiran = istemci.clone();
        let is = tokio::spawn(async move { cagiran.arac_cagir("ornek", bos_argumanlar()).await });
        assert_eq!(gunluk_satiri(&mut gunluk).await, "CALL 1");
        is.abort();
        let _ = is.await;
        assert_eq!(gunluk_satiri(&mut gunluk).await, "CANCEL");
        let mut istemci = Arc::try_unwrap(istemci).ok().unwrap();
        tokio::time::timeout(Duration::from_secs(3), istemci.kapat())
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), cocuk.wait())
            .await
            .unwrap()
            .unwrap();
    }
}

#[tokio::test]
async fn baslatma_suresi_istemcide_25_saniyeyle_kesilmez() {
    let mut cocuk = ProcessCommand::new(env!("CARGO_BIN_EXE_kalfa-mcp-test-sunucusu"))
        .arg("slow-initialize")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let (tasima, hata) =
        SinirliTasima::yeni(cocuk.stdout.take().unwrap(), cocuk.stdin.take().unwrap());
    let mut istemci =
        tokio::time::timeout(Duration::from_secs(30), McpIstemcisi::yeni(tasima, hata))
            .await
            .expect("tanımdaki 30 saniye dolmadı")
            .expect("26 saniyede hazır olan sunucu 25 saniyede kesilmemeli");
    istemci_kapat(&mut istemci, &mut cocuk).await;
}

/// Görev 12 yoklama sunucusu: açıklama değişince liste değişikliği
/// bildirilir ve yeniden listeleme yeni açıklamayı verir.
#[tokio::test]
async fn sinama_aciklama_degisince_liste_degisimi_bildirilir() {
    let dizin = tempfile::tempdir().unwrap();
    let dosya = dizin.path().join("not.txt");
    std::fs::write(&dosya, "YENI-ACIKLAMA").unwrap();
    let (mut istemci, mut cocuk, _gunluk) = istemci_baslat("sinama").await;
    let baslangic = std::time::Instant::now();
    let argumanlar =
        serde_json::from_value(serde_json::json!({"islem":"aciklamaya_koy","yol":dosya})).unwrap();
    istemci.arac_cagir("dene", argumanlar).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !istemci
            .yenileme_durumu()
            .yenileme_gerekli(baslangic + Duration::from_secs(5))
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("liste değişikliği bildirilmeli");
    let araclar = istemci.araclari_listele().await.unwrap();
    assert_eq!(araclar[0].description.as_deref(), Some("YENI-ACIKLAMA"));
    istemci_kapat(&mut istemci, &mut cocuk).await;
}
