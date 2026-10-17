use kalfa_mcp::tasima::{CERCEVE, SinirliTasima, satir};
use rmcp::transport::Transport;
use tokio::io::{AsyncWriteExt, BufReader};

#[tokio::test]
async fn temiz_eof_tasima_sahibine_olumcul_hata_olarak_ulasir() {
    let (peer, client) = tokio::io::duplex(64);
    let (r, w) = tokio::io::split(client);
    let (tasima, mut hata) = SinirliTasima::yeni(r, w);
    drop(peer);
    let degisti = tokio::time::timeout(std::time::Duration::from_secs(2), hata.changed()).await;
    assert!(
        degisti.is_ok(),
        "temiz EOF yöneticinin ölümcül hata kanalına ulaşmalı"
    );
    assert!(
        hata.borrow().is_some(),
        "temiz EOF receive sonucu olarak kaybolmamalı"
    );
    drop(tasima);
}

#[tokio::test]
async fn kapatma_bekleyen_stdout_okumasini_iptal_edip_bekler() {
    let (peer, client) = tokio::io::duplex(64);
    let (r, w) = tokio::io::split(client);
    let (mut tasima, _) = SinirliTasima::yeni(r, w);
    tokio::time::timeout(std::time::Duration::from_secs(2), tasima.close())
        .await
        .unwrap()
        .unwrap();
    drop(peer);
}

#[tokio::test]
async fn yalniz_eslesen_aktif_abonelik_iptali_yoklamaya_gecirir() {
    let (mut sunucu, istemci) = tokio::io::duplex(1024);
    let (okuyucu, yazici) = tokio::io::split(istemci);
    let (mut tasima, hata) = SinirliTasima::yeni(okuyucu, yazici);
    let istek = serde_json::from_value(serde_json::json!({
        "jsonrpc": "2.0", "id": 5, "method": "subscriptions/listen", "params": {}
    }))
    .unwrap();
    tasima.send(istek).await.unwrap();
    let mut satirlar = tokio::io::BufReader::new(&mut sunucu);
    satir(&mut satirlar, CERCEVE).await.unwrap().unwrap();
    drop(satirlar);

    sunucu
        .write_all(br#"{"jsonrpc":"2.0","method":"notifications/subscriptions/acknowledged","params":{"notifications":{"toolsListChanged":true},"_meta":{"io.modelcontextprotocol/subscriptionId":5}}}"#)
        .await
        .unwrap();
    sunucu.write_all(b"\n").await.unwrap();
    assert!(tasima.receive().await.is_some());

    sunucu
        .write_all(br#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":5,"reason":"gizli-yansi"}}"#)
        .await
        .unwrap();
    sunucu.write_all(b"\n").await.unwrap();
    let iptal = tokio::time::timeout(std::time::Duration::from_secs(2), tasima.receive())
        .await
        .unwrap();
    assert!(iptal.is_some());
    assert!(
        tasima
            .yenileme
            .yenileme_gerekli(std::time::Instant::now() + std::time::Duration::from_secs(30))
    );
    assert!(hata.borrow().is_none());
}

#[tokio::test]
async fn beklenmeyen_abonelik_ack_kapatilir() {
    let (mut yazici, okuyucu) = tokio::io::duplex(1024);
    let (okuyucu, yazici_arka) = tokio::io::split(okuyucu);
    let (tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    yazici.write_all(
        br#"{"jsonrpc":"2.0","method":"notifications/subscriptions/acknowledged","params":{"notifications":{"toolsListChanged":true},"_meta":{"io.modelcontextprotocol/subscriptionId":77}}}"#
    ).await.unwrap();
    yazici.write_all(b"\n").await.unwrap();
    let degisti = tokio::time::timeout(std::time::Duration::from_secs(2), hata.changed()).await;
    assert!(degisti.is_ok(), "beklenmeyen ACK kimliği ölümcül olmalı");
    assert!(hata.borrow().is_some());
    drop(tasima);
}

#[tokio::test]
async fn bozuk_json_ham_metni_hata_kanalina_tasinmaz() {
    let (mut yazici, okuyucu) = tokio::io::duplex(256);
    let (okuyucu, yazici_arka) = tokio::io::split(okuyucu);
    let (_tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    yazici.write_all(b"gizli-yansi-bozuk-json\n").await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        *hata.borrow(),
        Some(kalfa_mcp::tasima::TasimaHatasi::BozukJson)
    );
    assert!(!format!("{:?}", *hata.borrow()).contains("gizli-yansi"));
}

#[tokio::test]
async fn eslesmeyen_iptaller_atilir_aktif_64_gelen_kuyruk_siniri_korunur() {
    let (sunucu, istemci) = tokio::io::duplex(64 * 1024);
    let (okuyucu, yazici_arka) = tokio::io::split(istemci);
    let (mut tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    let (mut sunucu_okuyucu, mut sunucu_yazici) = tokio::io::split(sunucu);

    for sira in 0..1_000 {
        sunucu_yazici
            .write_all(
                serde_json::to_string(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/cancelled",
                    "params": {"requestId": 10_000 + sira, "reason": "gizli-yansi"}
                }))
                .unwrap()
                .as_bytes(),
            )
            .await
            .unwrap();
        sunucu_yazici.write_all(b"\n").await.unwrap();
    }

    for id in 1..=64 {
        let method = if id == 1 {
            "subscriptions/listen"
        } else {
            "ping"
        };
        let istek = serde_json::from_value(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": {}
        }))
        .unwrap();
        tasima.send(istek).await.unwrap();
    }
    let mut sunucu_okuyucu = tokio::io::BufReader::new(&mut sunucu_okuyucu);
    for _ in 0..64 {
        satir(&mut sunucu_okuyucu, CERCEVE).await.unwrap().unwrap();
    }

    let mut gelenler = Vec::new();
    gelenler.extend_from_slice(
        br#"{"jsonrpc":"2.0","method":"notifications/subscriptions/acknowledged","params":{"notifications":{"toolsListChanged":true},"_meta":{"io.modelcontextprotocol/subscriptionId":1}}}"#,
    );
    gelenler.push(b'\n');
    for id in 2..=64 {
        gelenler.extend_from_slice(
            serde_json::to_vec(&serde_json::json!({"jsonrpc":"2.0","id":id,"result":{}}))
                .unwrap()
                .as_slice(),
        );
        gelenler.push(b'\n');
    }
    gelenler.extend_from_slice(
        br#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1,"reason":"gizli-yansi"}}"#,
    );
    gelenler.push(b'\n');
    sunucu_yazici.write_all(&gelenler).await.unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(2), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        *hata.borrow(),
        Some(kalfa_mcp::tasima::TasimaHatasi::IletiSeli)
    );
    drop(tasima);
}

#[tokio::test]
async fn bes_mib_gorsel_4_mib_cercevede_ayristirilmadan_reddedilir() {
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD;

    let gorsel = STANDARD.encode(vec![0_u8; 5 * 1024 * 1024]);
    let ham = serde_json::to_vec(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "result": {"content": [{"type": "image", "data": gorsel, "mimeType": "image/png"}]}
    }))
    .unwrap();
    assert!(ham.len() > CERCEVE);
    let (mut yazici, okuyucu) = tokio::io::duplex(ham.len() + 16);
    let (okuyucu, yazici_arka) = tokio::io::split(okuyucu);
    let (_tasima, mut hata) = SinirliTasima::yeni(okuyucu, yazici_arka);
    yazici.write_all(&ham).await.unwrap();
    yazici.write_all(b"\n").await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        *hata.borrow(),
        Some(kalfa_mcp::tasima::TasimaHatasi::Cerceve)
    );
}

#[tokio::test]
async fn cr_bayti_cevre_sinirina_dahildir() {
    let mut girdi = vec![b' '; CERCEVE - 1];
    girdi.push(b'\r');
    girdi.push(b'\n');
    assert_eq!(
        satir(&mut BufReader::new(girdi.as_slice()), CERCEVE)
            .await
            .unwrap()
            .unwrap()
            .len(),
        CERCEVE
    );

    let mut fazla = vec![b' '; CERCEVE];
    fazla.push(b'\r');
    fazla.push(b'\n');
    assert!(
        satir(&mut BufReader::new(fazla.as_slice()), CERCEVE)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn turkce_utf8_bayt_bayt_gelir_ve_birden_fazla_satir_ayrilir() {
    let (mut yazici, okuyucu) = tokio::io::duplex(8);
    let is = tokio::spawn(async move {
        for b in "{\"metin\":\"çığüşöİ\"}\n{\"sayi\":2}\n".bytes() {
            yazici.write_all(&[b]).await.unwrap();
            tokio::task::yield_now().await;
        }
    });
    let mut okuyucu = BufReader::with_capacity(3, okuyucu);
    let ilk = satir(&mut okuyucu, CERCEVE).await.unwrap().unwrap();
    let ikinci = satir(&mut okuyucu, CERCEVE).await.unwrap().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&ilk).unwrap()["metin"],
        "çığüşöİ"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&ikinci).unwrap()["sayi"],
        2
    );
    assert!(satir(&mut okuyucu, CERCEVE).await.unwrap().is_none());
    is.await.unwrap();
}

#[tokio::test]
async fn bin_liste_bildirimi_sdk_kuyruguna_tek_bildirim_bile_birakmaz() {
    let (mut sunucu, istemci) = tokio::io::duplex(8192);
    let (okuma, yazma) = tokio::io::split(istemci);
    let (mut tasima, hata) = SinirliTasima::yeni(okuma, yazma);
    tasima
        .send(
            serde_json::from_value(serde_json::json!({
                "jsonrpc":"2.0", "id":77, "method":"ping"
            }))
            .unwrap(),
        )
        .await
        .unwrap();
    satir(&mut BufReader::new(&mut sunucu), CERCEVE)
        .await
        .unwrap()
        .unwrap();
    for _ in 0..1000 {
        sunucu
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/tools/list_changed\"}\n")
            .await
            .unwrap();
    }
    sunucu
        .write_all(b"{\"jsonrpc\":\"2.0\",\"id\":77,\"result\":{}}\n")
        .await
        .unwrap();
    let gelen = tokio::time::timeout(std::time::Duration::from_secs(3), tasima.receive())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        gelen,
        rmcp::model::ServerJsonRpcMessage::Response(_)
    ));
    assert!(
        tasima
            .yenileme
            .yenileme_gerekli(std::time::Instant::now() + std::time::Duration::from_secs(5))
    );
    assert!(hata.borrow().is_none());
    tasima.close().await.unwrap();
}

#[tokio::test]
async fn bos_ve_gecersiz_utf8_json_hata_olur_ilk_hata_degismez() {
    use kalfa_mcp::tasima::TasimaHatasi;
    for ham in [b"\n".as_slice(), &[255, b'\n']] {
        let (mut sunucu, istemci) = tokio::io::duplex(128);
        let (okuma, yazma) = tokio::io::split(istemci);
        let (mut tasima, mut hata) = SinirliTasima::yeni(okuma, yazma);
        sunucu.write_all(ham).await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), hata.changed())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(*hata.borrow(), Some(TasimaHatasi::BozukJson));
        drop(sunucu);
        let istek =
            serde_json::from_value(serde_json::json!({"jsonrpc":"2.0", "id":1,"method":"ping"}))
                .unwrap();
        assert!(tasima.send(istek).await.is_err());
        tasima.close().await.unwrap();
        assert_eq!(*hata.borrow(), Some(TasimaHatasi::BozukJson));
    }
}

#[tokio::test]
async fn okuma_gorevinin_panigi_sahibine_hata_bildirir_ve_kapatma_takilmaz() {
    struct PanikOkuyucu;
    impl tokio::io::AsyncRead for PanikOkuyucu {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            _: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            panic!("sahte okuyucunun beklenmeyen bitişi");
        }
    }
    let (mut tasima, mut hata) = SinirliTasima::yeni(PanikOkuyucu, tokio::io::sink());
    tokio::time::timeout(std::time::Duration::from_secs(2), hata.changed())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        *hata.borrow(),
        Some(kalfa_mcp::tasima::TasimaHatasi::GirdiCikti)
    );
    tokio::time::timeout(std::time::Duration::from_secs(2), tasima.close())
        .await
        .unwrap()
        .unwrap();
}
