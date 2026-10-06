use std::net::IpAddr;

use kalfa_sandbox::vekil::{
    AdresReddi, HostKaynagi, HostKurali, IzinListesi, VARSAYILAN_HOSTLAR, adres_izinli_mi,
    ag_hostu_coz,
};

#[test]
fn varsayilan_liste_tasarimdaki_hostlar() {
    assert_eq!(VARSAYILAN_HOSTLAR.len(), 8);
    let beklenen = [
        "index.crates.io",
        "static.crates.io",
        "registry.npmjs.org",
        "pypi.org",
        "files.pythonhosted.org",
        "codeload.github.com",
        "objects.githubusercontent.com",
        "raw.githubusercontent.com",
    ];
    for host in &beklenen {
        assert!(
            VARSAYILAN_HOSTLAR.contains(host),
            "{host} varsayılan listede olmalı"
        );
    }

    let liste = IzinListesi::yeni(&[], &[]);
    for host in &beklenen {
        let kural = liste.bul(host, 443);
        assert!(kural.is_some(), "{host}:443 bulunmalı");
        let k = kural.unwrap();
        assert_eq!(k.kaynak, HostKaynagi::Varsayilan);
        assert!(!k.ozel_ag);
    }

    // Listede olmaması gerekenler
    assert!(!VARSAYILAN_HOSTLAR.contains(&"github.com"));
    assert!(!VARSAYILAN_HOSTLAR.contains(&"api.github.com"));
    assert!(!VARSAYILAN_HOSTLAR.contains(&"crates.io"));
    assert!(!VARSAYILAN_HOSTLAR.contains(&"upload.pypi.org"));

    assert!(liste.bul("github.com", 443).is_none());
    assert!(liste.bul("api.github.com", 443).is_none());
    assert!(liste.bul("crates.io", 443).is_none());
    assert!(liste.bul("upload.pypi.org", 443).is_none());
}

#[test]
fn host_tam_eslesir() {
    let liste = IzinListesi::yeni(&[], &[]);

    // index.crates.io listede var, ama ön/arka eki olanlar tam eşleşmediği için bulunamaz
    assert!(liste.bul("evil-index.crates.io", 443).is_none());
    assert!(liste.bul("index.crates.io.evil.com", 443).is_none());

    // Büyük harfle aransa bile eşleşmeli (bul çağrısı küçük harfe dönüştürülmeli ya da küçük aranmalı)
    assert!(liste.bul("INDEX.CRATES.IO", 443).is_some());
    assert!(liste.bul("Index.Crates.Io", 443).is_some());
}

#[test]
fn port_varsayilani_443_ve_baska_port_eslesmez() {
    let liste = IzinListesi::yeni(&[], &[]);

    // Varsayılan listedeki host 443 ile var
    assert!(liste.bul("index.crates.io", 443).is_some());
    // Başka portlar eşleşmez
    assert!(liste.bul("index.crates.io", 80).is_none());
    assert!(liste.bul("index.crates.io", 8080).is_none());
    assert!(liste.bul("index.crates.io", 8443).is_none());

    // ag_hostu_coz varsayılan olarak 443 verir
    let (h, p) = ag_hostu_coz("example.com").unwrap();
    assert_eq!(h, "example.com");
    assert_eq!(p, 443);

    let (h, p) = ag_hostu_coz("example.com:8080").unwrap();
    assert_eq!(h, "example.com");
    assert_eq!(p, 8080);
}

#[test]
fn gecersiz_host_reddedilir() {
    // Joker kabul edilmez
    assert!(ag_hostu_coz("*.github.com").is_err());
    assert!(ag_hostu_coz("github.*.com").is_err());
    assert!(ag_hostu_coz("*").is_err());

    // Boş etiket kabul edilmez
    assert!(ag_hostu_coz("a..b").is_err());
    assert!(ag_hostu_coz(".example.com").is_err());
    assert!(ag_hostu_coz("example.com.").is_err());

    // 253 karakterden uzun ad kabul edilmez
    let cok_uzun_etiket = "a".repeat(63);
    let cok_uzun_ad =
        format!("{cok_uzun_etiket}.{cok_uzun_etiket}.{cok_uzun_etiket}.{cok_uzun_etiket}.com");
    assert!(cok_uzun_ad.len() > 253);
    assert!(ag_hostu_coz(&cok_uzun_ad).is_err());

    // 0 portu ve 65535'ten büyük port kabul edilmez
    assert!(ag_hostu_coz("host:0").is_err());
    assert!(ag_hostu_coz("host:99999").is_err());
    assert!(ag_hostu_coz("host:abc").is_err());

    // ASCII olmayan ad kabul edilmez
    assert!(ag_hostu_coz("örnek.com").is_err());
    assert!(ag_hostu_coz("türkçe.org:443").is_err());

    // Boş metin kabul edilmez
    assert!(ag_hostu_coz("").is_err());
    assert!(ag_hostu_coz(":443").is_err());

    // Punycode geçerli
    assert!(ag_hostu_coz("xn--rnek-5qa.com").is_ok());
}

#[test]
fn yasak_adresler() {
    let yasaklar: &[&str] = &[
        "127.0.0.1",
        "::1",
        "169.254.169.254",
        "fe80::1",
        "224.0.0.1",
        "ff02::1",
        "0.0.0.0",
        "::",
    ];

    for ip_str in yasaklar {
        let ip: IpAddr = ip_str.parse().unwrap();
        // ozel_ag: false iken Yerel hatası
        let res_false = adres_izinli_mi(ip, false);
        assert_eq!(
            res_false,
            Err(AdresReddi::Yerel),
            "{ip_str} ozel_ag: false iken Yerel ile reddedilmeli"
        );

        // ozel_ag: true olsa da yasak
        let res_true = adres_izinli_mi(ip, true);
        assert_eq!(
            res_true,
            Err(AdresReddi::Yerel),
            "{ip_str} ozel_ag: true iken de Yerel ile reddedilmeli"
        );
    }
}

#[test]
fn ozel_ag_yalnizca_isaretle() {
    let ozel_ipler: &[&str] = &[
        "10.1.2.3",
        "172.16.0.1",
        "192.168.1.1",
        "100.64.0.1",
        "fd00::1",
    ];

    for ip_str in ozel_ipler {
        let ip: IpAddr = ip_str.parse().unwrap();

        // İşaretsiz (ozel_ag: false) yasak
        assert_eq!(
            adres_izinli_mi(ip, false),
            Err(AdresReddi::OzelAg),
            "{ip_str} ozel_ag: false iken OzelAg ile reddedilmeli"
        );

        // İşaretli (ozel_ag: true) izinli
        assert!(
            adres_izinli_mi(ip, true).is_ok(),
            "{ip_str} ozel_ag: true iken izinli olmalı"
        );
    }

    // Normal bir genel IP hem işaretsiz hem işaretli izinlidir
    let genel_ip: IpAddr = "8.8.8.8".parse().unwrap();
    assert!(adres_izinli_mi(genel_ip, false).is_ok());
    assert!(adres_izinli_mi(genel_ip, true).is_ok());
}

#[test]
fn ipv4e_eslenmis_ipv6_ipv4_gibi_denetlenir() {
    let loopback_mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
    assert_eq!(
        adres_izinli_mi(loopback_mapped, false),
        Err(AdresReddi::Yerel)
    );
    assert_eq!(
        adres_izinli_mi(loopback_mapped, true),
        Err(AdresReddi::Yerel)
    );

    let private_mapped: IpAddr = "::ffff:10.0.0.1".parse().unwrap();
    assert_eq!(
        adres_izinli_mi(private_mapped, false),
        Err(AdresReddi::OzelAg)
    );
    assert!(adres_izinli_mi(private_mapped, true).is_ok());

    let public_mapped: IpAddr = "::ffff:93.184.216.34".parse().unwrap();
    assert!(adres_izinli_mi(public_mapped, false).is_ok());
    assert!(adres_izinli_mi(public_mapped, true).is_ok());
}

#[test]
fn cagri_hostu_ozel_aga_isaretlenemez() {
    let cagri = vec![("dahili.sirket.com".to_string(), 443)];
    let liste = IzinListesi::yeni(&[], &cagri);

    let kural = liste.bul("dahili.sirket.com", 443).unwrap();
    assert_eq!(kural.kaynak, HostKaynagi::Cagri);
    assert!(
        !kural.ozel_ag,
        "Çağrı hostunda ozel_ag her zaman false olmalı"
    );

    // Kullanıcı hostu ozel_ag: true alabilir
    let kullanici = vec![HostKurali {
        host: "dahili.sirket.com".to_string(),
        port: 443,
        ozel_ag: true,
        kaynak: HostKaynagi::Kullanici,
    }];
    let liste2 = IzinListesi::yeni(&kullanici, &[]);
    let kural2 = liste2.bul("dahili.sirket.com", 443).unwrap();
    assert_eq!(kural2.kaynak, HostKaynagi::Kullanici);
    assert!(kural2.ozel_ag);
}

#[test]
fn ozet_kararli_ve_liste_degisince_degisir() {
    let k1 = HostKurali {
        host: "b.com".to_string(),
        port: 443,
        ozel_ag: false,
        kaynak: HostKaynagi::Kullanici,
    };
    let k2 = HostKurali {
        host: "a.com".to_string(),
        port: 443,
        ozel_ag: false,
        kaynak: HostKaynagi::Kullanici,
    };

    let liste_1 = IzinListesi::yeni(&[k1.clone(), k2.clone()], &[]);
    let liste_2 = IzinListesi::yeni(&[k2.clone(), k1.clone()], &[]);

    // Sıra farkı özeti değiştirmez
    assert_eq!(liste_1.ozet(), liste_2.ozet());

    // Host değişince özet değişir
    let k3 = HostKurali {
        host: "c.com".to_string(),
        port: 443,
        ozel_ag: false,
        kaynak: HostKaynagi::Kullanici,
    };
    let liste_3 = IzinListesi::yeni(&[k1.clone(), k3], &[]);
    assert_ne!(liste_1.ozet(), liste_3.ozet());

    // Port değişince özet değişir
    let k1_port = HostKurali {
        host: "b.com".to_string(),
        port: 8443,
        ozel_ag: false,
        kaynak: HostKaynagi::Kullanici,
    };
    let liste_4 = IzinListesi::yeni(&[k1_port, k2.clone()], &[]);
    assert_ne!(liste_1.ozet(), liste_4.ozet());

    // ozel_ag işareti değişince özet değişir
    let k1_ozel = HostKurali {
        host: "b.com".to_string(),
        port: 443,
        ozel_ag: true,
        kaynak: HostKaynagi::Kullanici,
    };
    let liste_5 = IzinListesi::yeni(&[k1_ozel, k2], &[]);
    assert_ne!(liste_1.ozet(), liste_5.ozet());
}
