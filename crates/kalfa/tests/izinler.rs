//! Pencere izinleri (1c tasarımı §6, §11.2). Tauri'nin ACL'i mock runtime'da
//! da uygulanır: her pencere yalnızca kendi komutlarını çağırabilir, Tauri'nin
//! ve eklentilerin komutları hiçbir pencereye açık değildir.

use kalfa_kabuk::komut_adlari::{ANA_PENCERE_KOMUTLARI, SART_PENCERESI_KOMUTLARI};
use serde_json::Value;
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

fn istek(komut: &str) -> InvokeRequest {
    InvokeRequest {
        cmd: komut.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: if cfg!(windows) {
            "http://tauri.localhost"
        } else {
            "tauri://localhost"
        }
        .parse()
        .unwrap(),
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_string(),
    }
}

/// Komutun ACL'den geçip geçmediği. `Uygulama` durumu yönetilmediği için
/// izinli komutlar da hata döner; ama o hata ACL hatası değildir.
fn izinli(pencere: &WebviewWindow<MockRuntime>, komut: &str) -> bool {
    match get_ipc_response(pencere, istek(komut)) {
        Ok(_) => true,
        Err(hata) => {
            let metin = hata.to_string();
            assert!(
                !metin.contains("not found"),
                "{komut} komutu kayıtlı değil: {metin}"
            );
            !metin.contains("not allowed")
        }
    }
}

fn uygulama() -> App<MockRuntime> {
    kalfa_kabuk::komutlar::kur(kalfa_kabuk::eklentiler(mock_builder()))
        .build(tauri::generate_context!())
        .unwrap()
}

fn pencere(app: &App<MockRuntime>, etiket: &str) -> WebviewWindow<MockRuntime> {
    WebviewWindowBuilder::new(app, etiket, WebviewUrl::default())
        .build()
        .unwrap()
}

/// Tauri'nin ve eklentilerin, webview'dan çağrılırsa tehlikeli olacak
/// komutları.
const CEKIRDEK_KOMUTLARI: [&str; 8] = [
    "plugin:window|close",
    "plugin:webview|create_webview_window",
    "plugin:event|emit",
    "plugin:path|resolve_directory",
    "plugin:app|app_hide",
    "plugin:dialog|open",
    "plugin:dialog|message",
    "plugin:opener|open_url",
];

#[test]
fn her_pencere_yalnizca_kendi_komutlarini_cagirir() {
    let app = uygulama();
    let ana = pencere(&app, "ana");
    let sartlar = pencere(&app, "sartlar");

    for komut in ANA_PENCERE_KOMUTLARI {
        assert!(izinli(&ana, komut), "ana pencere {komut} çağıramadı");
    }
    for komut in SART_PENCERESI_KOMUTLARI {
        assert!(izinli(&sartlar, komut), "şart penceresi {komut} çağıramadı");
    }
    // Şart kabulü yalnızca şart penceresinden gider (üst belge §11.8).
    for komut in SART_PENCERESI_KOMUTLARI {
        if !ANA_PENCERE_KOMUTLARI.contains(&komut) {
            assert!(!izinli(&ana, komut), "ana pencere {komut} çağırabildi");
        }
    }
    for komut in ANA_PENCERE_KOMUTLARI {
        if !SART_PENCERESI_KOMUTLARI.contains(&komut) {
            assert!(
                !izinli(&sartlar, komut),
                "şart penceresi {komut} çağırabildi"
            );
        }
    }
}

#[test]
fn cekirdek_ve_eklenti_komutlari_kapali() {
    let app = uygulama();
    for etiket in ["ana", "sartlar"] {
        let p = pencere(&app, etiket);
        for komut in CEKIRDEK_KOMUTLARI {
            assert!(!izinli(&p, komut), "{etiket} penceresi {komut} çağırabildi");
        }
    }
}

#[test]
fn baska_bir_pencere_hicbir_komutu_cagiramaz() {
    let app = uygulama();
    let baska = pencere(&app, "baska");
    for komut in ANA_PENCERE_KOMUTLARI
        .iter()
        .chain(SART_PENCERESI_KOMUTLARI.iter())
    {
        assert!(
            !izinli(&baska, komut),
            "tanımsız pencere {komut} çağırabildi"
        );
    }
}

fn json(yol: &str) -> Value {
    let tam = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(yol);
    serde_json::from_str(&std::fs::read_to_string(tam).unwrap()).unwrap()
}

fn izin_adlari(komutlar: &[&str]) -> Vec<String> {
    komutlar
        .iter()
        .map(|k| format!("allow-{}", k.replace('_', "-")))
        .collect()
}

#[test]
fn capability_dosyalari_komut_listeleriyle_ayni() {
    for (dosya, pencere, komutlar) in [
        (
            "capabilities/ana-pencere.json",
            "ana",
            &ANA_PENCERE_KOMUTLARI[..],
        ),
        (
            "capabilities/sart-penceresi.json",
            "sartlar",
            &SART_PENCERESI_KOMUTLARI[..],
        ),
    ] {
        let c = json(dosya);
        assert_eq!(c["windows"], serde_json::json!([pencere]), "{dosya}");
        assert!(c.get("webviews").is_none(), "{dosya}");
        assert!(
            c.get("remote").is_none(),
            "{dosya}: uzak köken izinli olmamalı"
        );
        let izinler: Vec<String> = c["permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect();
        assert_eq!(izinler, izin_adlari(komutlar), "{dosya}");
    }
}

#[test]
fn yapilandirma_pencere_acmaz_ve_csp_sikidir() {
    let c = json("tauri.conf.json");
    // Pencereleri yalnızca Rust açar (1c tasarımı §6).
    assert_eq!(c["app"]["windows"], serde_json::json!([]));
    assert_eq!(c["app"]["withGlobalTauri"], Value::Bool(false));
    let g = &c["app"]["security"];
    assert_eq!(g["freezePrototype"], Value::Bool(true));
    let csp = g["csp"].as_str().unwrap();
    for parca in [
        "default-src 'none'",
        "script-src 'self'",
        "style-src 'self'",
        "connect-src ipc: http://ipc.localhost",
        "base-uri 'none'",
        "form-action 'none'",
    ] {
        assert!(csp.contains(parca), "CSP'de {parca} yok: {csp}");
    }
    assert!(!csp.contains("unsafe"), "sürüm CSP'si gevşek: {csp}");
    assert!(
        !csp.contains("http://localhost"),
        "sürüm CSP'si geliştirme kökenine açık: {csp}"
    );
    let dev = g["devCsp"].as_str().unwrap();
    assert!(
        !dev.contains("unsafe-eval"),
        "geliştirme CSP'si eval'e açık: {dev}"
    );
}
