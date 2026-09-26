//! Gerçek AppContainer yalıtımı testleri (aşama 2 tasarımı §12.2, Windows).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use kalfa_sandbox::profil::{GitKimligi, OzelDizinler, alan_kimligi};
use kalfa_sandbox::windows::guvenlik::{
    self, Izin, MASKE_LISTELE, MASKE_OKU, MASKE_TAM, MIRAS_YOK, SISTEM, Sid, YONETICILER,
};
use kalfa_sandbox::windows::yalitici::{WindowsAyari, WindowsYalitici};
use kalfa_sandbox::{
    Akis, AlanBilgisi, BaslatmaHatasi, Bitis, CalistirmaIstegi, KomutBicimi, Korumali, Yalitici,
    YalitimDurumu, tarama,
};
use tokio_util::sync::CancellationToken;

const UZUN: Duration = Duration::from_secs(45);

struct Kurgu {
    gecici: Option<tempfile::TempDir>,
    taban: PathBuf,
    ev: PathBuf,
    yalitim_dizini: PathBuf,
    kok: PathBuf,
    baska_kok: PathBuf,
    y: WindowsYalitici,
}

struct Sonuc {
    bitis: Bitis,
    standart: String,
    hata: String,
}

fn windows_koku() -> PathBuf {
    std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

fn powershell_yolu() -> PathBuf {
    windows_koku().join(r"System32\WindowsPowerShell\v1.0\powershell.exe")
}

fn kurgu() -> Kurgu {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().to_path_buf();
    kurgu_olustur(Some(gecici), taban)
}

fn kurgu_paylasimli(taban: PathBuf) -> Kurgu {
    kurgu_olustur(None, taban)
}

fn kurgu_olustur(gecici: Option<tempfile::TempDir>, taban: PathBuf) -> Kurgu {
    let ev = taban.join("ev");
    let kok = taban.join("calisma-alani-a");
    let baska_kok = taban.join("calisma-alani-b");
    let yalitim_dizini = taban.join("yalitim");
    std::fs::create_dir_all(ev.join(r".cargo\bin")).unwrap();
    std::fs::create_dir_all(ev.join(".ssh")).unwrap();
    std::fs::create_dir_all(&kok).unwrap();
    std::fs::create_dir_all(&baska_kok).unwrap();
    let y = WindowsYalitici::yeni(
        WindowsAyari {
            ev: ev.clone(),
            yalitim_dizini: yalitim_dizini.clone(),
            ..WindowsAyari::default()
        },
        Box::new(|_| {}),
    );
    match y.durum() {
        YalitimDurumu::Hazir { .. } => {}
        durum => panic!("Windows yalıtımı hazır değil: {durum:?}"),
    }
    Kurgu {
        gecici,
        taban,
        ev,
        yalitim_dizini,
        kok,
        baska_kok,
        y,
    }
}

fn gizli_mi(yol: &Path, dizin: bool) -> bool {
    let ad = yol.file_name().and_then(|a| a.to_str()).unwrap_or("");
    if dizin {
        ad.eq_ignore_ascii_case("sirlar")
    } else {
        ad.eq_ignore_ascii_case(".env") || ad.to_ascii_lowercase().ends_with(".pem")
    }
}

fn alan_bilgisi(kok: &Path, korumali: Vec<Korumali>) -> AlanBilgisi {
    AlanBilgisi {
        kok: std::fs::canonicalize(kok).unwrap(),
        gizli: tarama::tara(kok, &gizli_mi),
        korumali,
        git_dizinleri: Vec::new(),
    }
}

impl Kurgu {
    fn hazirla(&self, kok: &Path, korumali: Vec<Korumali>) -> AlanBilgisi {
        let alan = alan_bilgisi(kok, korumali);
        let mut ilerleme = Vec::new();
        self.y
            .alani_hazirla(&alan, true, &mut |satir| ilerleme.push(satir.to_string()))
            .unwrap_or_else(|hata| panic!("çalışma alanı hazırlanamadı: {hata:?}; {ilerleme:?}"));
        alan
    }

    fn istek(&self, alan: &AlanBilgisi, ag: bool, metin: &str) -> CalistirmaIstegi {
        CalistirmaIstegi {
            komut: KomutBicimi::Program {
                program: powershell_yolu().display().to_string(),
                argumanlar: vec![
                    "-NoLogo".into(),
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    metin.into(),
                ],
            },
            dizin: alan.kok.clone(),
            ortam: vec![(
                "PATH".into(),
                windows_koku().join("System32").display().to_string(),
            )],
            ag,
            alan: Some(alan.clone()),
        }
    }

    async fn calistir(&self, alan: &AlanBilgisi, ag: bool, metin: &str) -> Sonuc {
        self.istegi_calistir(&self.istek(alan, ag, metin)).await
    }

    async fn istegi_calistir(&self, istek: &CalistirmaIstegi) -> Sonuc {
        let surec = self
            .y
            .baslat(istek)
            .unwrap_or_else(|hata| panic!("yalıtımlı komut başlatılamadı: {hata:?}"));
        let mut standart = Vec::new();
        let mut hata = Vec::new();
        let sonuc = surec
            .bekle(UZUN, CancellationToken::new(), |akis, veri| match akis {
                Akis::Standart => standart.extend_from_slice(veri),
                Akis::Hata => hata.extend_from_slice(veri),
            })
            .await;
        Sonuc {
            bitis: sonuc.bitis,
            standart: String::from_utf8_lossy(&standart).into_owned(),
            hata: String::from_utf8_lossy(&hata).into_owned(),
        }
    }

    fn temizle(&self, alanlar: &[AlanBilgisi]) {
        for alan in alanlar {
            let _ = self.y.alani_kaldir(alan);
        }
    }
}

impl Drop for Kurgu {
    fn drop(&mut self) {
        for kok in [&self.kok, &self.baska_kok] {
            let alan = alan_bilgisi(kok, Vec::new());
            let _ = self.y.alani_kaldir(&alan);
        }
        let _ = &self.gecici;
        let _ = &self.taban;
    }
}

fn deger<'a>(metin: &'a str, ad: &str) -> &'a str {
    metin
        .lines()
        .find_map(|satir| satir.strip_prefix(&format!("{ad}=")))
        .unwrap_or_else(|| panic!("{ad} yok: {metin}"))
}

fn ps(yol: &Path) -> String {
    format!("'{}'", yol.to_string_lossy().replace('\'', "''"))
}

fn junction_kur(yol: &Path, hedef: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // `arg` tırnağı `\"` diye kaçırır; cmd.exe bunu anlamaz.
    let komut = format!("mklink /J \"{}\" \"{}\"", yol.display(), hedef.display());
    let sonuc = Command::new(windows_koku().join("System32\\cmd.exe"))
        .args(["/d", "/c"])
        .raw_arg(komut)
        .output()
        .map_err(|hata| format!("cmd.exe başlatılamadı: {hata}"))?;
    if sonuc.status.success() {
        Ok(())
    } else {
        Err(format!(
            "junction kurulamadı: {}{}",
            String::from_utf8_lossy(&sonuc.stdout),
            String::from_utf8_lossy(&sonuc.stderr)
        ))
    }
}

const UI_KISITLARI_KODU: &str = r#"
using System;
using System.Runtime.InteropServices;

public static class KalfaUiDenemesi {
    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool OpenClipboard(IntPtr owner);
    [DllImport("user32.dll")]
    private static extern bool CloseClipboard();
    [DllImport("user32.dll")]
    private static extern IntPtr GetShellWindow();
    [DllImport("user32.dll", SetLastError = true)]
    private static extern IntPtr SendMessageTimeout(IntPtr window, uint message,
        UIntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);

    public static int PanoyaErisim() {
        if (OpenClipboard(IntPtr.Zero)) {
            CloseClipboard();
            return 0;
        }
        return Marshal.GetLastWin32Error();
    }

    public static int PencereyeIleti() {
        IntPtr window = GetShellWindow();
        if (window == IntPtr.Zero) return -1;
        IntPtr result;
        IntPtr sent = SendMessageTimeout(window, 0, UIntPtr.Zero, IntPtr.Zero,
            2, 1000, out result);
        return sent == IntPtr.Zero ? Marshal.GetLastWin32Error() : 1;
    }
}
"#;

const JUNCTION_KODU: &str = r#"
using System;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;

public static class KalfaJunctionDenemesi {
    [DllImport("kernel32.dll", EntryPoint = "CreateFileW", CharSet = CharSet.Unicode,
        SetLastError = true)]
    private static extern SafeFileHandle DosyaAc(string yol, uint erisim, uint paylasim,
        IntPtr guvenlik, uint acilis, uint bayraklar, IntPtr sablon);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool DeviceIoControl(SafeFileHandle tutamac, uint kod,
        byte[] girdi, uint girdiUzunlugu, IntPtr cikti, uint ciktiUzunlugu,
        out uint baytSayisi, IntPtr ustuneYazim);

    public static int KokYenidenAyristirmaNoktasiKur(string kok, string hedef) {
        using (SafeFileHandle tutamac = DosyaAc(kok, 0x40000000, 7, IntPtr.Zero,
                3, 0x02200000, IntPtr.Zero)) {
            if (tutamac.IsInvalid) return Marshal.GetLastWin32Error();

            string yedekAd = @"\??\" + Path.GetFullPath(hedef);
            string gorunenAd = Path.GetFullPath(hedef);
            byte[] yedek = Encoding.Unicode.GetBytes(yedekAd);
            byte[] gorunen = Encoding.Unicode.GetBytes(gorunenAd);
            ushort veriUzunlugu = (ushort)(8 + yedek.Length + 2 + gorunen.Length + 2);
            byte[] veri = new byte[8 + veriUzunlugu];
            Buffer.BlockCopy(BitConverter.GetBytes(0xA0000003u), 0, veri, 0, 4);
            Buffer.BlockCopy(BitConverter.GetBytes(veriUzunlugu), 0, veri, 4, 2);
            Buffer.BlockCopy(BitConverter.GetBytes((ushort)0), 0, veri, 6, 2);
            Buffer.BlockCopy(BitConverter.GetBytes((ushort)0), 0, veri, 8, 2);
            Buffer.BlockCopy(BitConverter.GetBytes((ushort)yedek.Length), 0, veri, 10, 2);
            Buffer.BlockCopy(BitConverter.GetBytes((ushort)(yedek.Length + 2)), 0, veri, 12, 2);
            Buffer.BlockCopy(BitConverter.GetBytes((ushort)gorunen.Length), 0, veri, 14, 2);
            Buffer.BlockCopy(yedek, 0, veri, 16, yedek.Length);
            Buffer.BlockCopy(gorunen, 0, veri, 18 + yedek.Length, gorunen.Length);
            uint baytSayisi;
            return DeviceIoControl(tutamac, 0x000900A4, veri, (uint)veri.Length,
                IntPtr.Zero, 0, out baytSayisi, IntPtr.Zero)
                ? 0 : Marshal.GetLastWin32Error();
        }
    }
}
"#;

const BASKA_SID_KODU: &str = r#"
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class KalfaSidDenemesi {
    [StructLayout(LayoutKind.Sequential)]
    private struct BaslangicBilgisi {
        public int cb;
        public IntPtr rezerve;
        public IntPtr masaustu;
        public IntPtr baslik;
        public int x, y, xBoyut, yBoyut, xKarakter, yKarakter;
        public int dolgu;
        public int bayraklar;
        public short gosterim;
        public short rezerveUzunluk;
        public IntPtr rezerveBaytlar;
        public IntPtr standartGirdi, standartCikti, standartHata;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct BaslangicBilgisiEk {
        public BaslangicBilgisi baslangic;
        public IntPtr nitelikler;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct GuvenlikYetenekleri {
        public IntPtr paketSidi;
        public IntPtr yetenekler;
        public uint yetenekSayisi;
        public uint ayrilmis;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct SurecBilgisi {
        public IntPtr surec, isParcasi;
        public uint surecKimligi, isParcasiKimligi;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool InitializeProcThreadAttributeList(IntPtr liste,
        int adet, int bayraklar, ref IntPtr baytSayisi);
    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool UpdateProcThreadAttribute(IntPtr liste,
        uint bayraklar, IntPtr nitelik, ref GuvenlikYetenekleri deger,
        IntPtr degerBoyutu, IntPtr onceki, IntPtr oncekiBoyut);
    [DllImport("kernel32.dll")]
    private static extern void DeleteProcThreadAttributeList(IntPtr liste);
    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool ConvertStringSidToSidW(string sid, out IntPtr sonuc);
    [DllImport("kernel32.dll")]
    private static extern IntPtr LocalFree(IntPtr bellek);
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern bool CreateProcessW(string program, StringBuilder komut,
        IntPtr surecGuvenlik, IntPtr isGuvenlik, bool tutamaclariDevral,
        uint bayraklar, IntPtr ortam, string dizin,
        ref BaslangicBilgisiEk baslangic, out SurecBilgisi surec);
    [DllImport("kernel32.dll")]
    private static extern bool TerminateProcess(IntPtr surec, uint kod);
    [DllImport("kernel32.dll")]
    private static extern bool CloseHandle(IntPtr tutamac);

    public static int BaskaPaketleBaslat(string sid, string cmd, string dizin) {
        IntPtr sidBellek = IntPtr.Zero;
        IntPtr liste = IntPtr.Zero;
        bool listeKuruldu = false;
        try {
            if (!ConvertStringSidToSidW(sid, out sidBellek))
                return Marshal.GetLastWin32Error();
            IntPtr baytSayisi = IntPtr.Zero;
            InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref baytSayisi);
            liste = Marshal.AllocHGlobal(baytSayisi);
            if (!InitializeProcThreadAttributeList(liste, 1, 0, ref baytSayisi))
                return Marshal.GetLastWin32Error();
            listeKuruldu = true;
            GuvenlikYetenekleri yetenekler = new GuvenlikYetenekleri();
            yetenekler.paketSidi = sidBellek;
            if (!UpdateProcThreadAttribute(liste, 0, new IntPtr(0x00020009),
                    ref yetenekler, new IntPtr(Marshal.SizeOf(typeof(GuvenlikYetenekleri))),
                    IntPtr.Zero, IntPtr.Zero))
                return Marshal.GetLastWin32Error();
            BaslangicBilgisiEk baslangic = new BaslangicBilgisiEk();
            baslangic.baslangic.cb = Marshal.SizeOf(typeof(BaslangicBilgisiEk));
            baslangic.nitelikler = liste;
            SurecBilgisi surec;
            string komutSatiri = "\"" + cmd + "\" /d /c exit 0";
            bool basladi = CreateProcessW(cmd, new StringBuilder(komutSatiri),
                IntPtr.Zero, IntPtr.Zero, false, 0x00080004 | 0x08000000,
                IntPtr.Zero, dizin, ref baslangic, out surec);
            if (!basladi) return Marshal.GetLastWin32Error();
            TerminateProcess(surec.surec, 1);
            CloseHandle(surec.surec);
            CloseHandle(surec.isParcasi);
            return 0;
        } finally {
            if (listeKuruldu) DeleteProcThreadAttributeList(liste);
            if (liste != IntPtr.Zero) Marshal.FreeHGlobal(liste);
            if (sidBellek != IntPtr.Zero) LocalFree(sidBellek);
        }
    }
}
"#;

const PROFIL_KODU: &str = r#"
using System;
using System.Runtime.InteropServices;
public static class KalfaProfilDenemesi {
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)]
    public static extern int CreateAppContainerProfile(string name,
        string displayName, string description, IntPtr capabilities,
        uint capabilityCount, out IntPtr sid);
    [DllImport("userenv.dll", CharSet = CharSet.Unicode)]
    public static extern int DeleteAppContainerProfile(string name);
    [DllImport("advapi32.dll")]
    public static extern IntPtr FreeSid(IntPtr sid);
}
"#;

fn appcontainer_profil_sonucu(ad: &str, sil_yeniden_olusturulani: bool) -> i32 {
    let sil = if sil_yeniden_olusturulani {
        format!(
            "if ($hr -eq 0) {{ [KalfaProfilDenemesi]::DeleteAppContainerProfile({}) | Out-Null }}",
            ps(Path::new(ad))
        )
    } else {
        String::new()
    };
    let betik = format!(
        "Add-Type -TypeDefinition @'\n{PROFIL_KODU}\n'@; $sid = [IntPtr]::Zero; $hr = [KalfaProfilDenemesi]::CreateAppContainerProfile({}, 'Kalfa test', 'Profil temizleme testi', [IntPtr]::Zero, 0, [ref]$sid); if ($sid -ne [IntPtr]::Zero) {{ [KalfaProfilDenemesi]::FreeSid($sid) | Out-Null }}; 'HR=' + $hr; {sil}",
        ps(Path::new(ad)),
    );
    let sonuc = Command::new(powershell_yolu())
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
        .arg(betik)
        .output()
        .unwrap();
    assert!(
        sonuc.status.success(),
        "profil Win32 denemesi başarısız: {}",
        String::from_utf8_lossy(&sonuc.stderr)
    );
    let cikti = String::from_utf8_lossy(&sonuc.stdout);
    deger(&cikti, "HR").parse().unwrap()
}

#[tokio::test]
async fn evdeki_ssh_ve_cargo_kimlik_bilgileri_okunamaz() {
    let k = kurgu();
    std::fs::write(k.ev.join(r".ssh\id_rsa"), "GIZLI-SSH").unwrap();
    std::fs::create_dir_all(k.ev.join(".cargo")).unwrap();
    std::fs::write(k.ev.join(r".cargo\credentials.toml"), "GIZLI-CARGO").unwrap();
    let alan = k.hazirla(&k.kok, Vec::new());
    let betik = format!(
        "foreach ($p in @({}, {})) {{ try {{ [IO.File]::ReadAllText($p); 'OKUNDU' }} catch {{ 'ENGELLENDI' }} }}",
        ps(&k.ev.join(r".ssh\id_rsa")),
        ps(&k.ev.join(r".cargo\credentials.toml")),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert_eq!(
        sonuc.standart.matches("ENGELLENDI").count(),
        2,
        "{}",
        sonuc.standart
    );
    assert!(!sonuc.standart.contains("GIZLI"), "{}", sonuc.standart);
}

#[tokio::test]
async fn diger_calisma_alani_okunamaz_ve_disina_yazilamaz() {
    let k = kurgu();
    let gizli = k.baska_kok.join("gizli.txt");
    std::fs::write(&gizli, "BASKA-ALAN").unwrap();
    let alan = k.hazirla(&k.kok, Vec::new());
    let isaret = k.baska_kok.join("disari-yazildi.txt");
    let betik = format!(
        "try {{ [IO.File]::ReadAllText({}); 'OKUNDU' }} catch {{ 'OKUMA-ENGELLENDI' }}; try {{ [IO.File]::WriteAllText({}, 'YAZILDI'); 'YAZILDI' }} catch {{ 'YAZMA-ENGELLENDI' }}",
        ps(&gizli),
        ps(&isaret),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert!(
        sonuc.standart.contains("OKUMA-ENGELLENDI"),
        "{}",
        sonuc.standart
    );
    assert!(
        sonuc.standart.contains("YAZMA-ENGELLENDI"),
        "{}",
        sonuc.standart
    );
    assert!(!isaret.exists());
    assert!(!sonuc.standart.contains("BASKA-ALAN"));
}

#[tokio::test]
async fn iki_calisma_alani_ayni_anda_birbirini_okuyamaz_ve_yazamaz() {
    let k = kurgu();
    let a_dosya = k.kok.join("dosya.txt");
    let b_dosya = k.baska_kok.join("dosya.txt");
    std::fs::write(&a_dosya, "ALAN-A-ICERIK").unwrap();
    std::fs::write(&b_dosya, "ALAN-B-ICERIK").unwrap();
    let a = k.hazirla(&k.kok, Vec::new());
    let b = k.hazirla(&k.baska_kok, Vec::new());
    // Her komut öteki alana yazmayı dener: `b-yazdi.txt` A'nın kökünde,
    // B'nin denemesidir.
    let a_yazma = k.kok.join("b-yazdi.txt");
    let b_yazma = k.baska_kok.join("a-yazdi.txt");
    let a_betik = format!(
        "try {{ [IO.File]::ReadAllText({}); 'OKUNDU' }} catch {{ 'OKUMA-ENGELLENDI' }}; try {{ [IO.File]::WriteAllText({}, 'YAZILDI'); 'YAZILDI' }} catch {{ 'YAZMA-ENGELLENDI' }}",
        ps(&b_dosya),
        ps(&b_yazma),
    );
    let b_betik = format!(
        "try {{ [IO.File]::ReadAllText({}); 'OKUNDU' }} catch {{ 'OKUMA-ENGELLENDI' }}; try {{ [IO.File]::WriteAllText({}, 'YAZILDI'); 'YAZILDI' }} catch {{ 'YAZMA-ENGELLENDI' }}",
        ps(&a_dosya),
        ps(&a_yazma),
    );
    let (sa, sb) = tokio::join!(
        k.calistir(&a, false, &a_betik),
        k.calistir(&b, false, &b_betik)
    );
    for sonuc in [&sa, &sb] {
        assert!(
            sonuc.standart.contains("OKUMA-ENGELLENDI"),
            "{}{}",
            sonuc.standart,
            sonuc.hata
        );
        assert!(
            sonuc.standart.contains("YAZMA-ENGELLENDI"),
            "{}{}",
            sonuc.standart,
            sonuc.hata
        );
        assert!(!sonuc.standart.contains("ALAN-"), "{}", sonuc.standart);
    }
    assert!(!a_yazma.exists());
    assert!(!b_yazma.exists());
}

#[tokio::test]
async fn ic_ice_calisma_alanlari_dis_paketin_erismini_keser_ve_acl_geri_alinir() {
    let mut k = kurgu();
    k.baska_kok = k.kok.join("calisma-alani-b");
    std::fs::create_dir_all(&k.baska_kok).unwrap();
    let b_icerik = k.baska_kok.join("b-ozel.txt");
    let b_yazma = k.baska_kok.join("b-yazdi.txt");
    let a_yazma = k.baska_kok.join("a-yazdi.txt");
    std::fs::write(&b_icerik, "YALNIZ-B-OKUYABILIR").unwrap();

    let b_onceki_dacl = guvenlik::dacl_sddl(&k.baska_kok).unwrap();
    let alan_a = k.hazirla(&k.kok, Vec::new());
    let alan_b = k.hazirla(&k.baska_kok, Vec::new());
    let b_yalitilmis_dacl = guvenlik::dacl_sddl(&k.baska_kok).unwrap();
    assert_ne!(b_yalitilmis_dacl, b_onceki_dacl);

    let a_betik = format!(
        "try {{ [IO.File]::ReadAllText({}); 'B-OKUNDU' }} catch {{ 'B-OKUMA-ENGEL' }}; try {{ [IO.File]::WriteAllText({}, 'A-YAZDI'); 'B-YAZMA-IZIN' }} catch {{ 'B-YAZMA-ENGEL' }}",
        ps(&b_icerik),
        ps(&a_yazma),
    );
    let sonuc_a = k.calistir(&alan_a, false, &a_betik).await;
    assert_eq!(sonuc_a.bitis, Bitis::Cikti(Some(0)), "{}", sonuc_a.hata);
    assert!(
        sonuc_a.standart.contains("B-OKUMA-ENGEL"),
        "{}{}",
        sonuc_a.standart,
        sonuc_a.hata
    );
    assert!(
        sonuc_a.standart.contains("B-YAZMA-ENGEL"),
        "{}{}",
        sonuc_a.standart,
        sonuc_a.hata
    );
    assert!(!sonuc_a.standart.contains("YALNIZ-B-OKUYABILIR"));
    assert!(!a_yazma.exists());

    let b_betik = format!(
        "$icerik = [IO.File]::ReadAllText({}); [IO.File]::WriteAllText({}, 'B-YAZDI'); 'B-OKUNDU=' + $icerik",
        ps(&b_icerik),
        ps(&b_yazma),
    );
    let sonuc_b = k.calistir(&alan_b, false, &b_betik).await;
    assert_eq!(sonuc_b.bitis, Bitis::Cikti(Some(0)), "{}", sonuc_b.hata);
    assert!(
        sonuc_b.standart.contains("B-OKUNDU=YALNIZ-B-OKUYABILIR"),
        "{}{}",
        sonuc_b.standart,
        sonuc_b.hata
    );
    assert_eq!(std::fs::read_to_string(&b_yazma).unwrap(), "B-YAZDI");

    k.y.alani_kaldir(&alan_b)
        .unwrap_or_else(|hata| panic!("iç alan kaldırılamadı: {hata}"));
    let a_sonrasi = format!(
        "$icerik = [IO.File]::ReadAllText({}); [IO.File]::WriteAllText({}, 'A-YAZDI'); 'A-OKUNDU=' + $icerik",
        ps(&b_icerik),
        ps(&a_yazma),
    );
    let sonuc_a_sonrasi = k.calistir(&alan_a, false, &a_sonrasi).await;
    assert_eq!(
        sonuc_a_sonrasi.bitis,
        Bitis::Cikti(Some(0)),
        "{}",
        sonuc_a_sonrasi.hata
    );
    assert!(
        sonuc_a_sonrasi
            .standart
            .contains("A-OKUNDU=YALNIZ-B-OKUYABILIR"),
        "{}{}",
        sonuc_a_sonrasi.standart,
        sonuc_a_sonrasi.hata
    );
    assert_eq!(std::fs::read_to_string(&a_yazma).unwrap(), "A-YAZDI");

    k.y.alani_kaldir(&alan_a)
        .unwrap_or_else(|hata| panic!("dış alan kaldırılamadı: {hata}"));
    assert_eq!(
        guvenlik::dacl_sddl(&k.baska_kok).unwrap(),
        b_onceki_dacl,
        "iç alanın önceki DACL'i geri yüklenmedi"
    );
}

#[tokio::test]
async fn ic_alan_once_hazirlaninca_dis_kap_erisemez_ve_acl_geri_alinir() {
    let mut k = kurgu();
    k.baska_kok = k.kok.join("calisma-alani-b");
    std::fs::create_dir_all(&k.baska_kok).unwrap();
    let b_icerik = k.baska_kok.join("b-ozel.txt");
    let b_yazma = k.baska_kok.join("b-yazdi.txt");
    let a_yazma = k.baska_kok.join("a-yazdi.txt");
    std::fs::write(&b_icerik, "YALNIZ-B-OKUYABILIR").unwrap();

    let b_onceki_dacl = guvenlik::dacl_sddl(&k.baska_kok).unwrap();
    let alan_b = k.hazirla(&k.baska_kok, Vec::new());
    let b_yalitilmis_dacl = guvenlik::dacl_sddl(&k.baska_kok).unwrap();
    assert_ne!(b_yalitilmis_dacl, b_onceki_dacl);
    let alan_a = k.hazirla(&k.kok, Vec::new());

    let a_betik = format!(
        "try {{ [IO.File]::ReadAllText({}); 'B-OKUNDU' }} catch {{ 'B-OKUMA-ENGEL' }}; try {{ [IO.File]::WriteAllText({}, 'A-YAZDI'); 'B-YAZMA-IZIN' }} catch {{ 'B-YAZMA-ENGEL' }}",
        ps(&b_icerik),
        ps(&a_yazma),
    );
    let sonuc_a = k.calistir(&alan_a, false, &a_betik).await;
    assert_eq!(sonuc_a.bitis, Bitis::Cikti(Some(0)), "{}", sonuc_a.hata);
    assert!(
        sonuc_a.standart.contains("B-OKUMA-ENGEL"),
        "{}{}",
        sonuc_a.standart,
        sonuc_a.hata
    );
    assert!(
        sonuc_a.standart.contains("B-YAZMA-ENGEL"),
        "{}{}",
        sonuc_a.standart,
        sonuc_a.hata
    );
    assert!(!sonuc_a.standart.contains("YALNIZ-B-OKUYABILIR"));
    assert!(!a_yazma.exists());

    let b_betik = format!(
        "$icerik = [IO.File]::ReadAllText({}); [IO.File]::WriteAllText({}, 'B-YAZDI'); 'B-OKUNDU=' + $icerik",
        ps(&b_icerik),
        ps(&b_yazma),
    );
    let sonuc_b = k.calistir(&alan_b, false, &b_betik).await;
    assert_eq!(sonuc_b.bitis, Bitis::Cikti(Some(0)), "{}", sonuc_b.hata);
    assert!(
        sonuc_b.standart.contains("B-OKUNDU=YALNIZ-B-OKUYABILIR"),
        "{}{}",
        sonuc_b.standart,
        sonuc_b.hata
    );
    assert_eq!(std::fs::read_to_string(&b_yazma).unwrap(), "B-YAZDI");

    k.y.alani_kaldir(&alan_b)
        .unwrap_or_else(|hata| panic!("iç alan kaldırılamadı: {hata}"));
    let a_sonrasi = format!(
        "$icerik = [IO.File]::ReadAllText({}); [IO.File]::WriteAllText({}, 'A-YAZDI'); 'A-OKUNDU=' + $icerik",
        ps(&b_icerik),
        ps(&a_yazma),
    );
    let sonuc_a_sonrasi = k.calistir(&alan_a, false, &a_sonrasi).await;
    assert_eq!(
        sonuc_a_sonrasi.bitis,
        Bitis::Cikti(Some(0)),
        "{}",
        sonuc_a_sonrasi.hata
    );
    assert!(
        sonuc_a_sonrasi
            .standart
            .contains("A-OKUNDU=YALNIZ-B-OKUYABILIR"),
        "{}{}",
        sonuc_a_sonrasi.standart,
        sonuc_a_sonrasi.hata
    );
    assert_eq!(std::fs::read_to_string(&a_yazma).unwrap(), "A-YAZDI");

    k.y.alani_kaldir(&alan_a)
        .unwrap_or_else(|hata| panic!("dış alan kaldırılamadı: {hata}"));
    assert_eq!(
        guvenlik::dacl_sddl(&k.baska_kok).unwrap(),
        b_onceki_dacl,
        "iç alanın önceki DACL'i geri yüklenmedi"
    );
}

#[tokio::test]
async fn calisma_alani_disina_yazilamaz_ve_kendi_gecicisini_kullanir() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let ozel = OzelDizinler::yol(&k.yalitim_dizini, &alan_kimligi(&alan.kok));
    let isaret = k.baska_kok.join("disari-denemesi.txt");
    let betik = format!(
        "try {{ [IO.File]::WriteAllText({}, 'YAZILDI'); 'DISARI-YAZILDI' }} catch {{ 'DISARI-ENGELLENDI' }}; [IO.File]::WriteAllText([IO.Path]::Combine($env:TEMP, 'yalitim-denemesi.txt'), 'GECICI'); 'TEMP=' + $env:TEMP",
        ps(&isaret),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert!(
        sonuc.standart.contains("DISARI-ENGELLENDI"),
        "{}",
        sonuc.standart
    );
    assert!(!isaret.exists());
    let gecici = kapin_gecicisi(&ozel, &sonuc.standart);
    assert_eq!(
        std::fs::read_to_string(gecici.join("yalitim-denemesi.txt")).unwrap(),
        "GECICI"
    );
}

/// Sürücü kökü ile alan kökü arasındaki üst dizinler (ikisi hariç).
fn ust_dizinler(kok: &Path) -> Vec<PathBuf> {
    let kok = guvenlik::sade(&std::fs::canonicalize(kok).unwrap());
    kok.ancestors()
        .skip(1)
        .filter(|u| u.parent().is_some())
        .map(Path::to_path_buf)
        .collect()
}

fn konum_betigi() -> &'static str {
    "'KONUM=' + (Get-Location).Path; Set-Content -LiteralPath 'goreli.txt' -Value 'GORELI'"
}

fn konum_alanda(sonuc: &Sonuc, kok: &Path) {
    let beklenen = guvenlik::sade(&std::fs::canonicalize(kok).unwrap());
    assert!(
        deger(&sonuc.standart, "KONUM").eq_ignore_ascii_case(&beklenen.display().to_string()),
        "PowerShell çalışma alanında başlamadı: {}{}",
        sonuc.standart,
        sonuc.hata
    );
    assert_eq!(
        std::fs::read_to_string(kok.join("goreli.txt"))
            .unwrap()
            .trim(),
        "GORELI"
    );
}

/// A11: git ve PowerShell çalışma dizinini çözerken üst dizinleri listeler.
/// Hazırlık, sürücü kökü ile alan kökü arasındaki üst dizinlere yalnız bu
/// alanın paketine listeleme verir; kaldırma onları eski hâline döndürür.
#[tokio::test]
async fn calisma_dizini_ust_dizinlerle_cozulur_ve_izinler_geri_alinir() {
    let k = kurgu();
    let ust = ust_dizinler(&k.kok);
    // İlk üst dizin testin kendi geçici dizinidir; öteki üst dizinler
    // paralel testlerle ortaktır, onlarda yalnız bu alanın paketine bakılır.
    let taban_onceki = guvenlik::dacl_sddl(&ust[0]).unwrap();
    let alan = k.hazirla(&k.kok, Vec::new());
    let paket = guvenlik::profil_sidi(&format!("Kalfa.Alan.{}", alan_kimligi(&alan.kok)))
        .unwrap()
        .metin();
    let yetenek = guvenlik::arac_zinciri_sidi().unwrap();
    let paket_acesi = format!("(A;;0x1000a1;;;{paket})");
    for yol in &ust {
        let sddl = guvenlik::dacl_sddl(yol).unwrap();
        assert!(
            sddl.contains(&paket_acesi) || guvenlik::kok_izni_var_mi(yol, &yetenek).unwrap(),
            "{} listelenemez: {sddl}",
            yol.display()
        );
    }
    let sonuc = k.calistir(&alan, false, konum_betigi()).await;
    konum_alanda(&sonuc, &k.kok);

    k.y.alani_kaldir(&alan).unwrap();
    assert_eq!(guvenlik::dacl_sddl(&ust[0]).unwrap(), taban_onceki);
    for yol in &ust {
        let sddl = guvenlik::dacl_sddl(yol).unwrap();
        assert!(
            !sddl.contains(&paket),
            "{} geri alınmadı: {sddl}",
            yol.display()
        );
    }
}

/// İki alan aynı üst dizinleri paylaşır; her biri kendi paketinin iznini
/// taşır. Birinin kaldırılması ötekinin çalışma dizinini bozmaz.
#[tokio::test]
async fn ust_dizin_izni_alanlar_arasinda_bagimsizdir() {
    let k = kurgu();
    let a = k.hazirla(&k.kok, Vec::new());
    let b = k.hazirla(&k.baska_kok, Vec::new());
    k.y.alani_kaldir(&a).unwrap();
    let sonuc = k.calistir(&b, false, konum_betigi()).await;
    konum_alanda(&sonuc, &k.baska_kok);
}

/// Windows'un kap için seçtiği geçici dizin, ilk süreç PowerShell değilken de
/// vardır (PowerShell onu kendisi kurar; `link.exe` kurmaz, LNK1104 verir).
#[tokio::test]
async fn kapin_gecici_dizini_ilk_surecten_once_vardir() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let mut istek = k.istek(&alan, false, "");
    istek.komut = KomutBicimi::Program {
        program: windows_koku()
            .join(r"System32\cmd.exe")
            .display()
            .to_string(),
        argumanlar: vec![
            "/d".into(),
            "/c".into(),
            r"if exist %TEMP%\NUL (echo TEMP-VAR) else (echo TEMP-YOK)".into(),
        ],
    };
    let sonuc = k.istegi_calistir(&istek).await;
    assert!(
        sonuc.standart.contains("TEMP-VAR"),
        "{}{}",
        sonuc.standart,
        sonuc.hata
    );
}

/// Windows, AppContainer süreçlerinde `TEMP` ve `TMP`'yi verilen değerden
/// bağımsız olarak `%LOCALAPPDATA%\Packages\<paket>\AC\Temp`'e çevirir.
/// `LOCALAPPDATA` çalışma alanına özel ev olduğundan geçici dizin yine
/// çalışma alanına özel kökün altındadır; geliştiricinin `TEMP`'i değildir.
fn kapin_gecicisi(ozel: &OzelDizinler, standart: &str) -> PathBuf {
    let gecici = PathBuf::from(deger(standart, "TEMP"));
    let kok = ozel.kok.display().to_string().to_ascii_lowercase();
    assert!(
        gecici
            .display()
            .to_string()
            .to_ascii_lowercase()
            .starts_with(&kok),
        "TEMP çalışma alanına özel kökün altında değil: {}",
        gecici.display()
    );
    gecici
}

#[tokio::test]
async fn ozel_kok_tasinamaz_alt_dizinler_yazilabilir_ve_ev_junctionu_guvenle_yenilenir() {
    let k = kurgu();
    let alan_kimligi = alan_kimligi(&k.kok);
    let ozel = OzelDizinler::yol(&k.yalitim_dizini, &alan_kimligi);
    let kok_hedefi = k.taban.join("kok-junction-hedefi");
    let hedef_ev = kok_hedefi.join("ev");
    std::fs::create_dir_all(&hedef_ev).unwrap();
    std::fs::write(hedef_ev.join(".gitconfig"), "DIS-GITCONFIG-KORUNDU").unwrap();
    std::fs::write(hedef_ev.join(".npmrc"), "DIS-NPMRC-KORUNDU").unwrap();
    std::fs::create_dir_all(&k.yalitim_dizini).unwrap();
    junction_kur(&ozel.kok, &kok_hedefi)
        .unwrap_or_else(|hata| panic!("özel kök junction kurulamadı: {hata}"));

    let alan = k.hazirla(&k.kok, Vec::new());
    assert!(
        !std::fs::symlink_metadata(&ozel.kok)
            .unwrap()
            .file_type()
            .is_symlink(),
        "özel kök junction'ı hazırlanırken silinmedi"
    );
    assert_eq!(
        std::fs::read_to_string(hedef_ev.join(".gitconfig")).unwrap(),
        "DIS-GITCONFIG-KORUNDU"
    );
    assert_eq!(
        std::fs::read_to_string(hedef_ev.join(".npmrc")).unwrap(),
        "DIS-NPMRC-KORUNDU"
    );

    let tasinmis_kok = ozel.kok.with_file_name("ozel-kok-tasindi");
    let betik = format!(
        r##"Add-Type -TypeDefinition @'
{JUNCTION_KODU}
'@; $kok = {kok}; $tasinmis = {tasinmis}; $hedef = {hedef}; $evDosyasi = Join-Path $env:USERPROFILE 'kap-ev-denemesi.txt'; try {{ [IO.File]::WriteAllText($evDosyasi, 'EV-YAZILDI'); 'EV=YAZILDI' }} catch {{ 'EV=ENGEL' }}; $geciciDosyasi = Join-Path $env:TEMP 'kap-gecici-denemesi.txt'; try {{ [IO.File]::WriteAllText($geciciDosyasi, 'GECICI-YAZILDI'); 'GECICI=YAZILDI' }} catch {{ 'GECICI=ENGEL' }}; 'TEMP=' + $env:TEMP; $onbellekDosyasi = Join-Path $env:CARGO_HOME 'kap-onbellek-denemesi.txt'; try {{ [IO.File]::WriteAllText($onbellekDosyasi, 'ONBELLEK-YAZILDI'); 'ONBELLEK=YAZILDI' }} catch {{ 'ONBELLEK=ENGEL' }}; 'KOK-DOGRUDAN-REPARSE=' + [KalfaJunctionDenemesi]::KokYenidenAyristirmaNoktasiKur($kok, $hedef); try {{ [IO.Directory]::Move($kok, $tasinmis); 'KOK-TASINDI=IZIN' }} catch {{ 'KOK-TASINDI=ENGEL' }}; if (-not [IO.Directory]::Exists($kok)) {{ $cmd = Join-Path $env:SystemRoot 'System32\cmd.exe'; $komut = 'mklink /J "' + $kok + '" "' + $hedef + '"'; try {{ & $cmd /d /c $komut | Out-Null }} catch {{ }} }}; if ([IO.Directory]::Exists($kok)) {{ $ozellik = [IO.File]::GetAttributes($kok); if (($ozellik -band [IO.FileAttributes]::ReparsePoint) -ne 0) {{ 'KOK-JUNCTION=KURULDU' }} else {{ 'KOK-JUNCTION=KURULAMADI' }} }} else {{ 'KOK-JUNCTION=KURULAMADI' }}"##,
        JUNCTION_KODU = JUNCTION_KODU,
        kok = ps(&ozel.kok),
        tasinmis = ps(&tasinmis_kok),
        hedef = ps(&kok_hedefi),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    for beklenen in [
        "EV=YAZILDI",
        "GECICI=YAZILDI",
        "ONBELLEK=YAZILDI",
        "KOK-DOGRUDAN-REPARSE=5",
        "KOK-TASINDI=ENGEL",
        "KOK-JUNCTION=KURULAMADI",
    ] {
        assert!(
            sonuc.standart.contains(beklenen),
            "{beklenen} yok: {}{}",
            sonuc.standart,
            sonuc.hata
        );
    }
    assert_eq!(
        std::fs::read_to_string(ozel.ev.join("kap-ev-denemesi.txt")).unwrap(),
        "EV-YAZILDI"
    );
    assert_eq!(
        std::fs::read_to_string(
            kapin_gecicisi(&ozel, &sonuc.standart).join("kap-gecici-denemesi.txt")
        )
        .unwrap(),
        "GECICI-YAZILDI"
    );
    assert_eq!(
        std::fs::read_to_string(ozel.cargo().join("kap-onbellek-denemesi.txt")).unwrap(),
        "ONBELLEK-YAZILDI"
    );
    assert!(
        !std::fs::symlink_metadata(&ozel.kok)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        std::fs::read_to_string(hedef_ev.join(".gitconfig")).unwrap(),
        "DIS-GITCONFIG-KORUNDU"
    );
    assert_eq!(
        std::fs::read_to_string(hedef_ev.join(".npmrc")).unwrap(),
        "DIS-NPMRC-KORUNDU"
    );

    std::fs::remove_dir_all(&ozel.ev).unwrap();
    let ev_junction_hedefi = k.taban.join("ev-junction-hedefi");
    std::fs::create_dir_all(&ev_junction_hedefi).unwrap();
    std::fs::write(
        ev_junction_hedefi.join(".gitconfig"),
        "JUNCTION-GITCONFIG-KORUNDU",
    )
    .unwrap();
    std::fs::write(ev_junction_hedefi.join(".npmrc"), "JUNCTION-NPMRC-KORUNDU").unwrap();
    junction_kur(&ozel.ev, &ev_junction_hedefi)
        .unwrap_or_else(|hata| panic!("ev junction kurulamadı: {hata}"));

    ozel.hazirla(&GitKimligi::default())
        .unwrap_or_else(|hata| panic!("özel dizinler yeniden hazırlanamadı: {hata}"));
    assert_eq!(
        std::fs::read_to_string(ev_junction_hedefi.join(".gitconfig")).unwrap(),
        "JUNCTION-GITCONFIG-KORUNDU"
    );
    assert_eq!(
        std::fs::read_to_string(ev_junction_hedefi.join(".npmrc")).unwrap(),
        "JUNCTION-NPMRC-KORUNDU"
    );
    assert!(
        !std::fs::symlink_metadata(&ozel.ev)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        std::fs::read_to_string(ozel.gitconfig()).unwrap(),
        "[user]\n"
    );
    assert!(std::fs::read_to_string(ozel.npmrc()).unwrap().is_empty());
}

#[tokio::test]
async fn listeden_cikarilan_ek_arac_zinciri_yeni_kaba_mutlak_yoldan_okunamaz() {
    let k = kurgu();
    let ek_arac = k.taban.join("ek-arac-zinciri");
    std::fs::create_dir_all(&ek_arac).unwrap();
    let gizli = ek_arac.join("ek-arac-kimligi.txt");
    std::fs::write(&gizli, "EK-ARAC-ZINCIRI-GIZLI").unwrap();

    k.y.okunur_dizinleri_ayarla(vec![ek_arac]);
    let alan_a = k.hazirla(&k.kok, Vec::new());
    let betik = format!(
        "try {{ 'OKUNDU=' + [IO.File]::ReadAllText({}) }} catch {{ 'OKUMA-ENGELLENDI' }}",
        ps(&gizli),
    );
    let ilk = k.calistir(&alan_a, false, &betik).await;
    assert_eq!(ilk.bitis, Bitis::Cikti(Some(0)), "{}", ilk.hata);
    assert!(
        ilk.standart.contains("OKUNDU=EK-ARAC-ZINCIRI-GIZLI"),
        "{}{}",
        ilk.standart,
        ilk.hata
    );

    k.y.okunur_dizinleri_ayarla(Vec::new());
    let alan_b = k.hazirla(&k.baska_kok, Vec::new());
    let sonraki = k.calistir(&alan_b, false, &betik).await;
    assert_eq!(sonraki.bitis, Bitis::Cikti(Some(0)), "{}", sonraki.hata);
    assert!(
        sonraki.standart.contains("OKUMA-ENGELLENDI"),
        "{}{}",
        sonraki.standart,
        sonraki.hata
    );
    assert!(!sonraki.standart.contains("EK-ARAC-ZINCIRI-GIZLI"));
}

#[tokio::test]
async fn env_ve_hard_linki_okunamaz() {
    let k = kurgu();
    let sir = k.kok.join(".env");
    let bag = k.kok.join("env-kopyasi.txt");
    std::fs::write(&sir, "GIZLI-ENV").unwrap();
    std::fs::hard_link(&sir, &bag).unwrap();
    let alan = k.hazirla(&k.kok, Vec::new());
    assert!(alan.gizli.dosyalar.iter().any(|y| y == &sir));
    let betik = format!(
        "foreach ($p in @({}, {})) {{ try {{ [IO.File]::ReadAllText($p); 'OKUNDU' }} catch {{ 'ENGELLENDI' }} }}",
        ps(&sir),
        ps(&bag),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert_eq!(
        sonuc.standart.matches("ENGELLENDI").count(),
        2,
        "{}",
        sonuc.standart
    );
    assert!(!sonuc.standart.contains("GIZLI-ENV"), "{}", sonuc.standart);
    assert_eq!(std::fs::read_to_string(sir).unwrap(), "GIZLI-ENV");
}

#[tokio::test]
async fn ilk_hazirliktan_sonra_eklenen_gizli_yollar_yeniden_taranir() {
    let k = kurgu();
    k.hazirla(&k.kok, Vec::new());

    let yeni_env = k.kok.join(".env");
    let yeni_dizin = k.kok.join("sirlar");
    let yeni_sir = yeni_dizin.join("sonradan-eklenen.pem");
    std::fs::write(&yeni_env, "SONRADAN-GIZLI-ENV").unwrap();
    std::fs::create_dir_all(&yeni_dizin).unwrap();
    std::fs::write(&yeni_sir, "SONRADAN-GIZLI-ANAHTAR").unwrap();

    let alan = k.hazirla(&k.kok, Vec::new());
    assert!(alan.gizli.dosyalar.iter().any(|yol| yol == &yeni_env));
    assert!(alan.gizli.dizinler.iter().any(|yol| yol == &yeni_dizin));
    let betik = format!(
        "foreach ($p in @({}, {})) {{ try {{ [IO.File]::ReadAllText($p); 'OKUNDU' }} catch {{ 'ENGELLENDI' }} }}",
        ps(&yeni_env),
        ps(&yeni_sir),
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert_eq!(
        sonuc.standart.matches("ENGELLENDI").count(),
        2,
        "{}{}",
        sonuc.standart,
        sonuc.hata
    );
    assert!(!sonuc.standart.contains("SONRADAN-GIZLI"));
    assert_eq!(
        std::fs::read_to_string(yeni_env).unwrap(),
        "SONRADAN-GIZLI-ENV"
    );
    assert_eq!(
        std::fs::read_to_string(yeni_sir).unwrap(),
        "SONRADAN-GIZLI-ANAHTAR"
    );
}

#[tokio::test]
async fn git_dizini_worktree_dosyasi_ve_hook_hedefi_salt_okunur() {
    let k = kurgu();
    let git = k.kok.join(".git");
    let hooks = k.kok.join(".githooks");
    std::fs::create_dir_all(git.join("hooks")).unwrap();
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(git.join("config"), "[core]\n\thooksPath = ../.githooks\n").unwrap();
    std::fs::write(git.join("HEAD"), "ref: refs/heads/ana\n").unwrap();
    std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\n").unwrap();
    let worktree_dot_git = k.baska_kok.join(".git");
    std::fs::write(&worktree_dot_git, "gitdir: ../ana/.git/worktrees/alt\n").unwrap();
    let alan_a = k.hazirla(
        &k.kok,
        vec![
            Korumali {
                yol: git.clone(),
                dizin: true,
            },
            Korumali {
                yol: hooks.clone(),
                dizin: true,
            },
        ],
    );
    let alan_b = k.hazirla(
        &k.baska_kok,
        vec![Korumali {
            yol: worktree_dot_git.clone(),
            dizin: false,
        }],
    );
    let betik_a = format!(
        "foreach ($ad in @('degistir','sil','yeniden-adlandir','yeni-kanca','kanca-yolunu-adlandir')) {{ try {{ switch ($ad) {{ 'degistir' {{ [IO.File]::WriteAllText({}, 'KURCALANDI') }} 'sil' {{ [IO.File]::Delete({}) }} 'yeniden-adlandir' {{ [IO.Directory]::Move({}, {} ) }} 'yeni-kanca' {{ [IO.File]::WriteAllText({}, 'KOD') }} 'kanca-yolunu-adlandir' {{ [IO.Directory]::Move({}, {}) }} }}; \"$ad=IZIN\" }} catch {{ \"$ad=ENGEL\" }} }}",
        ps(&git.join("config")),
        ps(&git.join("HEAD")),
        ps(&git),
        ps(&k.kok.join(".git-yeni")),
        ps(&hooks.join("yeni")),
        ps(&hooks),
        ps(&k.kok.join(".githooks-yeni")),
    );
    let sonuc_a = k.calistir(&alan_a, false, &betik_a).await;
    for ad in [
        "degistir=ENGEL",
        "sil=ENGEL",
        "yeniden-adlandir=ENGEL",
        "yeni-kanca=ENGEL",
        "kanca-yolunu-adlandir=ENGEL",
    ] {
        assert!(
            sonuc_a.standart.contains(ad),
            "{}{}",
            sonuc_a.standart,
            sonuc_a.hata
        );
    }
    assert_eq!(
        std::fs::read_to_string(git.join("config")).unwrap(),
        "[core]\n\thooksPath = ../.githooks\n"
    );
    assert_eq!(
        std::fs::read_to_string(git.join("HEAD")).unwrap(),
        "ref: refs/heads/ana\n"
    );
    assert_eq!(
        std::fs::read_to_string(hooks.join("pre-commit")).unwrap(),
        "#!/bin/sh\n"
    );
    assert!(!k.kok.join(".git-yeni").exists());
    assert!(!hooks.join("yeni").exists());

    let betik_b = format!(
        "try {{ [IO.File]::WriteAllText({}, 'KURCALANDI'); 'degistir=IZIN' }} catch {{ 'degistir=ENGEL' }}; try {{ [IO.File]::Delete({}); 'sil=IZIN' }} catch {{ 'sil=ENGEL' }}; try {{ [IO.File]::Move({}, {}); 'yeniden-adlandir=IZIN' }} catch {{ 'yeniden-adlandir=ENGEL' }}",
        ps(&worktree_dot_git),
        ps(&worktree_dot_git),
        ps(&worktree_dot_git),
        ps(&k.baska_kok.join(".git-yeni")),
    );
    let sonuc_b = k.calistir(&alan_b, false, &betik_b).await;
    for ad in ["degistir=ENGEL", "sil=ENGEL", "yeniden-adlandir=ENGEL"] {
        assert!(
            sonuc_b.standart.contains(ad),
            "{}{}",
            sonuc_b.standart,
            sonuc_b.hata
        );
    }
    assert_eq!(
        std::fs::read_to_string(worktree_dot_git).unwrap(),
        "gitdir: ../ana/.git/worktrees/alt\n"
    );
}

#[test]
fn hazirlik_onay_olmadan_acl_degistirmez() {
    let k = kurgu();
    let alan = alan_bilgisi(&k.kok, Vec::new());
    assert!(k.y.hazirlik_gerekli(&alan.kok));
    let onceki = kalfa_sandbox::windows::guvenlik::dacl_sddl(&alan.kok).unwrap();
    let sonuc = k.y.alani_hazirla(&alan, false, &mut |_| {});
    assert!(matches!(sonuc, Err(BaslatmaHatasi::Yalitim(_))));
    assert_eq!(
        kalfa_sandbox::windows::guvenlik::dacl_sddl(&alan.kok).unwrap(),
        onceki
    );
    assert!(k.y.hazirlik_gerekli(&alan.kok));
    let mut ilerleme = Vec::new();
    k.y.alani_hazirla(&alan, true, &mut |satir| ilerleme.push(satir.to_string()))
        .unwrap_or_else(|hata| panic!("onaylı hazırlık başarısız: {hata:?}; {ilerleme:?}"));
    assert!(!k.y.hazirlik_gerekli(&alan.kok));
}

fn tcp_betik(adres: &str, port: u16) -> String {
    format!(
        "$c = New-Object Net.Sockets.TcpClient; try {{ $a = $c.BeginConnect('{adres}', {port}, $null, $null); if (-not $a.AsyncWaitHandle.WaitOne(8000)) {{ 'TCP=ZAMANASIMI' }} else {{ $c.EndConnect($a); 'TCP=BAGLANDI' }} }} catch {{ 'TCP=ENGELLENDI' }} finally {{ $c.Dispose() }}"
    )
}

#[tokio::test]
async fn agsiz_profilde_tcp_baglantisi_kurulamaz() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let sonuc = k.calistir(&alan, false, &tcp_betik("1.1.1.1", 443)).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert!(
        !sonuc.standart.contains("TCP=BAGLANDI"),
        "{}",
        sonuc.standart
    );
}

#[tokio::test]
async fn agli_profilde_tcp_baglantisi_kurulur() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let sonuc = k.calistir(&alan, true, &tcp_betik("1.1.1.1", 443)).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert!(
        sonuc.standart.contains("TCP=BAGLANDI"),
        "{}{}",
        sonuc.standart,
        sonuc.hata
    );
}

#[tokio::test]
async fn loopback_agli_ve_agssiz_profillerde_kapali() {
    let k = kurgu();
    let dinleyici = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let alan = k.hazirla(&k.kok, Vec::new());
    for ag in [false, true] {
        let sonuc = k.calistir(&alan, ag, &tcp_betik("127.0.0.1", port)).await;
        assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
        assert!(
            !sonuc.standart.contains("TCP=BAGLANDI"),
            "{}",
            sonuc.standart
        );
    }
    drop(dinleyici);
}

fn surec_aktif_mi(pid: u32) -> bool {
    let betik = format!(
        "if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'AKTIF' }} else {{ 'SONDU' }}"
    );
    Command::new(powershell_yolu())
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
        .arg(betik)
        .output()
        .map(|sonuc| String::from_utf8_lossy(&sonuc.stdout).contains("AKTIF"))
        .unwrap_or(false)
}

#[tokio::test]
async fn iptal_torun_surecini_de_durdurur() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let isaret = k.kok.join("torun.pid");
    let betik = format!(
        "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [IO.File]::WriteAllText({}, [string]$c.Id); Start-Sleep -Seconds 300",
        ps(&isaret),
    );
    let surec =
        k.y.baslat(&k.istek(&alan, false, &betik))
            .unwrap_or_else(|hata| panic!("iptal testi başlatılamadı: {hata:?}"));
    let iptal = CancellationToken::new();
    let tetik = iptal.clone();
    let bekleme = tokio::spawn(async move { surec.bekle(UZUN, iptal, |_, _| {}).await });
    tokio::time::timeout(Duration::from_secs(15), async {
        while !isaret.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("torun süreç PID'ini yazmadı; {}", isaret.display()));
    let pid: u32 = std::fs::read_to_string(&isaret).unwrap().parse().unwrap();
    assert!(surec_aktif_mi(pid), "torun henüz çalışmıyordu: {pid}");
    tetik.cancel();
    let sonuc = bekleme.await.unwrap();
    assert_eq!(sonuc.bitis, Bitis::Iptal);
    tokio::time::timeout(Duration::from_secs(8), async {
        while surec_aktif_mi(pid) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("iptalden sonra torun süreç yaşıyor: {pid}"));
}

/// Ebeveyn test süreci öldürülünce AppContainer işindeki torun da ölür.
#[tokio::test]
async fn yardimci_kalfa_gibi() {
    let Some(taban) = std::env::var_os("KALFA_TEST_YARDIMCI_DIZINI") else {
        return;
    };
    let k = kurgu_paylasimli(PathBuf::from(taban));
    let alan = k.hazirla(&k.kok, Vec::new());
    // Kap yalnızca çalışma alanına yazabilir.
    let isaret = k.kok.join("yardimci-torun.pid");
    let betik = format!(
        "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [IO.File]::WriteAllText({}, [string]$c.Id); Start-Sleep -Seconds 300",
        ps(&isaret),
    );
    let _surec =
        k.y.baslat(&k.istek(&alan, false, &betik))
            .unwrap_or_else(|hata| panic!("yardımcı komut başlatılamadı: {hata:?}"));
    tokio::time::timeout(Duration::from_secs(15), async {
        while !isaret.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("yardımcı torun PID'ini yazmadı"));
    println!("HAZIR");
    tokio::time::sleep(Duration::from_secs(300)).await;
}

#[test]
fn kalfanin_olumu_appcontainer_isindeki_torunu_durdurur() {
    let gecici = tempfile::tempdir().unwrap();
    let taban = gecici.path().to_path_buf();
    let ikili = std::env::current_exe().unwrap();
    let mut yardimci = Command::new(ikili)
        .args([
            "--exact",
            "windows::yardimci_kalfa_gibi",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("KALFA_TEST_YARDIMCI_DIZINI", &taban)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let isaret = taban.join("calisma-alani-a").join("yardimci-torun.pid");
    let son = std::time::Instant::now() + Duration::from_secs(25);
    while !isaret.exists() && std::time::Instant::now() < son {
        if let Some(kod) = yardimci.try_wait().unwrap() {
            panic!("yardımcı hazır olmadan çıktı: {kod}");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(isaret.exists(), "yardımcı yalıtımlı torunu başlatmadı");
    let pid: u32 = std::fs::read_to_string(&isaret).unwrap().parse().unwrap();
    assert!(
        surec_aktif_mi(pid),
        "yardımcı torun süreç çalışmıyor: {pid}"
    );
    yardimci.kill().unwrap();
    yardimci.wait().unwrap();

    let sonraki = kurgu_paylasimli(taban.clone());
    let alan = alan_bilgisi(&sonraki.kok, Vec::new());
    sonraki
        .y
        .alani_kaldir(&alan)
        .unwrap_or_else(|hata| panic!("yardımcı sonrası çalışma alanı kaldırılamadı: {hata}"));
    let son = std::time::Instant::now() + Duration::from_secs(8);
    while surec_aktif_mi(pid) && std::time::Instant::now() < son {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !surec_aktif_mi(pid),
        "Kalfa süreci öldükten sonra torun yaşıyor: {pid}"
    );
    sonraki.temizle(&[alan]);
}

#[tokio::test]
async fn surec_sayisi_siniri_uygulanir() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let betik = "$p = Join-Path $env:SystemRoot 'System32\\timeout.exe'; $n = 0; for ($i=0; $i -lt 300; $i++) { try { Start-Process -FilePath $p -ArgumentList @('/t','30','/nobreak') -PassThru -WindowStyle Hidden -ErrorAction Stop | Out-Null; $n++ } catch { \"SUREC-SINIRI=$n\"; break } }; if ($n -eq 300) { 'SUREC-SINIRI=YOK' }";
    let sonuc = k.calistir(&alan, false, betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    let adet = deger(&sonuc.standart, "SUREC-SINIRI");
    assert_ne!(adet, "YOK", "süreç sınırı uygulanmadı: {}", sonuc.standart);
    assert!(
        adet.parse::<usize>().unwrap_or_default() >= 200,
        "daha erken kaynak sınırı veya yanlış hata: {}{}",
        sonuc.standart,
        sonuc.hata
    );
}

#[tokio::test]
async fn appcontainer_baska_calisma_alaninin_sid_iyle_surec_baslatamaz() {
    let k = kurgu();
    let alan_a = k.hazirla(&k.kok, Vec::new());
    let alan_b = k.hazirla(&k.baska_kok, Vec::new());
    let b_profili = format!("Kalfa.Alan.{}", alan_kimligi(&alan_b.kok));
    let b_sidi = guvenlik::profil_sidi(&b_profili).unwrap().metin();
    let betik = format!(
        "Add-Type -TypeDefinition @'\n{BASKA_SID_KODU}\n'@; $hata = [KalfaSidDenemesi]::BaskaPaketleBaslat({}, {}, {}); 'SUREC_HATASI=' + $hata",
        ps(Path::new(&b_sidi)),
        ps(&windows_koku().join("System32").join("cmd.exe")),
        ps(&windows_koku().join("System32")),
    );
    let sonuc = k.calistir(&alan_a, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert_eq!(
        deger(&sonuc.standart, "SUREC_HATASI"),
        "87",
        "{}",
        sonuc.standart
    );
}

#[tokio::test]
async fn yalıtılmış_surec_pano_ve_kabuk_penceresi_kullanimini_engeller() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let betik = format!(
        "Add-Type -TypeDefinition @'\n{UI_KISITLARI_KODU}\n'@; 'PANO=' + [KalfaUiDenemesi]::PanoyaErisim(); 'PENCERE=' + [KalfaUiDenemesi]::PencereyeIleti()"
    );
    let sonuc = k.calistir(&alan, false, &betik).await;
    assert_eq!(sonuc.bitis, Bitis::Cikti(Some(0)), "{}", sonuc.hata);
    assert_eq!(deger(&sonuc.standart, "PANO"), "5", "{}", sonuc.standart);
    let pencere = deger(&sonuc.standart, "PENCERE");
    assert_ne!(pencere, "-1", "Windows kabuk penceresi bulunamadı");
    assert_ne!(
        pencere, "1",
        "AppContainer geliştiricinin penceresine ileti gönderdi"
    );
}

#[test]
fn kaldirma_dacl_mirasini_ve_profilini_geri_alir() {
    let k = kurgu();
    let gizli = k.kok.join(".env");
    std::fs::write(&gizli, "GIZLI").unwrap();
    let kok_onceki = guvenlik::dacl_sddl(&k.kok).unwrap();
    let gizli_onceki = guvenlik::dacl_sddl(&gizli).unwrap();
    let alan = k.hazirla(&k.kok, Vec::new());
    let profil = format!("Kalfa.Alan.{}", alan_kimligi(&alan.kok));

    k.y.alani_kaldir(&alan)
        .unwrap_or_else(|hata| panic!("çalışma alanı geri alınamadı: {hata}"));

    assert_eq!(guvenlik::dacl_sddl(&alan.kok).unwrap(), kok_onceki);
    assert_eq!(guvenlik::dacl_sddl(&gizli).unwrap(), gizli_onceki);
    assert_eq!(
        appcontainer_profil_sonucu(&profil, true),
        0,
        "AppContainer profili kaldırılmamış: {profil}"
    );
}

#[test]
fn hazirlama_ve_geri_alma_kayitsiz_ayni_sid_izinini_korur() {
    let k = kurgu();
    let kimlik = alan_kimligi(&k.kok);
    let paket = guvenlik::profil_sidi(&format!("Kalfa.Alan.{kimlik}")).unwrap();
    let ozel = OzelDizinler::yol(&k.yalitim_dizini, &kimlik);
    ozel.hazirla(&GitKimligi::default()).unwrap();
    guvenlik::izin_ekle(
        &ozel.kok,
        &Izin {
            sid: paket.clone(),
            maske: MASKE_OKU,
            miras: MIRAS_YOK,
        },
    )
    .unwrap();
    let onceki_dacl = guvenlik::dacl_sddl(&ozel.kok).unwrap();
    let alan = alan_bilgisi(&k.kok, Vec::new());
    let mut ilerleme = Vec::new();

    let sonuc =
        k.y.alani_hazirla(&alan, true, &mut |satir| ilerleme.push(satir.to_string()));
    assert!(
        sonuc.is_err(),
        "kayıtsız aynı-SID ACE üzerine izin eklenmemeli"
    );
    assert_eq!(
        guvenlik::dacl_sddl(&ozel.kok).unwrap(),
        onceki_dacl,
        "hazırlık mevcut aynı-SID ACE'yi değiştirdi"
    );

    k.y.alani_kaldir(&alan)
        .unwrap_or_else(|hata| panic!("başarısız hazırlığın geri alması tamamlanamadı: {hata}"));
    assert_eq!(
        guvenlik::dacl_sddl(&ozel.kok).unwrap(),
        onceki_dacl,
        "geri alma mevcut aynı-SID ACE'yi değiştirdi"
    );
    assert!(guvenlik::aceler_oku(&ozel.kok).unwrap().iter().any(|ace| {
        ace.sid.as_ref() == Some(&paket)
            && ace.tur == 0
            && ace.maske == MASKE_OKU
            && u32::from(ace.bayraklar) & 0x0F == MIRAS_YOK
    }));
}

#[test]
fn bozuk_ace_kaydi_yalnizca_kanitlanan_acl_leri_geri_alir() {
    let k = kurgu();
    let kayitli = k.kok.join("korunan");
    let kayitsiz = k.kok.join("kayit-disinda-kesik.txt");
    std::fs::create_dir_all(&kayitli).unwrap();
    std::fs::write(&kayitsiz, "yerinde kal").unwrap();
    let kayitli_onceki = guvenlik::dacl_sddl(&kayitli).unwrap();
    let kayitsiz_onceki = guvenlik::dacl_sddl(&kayitsiz).unwrap();
    let alan = k.hazirla(
        &k.kok,
        vec![Korumali {
            yol: kayitli.clone(),
            dizin: true,
        }],
    );

    let izinler = [
        guvenlik::kullanici_sidi().unwrap(),
        Sid::metinden(SISTEM).unwrap(),
        Sid::metinden(YONETICILER).unwrap(),
    ]
    .into_iter()
    .map(|sid| Izin {
        sid,
        maske: MASKE_TAM,
        miras: MIRAS_YOK,
    })
    .collect::<Vec<_>>();
    guvenlik::mirasi_kes(&kayitsiz, &izinler).unwrap();
    let kayitsiz_kesik = guvenlik::dacl_sddl(&kayitsiz).unwrap();
    assert_ne!(kayitsiz_kesik, kayitsiz_onceki);
    let profil = format!("Kalfa.Alan.{}", alan_kimligi(&alan.kok));
    let paket = guvenlik::profil_sidi(&profil).unwrap();
    guvenlik::izin_ekle(
        &kayitsiz,
        &Izin {
            sid: paket,
            maske: MASKE_LISTELE,
            miras: MIRAS_YOK,
        },
    )
    .unwrap();
    let kayitsiz_kesik = guvenlik::dacl_sddl(&kayitsiz).unwrap();

    let ace_yolu = k.yalitim_dizini.join("ace-kaydi.jsonl");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&ace_yolu)
        .unwrap()
        .write_all(b"{bozuk}\n")
        .unwrap();
    let bozuk_y = WindowsYalitici::yeni(
        WindowsAyari {
            ev: k.ev.clone(),
            yalitim_dizini: k.yalitim_dizini.clone(),
            ..WindowsAyari::default()
        },
        Box::new(|_| {}),
    );
    assert!(matches!(bozuk_y.durum(), YalitimDurumu::Hazir { .. }));
    assert!(matches!(
        bozuk_y.baslat(&k.istek(&alan, false, "exit 0")),
        Err(BaslatmaHatasi::Yalitim(_))
    ));
    let sonuc = bozuk_y.alani_hazirla(&alan, true, &mut |_| {});
    assert!(matches!(sonuc, Err(BaslatmaHatasi::Yalitim(_))));

    let kayit = std::fs::read(&ace_yolu).unwrap();
    assert!(kayit.ends_with(b"{bozuk}\n"));
    assert_eq!(guvenlik::dacl_sddl(&kayitli).unwrap(), kayitli_onceki);
    assert_eq!(guvenlik::dacl_sddl(&kayitsiz).unwrap(), kayitsiz_kesik);
    assert_eq!(
        appcontainer_profil_sonucu(&profil, false),
        0x8007_00B7_u32 as i32
    );

    guvenlik::dacl_geri_yaz(&kayitsiz, &kayitsiz_onceki).unwrap();
    k.temizle(&[alan]);
    // Bozuk kayıtta kaldırma profili bilerek bırakır; testin profili
    // makinede kalmasın.
    guvenlik::profil_sil(&profil).unwrap();
}
