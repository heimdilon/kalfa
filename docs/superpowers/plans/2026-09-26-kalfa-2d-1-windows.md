# Kalfa 2d-1 — Windows Arka Ucu Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows'ta kabuk komutlarının AppContainer'da yalıtımla çalışması: yönetici yardımcısı `kalfa-yalitim.exe`, kabın başlatılması, çalışma alanının hazırlanması ve geri alınması, hazırlığın yerel onayı.

**Architecture:**
- **`kalfa-sandbox/src/windows/`:** SID ve AppContainer profili, DACL ve kök izni (`guvenlik.rs`), kabın komut satırı (`komut_satiri.rs`) ve başlatılması (`kap.rs`), yönetici kurulumunun kaydı (`kurulum.rs`), ACE kaydı (`ace_kaydi.rs`), MSVC ortamı (`msvc.rs`). `Surec` kap sürecini de taşır.
- **`kalfa-yalitim`:** yönetici yardımcısı: `kur`, `kaldir`, `izin-ver`, `izin-geri-al`.
- **Çekirdek, araç, uygulama:** kabuk tarifine `alan_hazirligi` girer; izin kararı onu her kipte yerel onaya çevirir. `Yalitici`'ye `hazirlik_gerekli` ve `alani_hazirla` eklenir; kabuk aracı hazırlığı komuttan önce, yalnızca onaylanmışsa yaptırır. Windows'ta yalıtım dizini `%LOCALAPPDATA%\Kalfa\yalitim`'dir.
- **`WindowsYalitici`** (Görev 4) bunları birleştirir; platform yalıtıcısı Windows'ta odur.

**Tech Stack:** Rust 1.98 · windows-sys 0.61 · tokio 1.53 · Tauri 2.11 · Svelte 5 · Vitest

**Spec:** `docs/superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md` (aşama 2 tasarımı): §3.2, §6, §7, §9, §10.5, §11, §12.2, §12.3, §14 (A11–A13, A15).

**Temel:** Bu plan `guvenlik/git-salt-okunur` dalının (PR #11) üstündedir: yalıtımda `.git` salt okunur, dış git dizinleri doğrulanır. PR #11 önce birleşir.

**Revizyon (2026-09-26):** Codex'in Görev 1 incelemesinde bulduğu üç sorun düzeltildi:
- `sade` gerçek `\\?\` ve `\\?\UNC\` öneklerini tanımıyordu; testleri de yanlış biçimi sınıyordu. Gerçek `canonicalize()` çıktısıyla bir test eklendi.
- ACE kaydının `Izin` girdisi maskeyi ve devralmayı taşımıyordu.
- Kayıt bozulmayı bildirmiyordu; artık `bozuk_mu()` var.

**İkinci revizyon (2026-09-26):** Codex'in revize Görev 1 incelemesinde bulduğu iki sorun ve aynı gözle bulunan bir üçüncüsü düzeltildi:
- `AceKaydi::oku` ortadaki boş satırı sessizce atlıyordu; artık kaydı bozuk yapar.
- Satır sonu olmayan son parça geçerli JSON'sa okunuyordu; artık hiç okunmaz, böylece bellekteki girdiler diskle aynı kalır.
- `KurulumKaydi::oku` bozuk kaydı "yok" sayıyordu; `kaldir` kök izinlerini yerinde bırakabilirdi. Artık bozuk kayıt hatadır ve `kaldir` kök izinlerini sürücülerden de bulur.

**Kod hakkında:**

Görev 1–3'ün kodu plan yazılırken ayrı bir kopyada (`deneme/2d-1-gorev`) derlendi ve doğrulandı (2026-09-26); Görev 4–6 kodsuzdur.

- **Windows 11:** `cargo test --workspace --locked` 397 test; `e2e` özellikli 68 test; clippy (normal ve `e2e`) ve fmt temiz; `ui/`'da check 0/0, 218 test, build temiz. Her görevin commit'i ayrı ayrı temiz.
- **Linux (Ubuntu 24.04, WSL2):** 380 test; üç clippy derlemesi (normal, `e2e`, `yalitim-testleri`) temiz; `kalfatest` ile `yalitim` 17 ve `uyumluluk` 4 test.
- **Makinede ölçülenler:** `kalfa-yalitim kur` geliştiricinin makinesinde çalıştı; `C:\` kökünün `D:PAI` bayrakları korundu. ACL birim testi (izin ekle, mirası kes, geri yaz) gidiş dönüşte DACL'i aynen geri getiriyor.

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında (2d-2):**
- Uygulamanın `kalfa-yalitim.exe`'yi kopyalayıp UAC ile başlatması (§9.1).
- Ayarlar'da Windows düğmeleri, hazırlanmış çalışma alanları, önbellek silme, ek ortam değişkenleri (§10.4).
- Windows'ta kabuk uçtan uca testlerinin gerçek yalıtımla koşması.

## PR ile yürütme

- **Dal ve PR:** çalışma dalı `2d-1/windows`, PR'ın hedefi `kalfa-asama-1`. PR #11 birleşmeden başlanmaz.
- **Hazır gelenler (plan commit'i):** plan; kök `Cargo.toml`'da `windows-sys` özellikleri; `crates/kalfa-sandbox/Cargo.toml`'da tokio `fs` özelliği ve Windows'ta `serde_json`; `Cargo.lock`. Bu dosyaları Görev 2'dekiler dışında değiştirme.
- **Ortam:** Rust 1.98.1; Node `^22.22.2 || >=24.15.0`. Yeni crate ya da npm paketi eklenmez. Windows'a özel kod `#[cfg(windows)]` arkasındadır.
- **Yönetici adımları:** Windows yalıtım testleri (Görev 4) kurulumdan sonra koşar. `kalfa-yalitim.exe kur` yönetici ister; ajan çalıştırmaz. Kullanıcıya PR yorumuyla sorulur; geri alma `kalfa-yalitim.exe kaldir`'dır.
- **Plandaki değişiklik biçimi:** "(tam içerik)" başlıklı dosya olduğu gibi yazılır. "şu bloğu … şununla değiştir" çiftlerinde eski blok dosyada tam olarak bir kez geçer; çiftler dosyadaki sırasıyla verilmiştir.
- **Sıra:** görevler sırayla; her görev kendi commit'ini alır; her commit'ten sonra push. Geçmiş yeniden yazılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz.
- **İnceleme:** Claude inceler; Windows'ta ve Linux'ta (WSL2) doğrular.

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:** yeni crate ya da npm paketi eklenmez.
- **Güvenlik sınırları:** aşama 2 tasarımı §7.2–§7.4, §9.2–§9.4 ve §12.2'deki Windows sütunu. Filtreler izin listesiyle yazılır; istek onaylandığı kipte çalışır (yalıtımlı onaylanan komut yalıtım yoksa, yalıtımsız onaylanan komut yalıtım varsa başlamaz). Test edilmeyen güvenlik sınırı yoktur.
- **Hazırlık:** hazırlanmamış çalışma alanı yalnızca onaylanan tarifte `alan_hazirligi` varsa hazırlanır; hazırlığın onayı her kipte yerel penceredir.
- **`unsafe`:** yalnızca `crates/kalfa-sandbox/src/agac.rs`, `crates/kalfa-sandbox/src/linux/mod.rs` ve `crates/kalfa-sandbox/src/windows/`'da, her blok `// SAFETY:` açıklamasıyla.
- **Dil:** kod, test ve açıklamalar Türkçe; kullanıcıya görünen metinler "siz" diliyle; arayüzde "yalıtım". `unwrap()`/`expect()` yalnızca testlerde ve `test_destek.rs`'de.
- **Her görevin sonunda** (Windows'ta; Linux'ta inceleme koşar):
  - `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check`.
  - `kalfa-uygulama`/`kalfa`'ya dokunulduysa `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı özellikle clippy.
  - Arayüze dokunulduysa `ui/`'da `npm run check` (0/0), `npm test`, `npm run build`.
- **Commit:** Türkçe, önekli (`sandbox:`, `çekirdek:`, `araçlar:`, `uygulama:`, `ui:`, `docs:`). Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-sandbox/src/windows/
  mod.rs          modüller; Görev 4'te WindowsYalitici                  yeni
  guvenlik.rs     Sid, profil, DACL, kök izni, sabit diskler             yeni
  komut_satiri.rs kabın komut satırı (.cmd/.bat dahil)                   yeni
  kap.rs          CreateProcessW + güvenlik yetenekleri + Job Object     yeni
  kurulum.rs      %ProgramData%\Kalfa\yalitim\kurulum.json               yeni
  ace_kaydi.rs    ACE kaydı (önce yazılır)                               yeni
  msvc.rs         MSVC PATH/LIB/INCLUDE                                  yeni
crates/kalfa-sandbox/src/{agac,surec,lib}.rs                             (değişir)
crates/kalfa-yalitim/                                                    yeni
crates/kalfa-core/src/{kabuk,izin}.rs, crates/kalfa-tools/src/kabuk/mod.rs,
crates/kalfa-uygulama/src/{dizin,guvenlik,oturumlar,uygulama,yalitim}.rs,
crates/kalfa/src/lib.rs, ui/src/{ipc/tipler.ts,dil/tr.ts}                (değişir)
```

---

### Task 1: `kalfa-sandbox`: Windows temeli

**Files:** `crates/kalfa-sandbox/src/windows/*` (yeni), `crates/kalfa-sandbox/src/{agac,surec,lib}.rs` (değişir).

**Interfaces (Produces):**
- `windows::guvenlik`: `Sid`, `Izin`, `Ace`, `yetenek_sidi`, `arac_zinciri_sidi`, `profil_olustur`, `profil_sidi`, `profil_sil`, `kullanici_sidi`, `yukseltilmis_mi`, `aceler_oku`, `mirasi_kesik_mi`, `dacl_sddl`, `izin_ekle`, `izni_kaldir`, `mirasi_kes`, `dacl_geri_yaz`, `kok_izni_var_mi`, `kok_izni_ver`, `kok_izni_kaldir`, `sabit_diskler`, `surucu_koku`, `sade`, `program_data`; maske ve miras sabitleri.
- `windows::kap` (crate içi): `KapIstegi`, `Kap`, `baslat(&KapIstegi, &Is)`, `bekle`, `hata_kutularini_kapat`, `sistem_dizini`.
- `windows::komut_satiri` (crate içi): `KomutSatiri`, `uret(program, argumanlar, cmd)`.
- `windows::kurulum`: `KurulumKaydi { kok_izinleri, izinli_dizinler }` (`oku(&Path) -> io::Result<Option<Self>>`: yoksa `None`, bozuksa `InvalidData` hatası; `yaz`, `sil`), `kayit_dizini()`.
- `windows::ace_kaydi`: `AceKaydi` (`oku`, `ekle`, `bozuk_mu`, `hazir_mi`, `hazir_alanlar`, `ilk_dacl`, `baska_alanlarin_sidleri`, `alani_cikar`), `Girdi { Profil, Izin { alan, yol, sid, maske, miras }, MirasKesildi, Hazir }`, `ORTAK`. Son satır sonundan sonraki parça (yarım kalmış girdi) geçerli JSON olsa da okunmaz ve bir sonraki `ekle`'de kesilir. Tamamlanmış her satır (boş satır dahil) çözülmelidir; çözülemeyen satır ya da okunamayan dosya `bozuk_mu()`'yu doğru yapar.
- `windows::msvc`: `MsvcOrtami { bin, lib, include }`, `bul`, `coz`, `uygula`.
- `Surec::kaptan(Kap, Is)`, `Agac::isten(Is)`, `Is::yeni(ui_kisiti)`.

Görev 4'e kadar kullanılmayan kod `#[allow(dead_code)]` taşır; Görev 4 bunları kaldırır.

- [ ] **Step 1: Dosyaları yaz**

**`crates/kalfa-sandbox/src/windows/mod.rs`** (tam içerik):

```rust
//! Windows arka ucu: AppContainer + ACL (aşama 2 tasarımı §9).

// Kap başlatma Görev 4'teki `WindowsYalitici`'ye kadar kullanılmaz.
// TODO(2d-1 Görev 4): bu satırı kaldır.
#![allow(dead_code)]

pub mod ace_kaydi;
pub mod guvenlik;
pub(crate) mod kap;
pub(crate) mod komut_satiri;
pub mod kurulum;
pub mod msvc;
```

**`crates/kalfa-sandbox/src/windows/guvenlik.rs`** (tam içerik):

```rust
//! Windows güvenlik yardımcıları (aşama 2 tasarımı §9): SID'ler,
//! AppContainer profilleri, dosya izinleri (DACL) ve sürücü köklerindeki
//! listeleme izni.
//!
//! İzinler Win32 API'siyle verilir: `icacls` hiçbir hesaba karşılık gelmeyen
//! SID'leri (paket ve yetenek SID'leri) reddediyor (§9.4).

use std::ffi::{OsStr, c_void};
use std::fmt;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS, GetLastError, HANDLE, HLOCAL, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSecurityDescriptorToStringSecurityDescriptorW, ConvertSidToStringSidW,
    ConvertStringSecurityDescriptorToSecurityDescriptorW, ConvertStringSidToSidW,
    EXPLICIT_ACCESS_W, GRANT_ACCESS, GetNamedSecurityInfoW, NO_MULTIPLE_TRUSTEE, REVOKE_ACCESS,
    SDDL_REVISION_1, SE_FILE_OBJECT, SetEntriesInAclW, SetNamedSecurityInfoW, TRUSTEE_IS_SID,
    TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
};
use windows_sys::Win32::Security::Isolation::{
    CreateAppContainerProfile, DeleteAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
    DACL_SECURITY_INFORMATION, DeriveCapabilitySidsFromName, FreeSid, GetAce, GetAclInformation,
    GetFileSecurityW, GetLengthSid, GetSecurityDescriptorControl, GetSecurityDescriptorDacl,
    GetTokenInformation, INHERIT_ONLY_ACE, InitializeSecurityDescriptor,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SE_DACL_AUTO_INHERIT_REQ,
    SE_DACL_AUTO_INHERITED, SE_DACL_PROTECTED, SECURITY_DESCRIPTOR, SetFileSecurityW,
    SetSecurityDescriptorControl, SetSecurityDescriptorDacl, TOKEN_ELEVATION, TOKEN_QUERY,
    TOKEN_USER, TokenElevation, TokenUser, UNPROTECTED_DACL_SECURITY_INFORMATION,
};
use windows_sys::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::SystemServices::SECURITY_DESCRIPTOR_REVISION;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath};

/// Kalfa'nın bütün kaplarının taşıdığı araç zinciri yeteneğinin adı (§9).
/// SID'i addan türetilir: `S-1-15-3-1024-…`.
pub const ARAC_ZINCIRI_YETENEGI: &str = "kalfaAracZinciri";
/// `internetClient` yeteneği; ağlı çalıştırmada verilir (§9.3).
pub const INTERNET_ISTEMCISI: &str = "S-1-15-3-1";
/// Yöneticiler ve SYSTEM; mirası kesilen nesnelerde tam izin alırlar.
pub const YONETICILER: &str = "S-1-5-32-544";
pub const SISTEM: &str = "S-1-5-18";

/// `ListDirectory | ReadAttributes | Traverse | Synchronize` (`0x1000A1`):
/// kök ve üst dizinlerde adları listelemek için (§9.2).
pub const MASKE_LISTELE: u32 = 0x0010_00A1;
/// `ReadAndExecute`.
pub const MASKE_OKU: u32 = 0x0012_00A9;
/// `ReadAndExecute | Write`: silme ve yeniden adlandırma yok.
pub const MASKE_OKU_YAZ: u32 = 0x0012_01BF;
/// `Modify`: okuma, yazma, çalıştırma ve silme.
pub const MASKE_DEGISTIR: u32 = 0x0013_01BF;
/// `FullControl`.
pub const MASKE_TAM: u32 = 0x001F_01FF;

/// Miras bayrakları.
pub const MIRAS_YOK: u32 = 0;
/// `OI|CI`: nesnenin kendisi ve altındaki her şey.
pub const MIRAS: u32 = 0x3;
/// `OI|CI|IO`: yalnızca alttakiler, nesnenin kendisi değil.
pub const YALNIZ_ALTTAKILER: u32 = 0x3 | INHERIT_ONLY_ACE;

const HRESULT_ZATEN_VAR: i32 = 0x8007_00B7_u32 as i32;
const HRESULT_BULUNAMADI: i32 = 0x8007_0002_u32 as i32;
const DRIVE_FIXED: u32 = 3;
const FILE_PERSISTENT_ACLS: u32 = 0x8;

/// NUL ile biten UTF-16 dize.
pub(crate) fn genis(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

/// # Safety
/// `p`, NUL ile biten bir UTF-16 dizeyi göstermelidir.
unsafe fn genisten(p: *const u16) -> String {
    // SAFETY: çağıran dizenin NUL ile bittiğini garanti eder.
    unsafe {
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(p, n))
    }
}

fn son_hata(adim: &str) -> io::Error {
    let e = io::Error::last_os_error();
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

fn win32_hatasi(adim: &str, kod: u32) -> io::Error {
    let e = io::Error::from_raw_os_error(kod as i32);
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

fn hresult_hatasi(adim: &str, hr: i32) -> io::Error {
    // HRESULT_FROM_WIN32: alt 16 bit Win32 hata kodudur.
    let kod = if (hr as u32) & 0xFFFF_0000 == 0x8007_0000 {
        (hr as u32) & 0xFFFF
    } else {
        hr as u32
    };
    win32_hatasi(adim, kod)
}

/// `LocalFree` ile bırakılan bellek.
struct Yerel(*mut c_void);

impl Drop for Yerel {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: işaretçi bir Win32 işlevinin LocalAlloc ile ayırdığı
            // bellektir ve yalnızca burada bırakılır.
            unsafe { LocalFree(self.0 as HLOCAL) };
        }
    }
}

/// Sahipli bir SID.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Sid(Vec<u8>);

impl Sid {
    /// `S-1-…` biçiminden.
    pub fn metinden(s: &str) -> io::Result<Sid> {
        let mut p: PSID = null_mut();
        // SAFETY: dize NUL ile biter; dönen SID LocalFree ile bırakılır.
        unsafe {
            if ConvertStringSidToSidW(genis(s).as_ptr(), &mut p) == 0 {
                return Err(son_hata(&format!("SID çözülemedi ({s})")));
            }
            let _bellek = Yerel(p);
            Ok(Sid::kopyala(p))
        }
    }

    /// # Safety
    /// `p` geçerli bir SID'i göstermelidir.
    unsafe fn kopyala(p: PSID) -> Sid {
        // SAFETY: çağıran SID'in geçerli olduğunu garanti eder.
        unsafe {
            let n = GetLengthSid(p) as usize;
            Sid(std::slice::from_raw_parts(p as *const u8, n).to_vec())
        }
    }

    pub(crate) fn ptr(&self) -> PSID {
        self.0.as_ptr() as PSID
    }

    pub fn metin(&self) -> String {
        let mut p: *mut u16 = null_mut();
        // SAFETY: SID geçerlidir; dize LocalFree ile bırakılır.
        unsafe {
            if ConvertSidToStringSidW(self.ptr(), &mut p) == 0 {
                return String::from("S-?");
            }
            let _bellek = Yerel(p as *mut c_void);
            genisten(p)
        }
    }
}

impl fmt::Debug for Sid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.metin())
    }
}

/// Addan türetilen yetenek SID'i (`DeriveCapabilitySidsFromName`).
pub fn yetenek_sidi(ad: &str) -> io::Result<Sid> {
    let mut gruplar: *mut PSID = null_mut();
    let mut grup_sayisi = 0u32;
    let mut yetenekler: *mut PSID = null_mut();
    let mut yetenek_sayisi = 0u32;
    // SAFETY: çıktı dizileri ve içlerindeki SID'ler LocalFree ile bırakılır.
    unsafe {
        if DeriveCapabilitySidsFromName(
            genis(ad).as_ptr(),
            &mut gruplar,
            &mut grup_sayisi,
            &mut yetenekler,
            &mut yetenek_sayisi,
        ) == 0
        {
            return Err(son_hata("DeriveCapabilitySidsFromName"));
        }
        let sonuc = if yetenek_sayisi == 0 {
            Err(io::Error::other("yetenek SID'i türetilemedi"))
        } else {
            Ok(Sid::kopyala(*yetenekler))
        };
        for i in 0..grup_sayisi as usize {
            LocalFree(*gruplar.add(i) as HLOCAL);
        }
        for i in 0..yetenek_sayisi as usize {
            LocalFree(*yetenekler.add(i) as HLOCAL);
        }
        LocalFree(gruplar as HLOCAL);
        LocalFree(yetenekler as HLOCAL);
        sonuc
    }
}

/// Araç zinciri yeteneğinin SID'i.
pub fn arac_zinciri_sidi() -> io::Result<Sid> {
    yetenek_sidi(ARAC_ZINCIRI_YETENEGI)
}

/// AppContainer profilini oluşturur; varsa SID'ini addan türetir.
pub fn profil_olustur(ad: &str, aciklama: &str) -> io::Result<Sid> {
    let mut p: PSID = null_mut();
    let ad_g = genis(ad);
    // SAFETY: dizeler NUL ile biter; SID FreeSid ile bırakılır.
    unsafe {
        let hr = CreateAppContainerProfile(
            ad_g.as_ptr(),
            ad_g.as_ptr(),
            genis(aciklama).as_ptr(),
            null(),
            0,
            &mut p,
        );
        if hr == HRESULT_ZATEN_VAR {
            return profil_sidi(ad);
        }
        if hr != 0 {
            return Err(hresult_hatasi("CreateAppContainerProfile", hr));
        }
        let sid = Sid::kopyala(p);
        FreeSid(p);
        Ok(sid)
    }
}

/// Profilin SID'i addan türetilir; profil var olmasa da aynıdır.
pub fn profil_sidi(ad: &str) -> io::Result<Sid> {
    let mut p: PSID = null_mut();
    // SAFETY: dize NUL ile biter; SID FreeSid ile bırakılır.
    unsafe {
        let hr = DeriveAppContainerSidFromAppContainerName(genis(ad).as_ptr(), &mut p);
        if hr != 0 {
            return Err(hresult_hatasi(
                "DeriveAppContainerSidFromAppContainerName",
                hr,
            ));
        }
        let sid = Sid::kopyala(p);
        FreeSid(p);
        Ok(sid)
    }
}

/// Profili siler; profil yoksa başarılı sayılır. Yükseltilmemiş süreçten
/// çağrılmalıdır: 2c'de yükseltilmiş süreçten silinen profiller yerinde kaldı.
pub fn profil_sil(ad: &str) -> io::Result<()> {
    // SAFETY: dize NUL ile biter.
    let hr = unsafe { DeleteAppContainerProfile(genis(ad).as_ptr()) };
    if hr != 0 && hr != HRESULT_BULUNAMADI {
        return Err(hresult_hatasi("DeleteAppContainerProfile", hr));
    }
    Ok(())
}

/// Sürecin belirteci; düşünce kapanır.
struct Belirtec(HANDLE);

impl Belirtec {
    fn ac() -> io::Result<Self> {
        let mut h: HANDLE = null_mut();
        // SAFETY: sözde tutamaç ve çıktı tutamacı; tutamaç Drop'ta kapanır.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h) } == 0 {
            return Err(son_hata("OpenProcessToken"));
        }
        Ok(Self(h))
    }

    fn bilgi(&self, sinif: i32) -> io::Result<Vec<u64>> {
        let mut gerekli = 0u32;
        // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
        unsafe {
            GetTokenInformation(self.0, sinif, null_mut(), 0, &mut gerekli);
            let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8).max(1)];
            if GetTokenInformation(
                self.0,
                sinif,
                tampon.as_mut_ptr() as *mut c_void,
                // Bazı sınıflar (TokenElevation) tam boyut ister.
                gerekli,
                &mut gerekli,
            ) == 0
            {
                return Err(son_hata("GetTokenInformation"));
            }
            Ok(tampon)
        }
    }
}

impl Drop for Belirtec {
    fn drop(&mut self) {
        // SAFETY: tutamaç bu yapıya aittir.
        unsafe { CloseHandle(self.0) };
    }
}

/// Geliştiricinin (sürecin kullanıcısının) SID'i.
pub fn kullanici_sidi() -> io::Result<Sid> {
    let tampon = Belirtec::ac()?.bilgi(TokenUser)?;
    // SAFETY: tampon bir TOKEN_USER yapısıyla başlar; SID tamponun içindedir.
    unsafe {
        let kullanici = &*(tampon.as_ptr() as *const TOKEN_USER);
        Ok(Sid::kopyala(kullanici.User.Sid))
    }
}

/// Süreç yükseltilmiş (yönetici) mi?
pub fn yukseltilmis_mi() -> io::Result<bool> {
    let tampon = Belirtec::ac()?.bilgi(TokenElevation)?;
    // SAFETY: tampon bir TOKEN_ELEVATION yapısıdır.
    let yukseltme = unsafe { &*(tampon.as_ptr() as *const TOKEN_ELEVATION) };
    Ok(yukseltme.TokenIsElevated != 0)
}

/// Bir DACL girdisi: kime, hangi izin, hangi miras.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Izin {
    pub sid: Sid,
    pub maske: u32,
    pub miras: u32,
}

/// DACL'deki bir izin girdisi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ace {
    /// `ACCESS_ALLOWED_ACE_TYPE` (0) ya da `ACCESS_DENIED_ACE_TYPE` (1);
    /// diğer türlerin SID'i okunmaz.
    pub tur: u8,
    pub bayraklar: u8,
    pub maske: u32,
    pub sid: Option<Sid>,
}

impl Ace {
    /// Mirasla gelmiş girdi mi?
    pub fn miras_mi(&self) -> bool {
        u32::from(self.bayraklar) & 0x10 != 0
    }
}

/// Bir nesnenin güvenlik tanımlayıcısı (`GetNamedSecurityInfoW`).
struct Tanimlayici {
    sd: Yerel,
    dacl: *mut ACL,
}

impl Tanimlayici {
    fn oku(yol: &Path) -> io::Result<Self> {
        let mut dacl: *mut ACL = null_mut();
        let mut sd: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: yol NUL ile biter; tanımlayıcı LocalFree ile bırakılır; DACL
        // tanımlayıcının içini gösterir.
        let kod = unsafe {
            GetNamedSecurityInfoW(
                genis(yol).as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                &mut dacl,
                null_mut(),
                &mut sd,
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(win32_hatasi(
                &format!("GetNamedSecurityInfoW({})", yol.display()),
                kod,
            ));
        }
        Ok(Self {
            sd: Yerel(sd),
            dacl,
        })
    }

    fn denetim(&self) -> u16 {
        let mut denetim = 0u16;
        let mut surum = 0u32;
        // SAFETY: tanımlayıcı geçerlidir.
        unsafe { GetSecurityDescriptorControl(self.sd.0, &mut denetim, &mut surum) };
        denetim
    }

    fn aceler(&self) -> Vec<Ace> {
        // SAFETY: DACL geçerlidir ya da boştur (tam erişim); GetAce ile okunan
        // girdiler DACL'in içindedir.
        unsafe { aceler(self.dacl) }
    }
}

/// # Safety
/// `dacl` boş ya da geçerli bir ACL olmalıdır.
unsafe fn aceler(dacl: *const ACL) -> Vec<Ace> {
    if dacl.is_null() {
        return Vec::new();
    }
    // SAFETY: çağıran ACL'in geçerli olduğunu garanti eder.
    unsafe {
        let mut bilgi: ACL_SIZE_INFORMATION = std::mem::zeroed();
        if GetAclInformation(
            dacl,
            (&raw mut bilgi).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        ) == 0
        {
            return Vec::new();
        }
        let mut sonuc = Vec::with_capacity(bilgi.AceCount as usize);
        for i in 0..bilgi.AceCount {
            let mut p: *mut c_void = null_mut();
            if GetAce(dacl, i, &mut p) == 0 {
                continue;
            }
            let baslik = &*(p as *const ACE_HEADER);
            // İzin ve ret girdilerinin gövdesi aynıdır: maske ve SID.
            let sid = if baslik.AceType <= 1 {
                let girdi = p as *const ACCESS_ALLOWED_ACE;
                Some(Sid::kopyala((&raw const (*girdi).SidStart) as PSID))
            } else {
                None
            };
            let maske = if baslik.AceType <= 1 {
                (*(p as *const ACCESS_ALLOWED_ACE)).Mask
            } else {
                0
            };
            sonuc.push(Ace {
                tur: baslik.AceType,
                bayraklar: baslik.AceFlags,
                maske,
                sid,
            });
        }
        sonuc
    }
}

/// Nesnenin DACL'indeki girdiler.
pub fn aceler_oku(yol: &Path) -> io::Result<Vec<Ace>> {
    Ok(Tanimlayici::oku(yol)?.aceler())
}

/// Nesnenin DACL'i mirası kesilmiş (korumalı) mi?
pub fn mirasi_kesik_mi(yol: &Path) -> io::Result<bool> {
    Ok(Tanimlayici::oku(yol)?.denetim() & SE_DACL_PROTECTED != 0)
}

/// Nesnenin DACL'i SDDL olarak (`D:…`); geri almada yazılır.
pub fn dacl_sddl(yol: &Path) -> io::Result<String> {
    let t = Tanimlayici::oku(yol)?;
    let mut metin: *mut u16 = null_mut();
    // SAFETY: tanımlayıcı geçerlidir; dize LocalFree ile bırakılır.
    unsafe {
        if ConvertSecurityDescriptorToStringSecurityDescriptorW(
            t.sd.0,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut metin,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertSecurityDescriptorToStringSecurityDescriptorW",
            ));
        }
        let _bellek = Yerel(metin as *mut c_void);
        Ok(genisten(metin))
    }
}

fn erisim(izin: &Izin, kip: i32) -> EXPLICIT_ACCESS_W {
    EXPLICIT_ACCESS_W {
        grfAccessPermissions: izin.maske,
        grfAccessMode: kip,
        grfInheritance: izin.miras,
        Trustee: TRUSTEE_W {
            pMultipleTrustee: null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: izin.sid.ptr() as *mut u16,
        },
    }
}

/// Girdileri `eski` DACL'e ekler ya da eski DACL'den çıkarır; yeni DACL
/// LocalFree ile bırakılır.
fn yeni_dacl(eski: *const ACL, girdiler: &[EXPLICIT_ACCESS_W]) -> io::Result<Yerel> {
    let mut yeni: *mut ACL = null_mut();
    // SAFETY: girdilerin SID'leri çağrı boyunca yaşar; eski DACL boş ya da
    // geçerlidir.
    let kod =
        unsafe { SetEntriesInAclW(girdiler.len() as u32, girdiler.as_ptr(), eski, &mut yeni) };
    if kod != ERROR_SUCCESS {
        return Err(win32_hatasi("SetEntriesInAclW", kod));
    }
    Ok(Yerel(yeni as *mut c_void))
}

fn dacl_yaz(yol: &Path, dacl: *const ACL, korumali: bool) -> io::Result<()> {
    let koruma = if korumali {
        PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    // SAFETY: yol NUL ile biter; DACL geçerlidir. Değişiklik alttakilere
    // yayılır (mirasla gelen girdiler yeniden hesaplanır).
    let kod = unsafe {
        SetNamedSecurityInfoW(
            genis(yol).as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION | koruma,
            null_mut(),
            null_mut(),
            dacl,
            null(),
        )
    };
    if kod != ERROR_SUCCESS {
        return Err(win32_hatasi(
            &format!("SetNamedSecurityInfoW({})", yol.display()),
            kod,
        ));
    }
    Ok(())
}

/// Nesneye izin ekler. Mirası olan izin bütün alt ağaca yayılır; mirası
/// kesilmiş alt nesneler onu almaz.
pub fn izin_ekle(yol: &Path, izin: &Izin) -> io::Result<()> {
    let t = Tanimlayici::oku(yol)?;
    let yeni = yeni_dacl(t.dacl, &[erisim(izin, GRANT_ACCESS)])?;
    dacl_yaz(
        yol,
        yeni.0 as *const ACL,
        t.denetim() & SE_DACL_PROTECTED != 0,
    )
}

/// SID'in nesnedeki açık izinlerini siler; mirasla gelenler kaynağından
/// silinince kalkar.
pub fn izni_kaldir(yol: &Path, sid: &Sid) -> io::Result<()> {
    let t = Tanimlayici::oku(yol)?;
    let var = t
        .aceler()
        .iter()
        .any(|a| !a.miras_mi() && a.sid.as_ref() == Some(sid));
    if !var {
        return Ok(());
    }
    let girdi = Izin {
        sid: sid.clone(),
        maske: 0,
        miras: MIRAS_YOK,
    };
    let yeni = yeni_dacl(t.dacl, &[erisim(&girdi, REVOKE_ACCESS)])?;
    dacl_yaz(
        yol,
        yeni.0 as *const ACL,
        t.denetim() & SE_DACL_PROTECTED != 0,
    )
}

/// Nesnenin mirasını keser; yeni DACL yalnızca verilen izinlerden oluşur.
pub fn mirasi_kes(yol: &Path, izinler: &[Izin]) -> io::Result<()> {
    let girdiler: Vec<EXPLICIT_ACCESS_W> =
        izinler.iter().map(|i| erisim(i, GRANT_ACCESS)).collect();
    let yeni = yeni_dacl(null(), &girdiler)?;
    dacl_yaz(yol, yeni.0 as *const ACL, true)
}

/// SDDL'den okunan DACL'i nesneye yazar. Tanımlayıcı mirası kesilmiş değilse
/// miras yeniden açılır ve mirasla gelen girdiler üstten yeniden hesaplanır.
pub fn dacl_geri_yaz(yol: &Path, sddl: &str) -> io::Result<()> {
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: dize NUL ile biter; tanımlayıcı LocalFree ile bırakılır.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            genis(sddl).as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        ) == 0
        {
            return Err(son_hata(
                "ConvertStringSecurityDescriptorToSecurityDescriptorW",
            ));
        }
    }
    let sd = Yerel(sd);
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut denetim = 0u16;
    let mut surum = 0u32;
    // SAFETY: tanımlayıcı geçerlidir; DACL onun içini gösterir.
    unsafe {
        if GetSecurityDescriptorDacl(sd.0, &mut var, &mut dacl, &mut varsayilan) == 0 {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        GetSecurityDescriptorControl(sd.0, &mut denetim, &mut surum);
    }
    dacl_yaz(yol, dacl, denetim & SE_DACL_PROTECTED != 0)
}

/// Sürücü kökünün DACL'ini okur (`GetFileSecurityW`).
fn kok_tanimlayicisi(kok: &Path) -> io::Result<Vec<u64>> {
    let yol = genis(kok);
    let mut gerekli = 0u32;
    // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
    unsafe {
        GetFileSecurityW(
            yol.as_ptr(),
            DACL_SECURITY_INFORMATION,
            null_mut(),
            0,
            &mut gerekli,
        );
        if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
            return Err(son_hata("GetFileSecurityW"));
        }
        let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8)];
        if GetFileSecurityW(
            yol.as_ptr(),
            DACL_SECURITY_INFORMATION,
            tampon.as_mut_ptr() as PSECURITY_DESCRIPTOR,
            (tampon.len() * 8) as u32,
            &mut gerekli,
        ) == 0
        {
            return Err(son_hata("GetFileSecurityW"));
        }
        Ok(tampon)
    }
}

/// Kökte SID'in listeleme izni var mı?
pub fn kok_izni_var_mi(kok: &Path, sid: &Sid) -> io::Result<bool> {
    let tampon = kok_tanimlayicisi(kok)?;
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    // SAFETY: tampon GetFileSecurityW'nin yazdığı tanımlayıcıdır.
    let girdiler = unsafe {
        if GetSecurityDescriptorDacl(
            tampon.as_ptr() as PSECURITY_DESCRIPTOR,
            &mut var,
            &mut dacl,
            &mut varsayilan,
        ) == 0
        {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        aceler(dacl)
    };
    Ok(girdiler.iter().any(|a| {
        a.tur == 0
            && u32::from(a.bayraklar) & INHERIT_ONLY_ACE == 0
            && a.maske & MASKE_LISTELE == MASKE_LISTELE
            && a.sid.as_ref() == Some(sid)
    }))
}

/// Kökün DACL'ini değiştirir. Yalnızca kökü değiştiren `SetFileSecurityW`
/// kullanılır: `SetNamedSecurityInfoW` mirası bütün sürücüye yeniden yayar.
/// `SE_DACL_AUTO_INHERIT_REQ` verilmezse kökün `SE_DACL_AUTO_INHERITED`
/// bayrağı düşer (2c'de `D:PAI` → `D:P`).
fn kok_dacl_degistir(kok: &Path, girdi: EXPLICIT_ACCESS_W) -> io::Result<()> {
    let tampon = kok_tanimlayicisi(kok)?;
    let eski_sd = tampon.as_ptr() as PSECURITY_DESCRIPTOR;
    let mut var = 0;
    let mut varsayilan = 0;
    let mut dacl: *mut ACL = null_mut();
    let mut denetim = 0u16;
    let mut surum = 0u32;
    // SAFETY: tampon GetFileSecurityW'nin yazdığı tanımlayıcıdır.
    unsafe {
        if GetSecurityDescriptorDacl(eski_sd, &mut var, &mut dacl, &mut varsayilan) == 0 {
            return Err(son_hata("GetSecurityDescriptorDacl"));
        }
        GetSecurityDescriptorControl(eski_sd, &mut denetim, &mut surum);
    }
    let yeni = yeni_dacl(dacl, &[girdi])?;
    // SAFETY: yeni tanımlayıcı yığındadır ve DACL'i yeni ACL'i gösterir; ikisi
    // de SetFileSecurityW dönene kadar yaşar.
    unsafe {
        let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
        let p = (&raw mut sd).cast::<c_void>();
        if InitializeSecurityDescriptor(p, SECURITY_DESCRIPTOR_REVISION) == 0 {
            return Err(son_hata("InitializeSecurityDescriptor"));
        }
        if SetSecurityDescriptorDacl(p, 1, yeni.0 as *const ACL, 0) == 0 {
            return Err(son_hata("SetSecurityDescriptorDacl"));
        }
        let ilgili = SE_DACL_AUTO_INHERIT_REQ | SE_DACL_AUTO_INHERITED | SE_DACL_PROTECTED;
        let bitler =
            SE_DACL_AUTO_INHERIT_REQ | (denetim & (SE_DACL_AUTO_INHERITED | SE_DACL_PROTECTED));
        if SetSecurityDescriptorControl(p, ilgili, bitler) == 0 {
            return Err(son_hata("SetSecurityDescriptorControl"));
        }
        if SetFileSecurityW(genis(kok).as_ptr(), DACL_SECURITY_INFORMATION, p) == 0 {
            return Err(son_hata(&format!("SetFileSecurityW({})", kok.display())));
        }
    }
    Ok(())
}

/// Köke SID için listeleme izni verir; miras yok (§9.2). Yönetici gerekir.
pub fn kok_izni_ver(kok: &Path, sid: &Sid) -> io::Result<()> {
    let izin = Izin {
        sid: sid.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    kok_dacl_degistir(kok, erisim(&izin, GRANT_ACCESS))
}

/// Kökteki izni kaldırır. Yönetici gerekir.
pub fn kok_izni_kaldir(kok: &Path, sid: &Sid) -> io::Result<()> {
    let izin = Izin {
        sid: sid.clone(),
        maske: 0,
        miras: MIRAS_YOK,
    };
    kok_dacl_degistir(kok, erisim(&izin, REVOKE_ACCESS))
}

/// ACL taşıyan sabit disklerin kökleri (`C:\`, `D:\` …); FAT32 ve exFAT
/// sayılmaz (§9.6).
pub fn sabit_diskler() -> Vec<PathBuf> {
    // SAFETY: bağımsız çağrılar; dizeler NUL ile biter.
    let maske = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| maske & (1 << i) != 0)
        .map(|i| PathBuf::from(format!("{}:\\", char::from(b'A' + i))))
        .filter(|kok| {
            let g = genis(kok);
            // SAFETY: dize NUL ile biter; çıktılar yerel değişkenlerdir.
            unsafe {
                if GetDriveTypeW(g.as_ptr()) != DRIVE_FIXED {
                    return false;
                }
                let mut bayraklar = 0u32;
                GetVolumeInformationW(
                    g.as_ptr(),
                    null_mut(),
                    0,
                    null_mut(),
                    null_mut(),
                    &mut bayraklar,
                    null_mut(),
                    0,
                ) != 0
                    && bayraklar & FILE_PERSISTENT_ACLS != 0
            }
        })
        .collect()
}

/// `%ProgramData%`; ortam değişkeninden değil kabuğun bilinen dizininden
/// okunur: yükseltilmiş yardımcının ortamı çağıranın elindedir.
pub fn program_data() -> io::Result<PathBuf> {
    let mut p: *mut u16 = null_mut();
    // SAFETY: dönen dize CoTaskMemFree ile bırakılır.
    unsafe {
        let hr = SHGetKnownFolderPath(&FOLDERID_ProgramData, 0, null_mut(), &mut p);
        if hr != 0 {
            if !p.is_null() {
                CoTaskMemFree(p as *const c_void);
            }
            return Err(hresult_hatasi("SHGetKnownFolderPath", hr));
        }
        let yol = PathBuf::from(genisten(p));
        CoTaskMemFree(p as *const c_void);
        Ok(yol)
    }
}

/// `\\?\C:\…` biçimindeki yolu `C:\…`'ye, `\\?\UNC\…`'yu `\\…`'ya çevirir.
/// `canonicalize` Windows'ta uzun yol önekini ekler; cmd.exe bu önekli
/// çalışma dizinini UNC sanıp reddediyor.
pub fn sade(yol: &Path) -> PathBuf {
    let metin = yol.as_os_str().to_string_lossy();
    if let Some(kalan) = metin.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{kalan}"));
    }
    match metin.strip_prefix(r"\\?\") {
        Some(kalan) if kalan.as_bytes().get(1) == Some(&b':') => PathBuf::from(kalan),
        _ => yol.to_path_buf(),
    }
}

/// Yolun bulunduğu sürücünün kökü (`C:\`).
pub fn surucu_koku(yol: &Path) -> Option<PathBuf> {
    use std::path::{Component, Prefix};
    match yol.components().next()? {
        Component::Prefix(p) => match p.kind() {
            Prefix::Disk(h) | Prefix::VerbatimDisk(h) => {
                Some(PathBuf::from(format!("{}:\\", char::from(h))))
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yetenek_sidi_addan_ayni_turer() {
        let a = arac_zinciri_sidi().unwrap();
        let b = yetenek_sidi(ARAC_ZINCIRI_YETENEGI).unwrap();
        assert_eq!(a, b);
        assert!(a.metin().starts_with("S-1-15-3-1024-"), "{a:?}");
        assert_ne!(a, yetenek_sidi("baskaYetenek").unwrap());
    }

    #[test]
    fn kullanici_sidi_ve_yukseltme_okunur() {
        assert!(kullanici_sidi().unwrap().metin().starts_with("S-1-5-"));
        yukseltilmis_mi().unwrap();
    }

    #[test]
    fn sid_metinden_ve_metne() {
        let s = Sid::metinden(YONETICILER).unwrap();
        assert_eq!(s.metin(), YONETICILER);
        assert!(Sid::metinden("S-x").is_err());
    }

    #[test]
    fn profil_sidi_addan_turer_ve_olusturmadan_ayni() {
        let ad = format!("Kalfa.Test.{}", std::process::id());
        let turetilen = profil_sidi(&ad).unwrap();
        let olusan = profil_olustur(&ad, "Kalfa testi").unwrap();
        assert_eq!(turetilen, olusan);
        assert_eq!(profil_olustur(&ad, "Kalfa testi").unwrap(), olusan);
        assert!(olusan.metin().starts_with("S-1-15-2-"));
        profil_sil(&ad).unwrap();
        profil_sil(&ad).unwrap();
    }

    #[test]
    fn izin_eklenir_miras_kesilir_ve_geri_yazilir() {
        let d = tempfile::tempdir().unwrap();
        let alt = d.path().join("alt");
        std::fs::create_dir(&alt).unwrap();
        let dosya = alt.join("a.txt");
        std::fs::write(&dosya, "a").unwrap();
        let sid = yetenek_sidi("kalfaTestYetenegi").unwrap();
        let once = dacl_sddl(&dosya).unwrap();

        izin_ekle(
            d.path(),
            &Izin {
                sid: sid.clone(),
                maske: MASKE_DEGISTIR,
                miras: MIRAS,
            },
        )
        .unwrap();
        let miras = aceler_oku(&dosya).unwrap();
        assert!(
            miras
                .iter()
                .any(|a| a.miras_mi() && a.sid.as_ref() == Some(&sid)),
            "{miras:?}"
        );

        mirasi_kes(
            &dosya,
            &[Izin {
                sid: kullanici_sidi().unwrap(),
                maske: MASKE_TAM,
                miras: MIRAS_YOK,
            }],
        )
        .unwrap();
        assert!(mirasi_kesik_mi(&dosya).unwrap());
        assert!(
            !aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.sid.as_ref() == Some(&sid))
        );
        std::fs::read(&dosya).unwrap();

        dacl_geri_yaz(&dosya, &once).unwrap();
        assert!(!mirasi_kesik_mi(&dosya).unwrap());
        assert!(
            aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.miras_mi() && a.sid.as_ref() == Some(&sid))
        );

        izni_kaldir(d.path(), &sid).unwrap();
        assert!(
            !aceler_oku(&dosya)
                .unwrap()
                .iter()
                .any(|a| a.sid.as_ref() == Some(&sid))
        );
        assert_eq!(dacl_sddl(&dosya).unwrap(), once);
    }

    #[test]
    fn sade_uzun_yol_onekini_kaldirir() {
        assert_eq!(sade(Path::new(r"\\?\C:\a\b")), PathBuf::from(r"C:\a\b"));
        assert_eq!(
            sade(Path::new(r"\\?\UNC\sunucu\pay")),
            PathBuf::from(r"\\sunucu\pay")
        );
        assert_eq!(sade(Path::new(r"C:\a")), PathBuf::from(r"C:\a"));
        assert_eq!(
            sade(Path::new(r"\\?\Volume{x}\a")),
            PathBuf::from(r"\\?\Volume{x}\a")
        );
    }

    /// `canonicalize`'ın gerçek çıktısı: önek kalkar, yol aynı dizini gösterir.
    #[test]
    fn sade_canonicalize_ciktisini_sadelestirir() {
        let d = tempfile::tempdir().unwrap();
        let kanonik = d.path().canonicalize().unwrap();
        assert!(kanonik.as_os_str().to_string_lossy().starts_with(r"\\?\"));
        let s = sade(&kanonik);
        assert!(!s.as_os_str().to_string_lossy().starts_with(r"\\"), "{s:?}");
        assert_eq!(s.canonicalize().unwrap(), kanonik);
    }

    #[test]
    fn surucu_koku_ayrilir() {
        assert_eq!(
            surucu_koku(Path::new(r"C:\Users\a")),
            Some(PathBuf::from(r"C:\"))
        );
        assert_eq!(
            surucu_koku(Path::new(r"\\?\D:\x")),
            Some(PathBuf::from(r"D:\"))
        );
        assert_eq!(surucu_koku(Path::new(r"\\sunucu\pay\x")), None);
    }

    #[test]
    fn sistem_surucusu_sabit_diskler_arasinda() {
        let sistem = std::env::var("SystemDrive").unwrap();
        let diskler = sabit_diskler();
        assert!(
            diskler.contains(&PathBuf::from(format!("{sistem}\\"))),
            "{diskler:?}"
        );
    }
}
```

**`crates/kalfa-sandbox/src/windows/komut_satiri.rs`** (tam içerik):

```rust
//! Kabın komut satırı. Kap `CreateProcessW` ile başlar (§9.3); std'nin
//! `Command`'ı güvenlik yeteneklerini veremiyor. Argümanlar std'nin
//! kurallarıyla yazılır: program `CommandLineToArgvW` kurallarıyla, toplu
//! dosya (`.cmd`, `.bat`) ise cmd.exe'nin kurallarıyla. Toplu dosyada
//! `%DEGISKEN%` genişlemesi ve tırnak kaçışı engellenir (std'deki
//! "BatBadBut" düzeltmesinin karşılığı); satır sonu içeren argüman reddedilir.

use std::io;
use std::path::{Path, PathBuf};

/// Başlatılacak dosya ve komut satırı (NUL'suz UTF-16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KomutSatiri {
    pub uygulama: PathBuf,
    pub satir: Vec<u16>,
}

fn gecersiz(metin: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, metin.to_string())
}

fn toplu_dosya_mi(program: &Path) -> bool {
    program
        .extension()
        .and_then(|u| u.to_str())
        .is_some_and(|u| u.eq_ignore_ascii_case("cmd") || u.eq_ignore_ascii_case("bat"))
}

/// `program` mutlak ve çözülmüş bir yoldur. `cmd`: toplu dosyayı çalıştıracak
/// cmd.exe'nin yolu.
pub(crate) fn uret(program: &Path, argumanlar: &[String], cmd: &Path) -> io::Result<KomutSatiri> {
    let ad: Vec<u16> = program.as_os_str().encode_wide_vec();
    if ad.contains(&u16::from(b'"')) || ad.contains(&0) {
        return Err(gecersiz("Program yolu tırnak ya da NUL içeremez."));
    }
    if argumanlar.iter().any(|a| a.contains('\0')) {
        return Err(gecersiz("Argüman NUL içeremez."));
    }
    if toplu_dosya_mi(program) {
        if ad.last() == Some(&u16::from(b'\\')) {
            return Err(gecersiz("Toplu dosya yolu `\\` ile bitemez."));
        }
        let mut satir: Vec<u16> = "cmd.exe /e:ON /v:OFF /d /c \"".encode_utf16().collect();
        satir.push(u16::from(b'"'));
        satir.extend_from_slice(&ad);
        satir.push(u16::from(b'"'));
        for a in argumanlar {
            if a.contains('\r') || a.contains('\n') {
                return Err(gecersiz(
                    "Toplu dosyaya verilen argüman satır sonu içeremez.",
                ));
            }
            satir.push(u16::from(b' '));
            toplu_arguman(&mut satir, a);
        }
        satir.push(u16::from(b'"'));
        return Ok(KomutSatiri {
            uygulama: cmd.to_path_buf(),
            satir,
        });
    }
    let mut satir = vec![u16::from(b'"')];
    satir.extend_from_slice(&ad);
    satir.push(u16::from(b'"'));
    for a in argumanlar {
        satir.push(u16::from(b' '));
        arguman(&mut satir, a);
    }
    Ok(KomutSatiri {
        uygulama: program.to_path_buf(),
        satir,
    })
}

trait GenisVec {
    fn encode_wide_vec(&self) -> Vec<u16>;
}

impl GenisVec for std::ffi::OsStr {
    fn encode_wide_vec(&self) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        self.encode_wide().collect()
    }
}

/// `CommandLineToArgvW` kuralları: boşluk ya da sekme içeren (ya da boş)
/// argüman tırnaklanır; tırnaktan önceki ters bölüler ikilenir.
fn arguman(satir: &mut Vec<u16>, a: &str) {
    let tirnak = a.is_empty() || a.contains(' ') || a.contains('\t');
    if tirnak {
        satir.push(u16::from(b'"'));
    }
    let mut ters = 0usize;
    for c in a.encode_utf16() {
        if c == u16::from(b'\\') {
            ters += 1;
        } else {
            if c == u16::from(b'"') {
                satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters + 1));
            }
            ters = 0;
        }
        satir.push(c);
    }
    if tirnak {
        satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
        satir.push(u16::from(b'"'));
    }
}

/// cmd.exe kuralları: güvenli bilinmeyen her ASCII simgesi argümanı tırnaklatır;
/// tırnak ikilenerek kaçar; `%` ise `%%cd:~,%` olur: `cd` değişkeninin boş alt
/// dizesi hiçbir şeye genişler ve cmd'nin `%DEGISKEN%` genişletmesini böler.
fn toplu_arguman(satir: &mut Vec<u16>, a: &str) {
    const GUVENLI: &str = r"#$*+-./:?@\_";
    let mut tirnak = a.is_empty() || a.ends_with('\\');
    for c in a.chars() {
        let ascii_simge = c.is_ascii() && !(c.is_ascii_alphanumeric() || GUVENLI.contains(c));
        if ascii_simge || c.is_control() {
            tirnak = true;
        }
    }
    if tirnak {
        satir.push(u16::from(b'"'));
    }
    let mut ters = 0usize;
    for c in a.encode_utf16() {
        if c == u16::from(b'\\') {
            ters += 1;
        } else {
            if c == u16::from(b'"') {
                satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
                satir.push(u16::from(b'"'));
            } else if c == u16::from(b'%') {
                satir.extend("%%cd:~,".encode_utf16());
            }
            ters = 0;
        }
        satir.push(c);
    }
    if tirnak {
        satir.extend(std::iter::repeat_n(u16::from(b'\\'), ters));
        satir.push(u16::from(b'"'));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metin(k: &KomutSatiri) -> String {
        String::from_utf16(&k.satir).unwrap()
    }

    fn argv(argumanlar: &[&str]) -> Vec<String> {
        argumanlar.iter().map(|a| a.to_string()).collect()
    }

    const CMD: &str = r"C:\Windows\System32\cmd.exe";

    #[test]
    fn program_tirnaklanir_argumanlar_msvc_kurallariyla() {
        let k = uret(
            Path::new(r"C:\Araclar\git.exe"),
            &argv(&[
                "status",
                "iki kelime",
                "",
                r#"a"b"#,
                r"son\",
                r#"x\"y"#,
                r"c:\d\",
            ]),
            Path::new(CMD),
        )
        .unwrap();
        assert_eq!(k.uygulama, PathBuf::from(r"C:\Araclar\git.exe"));
        assert_eq!(
            metin(&k),
            r#""C:\Araclar\git.exe" status "iki kelime" "" a\"b son\ x\\\"y c:\d\"#
        );
    }

    #[test]
    fn toplu_dosya_cmd_ile_ve_yuzde_kacisiyla() {
        let k = uret(
            Path::new(r"C:\Program Files\nodejs\npm.cmd"),
            &argv(&["run", "test", "%PATH%", "a&b", r#"x"y"#, "", r"d\"]),
            Path::new(CMD),
        )
        .unwrap();
        assert_eq!(k.uygulama, PathBuf::from(CMD));
        assert_eq!(
            metin(&k),
            r#"cmd.exe /e:ON /v:OFF /d /c ""C:\Program Files\nodejs\npm.cmd" run test "%%cd:~,%PATH%%cd:~,%" "a&b" "x""y" "" "d\\"""#
        );
    }

    #[test]
    fn toplu_dosyada_satir_sonu_reddedilir() {
        let hata = uret(Path::new(r"C:\a\b.bat"), &argv(&["a\nb"]), Path::new(CMD)).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn nul_ve_tirnakli_program_reddedilir() {
        assert!(uret(Path::new(r"C:\a.exe"), &argv(&["a\0b"]), Path::new(CMD)).is_err());
        assert!(uret(Path::new(r#"C:\a".exe"#), &[], Path::new(CMD)).is_err());
    }

    #[test]
    fn uzanti_buyuk_harfli_toplu_dosya_da_taninir() {
        let k = uret(Path::new(r"C:\a\B.CMD"), &[], Path::new(CMD)).unwrap();
        assert_eq!(k.uygulama, PathBuf::from(CMD));
    }
}
```

**`crates/kalfa-sandbox/src/windows/kap.rs`** (tam içerik):

```rust
//! Komutun AppContainer'da başlatılması (aşama 2 tasarımı §9.3).
//!
//! - `CreateProcessW`'ye `STARTUPINFOEXW` verilir: güvenlik yetenekleri paket
//!   SID'ini ve yetenekleri, tutamaç listesi yalnızca üç standart tutamacı
//!   taşır.
//! - Süreç askıda başlar, Job Object'e atanır, sonra sürdürülür.
//! - Aracı süreç yoktur; standart girdi `NUL`'dur.

use std::ffi::c_void;
use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{
    GENERIC_READ, GetLastError, HANDLE, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE,
    SetHandleInformation, WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::{
    SECURITY_ATTRIBUTES, SECURITY_CAPABILITIES, SID_AND_ATTRIBUTES,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::Diagnostics::Debug::{
    SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX, SetErrorMode,
};
use windows_sys::Win32::System::Pipes::CreatePipe;
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;
use windows_sys::Win32::System::SystemServices::SE_GROUP_ENABLED;
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, CreateProcessW,
    DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT, GetExitCodeProcess, INFINITE,
    InitializeProcThreadAttributeList, LPPROC_THREAD_ATTRIBUTE_LIST,
    PROC_THREAD_ATTRIBUTE_HANDLE_LIST, PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES,
    PROCESS_INFORMATION, ResumeThread, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

use super::guvenlik::{Sid, genis};
use super::komut_satiri::KomutSatiri;
use crate::agac::Is;

pub(crate) struct KapIstegi<'a> {
    pub komut: &'a KomutSatiri,
    pub dizin: &'a Path,
    pub ortam: &'a [(String, String)],
    pub paket: &'a Sid,
    pub yetenekler: &'a [Sid],
}

/// Başlamış kap: süreç tutamacı ve iki çıktı borusunun okuma uçları.
pub(crate) struct Kap {
    pub surec: OwnedHandle,
    pub cikti: File,
    pub hata: File,
}

/// Yükleyici hata kutularını kapatır. Kap bu kipi devralır; yoksa eksik bir
/// DLL, UI kısıtlarına rağmen geliştiricinin masaüstünde iletişim kutusu
/// açar ve komut kutu kapanana kadar bekler (2c, P10).
pub(crate) fn hata_kutularini_kapat() {
    // SAFETY: yalnızca sürecin hata kipini değiştirir.
    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX);
    }
}

/// `System32`'nin yolu; toplu dosyaları çalıştıran cmd.exe oradadır.
pub(crate) fn sistem_dizini() -> PathBuf {
    let mut tampon = vec![0u16; 260];
    // SAFETY: tampon boyutuyla birlikte verilir.
    let n = unsafe { GetSystemDirectoryW(tampon.as_mut_ptr(), tampon.len() as u32) } as usize;
    if n == 0 || n >= tampon.len() {
        return PathBuf::from(r"C:\Windows\System32");
    }
    PathBuf::from(String::from_utf16_lossy(&tampon[..n]))
}

fn son_hata(adim: &str) -> io::Error {
    let e = io::Error::last_os_error();
    io::Error::new(e.kind(), format!("{adim}: {e}"))
}

/// Tutamaç; düşünce kapanır.
fn sahiplen(h: HANDLE) -> OwnedHandle {
    // SAFETY: tutamaç yeni açıldı ve yalnızca burada sahiplenilir.
    unsafe { OwnedHandle::from_raw_handle(h as _) }
}

/// Anonim boru: okuma ucu Kalfa'da kalır (miras alınmaz), yazma ucu çocuğa
/// geçer. Çocuğa yalnızca tutamaç listesindekiler geçer; yine de yazma ucu,
/// `CreateProcessW` ile kapatılışı arasında başka bir iş parçacığının
/// tutamaç listesiz başlattığı sürece geçebilir. Öyle olursa boru o süreç
/// bitene kadar açık kalır; `Surec::bekle` komut bittikten sonra en çok iki
/// saniye bekler.
fn boru() -> io::Result<(OwnedHandle, OwnedHandle)> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    let mut okuma: HANDLE = null_mut();
    let mut yazma: HANDLE = null_mut();
    // SAFETY: çıktı tutamaçları hemen sahiplenilir.
    unsafe {
        if CreatePipe(&mut okuma, &mut yazma, &sa, 0) == 0 {
            return Err(son_hata("CreatePipe"));
        }
    }
    let (okuma, yazma) = (sahiplen(okuma), sahiplen(yazma));
    // SAFETY: tutamaç geçerlidir.
    if unsafe { SetHandleInformation(okuma.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) } == 0
    {
        return Err(son_hata("SetHandleInformation"));
    }
    Ok((okuma, yazma))
}

fn nul() -> io::Result<OwnedHandle> {
    let sa = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: null_mut(),
        bInheritHandle: 1,
    };
    // SAFETY: dize NUL ile biter; tutamaç hemen sahiplenilir.
    let h = unsafe {
        CreateFileW(
            genis("NUL").as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &sa,
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(son_hata("CreateFileW(NUL)"));
    }
    Ok(sahiplen(h))
}

/// Ortam bloğu: `AD=DEĞER\0 … \0`, adlar büyük/küçük harf ayırmadan sıralı.
fn ortam_blogu(ortam: &[(String, String)]) -> Vec<u16> {
    let mut sirali: Vec<&(String, String)> = ortam.iter().collect();
    sirali.sort_by_key(|(ad, _)| ad.to_uppercase());
    let mut blok = Vec::new();
    for (ad, deger) in sirali {
        blok.extend(format!("{ad}={deger}").encode_utf16());
        blok.push(0);
    }
    if blok.is_empty() {
        blok.push(0);
    }
    blok.push(0);
    blok
}

/// Süreç öznitelik listesi; düşünce silinir.
struct Oznitelikler(Vec<u64>);

impl Oznitelikler {
    fn yeni(sayi: u32) -> io::Result<Self> {
        let mut boyut = 0usize;
        // SAFETY: önce boyut sorulur, sonra 8 bayt hizalı, yeterli tampon verilir.
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), sayi, 0, &mut boyut);
            let mut v = vec![0u64; boyut.div_ceil(8)];
            if InitializeProcThreadAttributeList(v.as_mut_ptr() as _, sayi, 0, &mut boyut) == 0 {
                return Err(son_hata("InitializeProcThreadAttributeList"));
            }
            Ok(Self(v))
        }
    }

    fn ptr(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.0.as_mut_ptr() as _
    }

    /// # Safety
    /// `deger`, liste kullanıldığı sürece yaşamalıdır.
    unsafe fn ekle(
        &mut self,
        oznitelik: usize,
        deger: *const c_void,
        boyut: usize,
    ) -> io::Result<()> {
        // SAFETY: çağıran değerin ömrünü garanti eder.
        let tamam = unsafe {
            UpdateProcThreadAttribute(self.ptr(), 0, oznitelik, deger, boyut, null_mut(), null())
        };
        if tamam == 0 {
            return Err(son_hata("UpdateProcThreadAttribute"));
        }
        Ok(())
    }
}

impl Drop for Oznitelikler {
    fn drop(&mut self) {
        // SAFETY: liste InitializeProcThreadAttributeList ile başlatıldı.
        unsafe { DeleteProcThreadAttributeList(self.ptr()) };
    }
}

/// Komutu kapta başlatır ve işe atar. `is` düşünce ağaç ölür.
pub(crate) fn baslat(istek: &KapIstegi, is: &Is) -> io::Result<Kap> {
    let (cikti_o, cikti_y) = boru()?;
    let (hata_o, hata_y) = boru()?;
    let girdi = nul()?;

    let mut yetenekler: Vec<SID_AND_ATTRIBUTES> = istek
        .yetenekler
        .iter()
        .map(|s| SID_AND_ATTRIBUTES {
            Sid: s.ptr(),
            Attributes: SE_GROUP_ENABLED as u32,
        })
        .collect();
    let guvenlik = SECURITY_CAPABILITIES {
        AppContainerSid: istek.paket.ptr(),
        Capabilities: if yetenekler.is_empty() {
            null_mut()
        } else {
            yetenekler.as_mut_ptr()
        },
        CapabilityCount: yetenekler.len() as u32,
        Reserved: 0,
    };
    let tutamaclar: [HANDLE; 3] = [
        girdi.as_raw_handle() as HANDLE,
        cikti_y.as_raw_handle() as HANDLE,
        hata_y.as_raw_handle() as HANDLE,
    ];
    let mut liste = Oznitelikler::yeni(2)?;
    // SAFETY: değerler bu işlevin sonuna kadar yaşar; liste de öyle.
    unsafe {
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            (&raw const guvenlik).cast(),
            size_of::<SECURITY_CAPABILITIES>(),
        )?;
        liste.ekle(
            PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
            tutamaclar.as_ptr().cast(),
            size_of_val(&tutamaclar),
        )?;
    }

    // SAFETY: sıfırla başlatılan yapı; alanlar aşağıda doldurulur.
    let mut si: STARTUPINFOEXW = unsafe { std::mem::zeroed() };
    si.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    si.StartupInfo.hStdInput = tutamaclar[0];
    si.StartupInfo.hStdOutput = tutamaclar[1];
    si.StartupInfo.hStdError = tutamaclar[2];
    si.lpAttributeList = liste.ptr();
    // SAFETY: sıfırla başlatılan çıktı yapısı.
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    let uygulama = genis(&istek.komut.uygulama);
    let mut satir = istek.komut.satir.clone();
    satir.push(0);
    let ortam = ortam_blogu(istek.ortam);
    let dizin = genis(istek.dizin);
    // SAFETY: bütün dizeler NUL ile biter ve çağrı boyunca yaşar; öznitelik
    // listesinin gösterdiği değerler de öyle.
    let tamam = unsafe {
        CreateProcessW(
            uygulama.as_ptr(),
            satir.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW,
            ortam.as_ptr().cast(),
            dizin.as_ptr(),
            &si.StartupInfo,
            &mut pi,
        )
    };
    if tamam == 0 {
        // SAFETY: yalnızca son hatayı okur.
        let kod = unsafe { GetLastError() };
        let e = io::Error::from_raw_os_error(kod as i32);
        return Err(io::Error::new(e.kind(), format!("kap başlatılamadı: {e}")));
    }
    let surec = sahiplen(pi.hProcess);
    let is_parcacigi = sahiplen(pi.hThread);
    if let Err(e) = is.ata(surec.as_raw_handle()) {
        // SAFETY: süreç askıda başladı ve bu işleve aittir.
        unsafe { TerminateProcess(surec.as_raw_handle() as HANDLE, 1) };
        return Err(e);
    }
    // SAFETY: iş parçacığı askıda başladı ve bu işleve aittir.
    unsafe { ResumeThread(is_parcacigi.as_raw_handle() as HANDLE) };
    drop((girdi, cikti_y, hata_y, liste, is_parcacigi));
    Ok(Kap {
        surec,
        cikti: File::from(cikti_o),
        hata: File::from(hata_o),
    })
}

/// Süreç bitene kadar bekler ve çıkış kodunu döner.
pub(crate) fn bekle(surec: &OwnedHandle) -> Option<i32> {
    let h = surec.as_raw_handle() as HANDLE;
    // SAFETY: tutamaç geçerlidir; çıktı yerel bir değişkendir.
    unsafe {
        if WaitForSingleObject(h, INFINITE) != WAIT_OBJECT_0 {
            return None;
        }
        let mut kod = 0u32;
        if GetExitCodeProcess(h, &mut kod) == 0 {
            return None;
        }
        Some(kod as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ortam_blogu_sirali_ve_cift_nul_ile_biter() {
        let blok = ortam_blogu(&[
            ("b".into(), "2".into()),
            ("A".into(), "1".into()),
            ("PATH".into(), r"C:\x".into()),
        ]);
        let metin = String::from_utf16(&blok).unwrap();
        assert_eq!(metin, "A=1\0b=2\0PATH=C:\\x\0\0");
        assert_eq!(ortam_blogu(&[]), vec![0, 0]);
    }
}
```

**`crates/kalfa-sandbox/src/windows/kurulum.rs`** (tam içerik):

```rust
//! Yönetici kurulumunun kaydı (aşama 2 tasarımı §9.1, §9.2):
//! `%ProgramData%\Kalfa\yalitim\kurulum.json`. `kalfa-yalitim` yazar;
//! uygulama ve kaldırma okur. Dizini yalnızca Administrators ve SYSTEM
//! yazabilir; oturum açmış kullanıcılar okuyabilir.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::guvenlik::{
    self, Izin, MASKE_OKU, MASKE_TAM, MIRAS, SISTEM, Sid, YONETICILER, mirasi_kes,
};

/// Oturum açmış kullanıcılar (Authenticated Users).
const OTURUM_ACMIS: &str = "S-1-5-11";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KurulumKaydi {
    /// Araç zinciri yeteneğine listeleme izni verilmiş sürücü kökleri.
    #[serde(default)]
    pub kok_izinleri: Vec<PathBuf>,
    /// `izin-ver` ile araç zinciri yeteneğine okuma izni verilmiş dizinler.
    #[serde(default)]
    pub izinli_dizinler: Vec<PathBuf>,
}

/// Kaydın dizini: `%ProgramData%\Kalfa\yalitim`.
pub fn kayit_dizini() -> io::Result<PathBuf> {
    Ok(guvenlik::program_data()?.join("Kalfa").join("yalitim"))
}

impl KurulumKaydi {
    /// Kaydı okur; yoksa `None`. Okunamayan ya da çözülemeyen kayıt hatadır:
    /// sessizce yok sayılsaydı `kaldir` kaydedilen izinleri yerinde bırakır,
    /// `kur` da `izin-ver` kayıtlarını ezerdi.
    pub fn oku(dizin: &Path) -> io::Result<Option<Self>> {
        let yol = dizin.join("kurulum.json");
        let metin = match std::fs::read(&yol) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        serde_json::from_slice(&metin).map(Some).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("kurulum kaydı bozuk ({}): {e}", yol.display()),
            )
        })
    }

    /// Kaydı yazar; dizinin izinleri her yazışta yeniden konur. Yönetici
    /// gerekir.
    pub fn yaz(&self, dizin: &Path) -> io::Result<()> {
        std::fs::create_dir_all(dizin)?;
        let izin = |sid: &str, maske| -> io::Result<Izin> {
            Ok(Izin {
                sid: Sid::metinden(sid)?,
                maske,
                miras: MIRAS,
            })
        };
        mirasi_kes(
            dizin,
            &[
                izin(YONETICILER, MASKE_TAM)?,
                izin(SISTEM, MASKE_TAM)?,
                izin(OTURUM_ACMIS, MASKE_OKU)?,
            ],
        )?;
        let metin = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let gecici = dizin.join("kurulum.json.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, dizin.join("kurulum.json"))
    }

    /// Kaydı siler; dizin boşaldıysa onu da.
    pub fn sil(dizin: &Path) -> io::Result<()> {
        match std::fs::remove_file(dizin.join("kurulum.json")) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let _ = std::fs::remove_dir(dizin);
        if let Some(ust) = dizin.parent() {
            let _ = std::fs::remove_dir(ust);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kayit_okunur_ve_bozuk_kayit_hata_verir() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(KurulumKaydi::oku(d.path()).unwrap(), None);
        std::fs::write(
            d.path().join("kurulum.json"),
            r#"{"kok_izinleri":["C:\\"],"izinli_dizinler":["C:\\Python314"]}"#,
        )
        .unwrap();
        let k = KurulumKaydi::oku(d.path()).unwrap().unwrap();
        assert_eq!(k.kok_izinleri, vec![PathBuf::from(r"C:\")]);
        assert_eq!(k.izinli_dizinler, vec![PathBuf::from(r"C:\Python314")]);
        std::fs::write(d.path().join("kurulum.json"), "{bozuk").unwrap();
        let hata = KurulumKaydi::oku(d.path()).unwrap_err();
        assert_eq!(hata.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn kayit_dizini_program_data_altinda() {
        let d = kayit_dizini().unwrap();
        assert!(d.ends_with(r"Kalfa\yalitim"), "{}", d.display());
    }
}
```

**`crates/kalfa-sandbox/src/windows/ace_kaydi.rs`** (tam içerik):

```rust
//! ACE kaydı (aşama 2 tasarımı §9.4): uygulamanın eklediği her izin,
//! mirasını kestiği her nesnenin önceki DACL'i, oluşturduğu AppContainer
//! profilleri. Geri alma bu kayda göre yapılır; yeniden tarama yapılmaz.
//!
//! Konum `<yalıtım dizini>\ace-kaydi.jsonl`; paket SID'lerine izin verilmez,
//! yalıtımda görünmez. Her satır bir girdidir ve işlemden **önce** yazılır:
//! sonunda satır sonu olmayan yarım son satır yapılmamış bir işlemdir ve
//! okurken atlanır. Başka her çözülemeyen satır kaydı bozuk yapar; o zaman
//! geri alma kayda güvenemez ve taramayla yapılır (§9.4).

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bütün çalışma alanlarının ortak girdileri (araç zinciri dizinleri).
pub const ORTAK: &str = "*";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case")]
pub enum Girdi {
    /// Çalışma alanının AppContainer profili.
    Profil { alan: String, ad: String },
    /// `sid`'e `yol` üzerinde verilen açık izin: maske ve devralma
    /// bayrakları (`guvenlik::MIRAS` gibi).
    Izin {
        alan: String,
        yol: PathBuf,
        sid: String,
        maske: u32,
        miras: u32,
    },
    /// `yol`'un mirası kesildi; `onceki` o anki DACL'dir (SDDL).
    MirasKesildi {
        alan: String,
        yol: PathBuf,
        onceki: String,
    },
    /// Çalışma alanının hazırlığı bitti; kök `kok`.
    Hazir { alan: String, kok: PathBuf },
}

impl Girdi {
    pub fn alan(&self) -> &str {
        match self {
            Girdi::Profil { alan, .. }
            | Girdi::Izin { alan, .. }
            | Girdi::MirasKesildi { alan, .. }
            | Girdi::Hazir { alan, .. } => alan,
        }
    }
}

#[derive(Debug)]
pub struct AceKaydi {
    yol: PathBuf,
    girdiler: Vec<Girdi>,
    bozuk: bool,
}

impl AceKaydi {
    /// Kaydı okur; yoksa boştur.
    ///
    /// - Son satır sonundan sonraki parça hiç okunmaz: dosya satır sonuyla
    ///   bitiyorsa boştur; bitmiyorsa yazılırken yarım kalmış bir girdidir ve
    ///   geçerli JSON olsa bile işlemi başlamamıştır.
    /// - Tamamlanmış her satır (boş satır dahil) bir girdi olarak
    ///   çözülmelidir; çözülemeyen satır ya da okunamayan dosya kaydı bozuk
    ///   yapar. Çözülebilen girdiler yine okunur.
    pub fn oku(yol: &Path) -> Self {
        let (metin, mut bozuk) = match std::fs::read_to_string(yol) {
            Ok(m) => (m, false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (String::new(), false),
            Err(_) => (String::new(), true),
        };
        let mut satirlar: Vec<&str> = metin.split('\n').collect();
        satirlar.pop();
        let mut girdiler = Vec::new();
        for satir in satirlar {
            match serde_json::from_str(satir) {
                Ok(g) => girdiler.push(g),
                Err(_) => bozuk = true,
            }
        }
        Self {
            yol: yol.to_path_buf(),
            girdiler,
            bozuk,
        }
    }

    pub fn girdiler(&self) -> &[Girdi] {
        &self.girdiler
    }

    /// Kayıt bozuk mu? Bozuksa geri alma kayda güvenemez (§9.4).
    pub fn bozuk_mu(&self) -> bool {
        self.bozuk
    }

    /// Girdiyi kaydın sonuna yazar ve diske indirir; aynısı varsa yazmaz.
    pub fn ekle(&mut self, girdi: Girdi) -> io::Result<()> {
        if self.girdiler.contains(&girdi) {
            return Ok(());
        }
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let mut satir = serde_json::to_string(&girdi).map_err(io::Error::other)?;
        satir.push('\n');
        // Yarım son satır yapılmamış bir işlemin kaydıdır: kesilip atılır;
        // kalsaydı yeni satırla ortaya düşer ve kayıt bozuk görünürdü.
        let tam_uzunluk = tam_satirlarin_uzunlugu(&self.yol)?;
        if std::fs::metadata(&self.yol).is_ok_and(|m| m.len() != tam_uzunluk) {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&self.yol)?
                .set_len(tam_uzunluk)?;
        }
        let mut dosya = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.yol)?;
        dosya.write_all(satir.as_bytes())?;
        dosya.sync_all()?;
        self.girdiler.push(girdi);
        Ok(())
    }

    pub fn hazir_mi(&self, alan: &str) -> bool {
        self.girdiler
            .iter()
            .any(|g| matches!(g, Girdi::Hazir { alan: a, .. } if a == alan))
    }

    /// Hazırlanmış çalışma alanları: (kimlik, kök).
    pub fn hazir_alanlar(&self) -> Vec<(String, PathBuf)> {
        self.girdiler
            .iter()
            .filter_map(|g| match g {
                Girdi::Hazir { alan, kok } => Some((alan.clone(), kok.clone())),
                _ => None,
            })
            .collect()
    }

    /// Nesnenin ilk kaydedilen önceki DACL'i. Aynı nesnenin mirasını birden
    /// çok çalışma alanı kestiyse (örneğin iki worktree'nin ortak git dizini)
    /// geri yazılacak olan, Kalfa'dan önceki hâlidir.
    pub fn ilk_dacl(&self, yol: &Path) -> Option<&str> {
        self.girdiler.iter().find_map(|g| match g {
            Girdi::MirasKesildi { yol: y, onceki, .. } if y == yol => Some(onceki.as_str()),
            _ => None,
        })
    }

    /// `yol`'da izni kayıtlı olan başka çalışma alanlarının SID'leri.
    pub fn baska_alanlarin_sidleri(&self, alan: &str, yol: &Path) -> Vec<String> {
        self.girdiler
            .iter()
            .filter_map(|g| match g {
                Girdi::Izin {
                    alan: a,
                    yol: y,
                    sid,
                    ..
                } if a != alan && a != ORTAK && y == yol => Some(sid.clone()),
                _ => None,
            })
            .collect()
    }

    /// Çalışma alanının girdilerini kayıttan çıkarır; `kalan` girdiler
    /// (geri alınamayanlar) kalır. Kayıt yeniden yazılır.
    pub fn alani_cikar(&mut self, alan: &str, kalan: &[Girdi]) -> io::Result<()> {
        let yeni: Vec<Girdi> = self
            .girdiler
            .iter()
            .filter(|g| g.alan() != alan || kalan.contains(g))
            .cloned()
            .collect();
        let mut metin = String::new();
        for g in &yeni {
            metin.push_str(&serde_json::to_string(g).map_err(io::Error::other)?);
            metin.push('\n');
        }
        let gecici = self.yol.with_extension("jsonl.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        self.girdiler = yeni;
        Ok(())
    }
}

/// Dosyanın son satır sonuna kadarki uzunluğu; dosya yoksa 0.
fn tam_satirlarin_uzunlugu(yol: &Path) -> io::Result<u64> {
    match std::fs::read(yol) {
        Ok(v) => Ok(v
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i as u64 + 1)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn izin(alan: &str, yol: &str, sid: &str) -> Girdi {
        Girdi::Izin {
            alan: alan.into(),
            yol: yol.into(),
            sid: sid.into(),
            maske: 0x0013_01BF,
            miras: 0x3,
        }
    }

    #[test]
    fn girdiler_eklenir_okunur_yinelenmez() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let mut k = AceKaydi::oku(&yol);
        k.ekle(izin("a", r"C:\x", "S-1-15-2-1")).unwrap();
        k.ekle(izin("a", r"C:\x", "S-1-15-2-1")).unwrap();
        k.ekle(Girdi::Hazir {
            alan: "a".into(),
            kok: r"C:\x".into(),
        })
        .unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 2);
        assert!(k.hazir_mi("a"));
        assert!(!k.hazir_mi("b"));
        assert_eq!(
            k.hazir_alanlar(),
            vec![("a".into(), PathBuf::from(r"C:\x"))]
        );
    }

    #[test]
    fn yarim_satir_atlanir_ve_sonraki_girdi_ayri_satira_yazilir() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\n{{\"tur\":\"izin\",\"al")).unwrap();
        let mut k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 1);
        assert!(!k.bozuk_mu());
        k.ekle(izin("b", r"C:\y", "S-2")).unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 2);
        assert!(!k.bozuk_mu());
        // Yarım parça kesilip atıldı.
        let metin = std::fs::read_to_string(&yol).unwrap();
        assert!(!metin.contains("\"al\n"), "{metin}");
        assert_eq!(metin.lines().count(), 2);
    }

    /// Satır sonu yazılmamış son parça geçerli JSON olsa da okunmaz; sonraki
    /// `ekle` onu keser ve bellekteki girdiler diskteki kayıtla aynı kalır.
    #[test]
    fn satir_sonu_olmayan_gecerli_son_parca_okunmaz() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let ilk = izin("a", r"C:\x", "S-1");
        let yarim = izin("b", r"C:\y", "S-2");
        let tam = serde_json::to_string(&ilk).unwrap();
        let son = serde_json::to_string(&yarim).unwrap();
        std::fs::write(&yol, format!("{tam}\n{son}")).unwrap();
        let mut k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler(), std::slice::from_ref(&ilk));
        assert!(!k.bozuk_mu());
        // Aynı girdi yeniden yazılır; yinelenme denetimi onu atlamaz.
        k.ekle(yarim.clone()).unwrap();
        let diskteki = AceKaydi::oku(&yol);
        assert_eq!(diskteki.girdiler(), k.girdiler());
        assert_eq!(diskteki.girdiler(), &[ilk, yarim]);
        assert!(!diskteki.bozuk_mu());
        assert!(std::fs::read_to_string(&yol).unwrap().ends_with('\n'));
    }

    #[test]
    fn ortadaki_bos_satir_kaydi_bozuk_yapar() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\n\n{tam}\n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        std::fs::write(&yol, format!("{tam}\n   \n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        std::fs::write(&yol, "").unwrap();
        assert!(!AceKaydi::oku(&yol).bozuk_mu());
    }

    #[test]
    fn ortadaki_ya_da_tamamlanmis_bozuk_satir_kaydi_bozuk_yapar() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\nbozuk\n{tam}\n")).unwrap();
        let k = AceKaydi::oku(&yol);
        assert!(k.bozuk_mu());
        assert_eq!(k.girdiler().len(), 2);
        std::fs::write(&yol, format!("{tam}\nbozuk\n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        assert!(!AceKaydi::oku(&d.path().join("yok.jsonl")).bozuk_mu());
    }

    #[test]
    fn izin_girdisi_maske_ve_devralmayi_tasir() {
        let g = serde_json::to_value(izin("a", r"C:\x", "S-1")).unwrap();
        assert_eq!(g["maske"], 0x0013_01BF);
        assert_eq!(g["miras"], 3);
        assert_eq!(g["tur"], "izin");
    }

    #[test]
    fn ilk_dacl_ve_baska_alanlar() {
        let d = tempfile::tempdir().unwrap();
        let mut k = AceKaydi::oku(&d.path().join("k.jsonl"));
        for (alan, onceki) in [("a", "D:AI(ilk)"), ("b", "D:PAI(sonraki)")] {
            k.ekle(Girdi::MirasKesildi {
                alan: alan.into(),
                yol: r"C:\depo\.git\config".into(),
                onceki: onceki.into(),
            })
            .unwrap();
        }
        k.ekle(izin("b", r"C:\depo\.git\config", "S-B")).unwrap();
        k.ekle(izin(ORTAK, r"C:\depo\.git\config", "S-O")).unwrap();
        assert_eq!(
            k.ilk_dacl(Path::new(r"C:\depo\.git\config")),
            Some("D:AI(ilk)")
        );
        assert_eq!(
            k.baska_alanlarin_sidleri("a", Path::new(r"C:\depo\.git\config")),
            vec!["S-B".to_string()]
        );
        assert!(
            k.baska_alanlarin_sidleri("b", Path::new(r"C:\depo\.git\config"))
                .is_empty()
        );
    }

    #[test]
    fn alan_cikarilir_kalanlar_tutulur() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("k.jsonl");
        let mut k = AceKaydi::oku(&yol);
        let kalan = izin("a", r"C:\kalan", "S-1");
        for g in [
            izin("a", r"C:\x", "S-1"),
            kalan.clone(),
            izin("b", r"C:\y", "S-2"),
        ] {
            k.ekle(g).unwrap();
        }
        k.alani_cikar("a", std::slice::from_ref(&kalan)).unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler(), &[kalan, izin("b", r"C:\y", "S-2")]);
    }
}
```

**`crates/kalfa-sandbox/src/windows/msvc.rs`** (tam içerik):

```rust
//! MSVC ortamı (aşama 2 tasarımı §9.3). rustc, Visual Studio'nun
//! `ProgramData`'daki kurulum kaydını kaptan okuyamadığı için `link.exe`'yi
//! bulamaz. Kalfa, geliştirici tarafında `vswhere` ile son MSVC araç takımını
//! ve Windows SDK'yı bulur; `PATH` eki, `LIB` ve `INCLUDE` kaba verilir. Bu
//! dizinler Program Files (x86) altındadır ve kaba zaten açıktır.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MsvcOrtami {
    /// `PATH`'e eklenecek dizin: `…\bin\Hostx64\x64`.
    pub bin: Option<PathBuf>,
    pub lib: Vec<PathBuf>,
    pub include: Vec<PathBuf>,
}

impl MsvcOrtami {
    /// Makinedeki kurulumdan bulur; bulunamazsa boştur.
    pub fn bul() -> Self {
        let Some(x86) = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from) else {
            return Self::default();
        };
        let vswhere = x86.join(r"Microsoft Visual Studio\Installer\vswhere.exe");
        let kurulum = std::process::Command::new(vswhere)
            .args([
                "-latest",
                "-products",
                "*",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .ok()
            .filter(|c| c.status.success())
            .map(|c| String::from_utf8_lossy(&c.stdout).trim().to_string())
            .unwrap_or_default();
        if kurulum.is_empty() {
            return Self::default();
        }
        Self::coz(Path::new(&kurulum), &x86.join(r"Windows Kits\10"))
    }

    /// `vs`: Visual Studio kurulumu; `sdk`: `Windows Kits\10`.
    pub fn coz(vs: &Path, sdk: &Path) -> Self {
        let Some(msvc) = son_surum(&vs.join(r"VC\Tools\MSVC"), |d| {
            d.join(r"bin\Hostx64\x64\link.exe").is_file()
        }) else {
            return Self::default();
        };
        let mut o = Self {
            bin: Some(msvc.join(r"bin\Hostx64\x64")),
            lib: vec![msvc.join(r"lib\x64")],
            include: vec![msvc.join("include")],
        };
        if let Some(lib) = son_surum(&sdk.join("Lib"), |d| d.join(r"um\x64").is_dir()) {
            o.lib.push(lib.join(r"um\x64"));
            o.lib.push(lib.join(r"ucrt\x64"));
        }
        if let Some(inc) = son_surum(&sdk.join("Include"), |d| d.join("um").is_dir()) {
            for alt in ["ucrt", "um", "shared", "winrt", "cppwinrt"] {
                o.include.push(inc.join(alt));
            }
        }
        o
    }

    /// Kabın ortamına eklenenler: `PATH` eki ile `LIB` ve `INCLUDE`.
    pub fn uygula(&self, ortam: &mut Vec<(String, String)>) {
        let Some(bin) = &self.bin else {
            return;
        };
        let birlestir = |yollar: &[PathBuf]| {
            yollar
                .iter()
                .map(|y| y.display().to_string())
                .collect::<Vec<_>>()
                .join(";")
        };
        match ortam
            .iter_mut()
            .find(|(ad, _)| ad.eq_ignore_ascii_case("PATH"))
        {
            Some((_, deger)) => {
                deger.push(';');
                deger.push_str(&bin.display().to_string());
            }
            None => ortam.push(("PATH".into(), bin.display().to_string())),
        }
        ortam.retain(|(ad, _)| {
            !ad.eq_ignore_ascii_case("LIB") && !ad.eq_ignore_ascii_case("INCLUDE")
        });
        ortam.push(("LIB".into(), birlestir(&self.lib)));
        ortam.push(("INCLUDE".into(), birlestir(&self.include)));
    }
}

/// `dizin`'in koşulu sağlayan, sürüm numarasına göre en büyük alt dizini.
fn son_surum(dizin: &Path, kosul: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    let surum = |d: &Path| -> Vec<u64> {
        d.file_name()
            .and_then(|a| a.to_str())
            .map(|a| a.split('.').map(|p| p.parse().unwrap_or(0)).collect())
            .unwrap_or_default()
    };
    std::fs::read_dir(dizin)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|d| d.is_dir() && kosul(d))
        .max_by_key(|d| surum(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yaz(yol: &Path) {
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, "").unwrap();
    }

    #[test]
    fn son_surum_secilir_ve_ortama_eklenir() {
        let d = tempfile::tempdir().unwrap();
        let vs = d.path().join("vs");
        let sdk = d.path().join("sdk");
        yaz(&vs.join(r"VC\Tools\MSVC\14.9.1\bin\Hostx64\x64\link.exe"));
        yaz(&vs.join(r"VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe"));
        // Bağlayıcısı olmayan sürüm seçilmez.
        std::fs::create_dir_all(vs.join(r"VC\Tools\MSVC\14.50.1\bin")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Lib\10.0.22621.0\um\x64")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Lib\10.0.26100.0\um\x64")).unwrap();
        std::fs::create_dir_all(sdk.join(r"Include\10.0.26100.0\um")).unwrap();
        let o = MsvcOrtami::coz(&vs, &sdk);
        let msvc = vs.join(r"VC\Tools\MSVC\14.44.35207");
        assert_eq!(o.bin, Some(msvc.join(r"bin\Hostx64\x64")));
        assert_eq!(
            o.lib,
            vec![
                msvc.join(r"lib\x64"),
                sdk.join(r"Lib\10.0.26100.0\um\x64"),
                sdk.join(r"Lib\10.0.26100.0\ucrt\x64"),
            ]
        );
        assert_eq!(o.include[0], msvc.join("include"));
        assert_eq!(o.include[2], sdk.join(r"Include\10.0.26100.0\um"));

        let mut ortam = vec![
            ("Path".to_string(), r"C:\a".to_string()),
            ("LIB".to_string(), r"C:\eski".to_string()),
        ];
        o.uygula(&mut ortam);
        assert_eq!(
            ortam[0].1,
            format!(r"C:\a;{}", msvc.join(r"bin\Hostx64\x64").display())
        );
        assert!(ortam.iter().all(|(_, d)| d != r"C:\eski"));
        assert!(ortam.iter().any(|(a, _)| a == "INCLUDE"));
    }

    #[test]
    fn kurulum_yoksa_bos() {
        let d = tempfile::tempdir().unwrap();
        let o = MsvcOrtami::coz(d.path(), d.path());
        assert_eq!(o, MsvcOrtami::default());
        let mut ortam = vec![("PATH".to_string(), "x".to_string())];
        o.uygula(&mut ortam);
        assert_eq!(ortam, vec![("PATH".to_string(), "x".to_string())]);
    }
}
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust

use tokio::process::{Child, Command};

pub(crate) struct Agac {
```

şununla değiştir:

```rust

use tokio::process::{Child, Command};

#[cfg(windows)]
pub(crate) use windows::Is;

pub(crate) struct Agac {
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust
    {
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        let is = windows::Is::yeni()?;
        komut.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let cocuk = komut.spawn()?;
```

şununla değiştir:

```rust
    {
        use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CREATE_SUSPENDED};
        let is = windows::Is::yeni(false)?;
        komut.creation_flags(CREATE_SUSPENDED | CREATE_NO_WINDOW);
        let cocuk = komut.spawn()?;
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust

impl Agac {
    /// Ağaca kibarca durmasını söyler (Linux'ta `SIGTERM`). Windows'ta
    /// kibar sonlandırma yoktur; ağaç hemen ölür.
```

şununla değiştir:

```rust

impl Agac {
    /// Kalfa'nın kendisinin başlattığı ve işe atadığı ağaç (Windows kabı).
    #[cfg(windows)]
    #[allow(dead_code)] // TODO(2d-1 Görev 4): kaldır.
    pub(crate) fn isten(is: Is) -> Self {
        Self { is }
    }

    /// Ağaca kibarca durmasını söyler (Linux'ta `SIGTERM`). Windows'ta
    /// kibar sonlandırma yoktur; ağaç hemen ölür.
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    };
```

şununla değiştir:

```rust
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_UILIMIT_DESKTOP,
        JOB_OBJECT_UILIMIT_DISPLAYSETTINGS, JOB_OBJECT_UILIMIT_EXITWINDOWS,
        JOB_OBJECT_UILIMIT_GLOBALATOMS, JOB_OBJECT_UILIMIT_HANDLES,
        JOB_OBJECT_UILIMIT_READCLIPBOARD, JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS,
        JOB_OBJECT_UILIMIT_WRITECLIPBOARD, JOBOBJECT_BASIC_UI_RESTRICTIONS,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectBasicUIRestrictions,
        JobObjectExtendedLimitInformation, SetInformationJobObject, TerminateJobObject,
    };
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust

    /// Kapanınca içindeki bütün süreçleri öldüren Job Object.
    pub(super) struct Is(HANDLE);

    // Tutamaç yalnızca çekirdek nesnesine bir başvurudur; iş parçacıkları
```

şununla değiştir:

```rust

    /// Kapanınca içindeki bütün süreçleri öldüren Job Object.
    pub(crate) struct Is(HANDLE);

    // Tutamaç yalnızca çekirdek nesnesine bir başvurudur; iş parçacıkları
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust
    // `Is`'in tutamacı değer yaşadıkça açıktır; `Drop`'ta bir kez kapanır.
    impl Is {
        pub(super) fn yeni() -> io::Result<Self> {
            let is = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if is.is_null() {
```

şununla değiştir:

```rust
    // `Is`'in tutamacı değer yaşadıkça açıktır; `Drop`'ta bir kez kapanır.
    impl Is {
        /// `ui_kisiti`: yalıtılmış komutlarda pano, genel atomlar, masaüstü ve
        /// başka süreçlerin kullanıcı arayüzü tutamaçları kapatılır (§9.3).
        pub(crate) fn yeni(ui_kisiti: bool) -> io::Result<Self> {
            let is = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if is.is_null() {
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust
                return Err(io::Error::last_os_error());
            }
            Ok(is)
        }

        pub(super) fn ata(&self, surec: RawHandle) -> io::Result<()> {
            if unsafe { AssignProcessToJobObject(self.0, surec as HANDLE) } == 0 {
                return Err(io::Error::last_os_error());
```

şununla değiştir:

```rust
                return Err(io::Error::last_os_error());
            }
            if ui_kisiti {
                let kisit = JOBOBJECT_BASIC_UI_RESTRICTIONS {
                    UIRestrictionsClass: JOB_OBJECT_UILIMIT_DESKTOP
                        | JOB_OBJECT_UILIMIT_DISPLAYSETTINGS
                        | JOB_OBJECT_UILIMIT_EXITWINDOWS
                        | JOB_OBJECT_UILIMIT_GLOBALATOMS
                        | JOB_OBJECT_UILIMIT_HANDLES
                        | JOB_OBJECT_UILIMIT_READCLIPBOARD
                        | JOB_OBJECT_UILIMIT_SYSTEMPARAMETERS
                        | JOB_OBJECT_UILIMIT_WRITECLIPBOARD,
                };
                let tamam = unsafe {
                    SetInformationJobObject(
                        is.0,
                        JobObjectBasicUIRestrictions,
                        (&raw const kisit).cast(),
                        size_of::<JOBOBJECT_BASIC_UI_RESTRICTIONS>() as u32,
                    )
                };
                if tamam == 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(is)
        }

        pub(crate) fn ata(&self, surec: RawHandle) -> io::Result<()> {
            if unsafe { AssignProcessToJobObject(self.0, surec as HANDLE) } == 0 {
                return Err(io::Error::last_os_error());
```

**`crates/kalfa-sandbox/src/agac.rs`**: şu bloğu

```rust
        }

        pub(super) fn sonlandir(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
```

şununla değiştir:

```rust
        }

        pub(crate) fn sonlandir(&self) {
            unsafe {
                TerminateJobObject(self.0, 1);
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
//! sonlandırılması (aşama 2 tasarımı §6.2, §6.4).

use std::process::Stdio;
use std::time::Duration;
```

şununla değiştir:

```rust
//! sonlandırılması (aşama 2 tasarımı §6.2, §6.4).

use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust

pub struct Surec {
    cocuk: Child,
    agac: Agac,
}
```

şununla değiştir:

```rust

pub struct Surec {
    cocuk: Cocuk,
    agac: Agac,
}

enum Cocuk {
    Tokio(Box<Child>),
    /// Windows kabı: Kalfa'nın `CreateProcessW` ile başlattığı süreç (§9.3).
    #[cfg(windows)]
    #[allow(dead_code)] // TODO(2d-1 Görev 4): kaldır.
    Kap(crate::windows::kap::Kap),
}

type Okuyucu = Box<dyn tokio::io::AsyncRead + Send + Unpin>;
type Bitisi = Pin<Box<dyn Future<Output = Option<i32>> + Send>>;

impl Cocuk {
    /// Çıktı okuyucuları ve sürecin bitişini bekleyen gelecek.
    fn ayir(self) -> (Option<Okuyucu>, Option<Okuyucu>, Bitisi) {
        match self {
            Cocuk::Tokio(mut c) => {
                let cikti = c.stdout.take().map(|o| Box::new(o) as Okuyucu);
                let hata = c.stderr.take().map(|o| Box::new(o) as Okuyucu);
                (
                    cikti,
                    hata,
                    Box::pin(async move { c.wait().await.ok().and_then(|d| d.code()) }),
                )
            }
            #[cfg(windows)]
            Cocuk::Kap(k) => {
                let cikti = Box::new(tokio::fs::File::from_std(k.cikti)) as Okuyucu;
                let hata = Box::new(tokio::fs::File::from_std(k.hata)) as Okuyucu;
                let surec = k.surec;
                (
                    Some(cikti),
                    Some(hata),
                    Box::pin(async move {
                        // Ağaç her durumda sonunda öldürülür; bekleme döner.
                        tokio::task::spawn_blocking(move || crate::windows::kap::bekle(&surec))
                            .await
                            .ok()
                            .flatten()
                    }),
                )
            }
        }
    }
}
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
            .kill_on_drop(true);
        let (cocuk, agac) = agac::baslat(&mut komut)?;
        Ok(Self { cocuk, agac })
    }
```

şununla değiştir:

```rust
            .kill_on_drop(true);
        let (cocuk, agac) = agac::baslat(&mut komut)?;
        Ok(Self {
            cocuk: Cocuk::Tokio(Box::new(cocuk)),
            agac,
        })
    }

    /// Kapta başlamış ve işe atanmış süreç.
    #[cfg(windows)]
    #[allow(dead_code)] // TODO(2d-1 Görev 4): kaldır.
    pub(crate) fn kaptan(kap: crate::windows::kap::Kap, is: agac::Is) -> Self {
        Self {
            cocuk: Cocuk::Kap(kap),
            agac: Agac::isten(is),
        }
    }
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
    /// sonlandırılmıştır.
    pub async fn bekle(
        mut self,
        sure: Duration,
        iptal: CancellationToken,
```

şununla değiştir:

```rust
    /// sonlandırılmıştır.
    pub async fn bekle(
        self,
        sure: Duration,
        iptal: CancellationToken,
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
        let baslangic = Instant::now();
        let (tx, mut rx) = mpsc::channel::<(Akis, Vec<u8>)>(16);
        let mut okuyucular = Vec::new();
        if let Some(o) = self.cocuk.stdout.take() {
            okuyucular.push(tokio::spawn(oku(o, Akis::Standart, tx.clone())));
        }
        if let Some(o) = self.cocuk.stderr.take() {
            okuyucular.push(tokio::spawn(oku(o, Akis::Hata, tx.clone())));
        }
```

şununla değiştir:

```rust
        let baslangic = Instant::now();
        let (tx, mut rx) = mpsc::channel::<(Akis, Vec<u8>)>(16);
        let (cikti_okuyucu, hata_okuyucu, mut bitisi) = self.cocuk.ayir();
        let agac = self.agac;
        let mut okuyucular = Vec::new();
        if let Some(o) = cikti_okuyucu {
            okuyucular.push(tokio::spawn(oku(o, Akis::Standart, tx.clone())));
        }
        if let Some(o) = hata_okuyucu {
            okuyucular.push(tokio::spawn(oku(o, Akis::Hata, tx.clone())));
        }
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
                    None => boru_kapali = true,
                },
                durum = self.cocuk.wait(), if !cikis_alindi => {
                    cikis_alindi = true;
                    if bitis.is_none() {
                        bitis = Some(Bitis::Cikti(durum.ok().and_then(|d| d.code())));
                    }
                    // Komut bitti; geride kalan torunlar da durur.
                    self.agac.durdur();
                    oldurme.get_or_insert(Instant::now() + KIBAR_SURE);
                    bosaltma = Some(Instant::now() + BOSALTMA_SURESI);
```

şununla değiştir:

```rust
                    None => boru_kapali = true,
                },
                kod = &mut bitisi, if !cikis_alindi => {
                    cikis_alindi = true;
                    if bitis.is_none() {
                        bitis = Some(Bitis::Cikti(kod));
                    }
                    // Komut bitti; geride kalan torunlar da durur.
                    agac.durdur();
                    oldurme.get_or_insert(Instant::now() + KIBAR_SURE);
                    bosaltma = Some(Instant::now() + BOSALTMA_SURESI);
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
                _ = sleep_until(son), if bitis.is_none() => {
                    bitis = Some(Bitis::ZamanAsimi);
                    self.agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = iptal.cancelled(), if bitis.is_none() => {
                    bitis = Some(Bitis::Iptal);
                    self.agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = sleep_until(oldurme.unwrap_or(uzak)), if oldurme.is_some() => {
                    self.agac.oldur();
                    oldurme = None;
                }
```

şununla değiştir:

```rust
                _ = sleep_until(son), if bitis.is_none() => {
                    bitis = Some(Bitis::ZamanAsimi);
                    agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = iptal.cancelled(), if bitis.is_none() => {
                    bitis = Some(Bitis::Iptal);
                    agac.durdur();
                    oldurme = Some(Instant::now() + KIBAR_SURE);
                }
                _ = sleep_until(oldurme.unwrap_or(uzak)), if oldurme.is_some() => {
                    agac.oldur();
                    oldurme = None;
                }
```

**`crates/kalfa-sandbox/src/surec.rs`**: şu bloğu

```rust
            }
        }
        self.agac.oldur();
        for okuyucu in okuyucular {
            okuyucu.abort();
```

şununla değiştir:

```rust
            }
        }
        agac.oldur();
        for okuyucu in okuyucular {
            okuyucu.abort();
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
mod surec;
pub mod tarama;
mod yalitimsiz;
```

şununla değiştir:

```rust
mod surec;
pub mod tarama;
#[cfg(windows)]
pub mod windows;
mod yalitimsiz;
```

- [ ] **Step 2: Testleri koş**

Run: `cargo test -p kalfa-sandbox --locked --lib windows`
Expected: `guvenlik`, `komut_satiri`, `kap`, `kurulum`, `ace_kaydi` ve `msvc` testleri geçer (Windows'ta). Sonra Global Constraints'teki denetimler.

- [ ] **Step 3: Commit**

```bash
git add crates/kalfa-sandbox
git commit -m "sandbox: Windows temeli — SID, AppContainer profili, DACL, kök izni, kap başlatma, ACE kaydı, MSVC ortamı"
```

---

### Task 2: `kalfa-yalitim`: yönetici yardımcısı

**Files:** `crates/kalfa-yalitim/` (yeni), kök `Cargo.toml` ve `Cargo.lock` (değişir).

**Interfaces:** Consumes Görev 1'in `windows::guvenlik` ve `windows::kurulum`'u. Alt komutlar yönetici değilse çalışmaz; `kur` kaydı izinden önce yazar; `kaldir` her adımı bağımsız dener, kök izinlerini kayıttakilere ek olarak sabit disklerde de arar (kayıt eksik ya da bozuksa da çalışır) ve kaldıramadıklarını kayıtta tutar. Bozuk kayıtla `kur`, `izin-ver` ve `izin-geri-al` durur.

- [ ] **Step 1: Dosyaları yaz**

**`crates/kalfa-yalitim/Cargo.toml`** (tam içerik):

```toml
[package]
name = "kalfa-yalitim"
description = "Kalfa'nın Windows yalıtımı için yönetici yardımcısı: kök izni, araç zinciri izinleri, kaldırma"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
authors.workspace = true

[target.'cfg(windows)'.dependencies]
kalfa-sandbox = { path = "../kalfa-sandbox" }
```

**`crates/kalfa-yalitim/src/main.rs`** (tam içerik):

```rust
//! `kalfa-yalitim`: Kalfa'nın Windows yalıtımı için yönetici yardımcısı
//! (aşama 2 tasarımı §9.1). Bütün alt komutları yönetici olarak çalışır:
//!
//! - `kur`: sabit disklerin köküne araç zinciri yeteneği için listeleme izni
//!   verir (§9.2) ve kurulum kaydını yazar.
//! - `kaldir`: kurulum kaydına göre her şeyi geri alır.
//! - `izin-ver <dizin>` / `izin-geri-al <dizin>`: geliştiricinin sahibi
//!   olmadığı bir araç zinciri dizinine yetenek için okuma izni verir ya da
//!   geri alır (§9.4).
//!
//! Komutu çalıştırmak için gerekmez: Kalfa kabı kendisi başlatır (§9.3).

use std::process::ExitCode;

#[cfg(windows)]
const KULLANIM: &str =
    "Kullanım: kalfa-yalitim kur | kaldir | izin-ver <dizin> | izin-geri-al <dizin>";

fn main() -> ExitCode {
    let argumanlar: Vec<String> = std::env::args().skip(1).collect();
    match calistir(&argumanlar) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(not(windows))]
fn calistir(_argumanlar: &[String]) -> Result<(), String> {
    Err("kalfa-yalitim yalnızca Windows'ta çalışır.".into())
}

#[cfg(windows)]
fn calistir(argumanlar: &[String]) -> Result<(), String> {
    let Some((komut, kalan)) = argumanlar.split_first() else {
        return Err(KULLANIM.into());
    };
    let dizin = || -> Result<std::path::PathBuf, String> {
        match kalan {
            [d] => Ok(std::path::PathBuf::from(d)),
            _ => Err(KULLANIM.into()),
        }
    };
    let is = match komut.as_str() {
        "kur" if kalan.is_empty() => windows::Is::Kur,
        "kaldir" if kalan.is_empty() => windows::Is::Kaldir,
        "izin-ver" => windows::Is::IzinVer(dizin()?),
        "izin-geri-al" => windows::Is::IzinGeriAl(dizin()?),
        _ => return Err(KULLANIM.into()),
    };
    windows::yap(is)
}

#[cfg(windows)]
mod windows {
    use std::path::{Path, PathBuf};

    use kalfa_sandbox::windows::guvenlik::{
        Izin, MASKE_OKU, MIRAS, arac_zinciri_sidi, izin_ekle, izni_kaldir, kok_izni_kaldir,
        kok_izni_var_mi, kok_izni_ver, sabit_diskler, sade, yukseltilmis_mi,
    };
    use kalfa_sandbox::windows::kurulum::{KurulumKaydi, kayit_dizini};

    pub enum Is {
        Kur,
        Kaldir,
        IzinVer(PathBuf),
        IzinGeriAl(PathBuf),
    }

    fn metin(e: impl std::fmt::Display) -> String {
        e.to_string()
    }

    pub fn yap(is: Is) -> Result<(), String> {
        if !yukseltilmis_mi().map_err(metin)? {
            return Err("Bu komut yönetici olarak çalıştırılmalıdır.".into());
        }
        let dizin = kayit_dizini().map_err(metin)?;
        // Bozuk kayıtla yalnızca `kaldir` sürer: kök izinlerini sürücülerden
        // bulur. Diğer komutlar kaydı ezmemek için durur.
        let okunan = KurulumKaydi::oku(&dizin);
        if let (Err(e), Is::Kaldir) = (&okunan, &is) {
            println!("{e}; kök izinleri sürücülerden bulunacak.");
        }
        let mut kayit = match (okunan, &is) {
            (Ok(k), _) => k.unwrap_or_default(),
            (Err(_), Is::Kaldir) => KurulumKaydi::default(),
            (Err(e), _) => return Err(format!("{e}. Kaydı inceleyin ya da önce `kaldir`.")),
        };
        match is {
            Is::Kur => kur(&dizin, &mut kayit),
            Is::Kaldir => kaldir(&dizin, &kayit),
            Is::IzinVer(d) => izin_ver(&dizin, &mut kayit, &d),
            Is::IzinGeriAl(d) => izin_geri_al(&dizin, &mut kayit, &d),
        }
    }

    /// Kök izni. Kayıt izinden önce yazılır: izin yarıda kalsa da `kaldir`
    /// onu kaldırmayı dener.
    fn kur(dizin: &Path, kayit: &mut KurulumKaydi) -> Result<(), String> {
        let sid = arac_zinciri_sidi().map_err(metin)?;
        let diskler = sabit_diskler();
        if diskler.is_empty() {
            return Err("ACL taşıyan sabit disk bulunamadı.".into());
        }
        for kok in diskler {
            if !kayit.kok_izinleri.contains(&kok) {
                kayit.kok_izinleri.push(kok.clone());
                kayit.yaz(dizin).map_err(metin)?;
            }
            if kok_izni_var_mi(&kok, &sid).map_err(metin)? {
                println!("{}: izin zaten var.", kok.display());
                continue;
            }
            kok_izni_ver(&kok, &sid).map_err(metin)?;
            println!("{}: listeleme izni verildi.", kok.display());
        }
        println!(
            "Kurulum tamam. Kayıt: {}",
            dizin.join("kurulum.json").display()
        );
        Ok(())
    }

    /// Her adım bağımsızdır: biri düşerse diğerleri yine denenir. Kayıt,
    /// ancak her şey geri alındıysa silinir. Kök izinleri kayıttakilerle
    /// sınırlı değildir: yeteneğin izni hangi sabit diskin kökündeyse kaldırılır
    /// (kayıt eksik ya da bozuksa da).
    fn kaldir(dizin: &Path, kayit: &KurulumKaydi) -> Result<(), String> {
        println!(
            "Kök izni bu makinedeki bütün hesapların Kalfa yalıtımı için ortaktır; kaldırınca \
             başka hesaplardaki yalıtım da çalışmaz."
        );
        let sid = arac_zinciri_sidi().map_err(metin)?;
        let mut kokler = kayit.kok_izinleri.clone();
        for kok in sabit_diskler() {
            if !kokler.contains(&kok) && kok_izni_var_mi(&kok, &sid).unwrap_or(false) {
                kokler.push(kok);
            }
        }
        let mut kalan = KurulumKaydi::default();
        for kok in &kokler {
            match kok_izni_kaldir(kok, &sid) {
                Ok(()) => println!("{}: listeleme izni kaldırıldı.", kok.display()),
                Err(e) => {
                    println!("{}: kaldırılamadı: {e}", kok.display());
                    kalan.kok_izinleri.push(kok.clone());
                }
            }
        }
        for d in &kayit.izinli_dizinler {
            if !d.exists() {
                continue;
            }
            match izni_kaldir(d, &sid) {
                Ok(()) => println!("{}: okuma izni kaldırıldı.", d.display()),
                Err(e) => {
                    println!("{}: kaldırılamadı: {e}", d.display());
                    kalan.izinli_dizinler.push(d.clone());
                }
            }
        }
        if kalan == KurulumKaydi::default() {
            KurulumKaydi::sil(dizin).map_err(metin)?;
            println!("Kaldırma tamam.");
            Ok(())
        } else {
            kalan.yaz(dizin).map_err(metin)?;
            Err("Bazı izinler kaldırılamadı; kayıt yalnızca onları tutuyor.".into())
        }
    }

    fn dizin_coz(d: &Path) -> Result<PathBuf, String> {
        let kanonik = d
            .canonicalize()
            .map_err(|e| format!("{}: {e}", d.display()))?;
        if !kanonik.is_dir() {
            return Err(format!("{} bir dizin değil.", d.display()));
        }
        Ok(sade(&kanonik))
    }

    fn izin_ver(dizin: &Path, kayit: &mut KurulumKaydi, d: &Path) -> Result<(), String> {
        let d = dizin_coz(d)?;
        let sid = arac_zinciri_sidi().map_err(metin)?;
        if !kayit.izinli_dizinler.contains(&d) {
            kayit.izinli_dizinler.push(d.clone());
            kayit.yaz(dizin).map_err(metin)?;
        }
        izin_ekle(
            &d,
            &Izin {
                sid,
                maske: MASKE_OKU,
                miras: MIRAS,
            },
        )
        .map_err(metin)?;
        println!("{}: okuma izni verildi.", d.display());
        Ok(())
    }

    fn izin_geri_al(dizin: &Path, kayit: &mut KurulumKaydi, d: &Path) -> Result<(), String> {
        let d = dizin_coz(d)?;
        let sid = arac_zinciri_sidi().map_err(metin)?;
        izni_kaldir(&d, &sid).map_err(metin)?;
        kayit.izinli_dizinler.retain(|x| x != &d);
        kayit.yaz(dizin).map_err(metin)?;
        println!("{}: okuma izni kaldırıldı.", d.display());
        Ok(())
    }
}
```

**`Cargo.toml`**: şu bloğu

```toml
    "crates/kalfa-uygulama",
    "crates/kalfa",
]
```

şununla değiştir:

```toml
    "crates/kalfa-uygulama",
    "crates/kalfa",
    "crates/kalfa-yalitim",
]
```

**`Cargo.lock`**: şu bloğu

```toml
 "thiserror 2.0.20",
 "tokio",
 "url",
]

[[package]]
```

şununla değiştir:

```toml
 "thiserror 2.0.20",
 "tokio",
 "url",
]

[[package]]
name = "kalfa-yalitim"
version = "0.1.0"
dependencies = [
 "kalfa-sandbox",
]

[[package]]
```

- [ ] **Step 2: Denetimler**

Global Constraints'teki denetimler. `kur`'u çalıştırma: yönetici ister (Görev 4).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock crates/kalfa-yalitim
git commit -m "sandbox: kalfa-yalitim — kur, kaldir, izin-ver, izin-geri-al"
```

---

### Task 3: Hazırlığın yerel onayı; Windows'ta yalıtım dizini

**Files:** `crates/kalfa-core/src/{kabuk,izin}.rs`, `crates/kalfa-sandbox/src/lib.rs`, `crates/kalfa-tools/src/kabuk/mod.rs`, `crates/kalfa-uygulama/src/{dizin,guvenlik,oturumlar,uygulama,yalitim,test_destek}.rs`, `crates/kalfa/src/lib.rs`, `ui/src/{ipc/tipler.ts,dil/tr.ts,ana/ekranlar.test.ts,onizleme/senaryo.ts}` ve testler.

**Interfaces (Produces):**
- `KabukTarifi.alan_hazirligi: bool` (serde varsayılanı `false`; mühre girer).
- `izin::ALAN_HAZIRLIGI_UYARISI`; `kabuk_karari` yalıtımlı ve `alan_hazirligi` tarifi her kipte `Karar::YerelOnayli` yapar (İnceleme yine yasak).
- `Yalitici::hazirlik_gerekli(&Path) -> bool` ve `Yalitici::alani_hazirla(&AlanBilgisi, hazirlik_onaylandi: bool, ilerleme: &mut dyn FnMut(&str)) -> Result<(), BaslatmaHatasi>`; varsayılanları sırasıyla `false` ve `Ok(())`.
- `guvenlik::yerel_onay_basligi`; `dizin::yalitim_dizini(veri, yerel_veri, ortam)`; `Bilesenler.yalitim_dizini`.

- [ ] **Step 1: Değişiklikleri uygula**

**`crates/kalfa-core/src/kabuk.rs`**: şu bloğu

```rust
    /// Bu çalışma alanındaki ilk kabuk çağrısı (aşama 2 tasarımı §4.2).
    pub ilk_cagri: bool,
}
```

şununla değiştir:

```rust
    /// Bu çalışma alanındaki ilk kabuk çağrısı (aşama 2 tasarımı §4.2).
    pub ilk_cagri: bool,
    /// Çalışma alanı yalıtım için hazırlanacak (Windows, aşama 2 tasarımı
    /// §9.4): klasörün izinleri değişir. Onayı her kipte yerel pencerede
    /// sorulur.
    #[serde(default)]
    pub alan_hazirligi: bool,
}
```

**`crates/kalfa-core/src/izin.rs`**: şu bloğu

```rust
dizinindeki dosyaları okuyabilir ve ağa erişebilir.";
pub const AGLI_KABUK_UYARISI: &str = "Komut ağa erişebilecek. Dosya yalıtımı sürer.";

/// Spec §7.4'teki varsayılan gizli dosya kalıpları. Her kalıp her derinlikte
```

şununla değiştir:

```rust
dizinindeki dosyaları okuyabilir ve ağa erişebilir.";
pub const AGLI_KABUK_UYARISI: &str = "Komut ağa erişebilecek. Dosya yalıtımı sürer.";
pub const ALAN_HAZIRLIGI_UYARISI: &str = "Kalfa bu klasörün izinlerine yalıtım için giriş \
ekleyecek. Klasörü ayarlardan kaldırınca geri alınır.";

/// Spec §7.4'teki varsayılan gizli dosya kalıpları. Her kalıp her derinlikte
```

**`crates/kalfa-core/src/izin.rs`**: şu bloğu

```rust
                uyarilar.push(AGLI_KABUK_UYARISI.to_string());
            }
            if kip == Kip::Serbest && uyarilar.is_empty() {
                Karar::Serbest
```

şununla değiştir:

```rust
                uyarilar.push(AGLI_KABUK_UYARISI.to_string());
            }
            // Hazırlık çalışma alanının izinlerini değiştirir (aşama 2
            // tasarımı §9.4): her kipte yerel pencerede sorulur.
            if tarif.alan_hazirligi {
                uyarilar.push(ALAN_HAZIRLIGI_UYARISI.to_string());
                return Karar::YerelOnayli { uyarilar };
            }
            if kip == Kip::Serbest && uyarilar.is_empty() {
                Karar::Serbest
```

**`crates/kalfa-core/tests/izin.rs`**: şu bloğu

```rust
        },
        ilk_cagri,
    }
}
```

şununla değiştir:

```rust
        },
        ilk_cagri,
        alan_hazirligi: false,
    }
}
```

**`crates/kalfa-core/tests/izin.rs`**: şu bloğu

```rust
        Karar::Yasak { .. }
    ));
    assert_eq!(
        kabuk_karari(Kip::Serbest, &kabuk(false, false, true)),
```

şununla değiştir:

```rust
        Karar::Yasak { .. }
    ));
    // Windows'ta çalışma alanının hazırlığı her kipte yerel onay ister.
    for kip in [Kip::Siki, Kip::Proje, Kip::Serbest] {
        let tarif = kalfa_core::kabuk::KabukTarifi {
            alan_hazirligi: true,
            ..kabuk(true, false, true)
        };
        assert_eq!(
            kabuk_karari(kip, &tarif),
            Karar::YerelOnayli {
                uyarilar: vec![ILK_KABUK_UYARISI.into(), ALAN_HAZIRLIGI_UYARISI.into()]
            },
            "{kip:?}"
        );
    }
    assert!(matches!(
        kabuk_karari(
            Kip::Inceleme,
            &kalfa_core::kabuk::KabukTarifi {
                alan_hazirligi: true,
                ..kabuk(true, false, false)
            }
        ),
        Karar::Yasak { .. }
    ));
    assert_eq!(
        kabuk_karari(Kip::Serbest, &kabuk(false, false, true)),
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
            },
            ilk_cagri: true,
        }),
        yerel_onay: true,
```

şununla değiştir:

```rust
            },
            ilk_cagri: true,
            alan_hazirligi: false,
        }),
        yerel_onay: true,
```

**`crates/kalfa-core/tests/protokol.rs`**: şu bloğu

```rust
            "ag": false,
            "yalitim": {"tip": "Yalitimsiz", "talimat": "yok"},
            "ilk_cagri": true
        })
    );
```

şununla değiştir:

```rust
            "ag": false,
            "yalitim": {"tip": "Yalitimsiz", "talimat": "yok"},
            "ilk_cagri": true,
            "alan_hazirligi": false
        })
    );
```

**`crates/kalfa-core/tests/arac_muhur.rs`**: şu bloğu

```rust
        },
        ilk_cagri: false,
    };
    let h = |t: KabukTarifi| Hazirlik {
```

şununla değiştir:

```rust
        },
        ilk_cagri: false,
        alan_hazirligi: false,
    };
    let h = |t: KabukTarifi| Hazirlik {
```

**`crates/kalfa-core/tests/arac_muhur.rs`**: şu bloğu

```rust
            ..tarif.clone()
        },
    ];
    for d in degisikler {
```

şununla değiştir:

```rust
            ..tarif.clone()
        },
        KabukTarifi {
            alan_hazirligi: true,
            ..tarif.clone()
        },
    ];
    for d in degisikler {
```

**`crates/kalfa-core/tests/oturum_kabuk.rs`**: şu bloğu

```rust
                yalitim,
                ilk_cagri: false,
            }),
            ..Default::default()
```

şununla değiştir:

```rust
                yalitim,
                ilk_cagri: false,
                alan_hazirligi: false,
            }),
            ..Default::default()
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
mod yalitimsiz;

use std::path::PathBuf;
use std::sync::Arc;
```

şununla değiştir:

```rust
mod yalitimsiz;

use std::path::{Path, PathBuf};
use std::sync::Arc;
```

**`crates/kalfa-sandbox/src/lib.rs`**: şu bloğu

```rust
    /// Kullanıcının eklediği araç zinciri dizinleri (§7.1).
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}
}
```

şununla değiştir:

```rust
    /// Kullanıcının eklediği araç zinciri dizinleri (§7.1).
    fn okunur_dizinleri_ayarla(&self, _dizinler: Vec<PathBuf>) {}

    /// Çalışma alanı yalıtım için hazırlanmalı mı (§9.4)? Hazırlık çalışma
    /// alanının izinlerini değiştirir; onayı her kipte yerel pencerede
    /// sorulur. Linux'ta hazırlık yoktur.
    fn hazirlik_gerekli(&self, _kok: &Path) -> bool {
        false
    }

    /// Yalıtımlı her çalıştırmadan hemen önce çağrılır (§9.4).
    ///
    /// - Hazırlanmamış çalışma alanı yalnızca `hazirlik_onaylandi` ise
    ///   hazırlanır: onaylanan tarif hazırlığı içeriyordu. Değilse hata döner;
    ///   onaydan sonra hazırlık geri alındıysa komut başlamaz.
    /// - Hazırlanmış çalışma alanında yeni gizli dosyalar ve korumalı yollar
    ///   işlenir.
    /// - `ilerleme` araç kartına giden satırlardır.
    fn alani_hazirla(
        &self,
        _alan: &AlanBilgisi,
        _hazirlik_onaylandi: bool,
        _ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        Ok(())
    }
}
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
use kalfa_core::git_koruma;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
```

şununla değiştir:

```rust
use kalfa_core::git_koruma;
use kalfa_core::kabuk::{KabukBicimi, KabukTarifi, YalitimOzeti};
use kalfa_core::protokol::CiktiAkisi;
use kalfa_core::yol::CalismaYolu;
use kalfa_sandbox::{
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
}

/// Açıklamadaki kısa gösterim: ilk satır, en çok 80 karakter.
fn kisa(gosterim: &str) -> String {
```

şununla değiştir:

```rust
}

impl KabukAraci {
    /// Çalışma alanını yalıtım için hazırlar (§9.4); ilerleme araç kartında
    /// görünür, modele gitmez.
    async fn alani_hazirla(
        &self,
        alan: &AlanBilgisi,
        onaylandi: bool,
        gonderici: &CiktiGonderici,
    ) -> Result<(), AracHatasi> {
        let yalitici = self.yalitici.clone();
        let alan = alan.clone();
        let gonderici = gonderici.clone();
        tokio::task::spawn_blocking(move || {
            yalitici.alani_hazirla(&alan, onaylandi, &mut |satir| {
                gonderici.gonder(CiktiAkisi::Standart, format!("{satir}\n"));
            })
        })
        .await
        .map_err(|e| gecersiz(format!("Çalışma alanının hazırlığı yarıda kaldı: {e}")))?
        .map_err(|e| gecersiz(e.to_string()))
    }
}

/// Açıklamadaki kısa gösterim: ilk satır, en çok 80 karakter.
fn kisa(gosterim: &str) -> String {
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
            },
        };
        Ok(Hazirlik {
            aciklama: format!("`{}` komutunu çalıştırmak istiyor.", kisa(&istek.gosterim)),
```

şununla değiştir:

```rust
            },
        };
        let yalitim = self.yalitim_ozeti(istek.ag);
        // Windows'ta çalışma alanı ilk yalıtımlı çağrıda hazırlanır (§9.4).
        let alan_hazirligi =
            yalitim.yalitimli_mi() && self.yalitici.hazirlik_gerekli(self.alan.kok());
        Ok(Hazirlik {
            aciklama: format!("`{}` komutunu çalıştırmak istiyor.", kisa(&istek.gosterim)),
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim: self.yalitim_ozeti(istek.ag),
                ilk_cagri: !self.guven.guvenildi_mi(),
            }),
            ..Default::default()
```

şununla değiştir:

```rust
                    .map_or_else(|| ".".to_string(), |d| d.as_str().to_string()),
                ag: istek.ag,
                yalitim,
                ilk_cagri: !self.guven.guvenildi_mi(),
                alan_hazirligi,
            }),
            ..Default::default()
```

**`crates/kalfa-tools/src/kabuk/mod.rs`**: şu bloğu

```rust
        };
        let alan = if yalitimli {
            Some(self.alan_bilgisi().await?)
        } else {
            None
```

şununla değiştir:

```rust
        };
        let alan = if yalitimli {
            let bilgi = self.alan_bilgisi().await?;
            let onaylandi = h.kabuk.as_ref().is_some_and(|t| t.alan_hazirligi);
            self.alani_hazirla(&bilgi, onaylandi, &gonderici).await?;
            Some(bilgi)
        } else {
            None
```

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
    Yalitimsiz,
};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
```

şununla değiştir:

```rust
use kalfa_core::protokol::CiktiAkisi;
use kalfa_sandbox::{
    AlanBilgisi, BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu,
    YalitimYoklukNedeni, Yalitimsiz,
};
use kalfa_tools::{CalismaAlani, KabukAraci, KabukGuveni};
```

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
    basarisiz: AtomicBool,
    yok: AtomicBool,
}
```

şununla değiştir:

```rust
    basarisiz: AtomicBool,
    yok: AtomicBool,
    /// Çalışma alanı hazırlanmamış (Windows, §9.4).
    hazirlanmamis: AtomicBool,
    /// `alani_hazirla` çağrılarının `hazirlik_onaylandi` değerleri.
    hazirliklar: std::sync::Mutex<Vec<bool>>,
}
```

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
        self.durum()
    }
}
```

şununla değiştir:

```rust
        self.durum()
    }

    fn hazirlik_gerekli(&self, _kok: &std::path::Path) -> bool {
        self.hazirlanmamis.load(Ordering::SeqCst)
    }

    fn alani_hazirla(
        &self,
        _alan: &AlanBilgisi,
        hazirlik_onaylandi: bool,
        ilerleme: &mut dyn FnMut(&str),
    ) -> Result<(), BaslatmaHatasi> {
        self.hazirliklar.lock().unwrap().push(hazirlik_onaylandi);
        if self.hazirlanmamis.load(Ordering::SeqCst) {
            if !hazirlik_onaylandi {
                return Err(BaslatmaHatasi::Yalitim(
                    "çalışma alanı hazırlanmamış".into(),
                ));
            }
            ilerleme("İzinler hazırlanıyor…");
            self.hazirlanmamis.store(false, Ordering::SeqCst);
        }
        Ok(())
    }
}
```

**`crates/kalfa-tools/tests/kabuk.rs`**: şu bloğu

```rust
    let arac = KabukAraci::yeni(alan, yalitici.clone(), Arc::new(SahteGuven::default()));
    (d, arac, yalitici)
}
```

şununla değiştir:

```rust
    let arac = KabukAraci::yeni(alan, yalitici.clone(), Arc::new(SahteGuven::default()));
    (d, arac, yalitici)
}

fn yazdir() -> Value {
    if cfg!(windows) {
        json!({"betik": "Write-Output tamam"})
    } else {
        json!({"betik": "echo tamam"})
    }
}

/// Windows'ta çalışma alanı ilk yalıtımlı çağrıda hazırlanır (§9.4): hazırlık
/// tarife girer (yerel onay), komuttan önce yapılır, ilerlemesi araç kartında
/// görünür.
#[tokio::test]
async fn hazirlik_tarife_girer_ve_komuttan_once_yapilir() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(t.alan_hazirligi);

    let (sonuc, parcalar) = calistir(&arac, &yazdir()).await;
    assert!(sonuc.unwrap().contains("tamam"));
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![true]);
    assert!(
        parcalar
            .iter()
            .any(|(_, m)| m.contains("İzinler hazırlanıyor")),
        "{parcalar:?}"
    );
    assert_eq!(yalitici.istekler.lock().unwrap().len(), 1);

    // Hazırlanmış çalışma alanında sonraki çağrı hazırlık istemez; araç yine
    // her çalıştırmadan önce yalıtıcıya sorar (yeni gizli dosyalar).
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(!t.alan_hazirligi);
    let (sonuc, _) = calistir(&arac, &yazdir()).await;
    sonuc.unwrap();
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![true, false]);
}

/// Onaylanan tarif hazırlık içermiyorsa (onaydan sonra hazırlık geri
/// alındıysa) komut başlamaz.
#[tokio::test]
async fn hazirlik_onaylanmadiysa_komut_baslamaz() {
    let (_d, arac, yalitici) = kur_yalitimli();
    let h = arac.hazirla(&yazdir()).await.unwrap();
    assert!(!h.kabuk.as_ref().unwrap().alan_hazirligi);
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let sonuc = arac.calistir(&yazdir(), &h, CancellationToken::new()).await;
    assert!(sonuc.is_err());
    assert_eq!(*yalitici.hazirliklar.lock().unwrap(), vec![false]);
    assert!(yalitici.istekler.lock().unwrap().is_empty());
}

#[tokio::test]
async fn yalitimsizken_hazirlik_tarife_girmez() {
    let (_d, arac, yalitici) = kur_yalitimli();
    yalitici.yok.store(true, Ordering::SeqCst);
    yalitici.hazirlanmamis.store(true, Ordering::SeqCst);
    let t = arac.hazirla(&yazdir()).await.unwrap().kabuk.unwrap();
    assert!(!t.alan_hazirligi);
}
```

**`crates/kalfa-uygulama/src/dizin.rs`**: şu bloğu

```rust
}

pub fn oturum_dizini(veri_dizini: &Path) -> PathBuf {
    veri_dizini.join("oturumlar")
```

şununla değiştir:

```rust
}

/// Yalıtımın çalışma alanına özel dizinlerinin kökü (aşama 2 tasarımı §7.3).
/// Windows'ta `%LOCALAPPDATA%\Kalfa\yalitim` (`yerel_veri`: Tauri'de
/// `local_data_dir()`): önbellekler dolaşan profile girmez. Linux'ta ve
/// `KALFA_VERI_DIZINI` verildiğinde veri dizinindeki `yalitim`.
pub fn yalitim_dizini(veri: &Path, yerel_veri: &Path, ortam: Option<OsString>) -> PathBuf {
    if cfg!(windows) && ortam.filter(|d| !d.is_empty()).is_none() {
        return yerel_veri.join("Kalfa").join("yalitim");
    }
    veri.join("yalitim")
}

pub fn oturum_dizini(veri_dizini: &Path) -> PathBuf {
    veri_dizini.join("oturumlar")
```

**`crates/kalfa-uygulama/src/guvenlik.rs`**: şu bloğu

```rust
    }
}

/// Yalıtımsız komutun yerel onay metni (aşama 2 tasarımı §5.2). Komut
/// çekirdeğin bekleyen onayından gelir.
pub fn yerel_onay_metni(ogeler: &[OnayOgesi]) -> String {
    let mut metin = String::from(
        "Aşağıdaki komut yalıtımsız çalışacak: senin bütün yetkilerinle çalışır, ev dizinindeki \
         dosyaları okuyabilir ve ağa erişebilir.\n",
    );
    let mut talimat = None;
    for o in ogeler {
        let Some(k) = &o.kabuk else { continue };
```

şununla değiştir:

```rust
    }
}

fn yalitimsiz_var(ogeler: &[OnayOgesi]) -> bool {
    ogeler
        .iter()
        .any(|o| o.kabuk.as_ref().is_some_and(|k| !k.yalitim.yalitimli_mi()))
}

/// Yerel onay penceresinin başlığı: yalıtımsız komut varsa onu, yoksa
/// çalışma alanının hazırlığını (Windows, aşama 2 tasarımı §9.4) söyler.
pub fn yerel_onay_basligi(ogeler: &[OnayOgesi]) -> &'static str {
    if yalitimsiz_var(ogeler) {
        "Komutu yalıtımsız çalıştır"
    } else {
        "Çalışma alanını yalıtım için hazırla"
    }
}

/// Yerel onay metni (aşama 2 tasarımı §5.2, §9.4). Komut çekirdeğin bekleyen
/// onayından gelir.
pub fn yerel_onay_metni(ogeler: &[OnayOgesi]) -> String {
    let mut metin = String::new();
    if yalitimsiz_var(ogeler) {
        metin.push_str(
            "Aşağıdaki komut yalıtımsız çalışacak: senin bütün yetkilerinle çalışır, ev \
             dizinindeki dosyaları okuyabilir ve ağa erişebilir.\n",
        );
    }
    let hazirlik = ogeler
        .iter()
        .any(|o| o.kabuk.as_ref().is_some_and(|k| k.alan_hazirligi));
    if hazirlik {
        metin.push_str(
            "Kalfa bu çalışma alanını yalıtım için hazırlayacak: klasörün izinlerine yalıtım için \
             giriş ekler, gizli dosyaları ve .git'i yalıtıma kapatır. Klasörü ayarlardan \
             kaldırınca geri alınır. Projenin betikleri (build.rs, package.json, Makefile) kod \
             olarak çalışır.\n",
        );
    }
    let mut talimat = None;
    for o in ogeler {
        let Some(k) = &o.kabuk else { continue };
```

**`crates/kalfa-uygulama/src/guvenlik.rs`**: şu bloğu

```rust
    metin
}
```

şununla değiştir:

```rust
    metin
}

#[cfg(test)]
mod tests {
    use kalfa_core::kabuk::{KabukBicimi, KabukTarifi};
    use kalfa_core::kimlik::CagriId;
    use kalfa_core::muhur::OnayMuhru;

    use super::*;

    fn oge(yalitim: YalitimOzeti, alan_hazirligi: bool) -> OnayOgesi {
        OnayOgesi {
            cagri: CagriId::yeni(),
            arac: "kabuk".into(),
            aciklama: String::new(),
            hedefler: Vec::new(),
            diff: None,
            uyarilar: Vec::new(),
            muhur: OnayMuhru("m".into()),
            kabuk: Some(KabukTarifi {
                komut: "cargo test".into(),
                bicim: KabukBicimi::Program,
                dizin: ".".into(),
                ag: false,
                yalitim,
                ilk_cagri: true,
                alan_hazirligi,
            }),
            yerel_onay: true,
        }
    }

    fn yalitimli() -> YalitimOzeti {
        YalitimOzeti::Yalitimli {
            arka_uc: "windows".into(),
            profil: "p".into(),
        }
    }

    fn yalitimsiz() -> YalitimOzeti {
        YalitimOzeti::Yalitimsiz {
            talimat: "Yalıtımı kurun.".into(),
        }
    }

    #[test]
    fn hazirlik_penceresi_izin_degisikligini_soyler() {
        let ogeler = [oge(yalitimli(), true)];
        assert_eq!(
            yerel_onay_basligi(&ogeler),
            "Çalışma alanını yalıtım için hazırla"
        );
        let metin = yerel_onay_metni(&ogeler);
        assert!(
            metin.contains("izinlerine yalıtım için giriş ekler"),
            "{metin}"
        );
        assert!(metin.contains("Komut: cargo test"), "{metin}");
        assert!(!metin.contains("yalıtımsız"), "{metin}");
    }

    #[test]
    fn yalitimsiz_komut_basligi_ve_talimati() {
        let ogeler = [oge(yalitimsiz(), false), oge(yalitimli(), true)];
        assert_eq!(yerel_onay_basligi(&ogeler), "Komutu yalıtımsız çalıştır");
        let metin = yerel_onay_metni(&ogeler);
        assert!(
            metin.starts_with("Aşağıdaki komut yalıtımsız çalışacak"),
            "{metin}"
        );
        assert!(
            metin.contains("izinlerine yalıtım için giriş ekler"),
            "{metin}"
        );
        assert!(metin.ends_with("Yalıtımı kurun."), "{metin}");
    }
}
```

**`crates/kalfa-uygulama/src/oturumlar.rs`**: şu bloğu

```rust
                .iletisim
                .onayla(
                    "Komutu yalıtımsız çalıştır",
                    &guvenlik::yerel_onay_metni(&yereller),
                    "Çalıştır",
```

şununla değiştir:

```rust
                .iletisim
                .onayla(
                    guvenlik::yerel_onay_basligi(&yereller),
                    &guvenlik::yerel_onay_metni(&yereller),
                    "Çalıştır",
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
    /// Kullanıcının ev dizini: araç zinciri dizinleri buna göre bulunur.
    pub ev_dizini: PathBuf,
    pub yalitim: YalitimKaynagi,
}
```

şununla değiştir:

```rust
    /// Kullanıcının ev dizini: araç zinciri dizinleri buna göre bulunur.
    pub ev_dizini: PathBuf,
    /// Çalışma alanına özel dizinlerin kökü (`dizin::yalitim_dizini`).
    pub yalitim_dizini: PathBuf,
    pub yalitim: YalitimKaynagi,
}
```

**`crates/kalfa-uygulama/src/uygulama.rs`**: şu bloğu

```rust
        let yalitici = yalitici_kur(
            b.yalitim,
            &b.veri_dizini,
            &b.ev_dizini,
            &arac_zinciri,
```

şununla değiştir:

```rust
        let yalitici = yalitici_kur(
            b.yalitim,
            &b.yalitim_dizini,
            &b.ev_dizini,
            &arac_zinciri,
```

**`crates/kalfa-uygulama/src/yalitim.rs`**: şu bloğu

```rust
pub(crate) fn yalitici_kur(
    kaynak: YalitimKaynagi,
    veri_dizini: &Path,
    ev: &Path,
    kayit: &AracZinciriKaydi,
```

şununla değiştir:

```rust
pub(crate) fn yalitici_kur(
    kaynak: YalitimKaynagi,
    yalitim_dizini: &Path,
    ev: &Path,
    kayit: &AracZinciriKaydi,
```

**`crates/kalfa-uygulama/src/yalitim.rs`**: şu bloğu

```rust
        YalitimKaynagi::Platform => kalfa_sandbox::platform_yaliticisi(PlatformAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: veri_dizini.join("yalitim"),
            ek_okunur: kayit.dizinler(),
            degisince,
```

şununla değiştir:

```rust
        YalitimKaynagi::Platform => kalfa_sandbox::platform_yaliticisi(PlatformAyari {
            ev: ev.to_path_buf(),
            yalitim_dizini: yalitim_dizini.to_path_buf(),
            ek_okunur: kayit.dizinler(),
            degisince,
```

**`crates/kalfa-uygulama/src/test_destek.rs`**: şu bloğu

```rust
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
        ev_dizini: dizin.join("ev"),
        yalitim,
```

şununla değiştir:

```rust
        pencereler: pencereler.clone(),
        aktarim: AktarimAyarlari::default(),
        yalitim_dizini: dizin.join("yalitim"),
        ev_dizini: dizin.join("ev"),
        yalitim,
```

**`crates/kalfa-uygulama/tests/e2e_parcalari.rs`**: şu bloğu

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: AktarimAyarlari::default(),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
```

şununla değiştir:

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: AktarimAyarlari::default(),
        yalitim_dizini: veri.join("yalitim"),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
```

**`crates/kalfa-uygulama/tests/sart_akisi.rs`**: şu bloğu

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: Default::default(),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
```

şununla değiştir:

```rust
        pencereler: Arc::new(SahtePencereler::default()),
        aktarim: Default::default(),
        yalitim_dizini: d.path().join("yalitim"),
        ev_dizini: std::env::temp_dir(),
        yalitim: YalitimKaynagi::Kurucu(Box::new(|_| Arc::new(Yalitimsiz))),
```

**`crates/kalfa-uygulama/tests/kurallar.rs`**: şu bloğu

```rust
use kalfa_evren::EvrenError;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu, DepoHatasi};
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini, veri_dizini};
use kalfa_uygulama::evren::DegisirSaglayici;
use kalfa_uygulama::guvenlik::{acilabilir_adres, gezinme_izinli, yukseltme_mi};
```

şununla değiştir:

```rust
use kalfa_evren::EvrenError;
use kalfa_uygulama::anahtar::{AnahtarDeposu, AnahtarKasasi, BellekDeposu, DepoHatasi};
use kalfa_uygulama::dizin::{kayit_yolu, oturum_dizini, veri_dizini, yalitim_dizini};
use kalfa_uygulama::evren::DegisirSaglayici;
use kalfa_uygulama::guvenlik::{acilabilir_adres, gezinme_izinli, yukseltme_mi};
```

**`crates/kalfa-uygulama/tests/kurallar.rs`**: şu bloğu

```rust
        Path::new("baska")
    );
    let oturum = OturumId::yeni();
    assert_eq!(
```

şununla değiştir:

```rust
        Path::new("baska")
    );
    // Yalıtım dizini Windows'ta yerel (dolaşmayan) veri dizinindedir.
    let yerel = Path::new("yerel");
    let yalitim = yalitim_dizini(Path::new("v"), yerel, None);
    if cfg!(windows) {
        assert_eq!(yalitim, yerel.join("Kalfa").join("yalitim"));
    } else {
        assert_eq!(yalitim, Path::new("v").join("yalitim"));
    }
    assert_eq!(
        yalitim_dizini(Path::new("v"), yerel, Some(OsString::from("v"))),
        Path::new("v").join("yalitim")
    );
    let oturum = OturumId::yeni();
    assert_eq!(
```

**`crates/kalfa/src/lib.rs`**: şu bloğu

```rust
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
```

şununla değiştir:

```rust
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let yalitim = kalfa_uygulama::dizin::yalitim_dizini(
        &veri,
        &app.path().local_data_dir()?,
        std::env::var_os(VERI_DIZINI_DEGISKENI),
    );
    let kalici: Option<Box<dyn AnahtarDeposu>> =
        if std::env::var(ANAHTAR_DEPOSU_DEGISKENI).as_deref() == Ok("bellek") {
```

**`crates/kalfa/src/lib.rs`**: şu bloğu

```rust
        aktarim: AktarimAyarlari::default(),
        ev_dizini: app.path().home_dir()?,
        yalitim: YalitimKaynagi::Platform,
    }))
```

şununla değiştir:

```rust
        aktarim: AktarimAyarlari::default(),
        ev_dizini: app.path().home_dir()?,
        yalitim_dizini: yalitim,
        yalitim: YalitimKaynagi::Platform,
    }))
```

**`ui/src/ipc/tipler.ts`**: şu bloğu

```ts
  yalitim: YalitimOzeti;
  ilk_cagri: boolean;
}
```

şununla değiştir:

```ts
  yalitim: YalitimOzeti;
  ilk_cagri: boolean;
  /** Çalışma alanı yalıtım için hazırlanacak (Windows, aşama 2 tasarımı §9.4). */
  alan_hazirligi: boolean;
}
```

**`ui/src/dil/tr.ts`**: şu bloğu

```ts
  'onay.kabuk.yalitimsiz': 'Yalıtımsız',
  'onay.kabuk.yerel':
    'Yalıtımsız komutlar, onayınızdan sonra yerel bir pencerede bir kez daha sorulur.',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
```

şununla değiştir:

```ts
  'onay.kabuk.yalitimsiz': 'Yalıtımsız',
  'onay.kabuk.yerel':
    'Yalıtımsız komutlar ve çalışma alanının yalıtım için hazırlığı, onayınızdan sonra yerel bir pencerede bir kez daha sorulur.',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
```

**`ui/src/ana/ekranlar.test.ts`**: şu bloğu

```ts
              yalitim: { tip: 'Yalitimsiz', talimat: 'yok' },
              ilk_cagri: false,
            },
            yerel_onay: true,
```

şununla değiştir:

```ts
              yalitim: { tip: 'Yalitimsiz', talimat: 'yok' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
            yerel_onay: true,
```

**`ui/src/onizleme/senaryo.ts`**: şu bloğu

```ts
                      yalitim: { tip: 'Yalitimsiz', talimat: arka.uygulama.yalitim.tip === 'Yok' ? arka.uygulama.yalitim.talimat : '' },
                      ilk_cagri: false,
                    },
                    yerel_onay: true,
```

şununla değiştir:

```ts
                      yalitim: { tip: 'Yalitimsiz', talimat: arka.uygulama.yalitim.tip === 'Yok' ? arka.uygulama.yalitim.talimat : '' },
                      ilk_cagri: false,
                      alan_hazirligi: false,
                    },
                    yerel_onay: true,
```

- [ ] **Step 2: Testleri koş**

Run: `cargo test -p kalfa-core --locked --test izin kabuk_kararlari`, `cargo test -p kalfa-tools --locked --test kabuk hazirlik`, `cargo test -p kalfa-uygulama --locked --lib guvenlik`
Expected: PASS. Sonra Global Constraints'teki bütün denetimler (arayüz dahil).

- [ ] **Step 3: Commit**

```bash
git add crates ui
git commit -m "çekirdek, araçlar, uygulama, ui: çalışma alanı hazırlığının yerel onayı; Windows'ta yalıtım dizini LOCALAPPDATA'da"
```

---

### Task 4: `WindowsYalitici`: algılama, çalışma alanı hazırlığı, kap

Aşama 2 tasarımı §9.2–§9.4'e göre uygulanır; kabul ölçütü §12.2'nin Windows sütunudur (`crates/kalfa-sandbox/tests/`'ta `yalitim-testleri` özelliğiyle, kurulumdan sonra). `Yalitici::hazirlik_gerekli` ve `alani_hazirla` Görev 3'te tanımlıdır. Bu görevin kodu plan yazılırken önceden doğrulanmadı.

- [ ] Uygula; `platform_yaliticisi` Windows'ta `WindowsYalitici`'yi döndürsün; Görev 1'in `allow(dead_code)`'larını kaldır.
- [ ] Kurulum için kullanıcıya PR yorumuyla sor; testleri koş; sonuçları PR'a yaz.
- [ ] Commit: `sandbox: WindowsYalitici`.

### Task 5: Windows araç zinciri uyumluluğu; A11, A12, A13, A15

`crates/kalfa-tools/tests/uyumluluk.rs` Windows'ta da koşsun (§12.3: git status/diff/log, cargo ağsız, npm, python). A11, A12, A13 ve A15'in sonuçlarını ölç ve PR'a yaz; çalışmayan her durum §14'e gerekçesiyle girer.

- [ ] Commit: `araçlar: Windows uyumluluk testleri`.

### Task 6: Belgeler

`docs/GELISTIRME_DURUMU.md`'ye 2d-1 bölümü (neler eklendi, doğrulama tablosu, bulunanlar), tasarım belgesinde §14'ün A11–A13, A15 satırları ve §15'te 2d'nin 2d-1/2d-2 bölünmesi.

- [ ] Commit: `docs: 2d-1 Windows arka ucu`.

---

## İnceleme (Claude)

- Görev 1–3'ün kodunun plandakiyle aynı olması.
- Görev 4–5: §12.2 Windows sütununun her satırının bir testi olması; testlerin kurulumdan sonra Windows'ta geçmesi; Linux'ta davranışın değişmemesi (`yalitim`, `uyumluluk`, uçtan uca).
- Belgelerdeki sayıların inceleme sonuçlarıyla aynı olması.
