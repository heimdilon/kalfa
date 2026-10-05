#![cfg(windows)]

use kalfa_core::geri_alma::GeriYukleyici;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::yol::CalismaYolu;
use kalfa_tools::CalismaAlani;
use std::sync::Arc;

/// Yalnız testin kendi geçici nesnesine kısa ad verir; sürücü ayarı değiştirmez.
fn kisa_ad_ver(yol: &std::path::Path) -> bool {
    let betik = r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class KalfaKisaAd {
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern SafeFileHandle CreateFileW(string p,uint a,uint s,IntPtr t,uint c,uint f,IntPtr h);
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
 public static extern bool SetFileShortNameW(SafeFileHandle h,string n);
}
'@
$h=[KalfaKisaAd]::CreateFileW($env:KALFA_KISA_AD_TESTI,0x10000,7,[IntPtr]::Zero,3,0x02000000,[IntPtr]::Zero)
try {
 if ($h.IsInvalid -or -not [KalfaKisaAd]::SetFileShortNameW($h,'KALFATST')) {
  Write-Output ([Runtime.InteropServices.Marshal]::GetLastWin32Error()); exit 2
 }
} finally { $h.Dispose() }
"#;
    let sonuc = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", betik])
        .env("KALFA_KISA_AD_TESTI", yol)
        .output()
        .unwrap();
    if !sonuc.status.success() {
        eprintln!(
            "Gerçek kısa ad testi çalışmadı (sürücü/yetki): {} {}",
            String::from_utf8_lossy(&sonuc.stdout),
            String::from_utf8_lossy(&sonuc.stderr)
        );
        return false;
    }
    true
}

#[test]
fn tildesiz_kisa_ad_gizli_ve_yapilandirma_sinirini_atlatamaz() {
    let mut sinanan = 0;
    for (ad, dizin) in [
        (".env.production", false),
        ("credentials-ozel", true),
        (".vscode", true),
    ] {
        let d = tempfile::tempdir().unwrap();
        let gercek = d.path().join(ad);
        let dosya = if dizin {
            std::fs::create_dir(&gercek).unwrap();
            gercek.join("token.txt")
        } else {
            gercek.clone()
        };
        std::fs::write(&dosya, b"BENZERSIZ_SIR=123").unwrap();
        if !kisa_ad_ver(&gercek) {
            continue;
        }
        sinanan += 1;
        let a = CalismaAlani::ac(Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap())).unwrap();
        let takma = CalismaYolu::coz(if dizin {
            "KALFATST/token.txt"
        } else {
            "KALFATST"
        })
        .unwrap();
        assert_eq!(
            a.oku(&takma).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            a.yaz(&takma, b"degismemeli").unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(a.durum(&takma).is_err());
        assert!(
            a.geri_yaz(
                &takma,
                Some(b"degismemeli"),
                &ozet_hex(b"BENZERSIZ_SIR=123")
            )
            .is_err()
        );
        if dizin {
            assert!(
                a.dizin_dogrula(&CalismaYolu::coz("KALFATST").unwrap())
                    .is_err()
            );
            assert!(
                a.yaz(&CalismaYolu::coz("KALFATST/yeni.txt").unwrap(), b"yeni")
                    .is_err()
            );
            assert!(!gercek.join("yeni.txt").exists());
        }
        assert_eq!(std::fs::read(&dosya).unwrap(), b"BENZERSIZ_SIR=123");
    }
    eprintln!("Gerçek tildesiz kısa ad sınırı: {sinanan}/3 nesne sınandı.");
}

#[test]
fn gercek_tilde_adi_ve_harf_farki_normal_islemleri_korur() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir(d.path().join("Normal~Dizin")).unwrap();
    let a = CalismaAlani::ac(Arc::new(Siniflandirici::yeni(d.path(), &[]).unwrap())).unwrap();
    let yol = CalismaYolu::coz("NORMAL~DIZIN/not~1.txt").unwrap();
    a.yaz(&yol, b"normal").unwrap();
    assert_eq!(a.oku(&yol).unwrap(), b"normal");
    a.geri_yaz(&yol, Some(b"eski"), &ozet_hex(b"normal"))
        .unwrap();
    a.geri_yaz(&yol, None, &ozet_hex(b"eski")).unwrap();
}
