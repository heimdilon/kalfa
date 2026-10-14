# Kalfa 1.1a-1 — Etkileşimli süreç: Uygulama planı

> **Ajanlar için:** Bu plan görev görev uygulanır. Adımlar onay kutusuyla
> (`- [ ]`) izlenir. Her görev önce düşen testle başlar.

**Durum:** İncelemede. Kullanıcı yalnız planın yazılmasını onayladı (14 Ekim).
Ürün kodu, plan incelendikten sonra ayrı onayla yazılır.
**Ön doğrulama:** AGENTS, planın kodunun önceden iki platformda doğrulanmasını
ister. Kullanıcı ürün koduna geçmeden önce planı incelemek istediği için
buradaki kod blokları henüz derlenmedi. Plan onaylanınca ilk iş, kodun ayrı
bir worktree'de iki platformda derlenip testlerin koşturulmasıdır. Sapma
çıkarsa önce bu plan düzeltilir.
**Taban:** `main` `04f16fc` ve PR #19 dalı (`docs/mcp-yerel-tasarim`).
**Tasarım:** [1.1a tasarımı](../specs/2026-10-14-kalfa-11a-mcp-yerel-design.md)
§3 (sunucunun özel kökü, ortam), §4.1 (etkileşimli süreç, çalışma kipi, ağ),
§5.6 (yetenek SID'i). [G1 raporu](../bulgular/2026-10-14-kalfa-11a-g1.md).

**Hedef:** `kalfa-sandbox`, MCP sunucusu gibi uzun yaşayan ve stdin/stdout
borulu bir süreci yalıtımlı ya da açıkça seçilmiş tam erişimle başlatabilir.
Sürecin ağacını güvenle kapatabilir. Sunucuya özel kökü Linux'ta bağlamayla,
Windows'ta yetenek SID'iyle kabuktan ayırır.

**Mimari:** Yeni `etkilesimli.rs`, platformdan bağımsız parçaları taşır:
istek, çalışma kipi, sunucu adı, özel kök ve ortam. `surec.rs`, mevcut
`Cocuk`/`Agac` altyapısıyla bir `EtkilesimliSurec` üretir. İki arka uç,
mevcut kabuk başlatmasının bwrap ve AppContainer kurulumunu yeniden kullanır;
yalnız özel kök, çalışma dizini, stdin borusu ve yetenekler farklıdır. Ajan
döngüsü, `rmcp` ve arayüz bu planın kapsamında değildir (1.1a-2).

**Yığın:** Rust 1.98.1, tokio, `windows-sys` (mevcut), bwrap/seccomp
(mevcut). Yeni bağımlılık yok.

## Global Constraints

- Yeni crate yok; `rmcp` 1.1a-2'dedir. `windows-sys` özellikleri yetmezse
  eklenecek özellik önce PR'a yazılır.
- `unsafe` yalnız zaten `unsafe` içeren dosyalarda (`windows/kap.rs`,
  `windows/guvenlik.rs`, `linux/mod.rs`) ve her blok `// SAFETY:`
  açıklamasıyla yazılır.
- Üretim kodunda `unwrap`/`expect` yok. Adlar, açıklamalar ve testler
  Türkçedir.
- Güvenlik sınırları izin listesiyle kurulur: sunucu adı
  `[a-z0-9-]{1,32}`; `EVREN_` ve `KALFA_` önekli ortam adları hiçbir yoldan
  sunucuya geçmez.
- **Çalışma kipi açıktır** (tasarım §4.1). Yalıtımlı istek yalıtım yoksa
  başlamaz. Tam erişim, yalıtım hazırken de yalıtımsız başlar. Mevcut
  `kip_uyusmasi` kabuğa özgüdür; MCP onu kullanmaz.
- **Özel kök** = `<yalitim_dizini>/<alan_kimligi>/mcp/<sunucu>/{ev,gecici,onbellek}`.
  - Kabuğun özel `ev`, `gecici` ve `onbellek` dizinleri sunucuya bağlanmaz.
  - Sunucunun kökü de kabuğa ve öbür sunuculara bağlanmaz.
  - Windows'ta bu kökte paket SID'inin hiçbir hakkı yoktur; değiştirme hakkı
    yalnız sunucunun yetenek SID'ine verilir (`kalfa-mcp-<alan>-<sunucu>`).
- **Ortam** (tasarım §3, G1 bulgusu):
  - `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TMPDIR`, `TEMP`,
    `TMP`, `NPM_CONFIG_CACHE`, `PIP_CACHE_DIR`, `UV_CACHE_DIR` ve
    `XDG_CACHE_HOME` özel köke yönelir.
  - Windows'ta `SystemRoot`, `windir` ve `SystemDrive` sistemin kendi
    değerlerini taşır.
- **`kapat()`:**
  1. stdin kapanır.
  2. En çok 2 sn kendiliğinden çıkış beklenir.
  3. Ağaç her durumda öldürülür.

  Döndüğünde ağaçta yaşayan süreç yoktur.
- **Windows ACE'leri** ACE kaydına `<alan_kimligi>/mcp/<sunucu>` anahtarıyla
  girer. Sunucu kaldırılınca ya da çalışma alanı kaldırılınca geri alınır.
  Testler makinede Kalfa profili ya da `S-1-15-2-`/`S-1-15-3-` ACE'si
  bırakmaz.
- **Testler:** Yalıtım testleri `yalitim-testleri` özelliğiyle gerçek
  yalıtımda koşar; yalıtım yoksa düşer. Windows'ta yönetici gerektiren bir
  adım yoktur.

## Review Focus

Spec'in ima ettiği ama ilk bakışta test edilmeyen beş durum. Her biri sahibi
olan görevde ayrı bir testle sabitlenmiştir:

1. **Sunucu adı yol aşımı:** `../x`, `a/b`, büyük harf ya da 33 karakterlik
   ad. Hiçbir dizin oluşturulmamalı, başlatma reddedilmeli (Görev 1).
2. **Sunucu stdin'i hiç okumuyor:** stdout'a sürekli yazan ama stdin'i
   okumayan sunucu. `kapat()`, boru dolu olsa da 2 sn + öldürme içinde
   dönmeli (Görev 2).
3. **Süreç `kapat()` çağrılmadan düşürülüyor:** Örneğin bir panik sırasında
   `EtkilesimliSurec` bırakılıyor. Ağaç yine ölmeli (Görev 2).
4. **Aynı alanda iki sunucu:** A sunucusu B'nin özel köküne yazamamalı ve
   okuyamamalı (Görev 4, Windows; Görev 3, Linux).
5. **Çalışma alanı kaldırılırken sunucu izinleri:** `alani_kaldir`, o alana
   bağlı bütün `<alan>/mcp/*` ACE'lerini de geri almalı (Görev 4).

---

## Dosya haritası

| Dosya | Sorumluluk |
|---|---|
| `crates/kalfa-sandbox/src/etkilesimli.rs` (yeni) | `CalistirmaKipi`, `EtkilesimliIstek`, `sunucu_adi_dogrula`, `sunucu_ozel`, `sunucu_ortami`, `mcp_kip_uyusmasi`, `KAPANMA_PAYI` |
| `crates/kalfa-sandbox/src/surec.rs` | `EtkilesimliSurec`, `KapanisSonucu`, `Surec::komutla_etkilesimli`, Windows için `kaptan_etkilesimli` |
| `crates/kalfa-sandbox/src/lib.rs` | Modül, dışa açılan tipler, `Yalitici::baslat_etkilesimli` ve `sunucu_kokunu_kaldir` |
| `crates/kalfa-sandbox/src/yalitimsiz.rs` | Yalıtımsız arka uçta yalnız tam erişim |
| `crates/kalfa-sandbox/src/linux/mod.rs` | `komut_kur`'un özel kök parametresi, ortak vekil kurulumu, Linux etkileşimli başlatma ve kaldırma |
| `crates/kalfa-sandbox/src/windows/kap.rs` | Borulu stdin (`Kap::girdi`, `baslat_etkilesimli`) |
| `crates/kalfa-sandbox/src/windows/yalitici.rs` | Sunucu kökünün hazırlığı ve ACE kaydı, Windows etkileşimli başlatma, kaldırma, `alani_kaldir`'da sunucu girdileri |
| `crates/kalfa-sandbox/tests/yalitim.rs` | Linux kabul testleri (G1 karşılıkları) |
| `crates/kalfa-sandbox/tests/yalitim/windows.rs` | Windows kabul testleri (G1 karşılıkları) |
| `docs/GELISTIRME_DURUMU.md`, tasarım §4.1 | Sonuç |

---

### Görev 1: Platformdan bağımsız sözleşme

**Dosyalar:**
- Oluştur: `crates/kalfa-sandbox/src/etkilesimli.rs`
- Değiştir: `crates/kalfa-sandbox/src/lib.rs`, `crates/kalfa-sandbox/src/yalitimsiz.rs`
- Test: `etkilesimli.rs` içindeki `#[cfg(test)]`, `crates/kalfa-sandbox/tests/yalitimsiz.rs`

**Arayüzler:**
- Kullanır: `profil::{AlanBilgisi, OzelDizinler}`, `vekil::IzinListesi`, `BaslatmaHatasi`.
- Üretir:
  - `pub enum CalistirmaKipi { Yalitimli, TamErisim }`
  - `pub struct EtkilesimliIstek { program, argumanlar, ortam, kip, sunucu, alan, ag_listesi }`
  - `pub fn sunucu_adi_dogrula(ad: &str) -> Result<(), BaslatmaHatasi>`
  - `pub fn sunucu_ozel(yalitim_dizini: &Path, alan_kimligi: &str, sunucu: &str) -> OzelDizinler`
  - `pub fn sunucu_ortami(kaynak: &[(String, String)], ozel: &OzelDizinler) -> Vec<(String, String)>`
  - `pub fn mcp_kip_uyusmasi(kip: CalistirmaKipi, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi>`
  - `pub const KAPANMA_PAYI: Duration`
  - `Yalitici::baslat_etkilesimli(&self, &EtkilesimliIstek) -> Result<EtkilesimliSurec, BaslatmaHatasi>`.
    Varsayılan gövdesi hata döner.
  - `Yalitici::sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String>`.
    Varsayılan gövdesi hata döner.

- [ ] **Adım 1: Düşen birim testlerini yazın** (`etkilesimli.rs` sonu):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sunucu_adi_yalniz_izin_listesindeki_karakterler() {
        for gecerli in ["a", "github", "fs-2", &"a".repeat(32)] {
            assert!(sunucu_adi_dogrula(gecerli).is_ok(), "{gecerli}");
        }
        for gecersiz in ["", "../x", "a/b", "a\\b", "A", "ğ", " a", &"a".repeat(33)] {
            assert!(sunucu_adi_dogrula(gecersiz).is_err(), "{gecersiz:?}");
        }
    }

    #[test]
    fn ozel_kok_alanin_mcp_dizininde() {
        let o = sunucu_ozel(Path::new("/v/yalitim"), "abcd", "github");
        assert_eq!(o.kok, Path::new("/v/yalitim/abcd/mcp/github"));
        assert_eq!(o.ev, o.kok.join("ev"));
        assert_eq!(o.gecici, o.kok.join("gecici"));
        assert_eq!(o.onbellek, o.kok.join("onbellek"));
    }

    #[test]
    fn ortam_ozel_koke_yonelir_ayrilmis_adlar_gecmez() {
        let o = sunucu_ozel(Path::new("/v"), "a", "s");
        let kaynak = vec![
            ("HOME".to_string(), "/home/kullanici".to_string()),
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("EVREN_API_KEY".to_string(), "gizli".to_string()),
            ("evren_base_url".to_string(), "x".to_string()),
            ("KALFA_E2E".to_string(), "1".to_string()),
            ("GITHUB_TOKEN".to_string(), "ghp".to_string()),
        ];
        let ortam = sunucu_ortami(&kaynak, &o);
        let deger = |ad: &str| ortam.iter().find(|(a, _)| a == ad).map(|(_, d)| d.as_str());
        assert_eq!(deger("HOME"), Some(o.ev.display().to_string().as_str()));
        assert_eq!(deger("TMPDIR"), Some(o.gecici.display().to_string().as_str()));
        assert_eq!(deger("NPM_CONFIG_CACHE"), Some(o.npm().display().to_string().as_str()));
        assert_eq!(deger("PIP_CACHE_DIR"), Some(o.pip().display().to_string().as_str()));
        assert_eq!(
            deger("UV_CACHE_DIR"),
            Some(o.onbellek.join("uv").display().to_string().as_str())
        );
        assert_eq!(deger("XDG_CACHE_HOME"), Some(o.onbellek.display().to_string().as_str()));
        assert_eq!(deger("PATH"), Some("/usr/bin"));
        assert_eq!(deger("GITHUB_TOKEN"), Some("ghp"));
        assert!(ortam.iter().all(|(a, _)| {
            let b = a.to_ascii_uppercase();
            !b.starts_with("EVREN_") && !b.starts_with("KALFA_")
        }));
        assert_eq!(ortam.iter().filter(|(a, _)| a == "HOME").count(), 1);
    }

    #[test]
    fn kip_uyusmasi_dort_hucre() {
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::Yalitimli, true).is_ok());
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::TamErisim, true).is_ok());
        assert!(mcp_kip_uyusmasi(CalistirmaKipi::TamErisim, false).is_ok());
        assert!(matches!(
            mcp_kip_uyusmasi(CalistirmaKipi::Yalitimli, false),
            Err(BaslatmaHatasi::Yalitim(_))
        ));
    }
}
```

- [ ] **Adım 2: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --locked --lib etkilesimli`.
  Beklenen: derleme hatası; `etkilesimli` modülü yok.

- [ ] **Adım 3: Modülü yazın** (`etkilesimli.rs`, testlerden önce):

```rust
//! Etkileşimli süreç (1.1a tasarımı §3, §4.1): stdin/stdout borulu,
//! kapatılana kadar yaşayan süreç. MCP sunucuları bununla başlar. Toplam
//! süre ve çıktı sınırı yoktur; süreç sınırları kabuktaki gibidir.

use std::path::Path;
use std::time::Duration;

use crate::profil::{AlanBilgisi, OzelDizinler};
use crate::{BaslatmaHatasi, vekil};

/// `kapat()`'ta stdin kapandıktan sonra kendiliğinden çıkış için tanınan
/// süre (§4.1).
pub const KAPANMA_PAYI: Duration = Duration::from_secs(2);

/// Sunucunun açıkça seçilmiş çalışma kipi (§4.1). Kabuktaki yalıtımsız
/// çalışma bir geri düşme yoludur; burada tam erişim kullanıcının seçimidir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalistirmaKipi {
    Yalitimli,
    TamErisim,
}

#[derive(Debug, Clone)]
pub struct EtkilesimliIstek {
    /// Çıplak program adı ya da mutlak yol.
    pub program: String,
    pub argumanlar: Vec<String>,
    /// Kalıtılan ve yapılandırılan ortam; `sunucu_ortami` özel köke çevirir.
    pub ortam: Vec<(String, String)>,
    pub kip: CalistirmaKipi,
    /// `[a-z0-9-]{1,32}`; özel kökün dizin adıdır.
    pub sunucu: String,
    /// Çalışma alanının bilgisi. Yalıtımlıda bağlamalar ve gizliler
    /// buradan gelir; tam erişimde yalnız `kok` kullanılır.
    pub alan: AlanBilgisi,
    /// Yalnız yalıtımlıda; `None` ağsızdır. Tam erişimde yok sayılmaz:
    /// verilirse başlatma reddedilir (yalıtımsız arka uç ağı kapatamaz).
    pub ag_listesi: Option<vekil::IzinListesi>,
}

/// Sunucu adı yol bileşeni olarak kullanılır; izin listesiyle sınırlıdır.
pub fn sunucu_adi_dogrula(ad: &str) -> Result<(), BaslatmaHatasi> {
    let gecerli = !ad.is_empty()
        && ad.len() <= 32
        && ad
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if gecerli {
        Ok(())
    } else {
        Err(BaslatmaHatasi::Yalitim(format!("geçersiz sunucu adı: {ad:?}")))
    }
}

/// Sunucunun çalışma alanına özel kökü (§3).
pub fn sunucu_ozel(yalitim_dizini: &Path, alan_kimligi: &str, sunucu: &str) -> OzelDizinler {
    OzelDizinler::yol(&yalitim_dizini.join(alan_kimligi).join("mcp"), sunucu)
}

const AYRILMIS_ONEKLER: &[&str] = &["EVREN_", "KALFA_"];

/// Sunucunun ortamı (§3): profil ve önbellek değişkenleri özel köke
/// yönelir; `EVREN_` ve `KALFA_` önekli adlar hiçbir koşulda geçmez.
pub fn sunucu_ortami(kaynak: &[(String, String)], ozel: &OzelDizinler) -> Vec<(String, String)> {
    let yol = |p: &Path| p.display().to_string();
    let mut yonlu: Vec<(&str, String)> = vec![
        ("HOME", yol(&ozel.ev)),
        ("USERPROFILE", yol(&ozel.ev)),
        ("APPDATA", yol(&ozel.ev.join("Roaming"))),
        ("LOCALAPPDATA", yol(&ozel.ev)),
        ("TMPDIR", yol(&ozel.gecici)),
        ("TEMP", yol(&ozel.gecici)),
        ("TMP", yol(&ozel.gecici)),
        ("NPM_CONFIG_CACHE", yol(&ozel.npm())),
        ("PIP_CACHE_DIR", yol(&ozel.pip())),
        ("UV_CACHE_DIR", yol(&ozel.onbellek.join("uv"))),
        ("XDG_CACHE_HOME", yol(&ozel.onbellek)),
    ];
    if cfg!(windows) {
        // G1: AppContainer başlatması bu değişkenler olmadan 203 ile düşer.
        let sistem = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
        let surucu = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        yonlu.push(("SystemRoot", sistem.clone()));
        yonlu.push(("windir", sistem));
        yonlu.push(("SystemDrive", surucu));
    }
    let ayni = |a: &str, b: &str| {
        if cfg!(windows) {
            a.eq_ignore_ascii_case(b)
        } else {
            a == b
        }
    };
    let mut ortam: Vec<(String, String)> = kaynak
        .iter()
        .filter(|(ad, _)| {
            let buyuk = ad.to_ascii_uppercase();
            !AYRILMIS_ONEKLER.iter().any(|o| buyuk.starts_with(o))
        })
        .filter(|(ad, _)| !yonlu.iter().any(|(y, _)| ayni(y, ad)))
        .cloned()
        .collect();
    ortam.extend(yonlu.into_iter().map(|(a, d)| (a.to_string(), d)));
    ortam
}

/// Seçilen kip ile yalıtımın durumu (§4.1). Yalıtımlı tarif hiçbir zaman
/// yalıtımsız arka uca düşmez; tam erişim her durumda yalıtımsız başlar.
pub fn mcp_kip_uyusmasi(kip: CalistirmaKipi, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi> {
    match (kip, yalitim_hazir) {
        (CalistirmaKipi::Yalitimli, false) => Err(BaslatmaHatasi::Yalitim(
            "sunucu yalıtımlı çalışmak üzere onaylandı ama yalıtım kullanılamıyor".into(),
        )),
        _ => Ok(()),
    }
}
```

`lib.rs`'e `pub mod etkilesimli;` ve şu satır eklenir:
`pub use etkilesimli::{CalistirmaKipi, EtkilesimliIstek, mcp_kip_uyusmasi, sunucu_ortami, sunucu_ozel};`
`Yalitici` trait'ine şunlar eklenir:

```rust
    /// MCP sunucusu gibi uzun yaşayan, borulu süreç (1.1a §4.1).
    fn baslat_etkilesimli(
        &self,
        _istek: &EtkilesimliIstek,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        Err(BaslatmaHatasi::Yalitim(
            "bu yalıtıcı etkileşimli süreç başlatamaz".into(),
        ))
    }

    /// Sunucunun özel kökünü ve Windows'ta ona verilen izinleri kaldırır
    /// (§3, §5.6).
    fn sunucu_kokunu_kaldir(&self, _alan_koku: &Path, _sunucu: &str) -> Result<(), String> {
        Err("bu yalıtıcı sunucu kökü kaldıramaz".into())
    }
```

- [ ] **Adım 4: Testlerin geçtiğini görün:**
  `cargo test -p kalfa-sandbox --locked --lib etkilesimli`. Beklenen: 4 test
  geçer. `EtkilesimliSurec` Görev 2'de tanımlanır. O tanıma kadar trait
  imzasını derlemek için Görev 1 ile Görev 2 aynı commit'te birleşebilir.
  Ayrı tutulacaksa Görev 2'nin ilk adımı önce yapılır.

- [ ] **Adım 5: Commit'leyin:** `sandbox: etkileşimli süreç sözleşmesi (kip, sunucu adı, özel kök, ortam)`.

---

### Görev 2: `EtkilesimliSurec` ve yalıtımsız arka uç (tam erişim)

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/surec.rs`, `src/yalitimsiz.rs`, `src/lib.rs`
- Test: `crates/kalfa-sandbox/tests/yalitimsiz.rs`

**Arayüzler:**
- Kullanır: Görev 1'in tipleri; `agac::{baslat, Agac}`; mevcut `Cocuk::ayir`.
- Üretir:
  - `pub struct EtkilesimliSurec`. Yöntemleri:
    - `girdi_al(&mut self) -> Option<Box<dyn AsyncWrite + Send + Unpin>>`
    - `cikti_al(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>>`
    - `hata_al(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>>`
    - `ag_kapat_ekle(&mut self, AgKapatici)`
    - `async bitisi_bekle(&mut self) -> Option<i32>`
    - `async kapat(self) -> KapanisSonucu`
  - `pub struct KapanisSonucu { pub kendiliginden: bool, pub cikis: Option<i32>, pub sure: Duration, pub ag: Option<vekil::AgOzeti> }`
  - `pub(crate) fn Surec::komutla_etkilesimli(komut: Command) -> Result<EtkilesimliSurec, BaslatmaHatasi>`
  - `pub(crate) fn tam_erisimle(istek: &EtkilesimliIstek, yalitim_dizini: &Path) -> Result<EtkilesimliSurec, BaslatmaHatasi>`.
    Üç arka uç da kullanır.

- [ ] **Adım 1: Düşen testleri yazın** (`tests/yalitimsiz.rs` sonuna). Bu
  testler tam erişimi her platformda yalıtımsız arka uçla sınar:

```rust
mod etkilesimli {
    use std::time::{Duration, Instant};

    use kalfa_sandbox::{
        AlanBilgisi, CalistirmaKipi, EtkilesimliIstek, Yalitici, Yalitimsiz, tarama,
    };
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    fn istek(kok: &std::path::Path, program: &str, arg: &[&str], kip: CalistirmaKipi) -> EtkilesimliIstek {
        EtkilesimliIstek {
            program: program.into(),
            argumanlar: arg.iter().map(|s| s.to_string()).collect(),
            ortam: kalfa_sandbox::ortam::uret(std::env::vars()),
            kip,
            sunucu: "deneme".into(),
            alan: AlanBilgisi {
                kok: kok.to_path_buf(),
                gizli: tarama::Tarama::default(),
                korumali: Vec::new(),
                git_dizinleri: Vec::new(),
            },
            ag_listesi: None,
        }
    }

    /// stdin satırlarını aynen geri yazan komut.
    fn yanki() -> (&'static str, Vec<&'static str>) {
        if cfg!(windows) {
            ("powershell", vec!["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
              "[Console]::InputEncoding=[Text.Encoding]::UTF8;[Console]::OutputEncoding=[Text.Encoding]::UTF8;while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine($s);[Console]::Out.Flush()}"])
        } else {
            ("cat", vec![])
        }
    }

    #[tokio::test]
    async fn tam_erisim_borulari_birebir_yankilar_ve_eofta_cikar() {
        let d = tempfile::tempdir().unwrap();
        let (p, a) = yanki();
        let y = Yalitimsiz::yeni(d.path().join("yalitim"));
        let mut s = y.baslat_etkilesimli(&istek(d.path(), p, &a, CalistirmaKipi::TamErisim)).unwrap();
        let mut girdi = s.girdi_al().unwrap();
        let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
        let mut gonderilen: Vec<String> = (0..1000)
            .map(|i| format!("{{\"jsonrpc\":\"2.0\",\"id\":{i},\"method\":\"ping\"}}"))
            .collect();
        gonderilen.push("x".repeat(1024 * 1024));
        gonderilen.push("{\"metin\":\"çğıöşü\"}".into());
        let yazici = tokio::spawn({
            let g = gonderilen.clone();
            async move {
                for s in g {
                    girdi.write_all(s.as_bytes()).await.unwrap();
                    girdi.write_all(b"\n").await.unwrap();
                }
                girdi.flush().await.unwrap();
                drop(girdi);
            }
        });
        let mut alinan = Vec::new();
        while let Some(s) = cikti.next_line().await.unwrap() {
            alinan.push(s.trim_end_matches('\r').to_string());
        }
        yazici.await.unwrap();
        assert_eq!(alinan, gonderilen);
        assert_eq!(s.bitisi_bekle().await, Some(0));
        let k = s.kapat().await;
        assert!(k.kendiliginden);
    }

    #[tokio::test]
    async fn kapat_stdini_okumayan_sureci_paydan_sonra_oldurur() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            ("powershell", vec!["-NoProfile", "-Command", "while($true){[Console]::Out.WriteLine('x'*1000)}"])
        } else {
            ("sh", vec!["-c", "trap '' TERM HUP; yes xxxxxxxxxx"])
        };
        let y = Yalitimsiz::yeni(d.path().join("yalitim"));
        let s = y.baslat_etkilesimli(&istek(d.path(), p, &a, CalistirmaKipi::TamErisim)).unwrap();
        // Çıktı okunmuyor: boru dolar, süreç yazmada bloke olur.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let bas = Instant::now();
        let k = s.kapat().await;
        assert!(!k.kendiliginden);
        assert!(bas.elapsed() < Duration::from_secs(8), "{:?}", bas.elapsed());
    }

    #[tokio::test]
    async fn yalitimsiz_arka_uc_yalitimli_istegi_reddeder() {
        let d = tempfile::tempdir().unwrap();
        let y = Yalitimsiz::yeni(d.path().join("yalitim"));
        let (p, a) = yanki();
        assert!(y.baslat_etkilesimli(&istek(d.path(), p, &a, CalistirmaKipi::Yalitimli)).is_err());
        assert!(!d.path().join("yalitim").exists(), "reddedilen istek dizin oluşturmamalı");
    }

    #[tokio::test]
    async fn gecersiz_sunucu_adi_dizin_olusturmaz() {
        let d = tempfile::tempdir().unwrap();
        let y = Yalitimsiz::yeni(d.path().join("yalitim"));
        let (p, a) = yanki();
        let mut i = istek(d.path(), p, &a, CalistirmaKipi::TamErisim);
        i.sunucu = "../kacak".into();
        assert!(y.baslat_etkilesimli(&i).is_err());
        assert!(!d.path().join("kacak").exists() && !d.path().join("yalitim").exists());
    }

    #[tokio::test]
    async fn birakilan_surecin_agaci_olur() {
        let d = tempfile::tempdir().unwrap();
        let isaret = d.path().join("torun.pid");
        let (p, a): (&str, Vec<String>) = if cfg!(windows) {
            ("powershell", vec!["-NoProfile".into(), "-Command".into(), format!(
                "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [IO.File]::WriteAllText('{}', [string]$c.Id); Start-Sleep 300",
                isaret.display()
            )])
        } else {
            ("sh", vec!["-c".into(), format!("sleep 300 & echo $! > '{}'; wait", isaret.display())])
        };
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        let y = Yalitimsiz::yeni(d.path().join("yalitim"));
        let s = y.baslat_etkilesimli(&istek(d.path(), p, &a, CalistirmaKipi::TamErisim)).unwrap();
        let son = Instant::now() + Duration::from_secs(15);
        while !isaret.exists() && Instant::now() < son {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let pid: u32 = std::fs::read_to_string(&isaret).unwrap().trim().parse().unwrap();
        drop(s); // kapat() çağrılmadan
        let son = Instant::now() + Duration::from_secs(8);
        while yasiyor(pid) && Instant::now() < son {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!yasiyor(pid), "bırakılan sürecin torunu yaşıyor: {pid}");
    }

    fn yasiyor(pid: u32) -> bool {
        if cfg!(windows) {
            std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", &format!("if (Get-Process -Id {pid} -ErrorAction SilentlyContinue) {{ 'AKTIF' }}")])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("AKTIF"))
                .unwrap_or(false)
        } else {
            std::path::Path::new(&format!("/proc/{pid}")).exists()
                && !std::fs::read_to_string(format!("/proc/{pid}/stat"))
                    .map(|s| s.contains(") Z "))
                    .unwrap_or(true)
        }
    }
}
```

`Yalitimsiz` bugün birim yapıdır (`#[derive(Default)] pub struct Yalitimsiz;`).
Etkileşimli sürecin özel kökü için `yalitim_dizini` alanı eklenir:
`pub struct Yalitimsiz { yalitim_dizini: Option<PathBuf> }`. Yeni
`pub fn yeni(yalitim_dizini: PathBuf) -> Self` eklenir; `Default`, `None`
ile kalır. Mevcut `Yalitimsiz` kullanımları (`platform_yaliticisi` ve
testler) `Yalitimsiz::default()` olarak güncellenir. `yalitim_dizini`
`None` ise `baslat_etkilesimli` hata döner.

- [ ] **Adım 2: Düştüğünü görün:** `cargo test -p kalfa-sandbox --locked --test yalitimsiz etkilesimli`.
  Beklenen: derleme hatası (`baslat_etkilesimli` varsayılanı hata döner,
  `Yalitimsiz::yeni` yok).

- [ ] **Adım 3: `surec.rs`'e ekleyin:**

```rust
type Yazici = Box<dyn tokio::io::AsyncWrite + Send + Unpin>;

/// Ağaç öldürüldükten sonra bitişin gelmesi için beklenen en uzun süre.
const OLUM_BEKLEMESI: Duration = Duration::from_secs(5);

/// Uzun yaşayan, borulu süreç (1.1a §4.1). Bırakılırsa ağaç ölür: Linux'ta
/// `Agac`'ın `Drop`'u, Windows'ta işin son tutamacının kapanması.
pub struct EtkilesimliSurec {
    girdi: Option<Yazici>,
    cikti: Option<Okuyucu>,
    hata: Option<Okuyucu>,
    bitisi: Option<Bitisi>,
    cikis: Option<Option<i32>>,
    agac: Agac,
    ag_kapat: Option<AgKapatici>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapanisSonucu {
    /// Süreç, stdin kapandıktan sonra pay içinde kendiliğinden çıktı.
    pub kendiliginden: bool,
    pub cikis: Option<i32>,
    pub sure: Duration,
    pub ag: Option<crate::vekil::AgOzeti>,
}

impl EtkilesimliSurec {
    fn yeni(girdi: Option<Yazici>, cocuk: Cocuk, agac: Agac) -> Self {
        let (cikti, hata, bitisi) = cocuk.ayir();
        Self {
            girdi,
            cikti,
            hata,
            bitisi: Some(bitisi),
            cikis: None,
            agac,
            ag_kapat: None,
        }
    }

    pub fn girdi_al(&mut self) -> Option<Box<dyn tokio::io::AsyncWrite + Send + Unpin>> {
        self.girdi.take()
    }

    pub fn cikti_al(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>> {
        self.cikti.take()
    }

    pub fn hata_al(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>> {
        self.hata.take()
    }

    pub fn ag_kapat_ekle(&mut self, kapat: AgKapatici) {
        self.ag_kapat = Some(kapat);
    }

    /// Süreç kendiliğinden bitene kadar bekler. İptal güvenlidir: gelecek
    /// bırakılırsa sonraki çağrı aynı bitişi beklemeyi sürdürür.
    pub async fn bitisi_bekle(&mut self) -> Option<i32> {
        if let Some(kod) = self.cikis {
            return kod;
        }
        let kod = match self.bitisi.as_mut() {
            Some(b) => b.await,
            None => None,
        };
        self.bitisi = None;
        self.cikis = Some(kod);
        kod
    }

    /// §4.1: stdin kapanır, `KAPANMA_PAYI` kadar kendiliğinden çıkış
    /// beklenir, ağaç her durumda öldürülür. Döndüğünde ağaçta yaşayan süreç
    /// yoktur.
    pub async fn kapat(mut self) -> KapanisSonucu {
        let bas = std::time::Instant::now();
        drop(self.girdi.take());
        let kendiliginden = self.cikis.is_some()
            || tokio::time::timeout(crate::etkilesimli::KAPANMA_PAYI, self.bitisi_bekle())
                .await
                .is_ok();
        self.agac.oldur();
        let cikis = if kendiliginden {
            self.cikis.flatten()
        } else {
            tokio::time::timeout(OLUM_BEKLEMESI, self.bitisi_bekle())
                .await
                .ok()
                .flatten()
        };
        let ag = match self.ag_kapat.take() {
            Some(k) => Some(k().await),
            None => None,
        };
        KapanisSonucu {
            kendiliginden,
            cikis,
            sure: bas.elapsed(),
            ag,
        }
    }
}

impl Surec {
    /// Kurulmuş bir komutu (örneğin bwrap) borulu stdin'le ağacın kökü olarak
    /// başlatır.
    pub(crate) fn komutla_etkilesimli(
        mut komut: Command,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        komut
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let (mut cocuk, agac) = agac::baslat(&mut komut)?;
        let girdi = cocuk.stdin.take().map(|g| Box::new(g) as Yazici);
        Ok(EtkilesimliSurec::yeni(girdi, Cocuk::Tokio(Box::new(cocuk)), agac))
    }
}

/// Tam erişimle, yalıtımsız başlatma (§4.1). Üç arka uç da kullanır.
pub(crate) fn tam_erisimle(
    istek: &crate::EtkilesimliIstek,
    yalitim_dizini: &std::path::Path,
) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
    crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
    if istek.ag_listesi.is_some() {
        return Err(BaslatmaHatasi::Yalitim(
            "tam erişimli sunucuda ağ listesi uygulanamaz".into(),
        ));
    }
    let ozel = crate::etkilesimli::sunucu_ozel(
        yalitim_dizini,
        &crate::profil::alan_kimligi(&istek.alan.kok),
        &istek.sunucu,
    );
    ozel.hazirla(&crate::profil::GitKimligi::oku())?;
    let ortam = crate::etkilesimli::sunucu_ortami(&istek.ortam, &ozel);
    let (yol, argumanlar) = program::coz(
        &crate::KomutBicimi::Program {
            program: istek.program.clone(),
            argumanlar: istek.argumanlar.clone(),
        },
        &ortam,
    )?;
    let mut komut = Command::new(yol);
    komut
        .args(argumanlar)
        .env_clear()
        .envs(ortam.iter().map(|(a, d)| (a, d)))
        .current_dir(&ozel.ev);
    Surec::komutla_etkilesimli(komut)
}
```

`ozel.hazirla` `io::Result` döner; `BaslatmaHatasi`'nın `From<io::Error>`'u
(`GirisCikis`) vardır. `yalitimsiz.rs`'e şunlar eklenir:

```rust
    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, false)?;
        let dizin = self.yalitim_dizini.as_deref().ok_or_else(|| {
            BaslatmaHatasi::Yalitim("yalıtım dizini ayarlanmamış".into())
        })?;
        crate::surec::tam_erisimle(istek, dizin)
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let dizin = self.yalitim_dizini.as_deref().ok_or("yalıtım dizini ayarlanmamış")?;
        let kok = crate::sunucu_ozel(dizin, &crate::profil::alan_kimligi(alan_koku), sunucu).kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
```

`lib.rs`'e `pub use surec::{EtkilesimliSurec, KapanisSonucu};` eklenir.

- [ ] **Adım 4: Geçtiğini görün:** İki platformda
  `cargo test -p kalfa-sandbox --locked --test yalitimsiz etkilesimli`. Beklenen: 5 test
  geçer. `kapat_stdini_okumayan…` testinde süre ≤ 2 sn + öldürme olmalı.
- [ ] **Adım 5: Commit'leyin:** `sandbox: EtkilesimliSurec ve tam erişimle başlatma`.

---

### Görev 3: Linux — yalıtımlı etkileşimli süreç

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/linux/mod.rs`
- Test: `crates/kalfa-sandbox/tests/yalitim.rs` (Linux bölümü)

**Arayüzler:**
- Kullanır: Görev 1–2; mevcut `komut_kur`, `yalitim_ortami`, `bwrap::secenekler`.
- Üretir: `LinuxYalitici` için `baslat_etkilesimli` ve `sunucu_kokunu_kaldir`.

- [ ] **Adım 1: `komut_kur`'u özel kökü dışarıdan alacak biçime getirin.** İmza:

```rust
struct KurIstegi<'a> {
    komut: &'a KomutBicimi,
    dizin: &'a Path,
    ortam: &'a [(String, String)],
    ag: bool,
    alan: &'a AlanBilgisi,
    ozel: &'a OzelDizinler,
}

fn komut_kur(
    bwrap: &Path,
    ayar: &LinuxAyari,
    k: &KurIstegi,
) -> Result<(std::process::Command, Vec<OwnedFd>, Option<OwnedFd>), BaslatmaHatasi>
```

  Gövdedeki `istek.alan`, `istek.ag`, `istek.komut`, `istek.dizin` ve
  `istek.ortam` kullanımları `k.` alanlarına geçer. İçeride kurulan
  `let ozel = OzelDizinler::yol(...)` satırı silinir; yerine `k.ozel`
  kullanılır. `ozel.hazirla` çağrısı çağırana taşınır. Mevcut `yalitimli`
  şöyle kurar:

```rust
        let alan = istek.alan.as_ref().ok_or_else(|| BaslatmaHatasi::Yalitim("çalışma alanı bilgisi yok".into()))?;
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&alan.kok));
        ozel.hazirla(&ayar.git)?;
        let (komut, fdler, vekil_kanali) = komut_kur(bwrap, &ayar, &KurIstegi {
            komut: &istek.komut, dizin: &istek.dizin, ortam: &istek.ortam,
            ag: istek.ag, alan, ozel: &ozel,
        })?;
```

  `yalitimli`'deki vekil kurulumu bloğu (`if istek.ag { … }`) değişmeden
  `fn vekili_bagla(&self, ayar: &LinuxAyari, vekil_kanali: Option<OwnedFd>, liste: Option<IzinListesi>) -> Result<Option<AgKapatici>, BaslatmaHatasi>`
  işlevine taşınır. `yalitimli`, dönen kapatıcıyı `surec.ag_kapat_ekle`'ye
  verir.

- [ ] **Adım 2: Mevcut kabuk testleri yeşil kalmalı:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim`
  (`kalfatest`). Bu adım yalnız yeniden düzenlemedir; davranış değişmez.

- [ ] **Adım 3: Düşen kabul testlerini yazın** (`tests/yalitim.rs`, Linux bloğunun sonuna):

```rust
fn etkilesimli_istek(k: &Kurgu, program: &str, arg: &[&str], sunucu: &str) -> kalfa_sandbox::EtkilesimliIstek {
    kalfa_sandbox::EtkilesimliIstek {
        program: program.into(),
        argumanlar: arg.iter().map(|s| s.to_string()).collect(),
        ortam: vec![
            ("PATH".into(), format!("{}:/usr/bin:/bin", k.ev.join(".cargo/bin").display())),
            ("LANG".into(), "C.UTF-8".into()),
        ],
        kip: kalfa_sandbox::CalistirmaKipi::Yalitimli,
        sunucu: sunucu.into(),
        alan: AlanBilgisi {
            kok: k.kok.clone(),
            gizli: tarama::tara(&k.kok, &gizli_mi),
            korumali: korumali(&k.kok),
            git_dizinleri: Vec::new(),
        },
        ag_listesi: None,
    }
}

/// G1.1: 1 000 JSON-RPC satırı, 1 MiB'lık satır ve UTF-8 birebir; EOF'ta çıkış.
#[tokio::test]
async fn etkilesimli_borular_birebir_yankilanir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "cat", &[], "yanki")).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let mut gonderilen: Vec<String> = (0..1000)
        .map(|i| format!("{{\"jsonrpc\":\"2.0\",\"id\":{i},\"method\":\"ping\"}}"))
        .collect();
    gonderilen.push("x".repeat(1024 * 1024));
    gonderilen.push("{\"metin\":\"çğıöşü\"}".into());
    let g = gonderilen.clone();
    let yazici = tokio::spawn(async move {
        for s in g {
            girdi.write_all(s.as_bytes()).await.unwrap();
            girdi.write_all(b"\n").await.unwrap();
        }
        girdi.flush().await.unwrap();
    });
    let mut alinan = Vec::new();
    while let Some(s) = cikti.next_line().await.unwrap() {
        alinan.push(s);
    }
    yazici.await.unwrap();
    assert_eq!(alinan, gonderilen);
    assert_eq!(s.bitisi_bekle().await, Some(0));
    assert!(s.kapat().await.kendiliginden);
}

/// G1.2: torunlu süreç yaşar, ping'e yanıt verir, EOF'u yok sayar;
/// `kapat()` ağacı 2 sn pay + öldürmeyle durdurur.
#[tokio::test]
async fn etkilesimli_uzun_yasar_kapat_agaci_durdurur() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let isaret = format!("kalfa-g1-{}", std::process::id());
    let betik = format!(
        "perl -e 'sleep 600' {isaret} & trap '' TERM HUP; echo hazir; \
         while read l; do echo \"pong $l\"; done; exec perl -e '$SIG{{TERM}}=q(IGNORE); sleep 600' {isaret}"
    );
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", &betik], "uzun")).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    assert_eq!(cikti.next_line().await.unwrap().as_deref(), Some("hazir"));
    for i in 1..=5 {
        girdi.write_all(format!("{i}\n").as_bytes()).await.unwrap();
        girdi.flush().await.unwrap();
        assert_eq!(cikti.next_line().await.unwrap(), Some(format!("pong {i}")));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    drop(girdi);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(surecler(&isaret).len(), 2, "çocuk ve torun yaşamalı");
    let bas = Instant::now();
    let kapanis = s.kapat().await;
    assert!(!kapanis.kendiliginden, "EOF'u yok sayan süreç kendiliğinden çıkmamalı");
    assert!(bas.elapsed() < Duration::from_secs(5));
    assert!(kalmadi_mi(&isaret, Duration::from_secs(2)), "kapat'tan sonra süreç kaldı");
}

/// §3: sunucunun özel kökü kabuğa görünmez; kabuğun özel ev'i sunucuya
/// görünmez; sunucu kendi köküne yazar. Aynı alanda iki sunucu birbirini
/// göremez (Review Focus 4).
#[tokio::test]
async fn sunucu_ozel_koku_kabuktan_ve_obur_sunucudan_ayri() {
    let k = kurgu();
    let kimlik = kalfa_sandbox::alan_kimligi(&k.kok);
    let ydir = k.taban.join("veri/yalitim");
    let a = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "a");
    let b = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "b");
    let kabuk_ev = kalfa_sandbox::OzelDizinler::yol(&ydir, &kimlik).ev;
    // B'nin kökünü önce kurmak için B'yi bir kez başlatıp kapatın.
    let s = k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "true", &[], "b")).unwrap();
    s.kapat().await;
    let betik = format!(
        "touch \"$HOME/yazildi\" && echo kendi=TAMAM; \
         test -e '{}' && echo kabuk=GORUNUR || echo kabuk=YOK; \
         test -e '{}' && echo obur=GORUNUR || echo obur=YOK",
        kabuk_ev.display(), b.kok.display()
    );
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", &betik], "a")).unwrap();
    let mut metin = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut metin).await.unwrap();
    s.kapat().await;
    assert!(metin.contains("kendi=TAMAM"), "{metin}");
    assert!(metin.contains("kabuk=YOK"), "{metin}");
    assert!(metin.contains("obur=YOK"), "{metin}");
    assert!(a.ev.join("yazildi").exists());
    // Kabuk, sunucunun köküne ulaşamaz.
    let sonuc = k
        .betik(&k.kok, &format!("test -e '{}' && echo GORUNUR || echo YOK", a.kok.display()), false)
        .await;
    assert!(sonuc.standart.contains("YOK"), "{}", sonuc.standart);
    // Kaldırma kökü siler.
    k.y.sunucu_kokunu_kaldir(&k.kok, "a").unwrap();
    assert!(!a.kok.exists());
}

/// G1 bulgusu ve §3: ortam özel köke yönelir, ayrılmış adlar geçmez.
#[tokio::test]
async fn etkilesimli_ortam_ozel_koke_yonelir() {
    let k = kurgu();
    let mut i = etkilesimli_istek(&k, "env", &[], "ortam");
    i.ortam.push(("EVREN_API_KEY".into(), "evren_llm_x".into()));
    i.ortam.push(("GITHUB_TOKEN".into(), "ghp_deneme".into()));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let mut metin = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut metin).await.unwrap();
    s.kapat().await;
    let ozel = kalfa_sandbox::sunucu_ozel(
        &k.taban.join("veri/yalitim"), &kalfa_sandbox::alan_kimligi(&k.kok), "ortam");
    assert_eq!(deger(&metin, "HOME"), ozel.ev.display().to_string());
    assert_eq!(deger(&metin, "NPM_CONFIG_CACHE"), ozel.npm().display().to_string());
    assert_eq!(deger(&metin, "UV_CACHE_DIR"), ozel.onbellek.join("uv").display().to_string());
    assert!(!metin.contains("EVREN_API_KEY"), "{metin}");
    assert_eq!(deger(&metin, "GITHUB_TOKEN"), "ghp_deneme");
}

/// §4.1: tam erişim, yalıtım hazırken yalıtımsız başlar (ev'deki sırrı
/// okuyabilir); yalıtımlı aynı sırrı okuyamaz.
#[tokio::test]
async fn tam_erisim_yalitim_hazirken_yalitimsiz_baslar() {
    let k = kurgu();
    let sir = k.ev.join(".ssh/id_rsa");
    let oku = |kip| {
        let mut i = etkilesimli_istek(&k, "cat", &[sir.to_str().unwrap()], "kip");
        i.kip = kip;
        i
    };
    let mut tam = k.y.baslat_etkilesimli(&oku(kalfa_sandbox::CalistirmaKipi::TamErisim)).unwrap();
    let mut metin = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut tam.cikti_al().unwrap(), &mut metin).await.unwrap();
    tam.kapat().await;
    assert!(metin.contains("GIZLI-SSH"));
    let mut yal = k.y.baslat_etkilesimli(&oku(kalfa_sandbox::CalistirmaKipi::Yalitimli)).unwrap();
    let mut metin = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut yal.cikti_al().unwrap(), &mut metin).await.unwrap();
    yal.kapat().await;
    assert!(!metin.contains("GIZLI-SSH"));
}
```

  `k.betik(kok, metin, ag)`, `gizli_mi`, `korumali` ve `deger` mevcut Linux
  test yardımcılarıdır.

- [ ] **Adım 4: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim etkilesimli sunucu tam_erisim`.
  Beklenen: `baslat_etkilesimli` varsayılanı "bu yalıtıcı etkileşimli
  süreç başlatamaz" döner.

- [ ] **Adım 5: Uygulayın** (`linux/mod.rs`, `impl Yalitici for LinuxYalitici`):

```rust
    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        let ayar = self.ayar();
        if istek.kip == crate::CalistirmaKipi::TamErisim {
            return crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini);
        }
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let ozel = crate::sunucu_ozel(
            &ayar.yalitim_dizini,
            &alan_kimligi(&istek.alan.kok),
            &istek.sunucu,
        );
        ozel.hazirla(&ayar.git)?;
        korumalilari_hazirla(&istek.alan.korumali)?;
        let ortam = crate::sunucu_ortami(&istek.ortam, &ozel);
        let komut = KomutBicimi::Program {
            program: istek.program.clone(),
            argumanlar: istek.argumanlar.clone(),
        };
        let ag = istek.ag_listesi.is_some();
        let (komut, fdler, vekil_kanali) = komut_kur(
            bwrap,
            &ayar,
            &KurIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                ag,
                alan: &istek.alan,
                ozel: &ozel,
            },
        )?;
        let mut surec = Surec::komutla_etkilesimli(komut.into())?;
        drop(fdler);
        if let Some(kapat) = self.vekili_bagla(&ayar, vekil_kanali, istek.ag_listesi.clone())? {
            surec.ag_kapat_ekle(kapat);
        }
        Ok(surec)
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kok = crate::sunucu_ozel(&self.ayar().yalitim_dizini, &alan_kimligi(alan_koku), sunucu).kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
```

  `komut_kur`'un kurduğu bwrap; `k.ozel.ev`, `k.ozel.onbellek` ve `/tmp`'ye
  `k.ozel.gecici`'yi bağlar. Burada `k.ozel` sunucunun köküdür, dolayısıyla
  kabuğun özel dizinleri bağlanmaz. Proje kökü (`alan.kok`) kabuktaki gibi
  yazılabilir bağlanır. `--chdir`, `k.dizin` yani sunucunun `ev`'idir.

- [ ] **Adım 6: Geçtiğini görün** (WSL Ubuntu 24.04, `kalfatest`):
  `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim`.
  Beklenen: yeni 5 test ve bütün eski testler geçer.
- [ ] **Adım 7: Commit'leyin:** `sandbox: Linux'ta yalıtımlı etkileşimli süreç ve sunucu özel kökü`.

---

### Görev 4: Windows — borulu stdin, yetenek SID'i, ACE kaydı

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/windows/kap.rs`, `src/windows/yalitici.rs`, `src/surec.rs`
- Test: `crates/kalfa-sandbox/tests/yalitim/windows.rs`

**Arayüzler:**
- Kullanır: Görev 1–2; mevcut `guvenlik::{yetenek_sidi, profil_olustur, arac_zinciri_sidi}`,
  `izin_kaydet_ve_ver`, `kayda_gore_geri_al`, `AceKaydi::alani_cikar`.
- Üretir:
  - `kap::Kap { …, girdi: Option<File> }`
  - `pub(crate) fn kap::baslat_etkilesimli(&KapIstegi, &Is) -> io::Result<Kap>`
  - `pub(crate) fn EtkilesimliSurec::kaptan(Kap, Is) -> EtkilesimliSurec`
  - `fn sunucu_kaydi(alan: &str, sunucu: &str) -> String`, değeri `"{alan}/mcp/{sunucu}"`
  - `fn sunucu_yetenek_adi(alan: &str, sunucu: &str) -> String`, değeri `"kalfa-mcp-{alan}-{sunucu}"`
  - `WindowsYalitici` için `baslat_etkilesimli` ve `sunucu_kokunu_kaldir`

- [ ] **Adım 1: `kap.rs`'e borulu stdin.** `Kap`'a alan ekleyin:

```rust
pub(crate) struct Kap {
    pub surec: OwnedHandle,
    pub cikti: File,
    pub hata: File,
    /// Etkileşimli süreçte stdin'in yazma ucu; kabukta `None` (stdin `NUL`).
    pub girdi: Option<File>,
}
```

  `fn baslat(istek, is)` gövdesini `fn baslat_ortak(istek, is, borulu: bool)`
  olarak yeniden adlandırın. `baslat`, `baslat_ortak(istek, is, false)`;
  `baslat_etkilesimli`, `baslat_ortak(istek, is, true)` olur. Gövdede
  `let girdi = nul()?;` satırını şununla değiştirin:

```rust
    let (girdi, girdi_yazma) = if borulu {
        let (okuma, yazma) = girdi_borusu()?;
        (okuma, Some(yazma))
    } else {
        (nul()?, None)
    };
```

  Sondaki `Kap { … }`'a `girdi: girdi_yazma.map(File::from)` ekleyin. Yeni
  yardımcı, mevcut `boru()`'nun aynasıdır: okuma ucu çocuğa geçer, yazma
  ucu Kalfa'da kalır.

```rust
/// stdin borusu: okuma ucu çocuğa geçer (tutamaç listesinde), yazma ucu
/// Kalfa'da kalır ve miras alınmaz.
fn girdi_borusu() -> io::Result<(OwnedHandle, OwnedHandle)> {
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
            return Err(son_hata("CreatePipe(girdi)"));
        }
    }
    let (okuma, yazma) = (sahiplen(okuma), sahiplen(yazma));
    // SAFETY: tutamaç geçerlidir.
    if unsafe { SetHandleInformation(yazma.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) } == 0 {
        return Err(son_hata("SetHandleInformation(girdi)"));
    }
    Ok((okuma, yazma))
}
```

  Kabuğun `kap::baslat` çağrıları değişmez; `Cocuk::Kap` kolu `girdi`'yi
  kullanmaz.

- [ ] **Adım 2: `surec.rs`'e Windows yapıcısını ekleyin:**

```rust
#[cfg(windows)]
impl EtkilesimliSurec {
    pub(crate) fn kaptan(mut kap: crate::windows::kap::Kap, is: agac::Is) -> Self {
        let girdi = kap
            .girdi
            .take()
            .map(|g| Box::new(tokio::fs::File::from_std(g)) as Yazici);
        Self::yeni(girdi, Cocuk::Kap(kap), Agac::isten(is))
    }
}
```

- [ ] **Adım 3: Düşen kabul testlerini yazın** (`tests/yalitim/windows.rs` sonuna):

```rust
fn etkilesimli_istek(alan: &AlanBilgisi, sunucu: &str, betik: &str) -> kalfa_sandbox::EtkilesimliIstek {
    kalfa_sandbox::EtkilesimliIstek {
        program: powershell_yolu().display().to_string(),
        argumanlar: vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(),
                         "-Command".into(), betik.into()],
        ortam: kalfa_sandbox::ortam::uret(std::env::vars()),
        kip: kalfa_sandbox::CalistirmaKipi::Yalitimli,
        sunucu: sunucu.into(),
        alan: alan.clone(),
        ag_listesi: None,
    }
}

async fn tum_cikti(s: &mut kalfa_sandbox::EtkilesimliSurec) -> String {
    let mut m = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut m).await.unwrap();
    m
}

const YANKI: &str = "[Console]::InputEncoding=[Text.Encoding]::UTF8;[Console]::OutputEncoding=[Text.Encoding]::UTF8;while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine($s);[Console]::Out.Flush()}";

/// G1.1 (ürün başlatıcısıyla).
#[tokio::test]
async fn etkilesimli_borular_birebir_yankilanir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "yanki", YANKI)).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let mut gonderilen: Vec<String> = (0..1000)
        .map(|i| format!("{{\"jsonrpc\":\"2.0\",\"id\":{i},\"method\":\"ping\"}}"))
        .collect();
    gonderilen.push("x".repeat(1024 * 1024));
    gonderilen.push("{\"metin\":\"çğıöşü\"}".into());
    let g = gonderilen.clone();
    let yazici = tokio::spawn(async move {
        for s in g {
            girdi.write_all(s.as_bytes()).await.unwrap();
            girdi.write_all(b"\n").await.unwrap();
        }
        girdi.flush().await.unwrap();
    });
    let mut alinan = Vec::new();
    while let Some(s) = cikti.next_line().await.unwrap() {
        alinan.push(s.trim_end_matches('\r').to_string());
    }
    yazici.await.unwrap();
    assert_eq!(alinan, gonderilen);
    assert_eq!(s.bitisi_bekle().await, Some(0));
    assert!(s.kapat().await.kendiliginden);
    k.temizle(&[alan]);
}

/// G1.2 (ürün başlatıcısıyla): torun yaşar, EOF yok sayılır, kapat ağacı
/// durdurur.
#[tokio::test]
async fn etkilesimli_uzun_yasar_kapat_agaci_durdurur() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let betik = "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','600','/nobreak') -PassThru -WindowStyle Hidden; \
                 [Console]::Out.WriteLine(\"pid $PID $($c.Id)\"); [Console]::Out.Flush(); \
                 while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine(\"pong $s\");[Console]::Out.Flush()}; Start-Sleep 600";
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "uzun", betik)).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let ilk = cikti.next_line().await.unwrap().unwrap();
    let pidler: Vec<u32> = ilk.split_whitespace().filter_map(|p| p.trim().parse().ok()).collect();
    assert_eq!(pidler.len(), 2, "{ilk}");
    for i in 1..=5 {
        girdi.write_all(format!("{i}\n").as_bytes()).await.unwrap();
        girdi.flush().await.unwrap();
        assert_eq!(cikti.next_line().await.unwrap().map(|s| s.trim_end().to_string()), Some(format!("pong {i}")));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    drop(girdi);
    assert!(pidler.iter().all(|p| surec_aktif_mi(*p)));
    let bas = std::time::Instant::now();
    let kapanis = s.kapat().await;
    assert!(!kapanis.kendiliginden);
    assert!(bas.elapsed() < Duration::from_secs(5));
    for p in &pidler {
        assert!(!surec_aktif_mi(*p), "kapat'tan sonra süreç yaşıyor: {p}");
    }
    k.temizle(&[alan]);
}

/// G1.3 (ürün başlatıcısıyla) ve Review Focus 4, 5.
#[tokio::test]
async fn sunucu_ozel_koku_yetenek_sidiyle_ayrilir_ve_izinler_geri_alinir() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let baska = k.hazirla(&k.baska_kok, Vec::new());
    let kimlik = alan_kimligi(&k.kok);
    let a = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "a");
    let b = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "b");
    let dene = |hedef: &Path| format!(
        "try {{ Set-Content -LiteralPath (Join-Path '{0}' 'g1.txt') 'g1' -ErrorAction Stop; 'yaz=TAMAM' }} catch {{ 'yaz=RED' }}; \
         try {{ Get-ChildItem -LiteralPath '{0}' -ErrorAction Stop | Out-Null; 'oku=TAMAM' }} catch {{ 'oku=RED' }}",
        hedef.display());
    // B'nin kökü kurulsun.
    let s = k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "b", "exit 0")).unwrap();
    s.kapat().await;
    // A kendi köküne yazar, B'nin köküne yazamaz.
    let mut s = k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "a", &format!("{}; {}", dene(&a.ev), dene(&b.ev)))).unwrap();
    let m = tum_cikti(&mut s).await;
    s.kapat().await;
    let satir: Vec<&str> = m.lines().map(str::trim).collect();
    assert_eq!(satir, ["yaz=TAMAM", "oku=TAMAM", "yaz=RED", "oku=RED"], "{m}");
    // Kabuk (aynı paket, yeteneksiz) sunucunun köküne yazamaz ve okuyamaz.
    let sonuc = k.calistir(&alan, false, &dene(&a.ev)).await;
    assert!(sonuc.standart.contains("yaz=RED") && sonuc.standart.contains("oku=RED"), "{}", sonuc.standart);
    // Başka alanın kabuğu da.
    let sonuc = k.calistir(&baska, false, &dene(&a.ev)).await;
    assert!(sonuc.standart.contains("yaz=RED") && sonuc.standart.contains("oku=RED"), "{}", sonuc.standart);
    // Sunucu kaldırılınca yetenek SID'inin ACE'leri ve kayıt girdileri gider.
    let yetenek = guvenlik::yetenek_sidi(&format!("kalfa-mcp-{kimlik}-a")).unwrap();
    let mcp = a.kok.parent().unwrap().to_path_buf();
    k.y.sunucu_kokunu_kaldir(&k.kok, "a").unwrap();
    assert!(!a.kok.exists());
    assert!(guvenlik::aceler_oku(&mcp).unwrap().iter().all(|ace| ace.sid.as_ref() != Some(&yetenek)));
    // Alan kaldırılınca B'nin ACE'leri de geri alınır (Review Focus 5).
    let yetenek_b = guvenlik::yetenek_sidi(&format!("kalfa-mcp-{kimlik}-b")).unwrap();
    k.y.alani_kaldir(&alan).unwrap();
    if mcp.exists() {
        assert!(guvenlik::aceler_oku(&mcp).unwrap().iter().all(|ace| ace.sid.as_ref() != Some(&yetenek_b)));
    }
    k.temizle(&[baska]);
}

/// G1 bulgusu: ortam özel köke ve sistem değerlerine yönelir.
#[tokio::test]
async fn etkilesimli_ortam_ozel_koke_ve_sisteme_yonelir() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    let mut i = etkilesimli_istek(&alan, "ortam",
        "foreach($a in 'USERPROFILE','LOCALAPPDATA','HOME','NPM_CONFIG_CACHE','UV_CACHE_DIR','GITHUB_TOKEN','windir','SystemDrive','SystemRoot','EVREN_API_KEY'){ \"$a=$([Environment]::GetEnvironmentVariable($a))\" }");
    i.ortam.push(("EVREN_API_KEY".into(), "evren_llm_x".into()));
    i.ortam.push(("GITHUB_TOKEN".into(), "ghp_deneme".into()));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let m = tum_cikti(&mut s).await;
    s.kapat().await;
    let ozel = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &alan_kimligi(&k.kok), "ortam");
    let es = |a: &str, b: &Path| deger(&m, a).trim().eq_ignore_ascii_case(&b.display().to_string());
    assert!(es("USERPROFILE", &ozel.ev) && es("HOME", &ozel.ev), "{m}");
    assert!(es("NPM_CONFIG_CACHE", &ozel.npm()), "{m}");
    assert!(es("windir", &windows_koku()) && es("SystemRoot", &windows_koku()), "{m}");
    assert!(!deger(&m, "SystemDrive").trim().is_empty(), "{m}");
    assert!(deger(&m, "EVREN_API_KEY").trim().is_empty(), "{m}");
    assert_eq!(deger(&m, "GITHUB_TOKEN").trim(), "ghp_deneme", "{m}");
    assert!(es("UV_CACHE_DIR", &ozel.onbellek.join("uv")), "{m}");
    k.temizle(&[alan]);
}

/// §4.1: tam erişim, yalıtım hazırken yalıtımsız başlar.
#[tokio::test]
async fn tam_erisim_yalitim_hazirken_yalitimsiz_baslar() {
    let k = kurgu();
    let alan = k.hazirla(&k.kok, Vec::new());
    std::fs::write(k.ev.join(".ssh").join("id_rsa"), "GIZLI-SSH").unwrap();
    let betik = format!("Get-Content -LiteralPath '{}' -ErrorAction SilentlyContinue", k.ev.join(".ssh").join("id_rsa").display());
    let mut i = etkilesimli_istek(&alan, "kip", &betik);
    i.kip = kalfa_sandbox::CalistirmaKipi::TamErisim;
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    assert!(tum_cikti(&mut s).await.contains("GIZLI-SSH"));
    s.kapat().await;
    i.kip = kalfa_sandbox::CalistirmaKipi::Yalitimli;
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    assert!(!tum_cikti(&mut s).await.contains("GIZLI-SSH"));
    s.kapat().await;
    k.temizle(&[alan]);
}
```

  `k.calistir(&alan, ag, metin)` mevcut kabuk yardımcısıdır.
  `k.temizle(&alanlar)` ise alanı kaldırıp profil ve ACE'yi geri alır.

- [ ] **Adım 4: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim etkilesimli sunucu tam_erisim -- --test-threads=1`.

- [ ] **Adım 5: Sunucu kökünün hazırlığı ve ACE kaydı** (`yalitici.rs`):

```rust
fn sunucu_kaydi(alan: &str, sunucu: &str) -> String {
    format!("{alan}/mcp/{sunucu}")
}

fn sunucu_yetenek_adi(alan: &str, sunucu: &str) -> String {
    format!("kalfa-mcp-{alan}-{sunucu}")
}

/// Sunucunun özel kökü (§3, §5.6): paket SID'ine hak verilmez; `mcp` ve
/// sunucu kökünde yalnız listeleme, `ev`/`gecici`/`onbellek`'te miraslı
/// değiştirme yalnız sunucunun yetenek SID'ine verilir. ACE'ler sunucu
/// kaydına yazılır.
fn sunucu_kokunu_hazirla(
    ace: &mut AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    profil_adi: &str,
    yetenek: &Sid,
    git: &GitKimligi,
) -> io::Result<OzelDizinler> {
    let kayit = sunucu_kaydi(alan, sunucu);
    let alan_ozel = OzelDizinler::yol(yalitim_dizini, alan);
    let mcp = profil::guvenli_dizin(&alan_ozel.kok, Path::new("mcp"))?;
    let ozel = crate::sunucu_ozel(yalitim_dizini, alan, sunucu);
    ozel.hazirla(git)?;
    for yol in [&mcp, &ozel.kok] {
        izin_kaydet_ve_ver(
            ace,
            &kayit,
            yol,
            &Izin { sid: yetenek.clone(), maske: MASKE_LISTELE, miras: MIRAS_YOK },
        )?;
    }
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        izin_kaydet_ve_ver(
            ace,
            &kayit,
            yol,
            &Izin { sid: yetenek.clone(), maske: MASKE_DEGISTIR, miras: MIRAS },
        )?;
    }
    // Windows kabın TEMP'ini `LOCALAPPDATA\Packages\<paket>\AC\Temp`'e çevirir.
    profil::guvenli_dizin(
        &ozel.kok,
        Path::new("ev/Packages").join(profil_adi).join("AC").join("Temp").as_path(),
    )?;
    profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming"))?;
    Ok(ozel)
}
```

- [ ] **Adım 6: Windows başlatmasını uygulayın.** Ağ köprüsü kopyası,
  `yalitilmis_baslat`'taki `if istek.ag { … }` bloğundan
  `fn kopruyu_hazirla(&self, ace: &mut AceKaydi, kimlik: &str, alan_ozel: &OzelDizinler) -> Result<PathBuf, BaslatmaHatasi>`
  işlevine taşınır. Kopya kabuğun `ozel.kok/kopru`'sunda kalır; paket SID'i
  orayı okur, sunucu da aynı paketi taşır. Vekil ve köprü bağlama bloğu da
  aynı biçimde `fn ag_baglantisi(...) -> Result<(Vec<(String, String)>, Option<AgKapatici>), BaslatmaHatasi>`'na taşınır.
  Böylece `yalitilmis_baslat` ile aynı kod çağrılır.

```rust
    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        let ayar = self.ayar();
        if istek.kip == crate::CalistirmaKipi::TamErisim {
            return crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini);
        }
        let mut ace = self.ace();
        if ace.bozuk_mu() {
            return Err(BaslatmaHatasi::Yalitim("ACE kaydı bozuk; sunucu başlatılmadı".into()));
        }
        let kimlik = alan_kimligi(&istek.alan.kok);
        if !ace.hazir_mi(&kimlik) {
            return Err(BaslatmaHatasi::Yalitim("çalışma alanı yalıtım için hazırlanmadı".into()));
        }
        let ad = profil_adi(&kimlik);
        let paket = guvenlik::profil_olustur(&ad, "Kalfa çalışma alanı yalıtımı").map_err(yalitim_hatasi)?;
        let arac = guvenlik::arac_zinciri_sidi().map_err(yalitim_hatasi)?;
        let sunucu_yetenegi = guvenlik::yetenek_sidi(&sunucu_yetenek_adi(&kimlik, &istek.sunucu))
            .map_err(yalitim_hatasi)?;
        let ozel = sunucu_kokunu_hazirla(
            &mut ace, &ayar.yalitim_dizini, &kimlik, &istek.sunucu, &ad, &sunucu_yetenegi, &ayar.git,
        )
        .map_err(yalitim_hatasi)?;
        let (araclar, uygun) = arac_yollarini_siniflandir(&ace, &ayar, Some(&istek.alan.kok))
            .map_err(yalitim_hatasi)?;
        if !uygun {
            return Err(BaslatmaHatasi::Yalitim(
                "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor".into(),
            ));
        }
        let ag = istek.ag_listesi.is_some();
        // `yalitim_ortami` kalıtılan ortamı izin listesinden geçirir ve
        // `TOKEN`/`KEY` içeren adları düşürür. Sunucunun ortamını ise 1.1a-2
        // §5.5'e göre birleştirmiştir; PATH dışındaki değerler geri yazılır.
        let sunucu_ortami = crate::sunucu_ortami(&istek.ortam, &ozel);
        let mut ortam = yalitim_ortami(&sunucu_ortami, &ozel, &ayar, &araclar, ag);
        for (ad, deger) in &sunucu_ortami {
            if !ad.eq_ignore_ascii_case("PATH") {
                deger_ata(&mut ortam, ad, deger);
            }
        }
        let kokler = izinli_kokler(&ayar, &istek.alan, &ozel, &araclar);
        let (program, argumanlar) = program::coz(
            &KomutBicimi::Program { program: istek.program.clone(), argumanlar: istek.argumanlar.clone() },
            &ortam,
        )?;
        let program = gorunen_program(&program, &kokler, &istek.alan, &ortam)?;
        let komut = komut_satiri::uret(&program, &argumanlar, &kap::sistem_dizini().join("cmd.exe"))
            .map_err(yalitim_hatasi)?;
        let (ortam, ag_kapat) = if ag {
            let alan_ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &kimlik);
            self.kopruyu_hazirla(&mut ace, &kimlik, &alan_ozel)?;
            self.ag_baglantisi(ortam, istek.ag_listesi.clone(), &alan_ozel)?
        } else {
            (ortam, None)
        };
        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        let yetenekler = vec![arac, sunucu_yetenegi];
        let kap = kap::baslat_etkilesimli(
            &kap::KapIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                paket: &paket,
                yetenekler: &yetenekler,
            },
            &is,
        )
        .map_err(yalitim_hatasi)?;
        drop(ace);
        let mut surec = crate::EtkilesimliSurec::kaptan(kap, is);
        if let Some(k) = ag_kapat {
            surec.ag_kapat_ekle(k);
        }
        Ok(surec)
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kimlik = alan_kimligi(alan_koku);
        let kayit = sunucu_kaydi(&kimlik, sunucu);
        let mut ace = self.ace();
        kayda_gore_geri_al(&mut ace, &kayit).map_err(|e| e.to_string())?;
        ace.alani_cikar(&kayit, &[]).map_err(|e| e.to_string())?;
        *ace = AceKaydi::oku(&self.ace_yolu);
        let kok = crate::sunucu_ozel(&self.ayar().yalitim_dizini, &kimlik, sunucu).kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
```

  `crate::agac::Is::yeni(true)`, kabuk yolunun kullandığı iş nesnesi
  kurucusudur (`yalitici.rs`). Kabukla aynı etkin süreç sınırı ve UI
  kısıtları uygulanır.

- [ ] **Adım 7: `alani_kaldir_kilitli`'ye sunucu girdilerini ekleyin.**
  `kayda_gore_geri_al(ace, kimlik)` çağrısından önce şunu ekleyin:

```rust
        let onek = format!("{kimlik}/mcp/");
        let mut sunucular: Vec<String> = ace
            .girdiler()
            .iter()
            .map(|g| g.alan().to_string())
            .filter(|a| a.starts_with(&onek))
            .collect();
        sunucular.sort();
        sunucular.dedup();
        for kayit in sunucular {
            kayda_gore_geri_al(ace, &kayit)?;
            ace.alani_cikar(&kayit, &[])?;
        }
```

  Bozuk kayıt yolu (`taramali_geri_al`) yetenek SID'lerini bilmez. Bu
  yüzden bozuk kayıtta sunucu ACE'leri için ayrı bir yol yazılmaz. Ancak
  yeni bir test, bozuk kayıtta `alani_kaldir`'ın hata döndüğünü ve sunucu
  ACE'lerine dokunmadığını sabitler. Bu, kabuktaki "yalnız kanıtlananı
  geri al" kuralıyla aynıdır.

- [ ] **Adım 8: Geçtiğini görün.** Windows'ta, kurulu makinede:
  `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim -- --test-threads=1`.
  Beklenen: yeni 5 test ve bütün eski testler geçer. Ardından makinede
  `kalfa-` önekli AppContainer profili ve `S-1-15-3-` ACE'si kalmadığını
  doğrulayın; yöntemi G1 raporundaki temizlik denetimiyle aynıdır.
- [ ] **Adım 9: Commit'leyin:** `sandbox: Windows'ta borulu AppContainer süreci, sunucu yetenek SID'i ve ACE kaydı`.

---

### Görev 5: İki platform doğrulaması ve belgeler

**Dosyalar:**
- Değiştir: `docs/GELISTIRME_DURUMU.md`, tasarım §4.1 (G1'in yanına uygulama sonucu)

- [ ] **Adım 1: Windows'ta** (ana klasör ya da worktree):

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings
cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim -- --test-threads=1
cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
```

- [ ] **Adım 2: WSL'de** (Ubuntu 24.04, `kalfatest`, ayrı kopya ve `CARGO_TARGET_DIR`):

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings
cargo build -p kalfa-sandbox --bin kalfa-kopru --locked
cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim
cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
```

- [ ] **Adım 3: Temizliği doğrulayın.**
  - Windows'ta `HKCU\…\AppContainer\Mappings` altında test profili, kalan
    test süreci ve `S-1-15-3-` ACE'si olmamalı.
  - WSL kopyası ve `CARGO_TARGET_DIR` silinmeli.
- [ ] **Adım 4: Belgeleri güncelleyin.** Sonuçları ve sayıları yazın:
  `GELISTIRME_DURUMU.md` ile tasarım §4.1. `e2e`, arayüz ve `kalfa-mcp` bu
  planın kapsamında değil. Yalnız sandbox değiştiği için arayüz ve e2e
  komutları gerekmez.
- [ ] **Adım 5: Commit'leyin ve PR'a yazın:** `docs: 1.1a-1 etkileşimli süreç sonucu`.

---

## Kapsam dışı (1.1a-2)

Bu planın kapsamında olmayan konular:

- sınırlı çerçeveleyici ve stderr satır sınırı,
- `rmcp`,
- koruma özeti K ve mutlak denetim yaşı,
- W ve yazma kuşağı,
- başlatma izni,
- mühür,
- tanım denetimi,
- ortam birleştirme ve `GizliKasa`,
- uygulama yöneticisi,
- arayüz.

Bu planda gelenler: stdin/stdout/stderr ham akışları, ağacın kapanışı,
özel kök ve çalışma kipi.

## Öz denetim

- **Tasarımın karşılığı:**

  | Tasarım | Görev |
  |---|---|
  | §4.1 etkileşimli süreç | G2–4 |
  | §4.1 çalışma kipi | G1, G2, G3, G4 |
  | §4.1 ağ | G3, G4; mevcut vekil yeniden kullanılır |
  | §3 özel kök ve ortam | G1, G3, G4 |
  | §5.6 yetenek SID'i ve ACE kaydı | G4 |
  | G1'in üç ölçümü | G3 ve G4'te ürün başlatıcısıyla yeniden kurulur |

  Ağlı etkileşimli süreç için ayrı bir kabul testi bu planda yok; mevcut
  vekil testleri aynı kodu sınar. İnceleme gerekli görürse Görev 3 ve 4'e
  birer ağlı test eklenir.
- **Tip tutarlılığı:** `EtkilesimliSurec::kaptan`, `tam_erisimle`,
  `sunucu_ozel`, `sunucu_ortami` ve `KurIstegi` bütün görevlerde aynı adla
  geçer.
- **Bilinen belirsizlikler:** Aşağıdakiler ön doğrulamada derleyiciyle
  çözülecek. Değişen her imza plana geri yazılacak.
  - `ozel.hazirla`'nın sunucu kökünde `.gitconfig` ve `.npmrc` yazması
    beklenen bir davranış.
  - `tokio::fs::File`'ın boru üzerinde `AsyncWrite` olarak flush gerektirmesi.
    Testler flush çağırıyor.
