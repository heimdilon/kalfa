use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_sandbox::vekil::{
    AdresReddi, AgKarari, Cozucu, HostKaynagi, HostKurali, IzinListesi, Vekil, VekilAyari,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Testler için sahte çözümleyici.
struct SahteCozucu {
    adresler: Vec<SocketAddr>,
    sayac: AtomicUsize,
}

impl SahteCozucu {
    fn tek(addr: SocketAddr) -> Self {
        Self {
            adresler: vec![addr],
            sayac: AtomicUsize::new(0),
        }
    }

    fn coklu(adresler: Vec<SocketAddr>) -> Self {
        Self {
            adresler,
            sayac: AtomicUsize::new(0),
        }
    }

    fn cagri_sayisi(&self) -> usize {
        self.sayac.load(Ordering::SeqCst)
    }
}

impl Cozucu for SahteCozucu {
    fn coz<'a>(
        &'a self,
        _host: &'a str,
        _port: u16,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = io::Result<Vec<SocketAddr>>> + Send + 'a>>
    {
        self.sayac.fetch_add(1, Ordering::SeqCst);
        let adresler = self.adresler.clone();
        Box::pin(async move { Ok(adresler) })
    }
}

/// Sentetik bir TLS ClientHello kaydı üretir (SNI ile ya da SNI'siz).
fn sentetik_client_hello(sni: Option<&str>) -> Vec<u8> {
    let mut ext_bytes = Vec::new();
    if let Some(host) = sni {
        let mut sni_data = Vec::new();
        let host_bytes = host.as_bytes();
        let list_len = (host_bytes.len() + 3) as u16;
        sni_data.extend_from_slice(&list_len.to_be_bytes());
        sni_data.push(0); // HostName type
        sni_data.extend_from_slice(&(host_bytes.len() as u16).to_be_bytes());
        sni_data.extend_from_slice(host_bytes);

        ext_bytes.extend_from_slice(&0u16.to_be_bytes()); // UZANTI_SNI (0x0000)
        ext_bytes.extend_from_slice(&(sni_data.len() as u16).to_be_bytes());
        ext_bytes.extend_from_slice(&sni_data);
    }

    let mut handshake = Vec::new();
    handshake.push(0x01); // ClientHello
    handshake.extend_from_slice(&[0, 0, 0]);
    handshake.extend_from_slice(&[0x03, 0x03]); // TLS 1.2
    handshake.extend_from_slice(&[0xAA; 32]); // Random
    handshake.push(0x00); // Session ID len = 0
    handshake.extend_from_slice(&2u16.to_be_bytes()); // Cipher suites len = 2
    handshake.extend_from_slice(&[0x00, 0x2F]); // TLS_RSA_WITH_AES_128_CBC_SHA
    handshake.push(0x01); // Compression methods len = 1
    handshake.push(0x00); // null compression

    handshake.extend_from_slice(&(ext_bytes.len() as u16).to_be_bytes());
    handshake.extend_from_slice(&ext_bytes);

    let hs_len = handshake.len() - 4;
    handshake[1] = ((hs_len >> 16) & 0xFF) as u8;
    handshake[2] = ((hs_len >> 8) & 0xFF) as u8;
    handshake[3] = (hs_len & 0xFF) as u8;

    let mut record = Vec::new();
    record.push(0x16); // Handshake
    record.extend_from_slice(&[0x03, 0x01]); // TLS 1.0 legacy
    record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
    record.extend_from_slice(&handshake);
    record
}

#[tokio::test]
async fn izinli_host_aktarilir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut soket, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 1024];
            let n = soket.read(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"SELAM");
            soket.write_all(b"MERHABA").await.unwrap();
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    let (istemci_akis, sunucu_akis) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu_akis);

    let mut istemci = istemci_akis;
    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\nHost: index.crates.io:443\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    let yanit_metni = String::from_utf8_lossy(&yanit[..n]);
    assert!(yanit_metni.starts_with("HTTP/1.1 200"));

    istemci.write_all(b"SELAM").await.unwrap();
    let mut gelen = [0u8; 1024];
    let n = istemci.read(&mut gelen).await.unwrap();
    assert_eq!(&gelen[..n], b"MERHABA");

    drop(istemci);
    tokio::time::sleep(Duration::from_millis(50)).await;

    let ozet = vekil.bitir().await;
    assert_eq!(ozet.baglantilar.len(), 1);
    let b = &ozet.baglantilar[0];
    assert_eq!(b.host, "index.crates.io");
    assert_eq!(b.port, 443);
    assert_eq!(b.karar, AgKarari::Izinli);
    assert!(b.giden > 0);
    assert!(b.gelen > 0);
}

#[tokio::test]
async fn http_1_0_connect_kabul_edilir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut soket, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 16];
            let _ = soket.read(&mut buf).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu_akis) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu_akis);

    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.0\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    let yanit_metni = String::from_utf8_lossy(&yanit[..n]);
    assert!(yanit_metni.contains("200 Connection Established") || yanit_metni.contains("200 OK"));

    drop(istemci);
    let ozet = vekil.bitir().await;
    assert_eq!(ozet.baglantilar.len(), 1);
    assert_eq!(ozet.baglantilar[0].karar, AgKarari::Izinli);
}

#[tokio::test]
async fn listede_olmayan_host_403_alir() {
    let cozucu = Arc::new(SahteCozucu::tek("127.0.0.1:80".parse().unwrap()));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu: cozucu.clone(),
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: false,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu_akis) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu_akis);

    istemci
        .write_all(b"CONNECT evil.com:443 HTTP/1.1\r\nHost: evil.com:443\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    let yanit_metni = String::from_utf8_lossy(&yanit[..n]);
    assert!(yanit_metni.starts_with("HTTP/1.1 403"));
    assert!(yanit_metni.contains("evil.com"));

    assert_eq!(cozucu.cagri_sayisi(), 0);

    drop(istemci);
    let ozet = vekil.bitir().await;
    assert_eq!(ozet.baglantilar.len(), 1);
    assert_eq!(ozet.baglantilar[0].karar, AgKarari::ListedeYok);
}

#[tokio::test]
async fn ip_ile_connect_reddedilir_ve_listede_ip_olarak_yazilan_adres_gecer() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut b = [0u8; 16];
            let _ = s.read(&mut b).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    // 1. IP listede yokken
    let ayar1 = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu: cozucu.clone(),
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: false,
    };
    let vekil1 = Vekil::baslat(ayar1);

    let (mut istemci1, sunucu1) = tokio::io::duplex(4096);
    vekil1.baglanti_ver(sunucu1);
    istemci1
        .write_all(b"CONNECT 93.184.216.34:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit1 = [0u8; 1024];
    let n1 = istemci1.read(&mut yanit1).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit1[..n1]).starts_with("HTTP/1.1 403"));
    drop(istemci1);
    let ozet1 = vekil1.bitir().await;
    assert_eq!(ozet1.baglantilar[0].karar, AgKarari::IpIleIstek);

    // 2. IP listede açıkça yazılmışsa geçer
    let kullanici_kurali = vec![HostKurali {
        host: "93.184.216.34".to_string(),
        port: 443,
        ozel_ag: false,
        kaynak: HostKaynagi::Kullanici,
    }];
    let ayar2 = VekilAyari {
        liste: IzinListesi::yeni(&kullanici_kurali, &[]),
        cozucu: cozucu.clone(),
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil2 = Vekil::baslat(ayar2);

    let (mut istemci2, sunucu2) = tokio::io::duplex(4096);
    vekil2.baglanti_ver(sunucu2);
    istemci2
        .write_all(b"CONNECT 93.184.216.34:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit2 = [0u8; 1024];
    let n2 = istemci2.read(&mut yanit2).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit2[..n2]).starts_with("HTTP/1.1 200"));
    drop(istemci2);
    let ozet2 = vekil2.bitir().await;
    assert_eq!(ozet2.baglantilar[0].karar, AgKarari::Izinli);
}

#[tokio::test]
async fn yasak_adrese_cozulen_host_baglanmaz() {
    let cozucu = Arc::new(SahteCozucu::coklu(vec![
        "127.0.0.1:443".parse().unwrap(),
        "10.0.0.1:443".parse().unwrap(),
    ]));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: false,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu_akis) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu_akis);

    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit[..n]).starts_with("HTTP/1.1 403"));

    drop(istemci);
    let ozet = vekil.bitir().await;
    assert_eq!(ozet.baglantilar.len(), 1);
    assert_eq!(
        ozet.baglantilar[0].karar,
        AgKarari::YasakAdres(AdresReddi::Yerel)
    );
}

#[tokio::test]
async fn ad_bir_kez_cozulur() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 16];
            let _ = s.read(&mut buf).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu: cozucu.clone(),
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu_akis) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu_akis);

    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let _ = istemci.read(&mut yanit).await.unwrap();

    assert_eq!(cozucu.cagri_sayisi(), 1);

    drop(istemci);
    let _ = vekil.bitir().await;
}

#[tokio::test]
async fn sni_farkliysa_kesilir_sni_yoksa_kesilir_tls_olmayan_akis_tasinir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 2048];
            if let Some(n) = s.read(&mut buf).await.ok().filter(|&n| n > 0) {
                let _ = s.write_all(&buf[..n]).await;
            }
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));

    // 1. SNI farklıysa kesilir
    {
        let ayar = VekilAyari {
            liste: IzinListesi::yeni(&[], &[]),
            cozucu: cozucu.clone(),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: true,
        };
        let vekil = Vekil::baslat(ayar);
        let (mut istemci, sunucu) = tokio::io::duplex(4096);
        vekil.baglanti_ver(sunucu);

        istemci
            .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        let mut yanit = [0u8; 1024];
        let _ = istemci.read(&mut yanit).await.unwrap();

        // Yanlış SNI gönder
        let ch = sentetik_client_hello(Some("farkli.com"));
        istemci.write_all(&ch).await.unwrap();

        // Vekil bağlantıyı kesmeli (EOF dönmeli)
        let n = istemci.read(&mut yanit).await.unwrap();
        assert_eq!(n, 0, "Farklı SNI durumunda bağlantı kesilmeli");

        let ozet = vekil.bitir().await;
        assert_eq!(
            ozet.baglantilar[0].karar,
            AgKarari::SniUyusmuyor {
                sni: Some("farkli.com".to_string())
            }
        );
    }

    // 2. SNI yoksa kesilir
    {
        let ayar = VekilAyari {
            liste: IzinListesi::yeni(&[], &[]),
            cozucu: cozucu.clone(),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: true,
        };
        let vekil = Vekil::baslat(ayar);
        let (mut istemci, sunucu) = tokio::io::duplex(4096);
        vekil.baglanti_ver(sunucu);

        istemci
            .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        let mut yanit = [0u8; 1024];
        let _ = istemci.read(&mut yanit).await.unwrap();

        // SNI'siz ClientHello gönder
        let ch = sentetik_client_hello(None);
        istemci.write_all(&ch).await.unwrap();

        let n = istemci.read(&mut yanit).await.unwrap();
        assert_eq!(n, 0, "SNI yoksa bağlantı kesilmeli");

        let ozet = vekil.bitir().await;
        assert_eq!(
            ozet.baglantilar[0].karar,
            AgKarari::SniUyusmuyor { sni: None }
        );
    }

    // 3. TLS olmayan akış taşınır
    {
        let ayar = VekilAyari {
            liste: IzinListesi::yeni(&[], &[]),
            cozucu: cozucu.clone(),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: true,
        };
        let vekil = Vekil::baslat(ayar);
        let (mut istemci, sunucu) = tokio::io::duplex(4096);
        vekil.baglanti_ver(sunucu);

        istemci
            .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
            .await
            .unwrap();
        let mut yanit = [0u8; 1024];
        let _ = istemci.read(&mut yanit).await.unwrap();

        // Düz baytlar gönder
        istemci.write_all(b"DUZ METIN VERISI").await.unwrap();
        let mut gelen = [0u8; 1024];
        let n = istemci.read(&mut gelen).await.unwrap();
        assert_eq!(&gelen[..n], b"DUZ METIN VERISI");

        drop(istemci);
        let ozet = vekil.bitir().await;
        assert_eq!(ozet.baglantilar[0].karar, AgKarari::Izinli);
    }
}

#[tokio::test]
async fn mutlak_http_yalnizca_80_listedeyse_gecer() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
            let _ = s
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK")
                .await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));

    // 1. Port 80 listede yok (varsayılan listede 443 var): 403 alır
    {
        let ayar = VekilAyari {
            liste: IzinListesi::yeni(&[], &[]),
            cozucu: cozucu.clone(),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: true,
        };
        let vekil = Vekil::baslat(ayar);
        let (mut istemci, sunucu) = tokio::io::duplex(4096);
        vekil.baglanti_ver(sunucu);

        istemci
            .write_all(
                b"GET http://index.crates.io/index.html HTTP/1.1\r\nHost: index.crates.io\r\n\r\n",
            )
            .await
            .unwrap();

        let mut yanit = [0u8; 1024];
        let n = istemci.read(&mut yanit).await.unwrap();
        assert!(String::from_utf8_lossy(&yanit[..n]).starts_with("HTTP/1.1 403"));

        drop(istemci);
        let ozet = vekil.bitir().await;
        assert_eq!(ozet.baglantilar[0].karar, AgKarari::ListedeYok);
    }

    // 2. Port 80 listede eklendiğinde geçer
    {
        let kural80 = vec![HostKurali {
            host: "index.crates.io".to_string(),
            port: 80,
            ozel_ag: false,
            kaynak: HostKaynagi::Kullanici,
        }];
        let ayar = VekilAyari {
            liste: IzinListesi::yeni(&kural80, &[]),
            cozucu: cozucu.clone(),
            azami_baglanti: 64,
            bosta_kalma: Duration::from_secs(300),
            yerel_izin: true,
        };
        let vekil = Vekil::baslat(ayar);
        let (mut istemci, sunucu) = tokio::io::duplex(4096);
        vekil.baglanti_ver(sunucu);

        istemci
            .write_all(
                b"GET http://index.crates.io/index.html HTTP/1.1\r\nHost: index.crates.io\r\n\r\n",
            )
            .await
            .unwrap();

        let mut yanit = [0u8; 1024];
        let n = istemci.read(&mut yanit).await.unwrap();
        assert!(String::from_utf8_lossy(&yanit[..n]).starts_with("HTTP/1.1 200"));

        drop(istemci);
        let ozet = vekil.bitir().await;
        assert_eq!(ozet.baglantilar[0].karar, AgKarari::Izinli);
    }
}

#[tokio::test]
async fn azami_baglanti_asilinca_yeni_baglanti_kapanir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 1,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    // 1. bağlantı açık
    let (mut istemci1, sunucu1) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu1);
    istemci1
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();
    let mut yanit1 = [0u8; 1024];
    let n1 = istemci1.read(&mut yanit1).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit1[..n1]).starts_with("HTTP/1.1 200"));

    // 2. bağlantı azami bağlantı aşıldığı için hemen kapanmalı
    let (mut istemci2, sunucu2) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu2);

    let mut yanit2 = [0u8; 1024];
    let n2 = istemci2.read(&mut yanit2).await.unwrap();
    assert_eq!(n2, 0, "Azami aşıldığında yeni akış kapatılmalı");

    drop(istemci1);
    drop(istemci2);
    let _ = vekil.bitir().await;
}

#[tokio::test]
async fn bosta_kalan_baglanti_kapanir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 64,
        bosta_kalma: Duration::from_millis(50),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu);
    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit[..n]).starts_with("HTTP/1.1 200"));

    tokio::time::sleep(Duration::from_millis(150)).await;

    let n = istemci.read(&mut yanit).await.unwrap();
    assert_eq!(n, 0, "Boşta kalan bağlantı kapanmalı");

    drop(istemci);
    let _ = vekil.bitir().await;
}

#[tokio::test]
async fn bitir_acik_baglantilari_kapatir() {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let yerel_addr = dinleyici.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut s, _)) = dinleyici.accept().await {
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf).await;
        }
    });

    let cozucu = Arc::new(SahteCozucu::tek(yerel_addr));
    let ayar = VekilAyari {
        liste: IzinListesi::yeni(&[], &[]),
        cozucu,
        azami_baglanti: 64,
        bosta_kalma: Duration::from_secs(300),
        yerel_izin: true,
    };
    let vekil = Vekil::baslat(ayar);

    let (mut istemci, sunucu) = tokio::io::duplex(4096);
    vekil.baglanti_ver(sunucu);
    istemci
        .write_all(b"CONNECT index.crates.io:443 HTTP/1.1\r\n\r\n")
        .await
        .unwrap();

    let mut yanit = [0u8; 1024];
    let n = istemci.read(&mut yanit).await.unwrap();
    assert!(String::from_utf8_lossy(&yanit[..n]).starts_with("HTTP/1.1 200"));

    let ozet = vekil.bitir().await;
    assert_eq!(ozet.baglantilar.len(), 1);

    let n = istemci.read(&mut yanit).await.unwrap();
    assert_eq!(n, 0, "Vekil bitince bağlantı kapatılmalı");
}
