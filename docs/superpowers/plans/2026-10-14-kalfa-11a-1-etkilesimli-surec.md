# Kalfa 1.1a-1 — Etkileşimli süreç: Uygulama planı

> **Ajanlar için:** Bu plan görev görev uygulanır. Adımlar onay kutusuyla
> (`- [ ]`) izlenir. Her görev önce düşen testle başlar. Kullanıcı alt
> ajanlı yöntemi seçti: her görevi yeni bir alt ajan uygular, ayrı bir
> gözden geçirici denetler. Görev 1 ve 2 tek bağımlılık grubudur; Görev 3
> ve 4, Görev 2'nin arayüzleri sabitlenmeden başlamaz. Ajanlar aynı
> worktree'de eşzamanlı düzenleme yapmaz.

**Durum:** Ürün uygulaması tamamlandı (r4). Codex, kullanıcının 14 Ekim'deki
doğrudan devriyle 1.1a-1'i uyguluyor. r3 incelemesindeki ara ebeveyn PID
karşı örneği düzeltildi. Bağımsız inceleme GPT-6 Luna / max ile yapıldı;
Windows ve WSL doğrulaması geçti. Ürün commit'i `c1d464f`; sonuçlar
[ürün raporunda](../bulgular/2026-10-14-kalfa-11a-1-urun.md). Ön doğrulanmış görev 1–4 kodu yeniden
kullanıldı; kalan düzeltme ayrı alt ajana verildi. Kullanıcının son model
ve devir talimatı önceki yöntem kaydının yerine geçer.

**Ön doğrulama:** Bu plandaki kod blokları ayrı bir worktree'de iki
platformda derlendi ve testleri koşturuldu:

- Dal `deneme/11a-1-on-dogrulama`, commit `0eab968` (yalnız
  yerel; push edilmedi).
- İlk kod blokları o commit'ten alındı. r4'te `agac.rs` blokları ürün
  kaynağındaki PID soy düzeltmesi ve güvenlik açıklamalarıyla güncellendi.
- Sonuçlar [ön doğrulama raporunda](../bulgular/2026-10-14-kalfa-11a-1-on-dogrulama.md).

**Taban:** `main` `04f16fc` ve PR #19 dalı (`docs/mcp-yerel-tasarim`).
**Tasarım:** [1.1a tasarımı](../specs/2026-10-14-kalfa-11a-mcp-yerel-design.md)
r8: §3 (sunucunun özel kökü, ortam), §4.1 (etkileşimli süreç, `kapat()`
sözleşmesi, çalışma kipi, ağ), §5.6 (yetenek SID'i, korumalı kök).
[G1 raporu](../bulgular/2026-10-14-kalfa-11a-g1.md).

**Hedef:** `kalfa-sandbox`, MCP sunucusu gibi uzun yaşayan ve stdin/stdout
borulu bir süreci yalıtımlı ya da açıkça seçilmiş tam erişimle başlatabilir.
Sürecin ağacını kapatır ve bittiğini doğrular. Sunucuya özel kökü Linux'ta
bağlamayla, Windows'ta yetenek SID'i ve korumalı DACL'le kabuktan ayırır.

**Mimari:**

- `etkilesimli.rs` platformdan bağımsız parçaları taşır: istek, çalışma
  kipi, sunucu adı, özel kök ve ortam.
- `agac.rs` ağacı öldürme ve bitişini doğrulama arayüzünü (`AgacDenetimi`)
  kazanır. Linux'ta kök, torunlarının alt süreç toplayıcısıdır; torunlar
  `pidfd` ile açılıp başlangıç zamanıyla doğrulanır.
- `surec.rs`, mevcut `Cocuk`/`Agac` altyapısıyla bir `EtkilesimliSurec`
  üretir. stdin'e bir görev aktarır; `kapat()` bu görevi keser.
- İki arka uç, kabuğun bwrap ve AppContainer kurulumunu yeniden kullanır.
  Farklı olanlar özel kök, çalışma dizini, stdin borusu ve yeteneklerdir.
  Ağlı başlatma kabuğun köprü ve vekil yolunu kullanır.
- Ajan döngüsü, `rmcp` ve arayüz bu planın kapsamında değildir (1.1a-2).

**Yığın:** Rust 1.98.1, tokio, `windows-sys` (mevcut özellikler),
bwrap/seccomp (mevcut). Yeni bağımlılık yok.

## Global Constraints

- Yeni crate ya da `windows-sys` özelliği yok; `rmcp` 1.1a-2'dedir.
- Bu planda eklenen `unsafe` yalnız şu dosyalarda ve her blok `// SAFETY:` açıklamasıyla:
  `agac.rs`, `windows/kap.rs`, `windows/guvenlik.rs`, `linux/mod.rs`.
- Üretim kodunda `unwrap`/`expect` yok. Adlar, açıklamalar ve testler
  Türkçedir.
- Güvenlik sınırları izin listesiyle kurulur:
  - Sunucu adı `[a-z0-9-]{1,32}`'dir.
  - `EVREN_` ve `KALFA_` önekli ortam adları hiçbir yoldan sunucuya geçmez.
    Windows köprüsünün `KALFA_KOPRU_*` değerleri köprünün kendi
    ortamındadır; köprü onları çocuğa geçirmez.
- **Çalışma kipi açıktır** (tasarım §4.1):
  - Yalıtımlı istek yalıtım yoksa başlamaz.
  - Tam erişim, yalıtım hazırken de yalıtımsız başlar.
  - Mevcut `kip_uyusmasi` kabuğa özgüdür; MCP onu kullanmaz.
  - `Yalitimsiz` arka ucu etkileşimli süreç başlatmaz (trait'in varsayılanı
    hata döner). O yalnız desteklenmeyen platformlarda kullanılır. Tam
    erişimi Linux ve Windows arka uçları başlatır.
- **Özel kök** = `<yalitim_dizini>/<alan_kimligi>/mcp/<sunucu>/{ev,gecici,onbellek}`.
  - Kök iki kipte ortaktır.
  - Kabuğun özel `ev`, `gecici` ve `onbellek` dizinleri sunucuya bağlanmaz.
  - Sunucunun kökü de kabuğa ve öbür sunuculara bağlanmaz.
  - Windows'ta `mcp` ve kök iki kipte de mirassız (korumalı) DACL'le
    kurulur: yalnız geliştirici, SYSTEM ve Administrators. Paket SID'inin
    bu kökte hiçbir hakkı yoktur; değiştirme hakkı yalnız sunucunun yetenek
    SID'ine verilir (`kalfa-mcp-<alan>-<sunucu>`). Var olan kök korumalı
    değilse ya da başka bir kimliğe izin veriyorsa başlatma reddedilir.
- **Ortam** (tasarım §3, G1 bulgusu):
  - `HOME`, `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `TMPDIR`, `TEMP`,
    `TMP`, `NPM_CONFIG_CACHE`, `PIP_CACHE_DIR`, `UV_CACHE_DIR` ve
    `XDG_CACHE_HOME` özel köke yönelir.
  - Windows'ta `SystemRoot`, `windir` ve `SystemDrive` sistemin kendi
    değerlerini taşır.
  - Windows'ta AppContainer `LOCALAPPDATA`'yı ve `TEMP`'i
    `ev\Packages\<profil>\AC` altına çevirir. Bu da özel kökün içindedir.
- **`kapat()`** (tasarım §4.1) `io::Result<KapanisSonucu>` döner:
  1. stdin kapanır: stdin'e aktaran görev kesilir. Dışarıya verilen yazıcı
     tutulsa bile süreç EOF alır.
  2. En çok 2 sn kendiliğinden çıkış beklenir.
  3. Ağaç her durumda öldürülür ve bitişi doğrulanır.

  `Ok` döndüğünde ağaçta yaşayan süreç yoktur. Windows'ta işin etkin
  süreç sayısı sıfırdır. Linux'ta süreç grubu boştur ve kök yaşarken
  bulunan bütün torunlar ölmüştür. Öldürme ya da doğrulama başarısızsa
  `Err` döner. Linux tam erişimin kalan sınırı tasarım §4.1'dedir.

  `kapat()` iptal edilirse ya da hata verirse (ağaç doğrulanmadıysa) ve
  süreç `kapat()`'sız bırakılırsa `Drop` temizlik yapar: stdin görevini
  her durumda keser, kök toplanmadıysa torunları öldürür, ağacı öldürür.
  Bu yolda bitiş doğrulanmaz; çağıran kilidi bırakmaz.
- **Linux torun taraması:** `/proc` tam okunamazsa (açılamayan dizin,
  okunamayan ya da ayrıştırılamayan kayıt) tarama hata döner; eksik tarama
  "torun yok" sayılmaz. Kaydı olmayan süreç toplanmıştır ve atlanır. Her
  torun `pidfd_open` ile açılır; açtıktan sonra yaprağın ve köke kadar
  bütün ara ebeveynlerin başlangıçları taramadakiyle karşılaştırılır.
  Değişmiş veya kaybolmuş halka varsa alt dal bu turda sinyal almaz;
  sonraki tarama köke bağlanmış yetimleri kapsar. Okuma hatası `Err` verir.
  Sinyal `pidfd_send_signal` ile gider.
- **Windows kök denetimi önce gelir:** Var olan `mcp`, kök, `ev`,
  `gecici` ve `onbellek` ilk dosya, ACL ya da kayıt değişikliğinden önce
  denetlenir. Ret hiçbir şeyi değiştirmez.
- **Windows ACE'leri** ACE kaydına `<alan_kimligi>/mcp/<sunucu>`
  anahtarıyla girer. Sunucu kaldırılınca ya da çalışma alanı kaldırılınca
  geri alınır. Testler makinede Kalfa profili ya da `S-1-15-2-`/`S-1-15-3-`
  ACE'si bırakmaz.
- **Etkileşimli süreç tokio çalışma zamanının içinden başlatılır.** Dışında
  başlatılırsa hata döner.
- **Testler:**
  - Bütün yeni testlerin adı `etkilesimli` içerir. Her komut tek bir test
    süzgeci alır.
  - Yalıtım testleri `yalitim-testleri` özelliğiyle gerçek yalıtımda koşar;
    yalıtım yoksa düşer.
  - Windows'ta yönetici gerektiren bir adım yoktur.
- **Ara commit'lerde ölü kod uyarısı:** `tam_erisimle` ve
  `komutla_etkilesimli`, Görev 2'de yalnız birim testlerinden çağrılır.
  Linux'ta Görev 3'e, Windows'ta Görev 4'e kadar `cargo clippy -D warnings`
  bu yüzden `dead_code` uyarısıyla düşer. Ara görevlerin doğrulaması
  testlerdir. Clippy, Görev 3 sonunda Linux'ta, Görev 4 sonunda Windows'ta
  ve Görev 5'te iki platformda temiz olmalıdır.

## Review Focus

Spec'in ima ettiği ama ilk bakışta test edilmeyen beş durum. Her biri sahibi
olan görevde ayrı bir testle sabitlenmiştir:

1. **Sunucu adı yol aşımı:** `../x`, `a/b`, büyük harf ya da 33 karakterlik
   ad. Hiçbir dizin oluşturulmamalı, başlatma reddedilmeli (Görev 1, 2).
2. **Sunucu stdin'i hiç okumuyor:** stdout'a sürekli yazan, stdin'i okumayan
   sunucuya boruyu aşan bir yazma sürerken `kapat()` çağrılıyor. `kapat()`
   2 sn + öldürme içinde dönmeli, yazma görevi hatayla bitmeli (Görev 2).
3. **Süreç `kapat()` çağrılmadan bırakılıyor:** Örneğin panik sırasında.
   Ağaç yine ölmeli (Görev 2; Linux'ta `setsid` torunu dahil).
4. **Aynı alanda iki sunucu:** A sunucusu B'nin özel köküne yazamamalı ve
   onu okuyamamalı (Görev 3, Görev 4).
5. **Çalışma alanı kaldırılırken sunucu izinleri:** `alani_kaldir`, o alana
   bağlı bütün `<alan>/mcp/*` ACE'lerini de geri almalı (Görev 4).

---

## Dosya haritası

| Dosya | Sorumluluk |
|---|---|
| `crates/kalfa-sandbox/src/etkilesimli.rs` (yeni) | `CalistirmaKipi`, `EtkilesimliIstek`, `sunucu_adi_dogrula`, `sunucu_ozel`, `ayrilmis_mi`, `sunucu_ortami`, `mcp_kip_uyusmasi`, `KAPANMA_PAYI` |
| `crates/kalfa-sandbox/src/agac.rs` | `baslat_etkilesimli` (Linux'ta alt süreç toplayıcı), `AgacDenetimi` ve `Agac` için uygulaması; Linux'ta `SurecKaynagi`, `torunlari_oldur_ile`, `ProcFs` (`/proc` + `pidfd`); Windows'ta `Is::{sonlandir_denetimli, etkin_surec_sayisi}`; sahte kaynakla birim testleri |
| `crates/kalfa-sandbox/src/surec.rs` | `EtkilesimliSurec`, `KapanisSonucu`, `agaci_oldur`, `bosalmayi_bekle`, `Surec::komutla_etkilesimli`, `tam_erisimle`, Windows için `EtkilesimliSurec::kaptan`; tam erişim, iptal ve sahte ağaç birim testleri |
| `crates/kalfa-sandbox/src/lib.rs` | Modül, dışa açılan tipler, `Yalitici::baslat_etkilesimli` ve `sunucu_kokunu_kaldir` |
| `crates/kalfa-sandbox/src/linux/mod.rs` | `KurIstegi`, `komut_kur`'un özel kök parametresi, `vekili_bagla`, Linux etkileşimli başlatma ve kaldırma |
| `crates/kalfa-sandbox/src/windows/kap.rs` | Borulu stdin (`Kap::girdi`, `baslat_etkilesimli`) |
| `crates/kalfa-sandbox/src/windows/yalitici.rs` | `ag_hazirla`, sunucu kökünün önce denetimi (`sunucu_kokunu_denetle`), korumalı kurulumu ve ACE kaydı, Windows etkileşimli başlatma, kaldırma, `alani_kaldir`'da sunucu girdileri |
| `crates/kalfa-sandbox/tests/yalitim.rs` | Linux kabul testleri (G1 karşılıkları, ağlı sunucu) |
| `crates/kalfa-sandbox/tests/yalitim/windows.rs` | Windows kabul testleri (G1 karşılıkları, korumalı kök, ağlı sunucu) |
| `docs/GELISTIRME_DURUMU.md`, tasarım §4.1 | Sonuç |

---

### Görev 1: Platformdan bağımsız sözleşme

**Dosyalar:**
- Oluştur: `crates/kalfa-sandbox/src/etkilesimli.rs`
- Değiştir: `crates/kalfa-sandbox/src/lib.rs`
- Test: `etkilesimli.rs` içindeki `#[cfg(test)]`

**Arayüzler:**
- Kullanır: `profil::{AlanBilgisi, OzelDizinler}`, `vekil::IzinListesi`, `BaslatmaHatasi`.
- Üretir:
  - `pub enum CalistirmaKipi { Yalitimli, TamErisim }`
  - `pub struct EtkilesimliIstek { program, argumanlar, ortam, kip, sunucu, alan, ag_listesi }`
  - `pub fn sunucu_adi_dogrula(ad: &str) -> Result<(), BaslatmaHatasi>`
  - `pub fn sunucu_ozel(yalitim_dizini: &Path, alan_kimligi: &str, sunucu: &str) -> OzelDizinler`
  - `pub(crate) fn ayrilmis_mi(ad: &str) -> bool`
  - `pub fn sunucu_ortami(kaynak: &[(String, String)], ozel: &OzelDizinler) -> Vec<(String, String)>`
  - `pub fn mcp_kip_uyusmasi(kip: CalistirmaKipi, yalitim_hazir: bool) -> Result<(), BaslatmaHatasi>`
  - `pub const KAPANMA_PAYI: Duration` (2 sn)

- [x] **Adım 1: Düşen birim testlerini yazın** (`etkilesimli.rs` sonu):

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
        assert_eq!(
            deger("TMPDIR"),
            Some(o.gecici.display().to_string().as_str())
        );
        assert_eq!(
            deger("NPM_CONFIG_CACHE"),
            Some(o.npm().display().to_string().as_str())
        );
        assert_eq!(
            deger("PIP_CACHE_DIR"),
            Some(o.pip().display().to_string().as_str())
        );
        assert_eq!(
            deger("UV_CACHE_DIR"),
            Some(o.onbellek.join("uv").display().to_string().as_str())
        );
        assert_eq!(
            deger("XDG_CACHE_HOME"),
            Some(o.onbellek.display().to_string().as_str())
        );
        assert_eq!(deger("PATH"), Some("/usr/bin"));
        assert_eq!(deger("GITHUB_TOKEN"), Some("ghp"));
        assert!(ortam.iter().all(|(a, _)| !ayrilmis_mi(a)));
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

- [x] **Adım 2: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --locked --lib etkilesimli::tests`.
  Beklenen: derleme hatası; `etkilesimli` modülü yok.

- [x] **Adım 3: Modülü yazın** (`etkilesimli.rs`, testlerden önce):

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
    /// Yalnız yalıtımlıda; `None` ağsızdır. Tam erişimde verilirse başlatma
    /// reddedilir: yalıtımsız süreçte ağ kapatılamaz.
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
        Err(BaslatmaHatasi::Yalitim(format!(
            "geçersiz sunucu adı: {ad:?}"
        )))
    }
}

/// Sunucunun çalışma alanına özel kökü (§3).
pub fn sunucu_ozel(yalitim_dizini: &Path, alan_kimligi: &str, sunucu: &str) -> OzelDizinler {
    OzelDizinler::yol(&yalitim_dizini.join(alan_kimligi).join("mcp"), sunucu)
}

const AYRILMIS_ONEKLER: &[&str] = &["EVREN_", "KALFA_"];

/// Ad Kalfa'ya ayrılmış mı? Bu adlar hiçbir yoldan sunucuya geçmez (§3).
pub(crate) fn ayrilmis_mi(ad: &str) -> bool {
    let buyuk = ad.to_ascii_uppercase();
    AYRILMIS_ONEKLER.iter().any(|o| buyuk.starts_with(o))
}

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
        .filter(|(ad, _)| !ayrilmis_mi(ad))
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

  `lib.rs`'te `mod agac;` satırının altına `pub mod etkilesimli;`, mevcut
  `pub use profil::…` satırının üstüne şu eklenir:

```rust
pub use etkilesimli::{
    CalistirmaKipi, EtkilesimliIstek, KAPANMA_PAYI, mcp_kip_uyusmasi, sunucu_ortami, sunucu_ozel,
};
```

- [x] **Adım 4: Geçtiğini görün:** İki platformda
  `cargo test -p kalfa-sandbox --locked --lib etkilesimli::tests`. Beklenen:
  4 test geçer.

- [x] **Adım 5: Commit'leyin:** `sandbox: etkileşimli süreç sözleşmesi (kip, sunucu adı, özel kök, ortam)`.

---

### Görev 2: Ağacın doğrulanan kapanışı, `EtkilesimliSurec` ve tam erişim

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/agac.rs`, `src/surec.rs`, `src/lib.rs`
- Test: `surec.rs` içindeki `#[cfg(test)]` (tam erişim birim testleri)

**Arayüzler:**
- Kullanır: Görev 1'in tipleri; mevcut `agac::{baslat, Agac}`, `Cocuk::ayir`, `program::coz`.
- Üretir:
  - `pub(crate) fn agac::baslat_etkilesimli(komut: &mut Command) -> io::Result<(Child, Agac)>`
  - `pub(crate) trait AgacDenetimi { torunlari_oldur(&self) -> io::Result<bool>; oldur_denetimli(&self) -> io::Result<()>; bos_mu(&self) -> io::Result<bool> }`
    ve `impl AgacDenetimi for Agac`
  - Linux: `SurecBilgisi`, `trait SurecKaynagi`, `fn torunlari_oldur_ile<K: SurecKaynagi>(&K, kok: i32) -> io::Result<bool>`,
    `ProcFs` (`/proc` ve `pidfd`)
  - Windows: `Is::sonlandir_denetimli(&self) -> io::Result<()>`, `Is::etkin_surec_sayisi(&self) -> io::Result<u32>`
  - `pub struct EtkilesimliSurec`. Yöntemleri:
    - `girdi_al(&mut self) -> Option<Box<dyn AsyncWrite + Send + Unpin>>`
    - `cikti_al(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>>`
    - `hata_al(&mut self) -> Option<Box<dyn AsyncRead + Send + Unpin>>`
    - `ag_kapat_ekle(&mut self, AgKapatici)`
    - `async bitisi_bekle(&mut self) -> Option<i32>`
    - `async kapat(self) -> io::Result<KapanisSonucu>`
  - `pub struct KapanisSonucu { pub kendiliginden: bool, pub cikis: Option<i32>, pub sure: Duration, pub ag: Option<vekil::AgOzeti> }`
  - `pub(crate) fn Surec::komutla_etkilesimli(komut: Command) -> Result<EtkilesimliSurec, BaslatmaHatasi>`
  - `async fn agaci_oldur(&impl AgacDenetimi, kok_toplanmadi: bool, son: Instant) -> io::Result<()>`,
    `async fn bosalmayi_bekle(&impl AgacDenetimi, son: Instant) -> io::Result<()>`
  - `pub(crate) fn tam_erisimle(istek: &EtkilesimliIstek, yalitim_dizini: &Path, git: &GitKimligi) -> Result<EtkilesimliSurec, BaslatmaHatasi>`
  - `Yalitici::baslat_etkilesimli` ve `Yalitici::sunucu_kokunu_kaldir` (varsayılanları hata döner)

**Neden böyle (Codex r1 bulgu 1 ve 4, r2 bulgu 1 ve 2):**

- Kök süreç beklenince ağacın bittiği kanıtlanmaz. Bitiş, Windows'ta işin
  etkin süreç sayısıyla, Linux'ta süreç grubunun boşalmasıyla doğrulanır.
- Linux'ta `setsid` ile gruptan ayrılan torunu süreç grubu kapsamaz. Kök
  alt süreç toplayıcısı olur. Kök toplanmadan önce `/proc`'tan bulunan
  torunları dondurulup öldürülür. Toplanan kökün numarası başka bir sürece
  verilebileceği için tarama yalnız kök toplanmamışken yapılır.
- Tarama tam yapılamazsa hata verir; eksik tarama "torun yok" sayılmaz.
  Torunun numarası da yeniden kullanılabilir: her torun `pidfd` ile açılır,
  başlangıç zamanı taramadakiyle karşılaştırılır, sinyal tutamaçla gider.
  İşletim sistemi kapısı `SurecKaynagi` arkasındadır; hata ve numara
  yeniden kullanımı sahte kaynakla sınanır.
- stdin'in yazma ucu yalnız bir aktarma görevindedir. Dışarıya
  `tokio::io::duplex`'in bir ucu verilir. `kapat()` ve `Drop` görevi
  keserek boruyu kapatır; dışarıdaki yazıcı o andan sonra hata alır.
- `dogrulandi` yalnız doğrulanmış kapanıştan sonra `true` olur. `kapat()`
  iptal edilirse gelecek bırakılır ve `Drop` temizliği yapar.
- Öldürme ve boşalma beklemesi `AgacDenetimi` üzerinden yazılır; hata,
  zaman aşımı ve `[etkin > 0, etkin > 0, etkin = 0]` dizisi sahte ağaçla
  sınanır.

- [x] **Adım 1: Düşen testleri yazın** (`surec.rs` sonu). Testler tam
  erişimi her platformda doğrudan `tam_erisimle` ile sınar:

```rust
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{Duration, Instant};

    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

    use super::{EtkilesimliSurec, tam_erisimle};
    use crate::{AlanBilgisi, CalistirmaKipi, EtkilesimliIstek, GitKimligi, tarama};

    fn istek(kok: &Path, program: &str, arg: &[&str]) -> EtkilesimliIstek {
        EtkilesimliIstek {
            program: program.into(),
            argumanlar: arg.iter().map(|s| s.to_string()).collect(),
            ortam: crate::ortam::uret(std::env::vars()),
            kip: CalistirmaKipi::TamErisim,
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

    fn baslat(kok: &Path, program: &str, arg: &[&str]) -> EtkilesimliSurec {
        tam_erisimle(
            &istek(kok, program, arg),
            &kok.join("yalitim"),
            &GitKimligi::default(),
        )
        .unwrap()
    }

    /// stdin satırlarını aynen geri yazan, EOF'ta çıkan komut.
    fn yanki() -> (&'static str, Vec<&'static str>) {
        if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoLogo",
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Console]::InputEncoding=[Text.Encoding]::UTF8;[Console]::OutputEncoding=[Text.Encoding]::UTF8;while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine($s);[Console]::Out.Flush()}",
                ],
            )
        } else {
            ("cat", vec![])
        }
    }

    fn yasiyor(pid: u32) -> bool {
        if cfg!(windows) {
            std::process::Command::new("tasklist")
                .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains(&format!("\"{pid}\"")))
                .unwrap_or(false)
        } else {
            std::fs::read_to_string(format!("/proc/{pid}/stat"))
                .map(|s| !s.contains(") Z ") && !s.contains(") X "))
                .unwrap_or(false)
        }
    }

    /// Test düşse bile kalan süreci öldürür.
    struct Temizlik(Vec<u32>);

    impl Drop for Temizlik {
        fn drop(&mut self) {
            for pid in &self.0 {
                let _ = if cfg!(windows) {
                    std::process::Command::new("taskkill")
                        .args(["/F", "/PID", &pid.to_string()])
                        .output()
                } else {
                    std::process::Command::new("kill")
                        .args(["-9", &pid.to_string()])
                        .output()
                };
            }
        }
    }

    async fn ilk_satir(s: &mut EtkilesimliSurec) -> String {
        let mut satir = String::new();
        BufReader::new(s.cikti_al().unwrap())
            .read_line(&mut satir)
            .await
            .unwrap();
        satir.trim().to_string()
    }

    /// G1.1 (tam erişim): 1 000 JSON-RPC satırı, 1 MiB'lık satır ve UTF-8
    /// birebir; EOF'ta kendiliğinden çıkış.
    #[tokio::test]
    async fn etkilesimli_tam_erisim_borulari_birebir_yankilar_ve_eofta_cikar() {
        let d = tempfile::tempdir().unwrap();
        let (p, a) = yanki();
        let mut s = baslat(d.path(), p, &a);
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
        });
        let mut alinan = Vec::new();
        while let Some(s) = cikti.next_line().await.unwrap() {
            alinan.push(s.trim_end_matches('\r').to_string());
        }
        yazici.await.unwrap();
        assert_eq!(alinan, gonderilen);
        assert_eq!(s.bitisi_bekle().await, Some(0));
        assert!(s.kapat().await.unwrap().kendiliginden);
    }

    /// Codex 4a: yazıcı dışarıda tutulurken de `kapat` stdin'i kapatır;
    /// EOF'ta çıkan süreç öldürülmeden kendiliğinden biter.
    #[tokio::test]
    async fn etkilesimli_kapat_disarida_tutulan_yaziciya_ragmen_eof_verir() {
        let d = tempfile::tempdir().unwrap();
        let (p, a) = yanki();
        let mut s = baslat(d.path(), p, &a);
        let mut girdi = s.girdi_al().unwrap();
        girdi.write_all(b"merhaba\n").await.unwrap();
        let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
        assert_eq!(
            cikti
                .next_line()
                .await
                .unwrap()
                .map(|s| s.trim().to_string()),
            Some("merhaba".into())
        );
        let k = s.kapat().await.unwrap();
        assert!(k.kendiliginden, "{k:?}");
        assert!(girdi.write_all(b"sonra\n").await.is_err());
    }

    /// Review Focus 2 ve Codex 4b: stdin'i hiç okumayan, stdout'u dolu
    /// süreçte boruyu aşan bir yazma sürerken `kapat` sınırlı sürede döner,
    /// yazma görevi de hatayla biter.
    #[tokio::test]
    async fn etkilesimli_kapat_okumayan_sureci_ve_bloke_yazmayi_bitirir() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoProfile",
                    "-Command",
                    "while($true){[Console]::Out.WriteLine('x'*1000)}",
                ],
            )
        } else {
            ("sh", vec!["-c", "trap '' TERM HUP; yes xxxxxxxxxx"])
        };
        let mut s = baslat(d.path(), p, &a);
        let mut girdi = s.girdi_al().unwrap();
        let yazici = tokio::spawn(async move {
            let parca = vec![b'y'; 64 * 1024];
            for _ in 0..64 {
                girdi.write_all(&parca).await?;
            }
            Ok::<(), std::io::Error>(())
        });
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!yazici.is_finished(), "yazma boruyu aşmalı ve beklemeli");
        let bas = Instant::now();
        let k = s.kapat().await.unwrap();
        assert!(!k.kendiliginden);
        assert!(
            bas.elapsed() < Duration::from_secs(8),
            "{:?}",
            bas.elapsed()
        );
        let yazma = tokio::time::timeout(Duration::from_secs(2), yazici)
            .await
            .expect("yazma görevi kapanıştan sonra sürmemeli")
            .unwrap();
        assert!(yazma.is_err());
    }

    /// Codex 1a: kök EOF'ta çıkar, torun stdio'yu kapatıp sürer. `kapat`
    /// `Ok` döndüğü anda torun yaşamaz.
    #[tokio::test]
    async fn etkilesimli_kapat_donunce_torun_yasamaz() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoProfile",
                    "-Command",
                    "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [Console]::Out.WriteLine($c.Id); [Console]::Out.Flush(); while([Console]::In.ReadLine() -ne $null){}",
                ],
            )
        } else {
            (
                "sh",
                vec![
                    "-c",
                    "sleep 300 </dev/null >/dev/null 2>&1 & echo $!; cat >/dev/null",
                ],
            )
        };
        let mut s = baslat(d.path(), p, &a);
        let pid: u32 = ilk_satir(&mut s).await.parse().unwrap();
        let _temizlik = Temizlik(vec![pid]);
        assert!(yasiyor(pid));
        let k = s.kapat().await.unwrap();
        assert!(k.kendiliginden, "kök EOF'ta çıkmalı: {k:?}");
        assert!(!yasiyor(pid), "kapat döndüğünde torun yaşıyor: {pid}");
    }

    /// Review Focus 3: `kapat` çağrılmadan bırakılan sürecin ağacı ölür.
    #[tokio::test]
    async fn etkilesimli_birakilan_surecin_agaci_olur() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoProfile",
                    "-Command",
                    "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [Console]::Out.WriteLine($c.Id); [Console]::Out.Flush(); Start-Sleep 300",
                ],
            )
        } else {
            ("sh", vec!["-c", "sleep 300 & echo $!; wait"])
        };
        let mut s = baslat(d.path(), p, &a);
        let pid: u32 = ilk_satir(&mut s).await.parse().unwrap();
        let _temizlik = Temizlik(vec![pid]);
        drop(s);
        let son = Instant::now() + Duration::from_secs(8);
        while yasiyor(pid) && Instant::now() < son {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(!yasiyor(pid), "bırakılan sürecin torunu yaşıyor: {pid}");
    }

    /// Codex 1c: Linux tam erişimde kendi oturumunu açan torun, kök
    /// yaşarken hem `kapat`'ta hem bırakmada öldürülür.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn etkilesimli_setsid_torun_kapat_ve_birakmada_olur() {
        let d = tempfile::tempdir().unwrap();
        let betik = "setsid sleep 300 </dev/null >/dev/null 2>&1 & echo $!; \
                     trap '' TERM HUP; while :; do sleep 1; done";
        for kapatilsin in [true, false] {
            let mut s = baslat(d.path(), "sh", &["-c", betik]);
            let pid: u32 = ilk_satir(&mut s).await.parse().unwrap();
            let _temizlik = Temizlik(vec![pid]);
            tokio::time::sleep(Duration::from_millis(200)).await;
            let oturum = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
            let oturum: Vec<&str> = oturum
                .rsplit_once(')')
                .unwrap()
                .1
                .split_whitespace()
                .collect();
            assert_eq!(oturum[3], pid.to_string(), "torun kendi oturumunda olmalı");
            if kapatilsin {
                let k = s.kapat().await.unwrap();
                assert!(!k.kendiliginden);
                assert!(!yasiyor(pid), "kapat döndüğünde setsid torunu yaşıyor");
            } else {
                drop(s);
                let son = Instant::now() + Duration::from_secs(3);
                while yasiyor(pid) && Instant::now() < son {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
                assert!(!yasiyor(pid), "bırakmada setsid torunu yaşıyor");
            }
        }
    }

    #[tokio::test]
    async fn etkilesimli_gecersiz_ad_ve_tam_erisimde_ag_listesi_reddedilir() {
        let d = tempfile::tempdir().unwrap();
        let (p, a) = yanki();
        let mut i = istek(d.path(), p, &a);
        i.sunucu = "../kacak".into();
        assert!(tam_erisimle(&i, &d.path().join("yalitim"), &GitKimligi::default()).is_err());
        let mut i = istek(d.path(), p, &a);
        i.ag_listesi = Some(crate::vekil::IzinListesi::yeni(&[], &[]));
        assert!(tam_erisimle(&i, &d.path().join("yalitim"), &GitKimligi::default()).is_err());
        assert!(!d.path().join("kacak").exists() && !d.path().join("yalitim").exists());
    }

    /// Tam erişimin çalışma dizini ve ortamı sunucunun özel köküdür (§3).
    #[tokio::test]
    async fn etkilesimli_tam_erisim_ozel_kokte_calisir() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec!["-NoProfile", "-Command", "(Get-Location).Path; $env:HOME"],
            )
        } else {
            ("sh", vec!["-c", "pwd; echo $HOME"])
        };
        let mut s = baslat(d.path(), p, &a);
        let mut m = String::new();
        s.cikti_al().unwrap().read_to_string(&mut m).await.unwrap();
        s.kapat().await.unwrap();
        let ozel = crate::sunucu_ozel(
            &d.path().join("yalitim"),
            &crate::alan_kimligi(d.path()),
            "deneme",
        );
        let satirlar: Vec<&str> = m.lines().map(str::trim).collect();
        assert_eq!(satirlar.len(), 2, "{m}");
        for s in satirlar {
            assert_eq!(Path::new(s), ozel.ev, "{m}");
        }
    }

    /// Yaşayan tokio görevleri (`#[tokio::test]`'in kendi geleceği sayılmaz).
    fn gorevler() -> usize {
        tokio::runtime::Handle::current()
            .metrics()
            .num_alive_tasks()
    }

    /// Codex r2 1: `kapat` çağrılmadan bırakılan süreçte dışarıda tutulan
    /// yazıcı varken stdin aktarma görevi de kesilir; sonraki yazma sınırlı
    /// sürede hata alır.
    #[tokio::test]
    async fn etkilesimli_birakmada_stdin_gorevi_kesilir() {
        let d = tempfile::tempdir().unwrap();
        let (p, a) = yanki();
        let once = gorevler();
        let mut s = baslat(d.path(), p, &a);
        let mut girdi = s.girdi_al().unwrap();
        assert_eq!(gorevler(), once + 1, "stdin aktarma görevi başlamalı");
        drop(s);
        let son = Instant::now() + Duration::from_secs(2);
        while gorevler() > once && Instant::now() < son {
            tokio::task::yield_now().await;
        }
        assert_eq!(gorevler(), once, "aktarma görevi kaldı");
        let yazma = tokio::time::timeout(Duration::from_secs(2), async {
            let parca = vec![b'y'; 64 * 1024];
            for _ in 0..64 {
                girdi.write_all(&parca).await?;
            }
            Ok::<(), std::io::Error>(())
        })
        .await
        .expect("yazma sınırlı sürede bitmeli");
        assert!(yazma.is_err());
    }

    /// Codex r2 1: EOF'u yok sayan kök ve (Linux'ta kendi oturumundaki)
    /// torun varken `kapat` iki saniyelik payda ve sonrasında iptal edilir.
    /// Torun yaşamaz; stdin görevi kalmaz.
    #[tokio::test]
    async fn etkilesimli_kapat_iptal_edilince_agac_olur() {
        let d = tempfile::tempdir().unwrap();
        let (p, a): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoProfile",
                    "-Command",
                    "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','300','/nobreak') -PassThru -WindowStyle Hidden; [Console]::Out.WriteLine($c.Id); [Console]::Out.Flush(); Start-Sleep 300",
                ],
            )
        } else {
            (
                "sh",
                vec![
                    "-c",
                    "setsid sleep 300 </dev/null >/dev/null 2>&1 & echo $!; \
                     trap '' TERM HUP; while :; do sleep 1; done",
                ],
            )
        };
        let once = gorevler();
        for bekleme in [Duration::from_millis(300), Duration::from_millis(2050)] {
            let mut s = baslat(d.path(), p, &a);
            let pid: u32 = ilk_satir(&mut s).await.parse().unwrap();
            let _temizlik = Temizlik(vec![pid]);
            assert!(yasiyor(pid));
            // Süre dolarsa kapanış geleceği bırakılır; 2050 ms'de kapanış
            // bitmiş de olabilir. İki durumda da torun yaşamamalı.
            let _ = tokio::time::timeout(bekleme, s.kapat()).await;
            let son = Instant::now() + Duration::from_secs(3);
            while (yasiyor(pid) || gorevler() > once) && Instant::now() < son {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            assert!(!yasiyor(pid), "{bekleme:?}: iptalden sonra torun yaşıyor");
            assert_eq!(gorevler(), once, "{bekleme:?}: aktarma görevi kaldı");
        }
    }

    /// Kapanış mantığının sahte ağacı: yanıtlar sırayla verilir.
    #[derive(Default)]
    struct SahteAgac {
        torun: std::cell::RefCell<std::collections::VecDeque<std::io::Result<bool>>>,
        oldurme_hatasi: bool,
        bos: std::cell::RefCell<std::collections::VecDeque<std::io::Result<bool>>>,
        bos_sorgusu: std::cell::Cell<usize>,
    }

    impl crate::agac::AgacDenetimi for SahteAgac {
        fn torunlari_oldur(&self) -> std::io::Result<bool> {
            self.torun.borrow_mut().pop_front().unwrap_or(Ok(false))
        }

        fn oldur_denetimli(&self) -> std::io::Result<()> {
            if self.oldurme_hatasi {
                Err(std::io::Error::other("sahte öldürme hatası"))
            } else {
                Ok(())
            }
        }

        fn bos_mu(&self) -> std::io::Result<bool> {
            self.bos_sorgusu.set(self.bos_sorgusu.get() + 1);
            self.bos.borrow_mut().pop_front().unwrap_or(Ok(false))
        }
    }

    fn uzak() -> tokio::time::Instant {
        tokio::time::Instant::now() + Duration::from_secs(5)
    }

    fn yakin() -> tokio::time::Instant {
        tokio::time::Instant::now() + Duration::from_millis(50)
    }

    /// Windows'ta geciken bitiş `[etkin > 0, etkin > 0, etkin = 0]`:
    /// bekleme son sıfırdan önce bitmez.
    #[tokio::test]
    async fn etkilesimli_bosalma_son_sifirdan_once_bitmez() {
        let a = SahteAgac::default();
        a.bos.borrow_mut().extend([Ok(false), Ok(false), Ok(true)]);
        super::bosalmayi_bekle(&a, uzak()).await.unwrap();
        assert_eq!(a.bos_sorgusu.get(), 3);
    }

    #[tokio::test]
    async fn etkilesimli_bosalmayan_ya_da_denetlenemeyen_agac_hatadir() {
        let a = SahteAgac::default();
        let e = super::bosalmayi_bekle(&a, yakin()).await.unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        let a = SahteAgac::default();
        a.bos
            .borrow_mut()
            .push_back(Err(std::io::Error::other("sorgu")));
        assert!(super::bosalmayi_bekle(&a, uzak()).await.is_err());
    }

    #[tokio::test]
    async fn etkilesimli_oldurme_ve_tarama_hatasi_kapanisi_basarisiz_kilar() {
        let a = SahteAgac {
            oldurme_hatasi: true,
            ..Default::default()
        };
        assert!(super::agaci_oldur(&a, true, uzak()).await.is_err());
        let a = SahteAgac::default();
        a.torun
            .borrow_mut()
            .push_back(Err(std::io::Error::other("tarama")));
        assert!(super::agaci_oldur(&a, true, uzak()).await.is_err());
        // Torunlar hiç bitmiyor.
        let a = SahteAgac::default();
        a.torun.borrow_mut().extend((0..10_000).map(|_| Ok(true)));
        let e = super::agaci_oldur(&a, true, yakin()).await.unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        // Kök toplandıysa torun taranmaz.
        let a = SahteAgac::default();
        a.torun
            .borrow_mut()
            .push_back(Err(std::io::Error::other("taranmamalı")));
        super::agaci_oldur(&a, false, uzak()).await.unwrap();
    }
}
```

- [x] **Adım 2: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --locked --lib surec::tests`.
  Beklenen: derleme hatası; `tam_erisimle` ve `EtkilesimliSurec` yok.

- [x] **Adım 3: `agac.rs`'e ekleyin.** `baslat`'tan sonra:

```rust
/// Etkileşimli sürecin kökü (1.1a §4.1). Linux'ta kök, kendi torunlarının
/// alt süreç toplayıcısı olur: aradaki bir süreç ölünce yetimler init'e değil
/// köke bağlanır ve kök yaşarken alt ağaç taramasında görünür. Kendi
/// oturumunu açan (`setsid`) torun da böylece kapsanır.
pub(crate) fn baslat_etkilesimli(komut: &mut Command) -> io::Result<(Child, Agac)> {
    // SAFETY: Kapanış `fork` ile `exec` arasında çocukta çalışır ve yalnızca
    // async-signal-safe `prctl`'ı çağırır. Ayar `exec`'ten sonra korunur.
    #[cfg(target_os = "linux")]
    unsafe {
        komut.pre_exec(|| {
            if libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    baslat(komut)
}
```

  `impl Agac`'tan sonra, Linux'un `Drop`'undan önce:

```rust
/// Etkileşimli sürecin ağacını öldürme ve bitişini doğrulama (1.1a §4.1).
/// `EtkilesimliSurec` bunu kullanır; testler sahtesini verir.
pub(crate) trait AgacDenetimi {
    /// Linux'ta kökü durdurur, kökün yaşayan torunlarını dondurup öldürür.
    /// Torun bulduysa `true` döner; çağıran, `false` dönene kadar yineler.
    /// Tarama tam yapılamazsa hata döner: eksik tarama "torun yok" sayılmaz.
    /// Yalnız kök henüz toplanmamışken (`wait`) çağrılır. Windows'ta
    /// torunları iş nesnesi kapsar; hep `false` döner.
    fn torunlari_oldur(&self) -> io::Result<bool>;
    /// Ağacı öldürür ve hatayı bildirir. Linux'ta zaten boş grup hata değildir.
    fn oldur_denetimli(&self) -> io::Result<()>;
    /// Ağaçta yaşayan süreç kalmadı mı? Linux'ta süreç grubunda (zombiler
    /// dahil) hiçbir süreç yoksa, Windows'ta işin etkin süreç sayısı sıfırsa.
    fn bos_mu(&self) -> io::Result<bool>;
}

impl AgacDenetimi for Agac {
    fn torunlari_oldur(&self) -> io::Result<bool> {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: `kill` yalnızca sayı ve sinyal alır. Kök toplanmamış
            // bir çocuktur; numarası ona aittir.
            unsafe {
                libc::kill(self.grup, libc::SIGSTOP);
            }
            torunlari_oldur_ile(
                &ProcFs {
                    kok: std::path::PathBuf::from("/proc"),
                },
                self.grup,
            )
        }
        #[cfg(not(target_os = "linux"))]
        Ok(false)
    }

    fn oldur_denetimli(&self) -> io::Result<()> {
        #[cfg(unix)]
        {
            // SAFETY: `durdur`'daki gibi.
            if unsafe { libc::killpg(self.grup, libc::SIGKILL) } == -1 {
                let e = io::Error::last_os_error();
                if e.raw_os_error() != Some(libc::ESRCH) {
                    return Err(e);
                }
            }
            Ok(())
        }
        #[cfg(windows)]
        self.is.sonlandir_denetimli()
    }

    fn bos_mu(&self) -> io::Result<bool> {
        #[cfg(unix)]
        {
            // SAFETY: 0 sinyali yalnızca varlığı sınar.
            if unsafe { libc::killpg(self.grup, 0) } == 0 {
                return Ok(false);
            }
            let e = io::Error::last_os_error();
            match e.raw_os_error() {
                Some(libc::ESRCH) => Ok(true),
                // Başka kullanıcının süreci gruptaysa (EPERM) grup boş değil.
                Some(libc::EPERM) => Ok(false),
                _ => Err(e),
            }
        }
        #[cfg(windows)]
        Ok(self.is.etkin_surec_sayisi()? == 0)
    }
}

/// `/proc/<pid>/stat`'tan okunan süreç kimliği.
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SurecBilgisi {
    pub pid: i32,
    pub ppid: i32,
    /// Açılıştan beri saat tıkı (`starttime`). Numara başka bir sürece
    /// verilirse değişir.
    pub baslangic: u64,
    pub zombi: bool,
}

/// Torun öldürmenin işletim sistemi kapısı; testler sahtesini verir.
#[cfg(target_os = "linux")]
pub(crate) trait SurecKaynagi {
    type Tutamac;
    /// Bütün süreçler. Tam okunamazsa hata döner.
    fn tara(&self) -> io::Result<Vec<SurecBilgisi>>;
    /// Süreci sabit bir başvuruyla açar; süreç yoksa `None`.
    fn ac(&self, pid: i32) -> io::Result<Option<Self::Tutamac>>;
    /// Açtıktan sonra yeniden okunan başlangıç; süreç yoksa `None`.
    fn baslangic(&self, pid: i32) -> io::Result<Option<u64>>;
    /// Açılan sürece sinyal; süreç bitmişse hata değildir.
    fn sinyal(&self, tutamac: &Self::Tutamac, sinyal: i32) -> io::Result<()>;
}

/// `kok`'ün zombi olmayan torunlarını bulur. Her adayın yaprağını pidfd
/// ile açıp başlangıcını, sonra köke kadar snapshot'taki her ebeveynin
/// başlangıcını doğrular. Değişmiş ya da kaybolmuş bir ara kimliğin alt dalı
/// bu taramayla sinyallenmez; sonraki tarama güncel ebeveynliği görür.
/// Kök toplanmadığı ve Linux'ta alt süreç toplayıcısı olduğu için doğrulama
/// sonrasında ara süreç çıkarsa yetimleri yine bu kökün altına bağlanır.
#[cfg(target_os = "linux")]
pub(crate) fn torunlari_oldur_ile<K: SurecKaynagi>(k: &K, kok: i32) -> io::Result<bool> {
    use std::collections::HashMap;

    let surecler = k.tara()?;
    let bilgiye_gore_pid: HashMap<i32, SurecBilgisi> =
        surecler.iter().map(|s| (s.pid, *s)).collect();
    let mut agac = vec![kok];
    let mut torunlar = Vec::new();
    let mut i = 0;
    while i < agac.len() {
        let ust = agac[i];
        for s in surecler.iter().filter(|s| s.ppid == ust) {
            if agac.contains(&s.pid) {
                continue;
            }
            agac.push(s.pid);
            if !s.zombi {
                torunlar.push(*s);
            }
        }
        i += 1;
    }
    let mut acik = Vec::new();
    for s in &torunlar {
        let Some(t) = k.ac(s.pid)? else {
            continue;
        };
        // Yaprak denetimi pidfd açıldıktan sonra sürer. Zincirin herhangi bir
        // eski halkası eşleşmezse aynı snapshot'taki alt dal hedef olamaz.
        let mut halka = s;
        loop {
            if k.baslangic(halka.pid)? != Some(halka.baslangic) {
                break;
            }
            if halka.ppid == kok {
                acik.push(t);
                break;
            }
            let Some(ebeveyn) = bilgiye_gore_pid.get(&halka.ppid) else {
                break;
            };
            halka = ebeveyn;
        }
    }
    for sinyal in [libc::SIGSTOP, libc::SIGKILL] {
        for t in &acik {
            k.sinyal(t, sinyal)?;
        }
    }
    Ok(!torunlar.is_empty())
}
/// Gerçek `/proc` ve `pidfd` (Linux 5.3+).
#[cfg(target_os = "linux")]
pub(crate) struct ProcFs {
    pub kok: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl SurecKaynagi for ProcFs {
    type Tutamac = std::os::fd::OwnedFd;

    fn tara(&self) -> io::Result<Vec<SurecBilgisi>> {
        let mut surecler = Vec::new();
        for girdi in std::fs::read_dir(&self.kok)? {
            let girdi = girdi?;
            let Some(pid) = girdi.file_name().to_str().and_then(|a| a.parse().ok()) else {
                continue;
            };
            if let Some(s) = stat_oku(&self.kok, pid)? {
                surecler.push(s);
            }
        }
        Ok(surecler)
    }

    fn ac(&self, pid: i32) -> io::Result<Option<Self::Tutamac>> {
        use std::os::fd::FromRawFd;
        // SAFETY: `pidfd_open` yalnız sayı alır; dönen tanımlayıcı hemen
        // sahiplenilir.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if fd < 0 {
            let e = io::Error::last_os_error();
            return match e.raw_os_error() {
                Some(libc::ESRCH) => Ok(None),
                _ => Err(e),
            };
        }
        // SAFETY: `fd` az önce açıldı ve başka bir sahibi yok.
        Ok(Some(unsafe {
            std::os::fd::OwnedFd::from_raw_fd(fd as i32)
        }))
    }

    fn baslangic(&self, pid: i32) -> io::Result<Option<u64>> {
        Ok(stat_oku(&self.kok, pid)?.map(|s| s.baslangic))
    }

    fn sinyal(&self, tutamac: &Self::Tutamac, sinyal: i32) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        // SAFETY: tanımlayıcı açık bir pidfd'dir; bilgi göstericisi boştur.
        let r = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                tutamac.as_raw_fd(),
                sinyal,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            )
        };
        if r < 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::ESRCH) {
                return Err(e);
            }
        }
        Ok(())
    }
}

/// `<kok>/<pid>/stat`. Kayıt yoksa süreç toplanmıştır: `None`. Başka okuma
/// hatası ya da ayrıştırılamayan kayıt hatadır.
#[cfg(target_os = "linux")]
fn stat_oku(kok: &std::path::Path, pid: i32) -> io::Result<Option<SurecBilgisi>> {
    let metin = match std::fs::read_to_string(kok.join(pid.to_string()).join("stat")) {
        Ok(m) => m,
        Err(e) if e.kind() == io::ErrorKind::NotFound || e.raw_os_error() == Some(libc::ESRCH) => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    };
    stat_ayristir(pid, &metin).map(Some).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{pid} sürecinin stat kaydı ayrıştırılamadı"),
        )
    })
}

/// `pid (ad) durum ppid … starttime …`: ad parantez içerebilir; son `)`'den
/// sonrası okunur. `starttime` 22. alandır.
#[cfg(target_os = "linux")]
fn stat_ayristir(pid: i32, metin: &str) -> Option<SurecBilgisi> {
    let (_, geri) = metin.rsplit_once(')')?;
    let alanlar: Vec<&str> = geri.split_whitespace().collect();
    let durum = *alanlar.first()?;
    Some(SurecBilgisi {
        pid,
        ppid: alanlar.get(1)?.parse().ok()?,
        baslangic: alanlar.get(19)?.parse().ok()?,
        zombi: durum == "Z" || durum == "X",
    })
}
```

  Dosyanın sonuna Linux birim testleri:

```rust
#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;

    use super::*;

    fn bilgi(pid: i32, ppid: i32, baslangic: u64, zombi: bool) -> SurecBilgisi {
        SurecBilgisi {
            pid,
            ppid,
            baslangic,
            zombi,
        }
    }

    /// Sahte süreç tablosu: taramadan sonra `simdiki` başlangıçlar okunur;
    /// gönderilen sinyaller kaydedilir.
    #[derive(Default)]
    struct Sahte {
        tablo: Vec<SurecBilgisi>,
        simdiki: HashMap<i32, u64>,
        tarama_hatasi: bool,
        acma_hatasi: bool,
        baslangic_hatasi: bool,
        sinyaller: RefCell<Vec<(i32, i32)>>,
    }

    impl SurecKaynagi for Sahte {
        type Tutamac = i32;

        fn tara(&self) -> io::Result<Vec<SurecBilgisi>> {
            if self.tarama_hatasi {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.tablo.clone())
        }

        fn ac(&self, pid: i32) -> io::Result<Option<i32>> {
            if self.acma_hatasi {
                return Err(io::Error::from(io::ErrorKind::Unsupported));
            }
            Ok(self.simdiki.contains_key(&pid).then_some(pid))
        }

        fn baslangic(&self, pid: i32) -> io::Result<Option<u64>> {
            if self.baslangic_hatasi {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(self.simdiki.get(&pid).copied())
        }
        fn sinyal(&self, t: &i32, sinyal: i32) -> io::Result<()> {
            self.sinyaller.borrow_mut().push((*t, sinyal));
            Ok(())
        }
    }

    /// Kök 10; 11 ve 12 torun (12 kendi oturumunda olabilir), 14 zombi
    /// torun, 13 ilgisiz.
    fn agac() -> Sahte {
        let tablo = vec![
            bilgi(11, 10, 100, false),
            bilgi(12, 11, 101, false),
            bilgi(13, 1, 102, false),
            bilgi(14, 11, 103, true),
        ];
        let simdiki = tablo.iter().map(|s| (s.pid, s.baslangic)).collect();
        Sahte {
            tablo,
            simdiki,
            ..Default::default()
        }
    }

    #[test]
    fn etkilesimli_torunlar_dondurulup_oldurulur_ilgisize_dokunulmaz() {
        let k = agac();
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        // Önce bütün torunlar durdurulur, sonra öldürülür.
        let beklenen = vec![
            (11, libc::SIGSTOP),
            (12, libc::SIGSTOP),
            (11, libc::SIGKILL),
            (12, libc::SIGKILL),
        ];
        assert_eq!(*k.sinyaller.borrow(), beklenen);
    }

    #[test]
    fn etkilesimli_numarasi_yeniden_kullanilan_toruna_sinyal_gitmez() {
        let mut k = agac();
        // 12 tarandıktan sonra bitti; numarası yeni bir sürece verildi.
        k.simdiki.insert(12, 999);
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert!(k.sinyaller.borrow().iter().all(|(p, _)| *p != 12));
        // 11 tarandıktan sonra toplandı: açılamaz, sinyal gitmez.
        let mut k = agac();
        k.simdiki.remove(&11);
        torunlari_oldur_ile(&k, 10).unwrap();
        assert!(k.sinyaller.borrow().iter().all(|(p, _)| *p != 11));
    }

    #[test]
    fn etkilesimli_yeniden_kullanilan_ara_ebeveynin_alt_dali_sinyal_almaz() {
        let mut k = agac();
        // 12'nin eski kimliği 11'in çocuğudur; yeni 12 ilgisiz 15'in ebeveynidir.
        k.tablo.push(bilgi(15, 12, 300, false));
        k.simdiki.insert(15, 300);
        k.simdiki.insert(12, 999);

        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        let sinyaller = k.sinyaller.borrow();
        assert!(
            sinyaller.iter().all(|(pid, _)| *pid != 12 && *pid != 15),
            "yeniden kullanılan ebeveynin alt dalına sinyal gitti: {sinyaller:?}"
        );
        assert!(
            sinyaller.iter().any(|(pid, _)| *pid == 11),
            "geçerli kök çocuğu hâlâ kapsanmalı: {sinyaller:?}"
        );
    }

    #[test]
    fn etkilesimli_kaybolan_ara_ebeveyn_eski_dali_atlar_yeni_tarama_yetimi_kapsar() {
        let mut k = agac();
        // İlk taramadan sonra 11 kaybolur; 12 yeniden ebeveynlenince yeni
        // /proc taraması onu doğrudan kökün çocuğu olarak görür.
        k.simdiki.remove(&11);

        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert!(
            k.sinyaller.borrow().is_empty(),
            "eski zincirden sinyal gitmemeli: {:?}",
            *k.sinyaller.borrow()
        );

        k.tablo = vec![bilgi(12, 10, 101, false)];
        assert!(torunlari_oldur_ile(&k, 10).unwrap());
        assert_eq!(
            *k.sinyaller.borrow(),
            vec![(12, libc::SIGSTOP), (12, libc::SIGKILL),]
        );
    }
    #[test]
    fn etkilesimli_tarama_acma_ve_baslangic_hatasi_basari_sayilmaz() {
        let mut k = agac();
        k.tarama_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        let mut k = agac();
        k.acma_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        assert!(k.sinyaller.borrow().is_empty());
        let mut k = agac();
        k.baslangic_hatasi = true;
        assert!(torunlari_oldur_ile(&k, 10).is_err());
        assert!(k.sinyaller.borrow().is_empty());
    }
    #[test]
    fn etkilesimli_torunu_olmayan_kok_false_doner() {
        let k = agac();
        assert!(!torunlari_oldur_ile(&k, 13).unwrap());
        assert!(k.sinyaller.borrow().is_empty());
    }

    fn stat_yaz(kok: &Path, pid: i32, metin: &str) {
        std::fs::create_dir_all(kok.join(pid.to_string())).unwrap();
        std::fs::write(kok.join(pid.to_string()).join("stat"), metin).unwrap();
    }

    fn stat_metni(pid: i32, ad: &str, durum: &str, ppid: i32, baslangic: u64) -> String {
        // 22. alan `starttime`; arada 17 alan.
        format!(
            "{pid} ({ad}) {durum} {ppid} {} {baslangic} 0 0\n",
            vec!["0"; 17].join(" ")
        )
    }

    #[test]
    fn etkilesimli_stat_parantezli_adla_ayristirilir() {
        let s = stat_ayristir(7, &stat_metni(7, "a) (b", "S", 3, 4242)).unwrap();
        assert_eq!(s, bilgi(7, 3, 4242, false));
        assert!(
            stat_ayristir(7, &stat_metni(7, "z", "Z", 3, 1))
                .unwrap()
                .zombi
        );
        assert!(stat_ayristir(7, "7 (kesik").is_none());
        assert!(stat_ayristir(7, "7 (a) S x 0").is_none());
    }

    #[test]
    fn etkilesimli_proc_okuma_hatalari_tarama_hatasidir() {
        let d = tempfile::tempdir().unwrap();
        let k = ProcFs {
            kok: d.path().to_path_buf(),
        };
        stat_yaz(d.path(), 11, &stat_metni(11, "a", "S", 10, 5));
        // Kaydı olmayan süreç toplanmıştır; atlanır.
        std::fs::create_dir_all(d.path().join("12")).unwrap();
        std::fs::create_dir_all(d.path().join("self")).unwrap();
        assert_eq!(k.tara().unwrap(), vec![bilgi(11, 10, 5, false)]);
        // Okunamayan kayıt (burada dizin) ve bozuk kayıt hatadır.
        std::fs::create_dir_all(d.path().join("13/stat")).unwrap();
        assert!(k.tara().is_err());
        std::fs::remove_dir_all(d.path().join("13")).unwrap();
        stat_yaz(d.path(), 14, "14 (bozuk");
        assert!(k.tara().is_err());
        // `/proc` açılamıyor.
        let yok = ProcFs {
            kok: d.path().join("yok"),
        };
        assert!(yok.tara().is_err());
    }
}
```

  Windows modülünde `use windows_sys::Win32::System::JobObjects::{…}`
  listesine `JOBOBJECT_BASIC_ACCOUNTING_INFORMATION`,
  `JobObjectBasicAccountingInformation` ve `QueryInformationJobObject`
  eklenir. `impl Is`'in içinde, `sonlandir`'dan sonra:

```rust
        /// `sonlandir` gibi; başarısızlığı bildirir. Sonlandırma eşzamansızdır:
        /// bitiş `etkin_surec_sayisi` ile ayrıca doğrulanır.
        pub fn sonlandir_denetimli(&self) -> io::Result<()> {
            // SAFETY: `self.0` bu nesnenin sahip olduğu açık iş tutamacıdır.
            if unsafe { TerminateJobObject(self.0, 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        /// İşte henüz bitmemiş süreçlerin sayısı.
        pub fn etkin_surec_sayisi(&self) -> io::Result<u32> {
            // SAFETY: Yapı yalnız sayısal alanlardan oluşur; sıfır geçerli bir değerdir.
            let mut bilgi: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = unsafe { std::mem::zeroed() };
            // SAFETY: Açık iş tutamacı ve yapının boyutuna uygun yazılabilir gösterici verilir.
            let tamam = unsafe {
                QueryInformationJobObject(
                    self.0,
                    JobObjectBasicAccountingInformation,
                    (&raw mut bilgi).cast(),
                    size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                    std::ptr::null_mut(),
                )
            };
            if tamam == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(bilgi.ActiveProcesses)
        }
```

- [x] **Adım 4: `surec.rs`'e ekleyin** (`async fn oku`'dan önce):

```rust
type Yazici = Box<dyn tokio::io::AsyncWrite + Send + Unpin>;

/// Ağaç öldürüldükten sonra bitişin doğrulanması için tanınan en uzun süre.
const OLUM_BEKLEMESI: Duration = Duration::from_secs(5);
/// Kalfa'nın stdin'e aktarmayı beklettiği en çok veri.
const GIRDI_TAMPONU: usize = 64 * 1024;
const YOKLAMA: Duration = Duration::from_millis(10);

/// Uzun yaşayan, borulu süreç (1.1a §4.1). Bırakılırsa (örneğin panik
/// sırasında) ağaç yine öldürülür; bitişi ise yalnız `kapat` doğrular.
pub struct EtkilesimliSurec {
    girdi: Option<Yazici>,
    /// stdin borusuna aktaran görev. Borunun yazma ucu yalnız bu görevdedir:
    /// `kapat` görevi keser ve boruyu kapatır; dışarıya verilen yazıcı o
    /// andan sonra hata alır.
    girdi_gorevi: Option<tokio::task::JoinHandle<()>>,
    cikti: Option<Okuyucu>,
    hata: Option<Okuyucu>,
    bitisi: Option<Bitisi>,
    cikis: Option<Option<i32>>,
    /// `agac`'tan önce bırakılır: Windows'ta boru sunucusu işin tutamacını
    /// kullanır.
    ag_kapat: Option<AgKapatici>,
    agac: Agac,
    /// Yalnız `kapat` ağacın bitişini doğruladıktan sonra `true` olur.
    /// İptal edilen ya da hata veren kapanışta `Drop` temizliği yapar.
    dogrulandi: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KapanisSonucu {
    /// Süreç, stdin kapandıktan sonra pay içinde kendiliğinden çıktı.
    pub kendiliginden: bool,
    pub cikis: Option<i32>,
    pub sure: Duration,
    pub ag: Option<crate::vekil::AgOzeti>,
}

fn zaman_asimi(ne: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::TimedOut, ne.to_string())
}

impl EtkilesimliSurec {
    fn yeni(girdi: Option<Yazici>, cocuk: Cocuk, agac: Agac) -> Result<Self, BaslatmaHatasi> {
        let (cikti, hata, bitisi) = cocuk.ayir();
        let (girdi, girdi_gorevi) = match girdi {
            Some(mut boru) => {
                let calisma = tokio::runtime::Handle::try_current().map_err(|_| {
                    BaslatmaHatasi::Yalitim(
                        "etkileşimli süreç tokio çalışma zamanının dışında başlatılamaz".into(),
                    )
                })?;
                let (dis, mut ic) = tokio::io::duplex(GIRDI_TAMPONU);
                let gorev = calisma.spawn(async move {
                    let _ = tokio::io::copy(&mut ic, &mut boru).await;
                    let _ = tokio::io::AsyncWriteExt::shutdown(&mut boru).await;
                });
                (Some(Box::new(dis) as Yazici), Some(gorev))
            }
            None => (None, None),
        };
        Ok(Self {
            girdi,
            girdi_gorevi,
            cikti,
            hata,
            bitisi: Some(bitisi),
            cikis: None,
            ag_kapat: None,
            agac,
            dogrulandi: false,
        })
    }

    /// stdin'in yazıcısı. Bırakılınca süreç EOF alır; `kapat` da yazıcı
    /// dışarıda tutulsa bile stdin'i kapatır.
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

    /// Kök süreç kendiliğinden bitene kadar bekler. İptal güvenlidir: gelecek
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
    /// beklenir, ağaç her durumda öldürülür. `Ok` döndüğünde ağaçta yaşayan
    /// süreç yoktur: Linux'ta süreç grubu boştur ve kök yaşarken bulunan
    /// bütün torunlar ölmüştür; Windows'ta işin etkin süreç sayısı sıfırdır.
    /// Öldürme ya da doğrulama başarısızsa `Err` döner; çağıran, ağacın
    /// durduğunu varsayamaz.
    pub async fn kapat(mut self) -> std::io::Result<KapanisSonucu> {
        let bas = std::time::Instant::now();
        self.girdiyi_kes();
        let kendiliginden = self.cikis.is_some()
            || tokio::time::timeout(crate::etkilesimli::KAPANMA_PAYI, self.bitisi_bekle())
                .await
                .is_ok();
        let sonuc = self.agaci_bitir().await;
        let ag = match self.ag_kapat.take() {
            Some(k) => Some(k().await),
            None => None,
        };
        sonuc?;
        self.dogrulandi = true;
        Ok(KapanisSonucu {
            kendiliginden,
            cikis: self.cikis.flatten(),
            sure: bas.elapsed(),
            ag,
        })
    }

    /// stdin'i kapatır: aktarma görevi kesilir, dışarıdaki yazıcı hata alır.
    fn girdiyi_kes(&mut self) {
        drop(self.girdi.take());
        if let Some(g) = self.girdi_gorevi.take() {
            g.abort();
        }
    }

    async fn agaci_bitir(&mut self) -> std::io::Result<()> {
        let son = Instant::now() + OLUM_BEKLEMESI;
        // Kök toplanmadıysa numarası ona aittir; torunlar köke bağlıdır.
        agaci_oldur(&self.agac, self.cikis.is_none(), son).await?;
        tokio::time::timeout_at(son, self.bitisi_bekle())
            .await
            .map_err(|_| zaman_asimi("kök süreç öldürüldükten sonra bitmedi"))?;
        bosalmayi_bekle(&self.agac, son).await
    }
}

/// Kök toplanmadıysa önce torunları, sonra ağacı öldürür.
async fn agaci_oldur(
    agac: &impl AgacDenetimi,
    kok_toplanmadi: bool,
    son: Instant,
) -> std::io::Result<()> {
    if kok_toplanmadi {
        while agac.torunlari_oldur()? {
            if Instant::now() >= son {
                return Err(zaman_asimi("torun süreçler öldürülemedi"));
            }
            tokio::time::sleep(YOKLAMA).await;
        }
    }
    agac.oldur_denetimli()
}

/// Ağaçta yaşayan süreç kalmayana kadar yoklar.
async fn bosalmayi_bekle(agac: &impl AgacDenetimi, son: Instant) -> std::io::Result<()> {
    while !agac.bos_mu()? {
        if Instant::now() >= son {
            return Err(zaman_asimi("süreç ağacı öldürüldükten sonra boşalmadı"));
        }
        tokio::time::sleep(YOKLAMA).await;
    }
    Ok(())
}

impl Drop for EtkilesimliSurec {
    fn drop(&mut self) {
        // `kapat` çağrılmadıysa (örneğin panik), iptal edildiyse ya da hata
        // verdiyse: stdin görevi her durumda kesilir. Bitiş doğrulanmadıysa
        // bekleyemeyiz, ama kök toplanmadıysa torunlar da öldürülür.
        self.girdiyi_kes();
        if self.dogrulandi {
            return;
        }
        if self.cikis.is_none() {
            let son = std::time::Instant::now() + Duration::from_millis(500);
            while matches!(self.agac.torunlari_oldur(), Ok(true)) && std::time::Instant::now() < son
            {
                std::thread::sleep(YOKLAMA);
            }
        }
        let _ = self.agac.oldur_denetimli();
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
        let (mut cocuk, agac) = agac::baslat_etkilesimli(&mut komut)?;
        let girdi = cocuk.stdin.take().map(|g| Box::new(g) as Yazici);
        EtkilesimliSurec::yeni(girdi, Cocuk::Tokio(Box::new(cocuk)), agac)
    }
}

/// Tam erişimle, yalıtımsız başlatma (§4.1). Linux ve Windows arka uçları
/// kullanır.
pub(crate) fn tam_erisimle(
    istek: &crate::EtkilesimliIstek,
    yalitim_dizini: &std::path::Path,
    git: &crate::profil::GitKimligi,
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
    ozel.hazirla(git)?;
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

  `lib.rs`'te `pub use surec::{…}` satırına `EtkilesimliSurec` ve
  `KapanisSonucu` eklenir. `surec.rs`'in `use crate::agac::{self, Agac};`
  satırı `use crate::agac::{self, Agac, AgacDenetimi};` olur. `Yalitici`
  trait'ine `baslat`'tan sonra şunlar eklenir:

```rust
    /// MCP sunucusu gibi uzun yaşayan, borulu süreç (1.1a §4.1). Tokio
    /// çalışma zamanının içinden çağrılır.
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

- [x] **Adım 5: Geçtiğini görün:** İki platformda (Linux'ta WSL,
  `kalfatest`) `cargo test -p kalfa-sandbox --locked --lib surec::tests`.
  Beklenen: Windows'ta 12, Linux'ta 13 test geçer (`setsid` testi yalnız
  Linux'ta). Linux'ta ayrıca
  `cargo test -p kalfa-sandbox --locked --lib agac::tests`: 6 test geçer.
- [x] **Adım 6: Commit'leyin:** `sandbox: EtkilesimliSurec, doğrulanan ağaç kapanışı ve tam erişim`.

---

### Görev 3: Linux — yalıtımlı etkileşimli süreç

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/linux/mod.rs`
- Test: `crates/kalfa-sandbox/tests/yalitim.rs` (Linux bloğu)

**Arayüzler:**
- Kullanır: Görev 1–2; mevcut `komut_kur`, `yalitim_ortami`, `bwrap::secenekler`.
- Üretir: `KurIstegi`, `vekili_bagla`, `LinuxYalitici::sunucuyu_baslat`;
  `LinuxYalitici` için `baslat_etkilesimli` ve `sunucu_kokunu_kaldir`.

- [x] **Adım 1: `komut_kur`'u özel kökü dışarıdan alacak biçime getirin.**
  `komut_kur`'un imzası ve ilk satırları şununla değişir. Gövdedeki
  `OzelDizinler::yol(…)`, `ozel.hazirla(…)` ve `korumalilari_hazirla(…)`
  satırları silinir; çağıran yapar.

```rust
/// `komut_kur`'un girdisi. `ozel`, kabukta çalışma alanının, etkileşimli
/// süreçte sunucunun özel köküdür (1.1a §3); hazırlanmış olmalıdır.
struct KurIstegi<'a> {
    komut: &'a KomutBicimi,
    dizin: &'a Path,
    ortam: &'a [(String, String)],
    ag: bool,
    alan: &'a AlanBilgisi,
    ozel: &'a OzelDizinler,
}

/// bwrap komutunu kurar. Dönen tanımlayıcılar komut başlayana kadar açık
/// kalmalıdır.
fn komut_kur(
    bwrap: &Path,
    ayar: &LinuxAyari,
    istek: &KurIstegi,
) -> Result<(std::process::Command, Vec<OwnedFd>, Option<OwnedFd>), BaslatmaHatasi> {
    let alan = istek.alan;
    let ozel = istek.ozel;
```

  Gövdede `registry_secimi(istek.ag, &ozel, ayar)` →
  `registry_secimi(istek.ag, ozel, ayar)`,
  `yalitim_ortami(&istek.ortam, &ozel, ayar)` →
  `yalitim_ortami(istek.ortam, ozel, ayar)`, `match &istek.komut` →
  `match istek.komut`, `Kurulum { … ozel: &ozel, dizin: &istek.dizin, … }`
  → `Kurulum { … ozel, dizin: istek.dizin, … }` olur.

  `deneme` (`/usr/bin/true` denemesi) artık `CalistirmaIstegi` kurmaz.
  `let istek = CalistirmaIstegi { … };` ve ardından gelen `komut_kur`
  çağrısı şununla değişir:

```rust
    let alan = AlanBilgisi {
        kok: kok.clone(),
        gizli: Default::default(),
        korumali: Vec::new(),
        git_dizinleri: Vec::new(),
    };
    let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&kok));
    ozel.hazirla(&ayar.git).map_err(|e| e.to_string())?;
    let komut = KomutBicimi::Program {
        program: "true".into(),
        argumanlar: Vec::new(),
    };
    let (mut komut, _fdler, _vekil_kanali) = komut_kur(
        bwrap,
        &ayar,
        &KurIstegi {
            komut: &komut,
            dizin: &kok,
            ortam: &[("PATH".into(), "/usr/bin:/bin".into())],
            ag: false,
            alan: &alan,
            ozel: &ozel,
        },
    )
    .map_err(|e| e.to_string())?;
```

  `yalitimli` ve ortak vekil kurulumu (`if istek.ag { … }` bloğu
  `vekili_bagla`'ya taşınır):

```rust
    fn yalitimli(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let ayar = self.ayar();
        let alan = istek
            .alan
            .as_ref()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("çalışma alanı bilgisi yok".into()))?;
        let ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &alan_kimligi(&alan.kok));
        ozel.hazirla(&ayar.git)?;
        korumalilari_hazirla(&alan.korumali)?;
        let (komut, fdler, vekil_kanali) = komut_kur(
            bwrap,
            &ayar,
            &KurIstegi {
                komut: &istek.komut,
                dizin: &istek.dizin,
                ortam: &istek.ortam,
                ag: istek.ag,
                alan,
                ozel: &ozel,
            },
        )?;
        let mut surec = Surec::komutla(komut.into())?;
        // Bellek dosyaları ve çocuğun kanalı çocuğa geçti; buradaki kopyalar kapanabilir.
        drop(fdler);
        if let Some(kapat) = vekili_bagla(&ayar, vekil_kanali, istek.ag_listesi.clone()) {
            surec.ag_kapat_ekle(kapat);
        }
        Ok(surec)
    }
```

```rust
/// Ağlı çalıştırmada vekili başlatır ve köprünün kanalına bağlar (§8.5).
/// Kanal yoksa (ağsız) bir şey yapmaz. Dönen kapatıcı kanalı ve vekili
/// kapatıp ağ özetini verir.
fn vekili_bagla(
    ayar: &LinuxAyari,
    vekil_kanali: Option<OwnedFd>,
    liste: Option<crate::vekil::IzinListesi>,
) -> Option<crate::surec::AgKapatici> {
    let vekil_kanali = vekil_kanali?;
    let ayar_vekil = crate::vekil::VekilAyari {
        liste: liste.unwrap_or_else(|| crate::vekil::IzinListesi::yeni(&[], &[])),
        cozucu: ayar
            .vekil_cozucu
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(crate::vekil::SistemCozucu)),
        yerel_izin: ayar.vekil_yerel_izin,
        ..Default::default()
    };
    let vekil = std::sync::Arc::new(crate::vekil::Vekil::baslat(ayar_vekil));
    let vekil_icin = vekil.clone();
    // Okuyucu ve kapatıcı aynı tanımlayıcıyı paylaşır; son sahibi
    // bırakınca kapanır. Ham numarayla paylaşılsaydı kapatıcının
    // kapattığı numara yeniden kullanılıp okuyucuya başka bir dosya
    // olarak görünebilirdi.
    let vekil_kanali = std::sync::Arc::new(vekil_kanali);
    let okuyucu_kanali = vekil_kanali.clone();

    tokio::task::spawn_blocking(move || {
        loop {
            let fd = match crate::vekil::kanal::al(okuyucu_kanali.as_raw_fd()) {
                Ok(Some(fd)) => fd,
                Ok(None) => break,
                // Tek tanımlayıcılı olmayan ya da kesik ileti reddedilir;
                // kanal açık kaldıkça sonraki iletiler okunur (§8.5).
                Err(e) if e.kind() == io::ErrorKind::InvalidData => continue,
                Err(_) => break,
            };
            // Bağlı bir TCP soketi olmayan tanımlayıcı burada kapanır.
            if crate::vekil::kanal::dogrula(&fd).is_err() {
                continue;
            }
            let std_stream = std::net::TcpStream::from(fd);
            if std_stream.set_nonblocking(true).is_ok()
                && let Ok(tokio_stream) = tokio::net::TcpStream::from_std(std_stream)
            {
                vekil_icin.baglanti_ver(tokio_stream);
            }
        }
    });

    let ag_kapatici: crate::surec::AgKapatici = Box::new(move || {
        Box::pin(async move {
            // SAFETY: tanımlayıcı `vekil_kanali` yaşadıkça açıktır;
            // `shutdown` okuyucunun `recvmsg`'ini 0 ile döndürür.
            unsafe {
                libc::shutdown(vekil_kanali.as_raw_fd(), libc::SHUT_RDWR);
            }
            drop(vekil_kanali);
            vekil.bitir().await
        })
    });
    Some(ag_kapatici)
}
```

- [x] **Adım 2: Mevcut kabuk testleri yeşil kalmalı** (WSL, `kalfatest`):
  `cargo build -p kalfa-sandbox --bin kalfa-kopru --locked`, ardından
  `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim`.
  Bu adım yalnız yeniden düzenlemedir; davranış değişmez.

- [x] **Adım 3: Düşen kabul testlerini yazın** (`tests/yalitim.rs`, Linux
  bloğunun kapanışından önce). `kurgu`, `kurgu_agli`, `TestCozucu`,
  `gizli_mi`, `korumali`, `surecler`, `deger` ve `Kurgu::betik` mevcut
  yardımcılardır.

```rust
// --- 1.1a-1: etkileşimli süreç (G1'in ürün başlatıcısıyla karşılıkları) ---

fn etkilesimli_istek(
    k: &Kurgu,
    program: &str,
    arg: &[&str],
    sunucu: &str,
) -> kalfa_sandbox::EtkilesimliIstek {
    kalfa_sandbox::EtkilesimliIstek {
        program: program.into(),
        argumanlar: arg.iter().map(|s| s.to_string()).collect(),
        ortam: vec![
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", k.ev.join(".cargo/bin").display()),
            ),
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

async fn tum_cikti(s: &mut kalfa_sandbox::EtkilesimliSurec) -> String {
    let mut m = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut m)
        .await
        .unwrap();
    m
}

/// G1.1 (ürün başlatıcısıyla): 1 000 JSON-RPC satırı, 1 MiB'lık satır ve
/// UTF-8 birebir; EOF'ta çıkış.
#[tokio::test]
async fn etkilesimli_borular_birebir_yankilanir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "cat", &[], "yanki"))
            .unwrap();
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
    });
    let mut alinan = Vec::new();
    while let Some(s) = cikti.next_line().await.unwrap() {
        alinan.push(s);
    }
    yazici.await.unwrap();
    assert_eq!(alinan, gonderilen);
    assert_eq!(s.bitisi_bekle().await, Some(0));
    assert!(s.kapat().await.unwrap().kendiliginden);
}

/// G1.2 (ürün başlatıcısıyla): kök ve kendi oturumundaki torun yaşar,
/// ping'lere yanıt verilir, EOF yok sayılır; `kapat` döndüğünde ikisi de
/// yaşamaz.
#[tokio::test]
async fn etkilesimli_uzun_yasar_kapat_agaci_durdurur() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let isaret = format!("kalfa-g1-{}", std::process::id());
    // İşaretler `$0`'dan kurulur; betiğin metninde (bwrap'ın komut
    // satırında) geçmez.
    let betik = "setsid perl -e 'sleep 600' \"$0-torun\" </dev/null >/dev/null 2>&1 & \
                 trap '' TERM HUP; echo hazir; \
                 while read l; do echo \"pong $l\"; done; \
                 exec perl -e '$SIG{TERM}=q(IGNORE); sleep 600' \"$0-kok\"";
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", betik, &isaret], "uzun"))
            .unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    assert_eq!(cikti.next_line().await.unwrap().as_deref(), Some("hazir"));
    for i in 1..=5 {
        girdi.write_all(format!("{i}\n").as_bytes()).await.unwrap();
        assert_eq!(cikti.next_line().await.unwrap(), Some(format!("pong {i}")));
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    drop(girdi);
    let torun = format!("{isaret}-torun");
    let kok = format!("{isaret}-kok");
    let son = Instant::now() + Duration::from_secs(5);
    while surecler(&kok).is_empty() && Instant::now() < son {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(surecler(&torun).len(), 1, "torun yaşamalı");
    assert_eq!(surecler(&kok).len(), 1, "kök yaşamalı");
    let bas = Instant::now();
    let kapanis = s.kapat().await.unwrap();
    assert!(!kapanis.kendiliginden, "EOF'u yok sayan süreç kendiliğinden çıkmamalı");
    assert!(bas.elapsed() < Duration::from_secs(5), "{:?}", bas.elapsed());
    assert!(surecler(&torun).is_empty() && surecler(&kok).is_empty());
}

/// G1.3 ve §3: sunucunun özel kökü kabuğa görünmez; kabuğun özel ev'i
/// sunucuya görünmez; sunucu kendi köküne yazar. Aynı alanda iki sunucu
/// birbirini göremez (Review Focus 4). Kaldırma kökü siler.
#[tokio::test]
async fn etkilesimli_sunucu_ozel_koku_kabuktan_ve_obur_sunucudan_ayri() {
    let k = kurgu();
    let kimlik = kalfa_sandbox::alan_kimligi(&k.kok);
    let ydir = k.taban.join("veri/yalitim");
    let a = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "a");
    let b = kalfa_sandbox::sunucu_ozel(&ydir, &kimlik, "b");
    let kabuk_ev = kalfa_sandbox::OzelDizinler::yol(&ydir, &kimlik).ev;
    let s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "true", &[], "b"))
            .unwrap();
    s.kapat().await.unwrap();
    // Kabuğun özel dizinleri kurulsun.
    k.betik(&k.kok, "true", false).await;
    assert!(b.ev.is_dir() && kabuk_ev.is_dir());
    let betik = format!(
        "touch \"$HOME/yazildi\" && echo kendi=TAMAM; \
         test -e '{}' && echo kabuk=GORUNUR || echo kabuk=YOK; \
         test -e '{}' && echo obur=GORUNUR || echo obur=YOK",
        kabuk_ev.display(),
        b.kok.display()
    );
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&k, "sh", &["-c", &betik], "a"))
            .unwrap();
    let metin = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    assert!(metin.contains("kendi=TAMAM"), "{metin}");
    assert!(metin.contains("kabuk=YOK"), "{metin}");
    assert!(metin.contains("obur=YOK"), "{metin}");
    assert!(a.ev.join("yazildi").exists());
    let sonuc = k
        .betik(
            &k.kok,
            &format!("test -e '{}' && echo GORUNUR || echo YOK", a.kok.display()),
            false,
        )
        .await;
    assert!(sonuc.standart.contains("YOK"), "{}", sonuc.standart);
    k.y.sunucu_kokunu_kaldir(&k.kok, "a").unwrap();
    assert!(!a.kok.exists() && b.kok.exists());
}

/// G1 bulgusu ve §3: ortam özel köke yönelir; `EVREN_`/`KALFA_` adları
/// geçmez, yapılandırılmış belirteç geçer.
#[tokio::test]
async fn etkilesimli_ortam_ozel_koke_yonelir() {
    let k = kurgu();
    let mut i = etkilesimli_istek(&k, "env", &[], "ortam");
    i.ortam.push(("EVREN_API_KEY".into(), "evren_llm_x".into()));
    i.ortam.push(("KALFA_E2E".into(), "1".into()));
    i.ortam.push(("GITHUB_TOKEN".into(), "ghp_deneme".into()));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let metin = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    let ozel = kalfa_sandbox::sunucu_ozel(
        &k.taban.join("veri/yalitim"),
        &kalfa_sandbox::alan_kimligi(&k.kok),
        "ortam",
    );
    assert_eq!(deger(&metin, "HOME"), ozel.ev.display().to_string());
    assert_eq!(
        deger(&metin, "NPM_CONFIG_CACHE"),
        ozel.npm().display().to_string()
    );
    assert_eq!(
        deger(&metin, "UV_CACHE_DIR"),
        ozel.onbellek.join("uv").display().to_string()
    );
    assert_eq!(deger(&metin, "GITHUB_TOKEN"), "ghp_deneme");
    assert!(
        !metin
            .lines()
            .any(|s| s.starts_with("EVREN_") || s.starts_with("KALFA_")),
        "{metin}"
    );
}

/// §4.1: tam erişim, yalıtım hazırken yalıtımsız başlar (ev'deki sırrı
/// okur); yalıtımlı aynı sırrı okuyamaz.
#[tokio::test]
async fn etkilesimli_tam_erisim_yalitim_hazirken_yalitimsiz_baslar() {
    let k = kurgu();
    let sir = k.ev.join(".ssh/id_rsa");
    for (kip, gorunur) in [
        (kalfa_sandbox::CalistirmaKipi::TamErisim, true),
        (kalfa_sandbox::CalistirmaKipi::Yalitimli, false),
    ] {
        let mut i = etkilesimli_istek(&k, "cat", &[sir.to_str().unwrap()], "kip");
        i.kip = kip;
        let mut s = k.y.baslat_etkilesimli(&i).unwrap();
        let metin = tum_cikti(&mut s).await;
        s.kapat().await.unwrap();
        assert_eq!(metin.contains("GIZLI-SSH"), gorunur, "{kip:?}: {metin}");
    }
}

/// Codex 2: ağlı sunucu köprüyle başlar. stdin/stdout köprüden geçer;
/// izinli hedefe vekilden ulaşılır, listede olmayan reddedilir. `kapat`
/// döndüğünde köprü dahil süreç kalmaz ve ağ özeti gelir.
#[tokio::test]
async fn etkilesimli_agli_sunucu_vekilden_cikar_ve_kapanista_temizlenir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let n = soket.read(&mut buf).await.unwrap_or(0);
            if String::from_utf8_lossy(&buf[..n]).starts_with("CONNECT") {
                let _ = soket
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await;
            } else {
                let _ = soket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nSELAM",
                    )
                    .await;
            }
        }
    });
    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("fake.crates.io".to_string(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let k = kurgu_agli(Some(std::sync::Arc::new(TestCozucu { eslemeler })), true);
    let isaret = format!("kalfa-agli-{}", std::process::id());
    let betik = format!(
        "echo \"ayrilmis=$(env | grep -c -e ^KALFA_ -e ^EVREN_)\"; \
         while read l; do case \"$l\" in \
           izinli) curl -p -s -S http://fake.crates.io:{port}/; echo ;; \
           yasak) curl -s -i http://evil.com/ | head -n 1 ;; \
           *) echo \"pong $l\" ;; esac; done"
    );
    let mut i = etkilesimli_istek(&k, "sh", &["-c", &betik, &isaret], "agli");
    i.ag_listesi = Some(kalfa_sandbox::vekil::IzinListesi::yeni(
        &[],
        &[("fake.crates.io".to_string(), port)],
    ));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    assert_eq!(
        cikti.next_line().await.unwrap().as_deref(),
        Some("ayrilmis=0")
    );
    let mut sor = async |satir: &str| {
        girdi
            .write_all(format!("{satir}\n").as_bytes())
            .await
            .unwrap();
        cikti.next_line().await.unwrap().unwrap()
    };
    assert_eq!(sor("1").await, "pong 1");
    assert!(sor("izinli").await.contains("SELAM"));
    assert!(sor("yasak").await.contains("403"));
    assert!(!surecler(&isaret).is_empty(), "köprü ve sunucu yaşamalı");
    let kapanis = s.kapat().await.unwrap();
    assert!(surecler(&isaret).is_empty(), "kapanıştan sonra süreç kaldı");
    let ag = kapanis.ag.expect("ağ özeti dönmeli");
    let karar = |host: &str, k: kalfa_sandbox::vekil::AgKarari| {
        ag.baglantilar.iter().any(|b| b.host == host && b.karar == k)
    };
    assert!(
        karar("fake.crates.io", kalfa_sandbox::vekil::AgKarari::Izinli),
        "{:?}",
        ag.baglantilar
    );
    assert!(
        karar("evil.com", kalfa_sandbox::vekil::AgKarari::ListedeYok),
        "{:?}",
        ag.baglantilar
    );
}
```

- [x] **Adım 4: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim etkilesimli`.
  Beklenen: `baslat_etkilesimli` varsayılanı "bu yalıtıcı etkileşimli
  süreç başlatamaz" döner.

- [x] **Adım 5: Uygulayın.** `impl LinuxYalitici`'ye, `yalitimli`'den sonra:

```rust
    /// Sunucuyu yalıtımda başlatır (1.1a §3, §4.1): kabuğun bwrap kurulumu,
    /// ama özel kök, çalışma dizini (`ev`) ve ortam sunucunundur. Kabuğun
    /// özel dizinleri sunucuya bağlanmaz.
    fn sunucuyu_baslat(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        let bwrap = bwrap_yolu()
            .ok_or_else(|| BaslatmaHatasi::Yalitim("bwrap artık sistem yollarında yok".into()))?;
        let ayar = self.ayar();
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
        let (komut, fdler, vekil_kanali) = komut_kur(
            bwrap,
            &ayar,
            &KurIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                ag: istek.ag_listesi.is_some(),
                alan: &istek.alan,
                ozel: &ozel,
            },
        )?;
        let mut surec = Surec::komutla_etkilesimli(komut.into())?;
        drop(fdler);
        if let Some(kapat) = vekili_bagla(&ayar, vekil_kanali, istek.ag_listesi.clone()) {
            surec.ag_kapat_ekle(kapat);
        }
        Ok(surec)
    }
```

  `impl Yalitici for LinuxYalitici`'ye, `yeniden_algila`'dan önce:

```rust
    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        match istek.kip {
            crate::CalistirmaKipi::TamErisim => {
                let ayar = self.ayar();
                crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini, &ayar.git)
            }
            crate::CalistirmaKipi::Yalitimli => self.sunucuyu_baslat(istek),
        }
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kok = crate::sunucu_ozel(
            &self.ayar().yalitim_dizini,
            &alan_kimligi(alan_koku),
            sunucu,
        )
        .kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
```

  `komut_kur`'un kurduğu bwrap; `ozel.ev`, `ozel.onbellek` ve `/tmp`'ye
  `ozel.gecici`'yi bağlar. Burada `ozel` sunucunun köküdür, dolayısıyla
  kabuğun özel dizinleri bağlanmaz. Proje kökü kabuktaki gibi yazılabilir
  bağlanır. `--chdir`, sunucunun `ev`'idir.

- [x] **Adım 6: Geçtiğini görün** (WSL, `kalfatest`):
  `cargo clippy -p kalfa-sandbox --features yalitim-testleri --all-targets --locked -- -D warnings`
  ve `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim`.
  Beklenen: clippy temiz; yeni 6 test ve bütün eski testler geçer
  (toplam 32).
- [x] **Adım 7: Commit'leyin:** `sandbox: Linux'ta yalıtımlı etkileşimli süreç ve sunucu özel kökü`.

---

### Görev 4: Windows — borulu stdin, korumalı kök, yetenek SID'i, köprülü ağ

**Dosyalar:**
- Değiştir: `crates/kalfa-sandbox/src/windows/kap.rs`, `src/windows/yalitici.rs`, `src/surec.rs`
- Test: `crates/kalfa-sandbox/tests/yalitim/windows.rs`

**Arayüzler:**
- Kullanır: Görev 1–2; mevcut `guvenlik::{yetenek_sidi, profil_olustur, arac_zinciri_sidi, korumali_dizin_olustur, mirasi_kesik_mi, yeniden_ayristirma_izlemeden_ac, aceler_oku}`,
  `izin_kaydet_ve_ver`, `kayda_gore_geri_al`, `AceKaydi::alani_cikar`, `kopru::BoruSunucusu`.
- Üretir:
  - `kap::Kap { …, girdi: Option<File> }`, `pub(crate) fn kap::baslat_etkilesimli(&KapIstegi, &Is) -> io::Result<Kap>`
  - `pub(crate) fn EtkilesimliSurec::kaptan(Kap, Is) -> Result<EtkilesimliSurec, BaslatmaHatasi>`
  - `fn ag_hazirla(…) -> Result<AgHazirligi, BaslatmaHatasi>`; kabuk ve sunucu ortak kullanır
  - `fn sunucu_kaydi(alan, sunucu) -> String` (`"{alan}/mcp/{sunucu}"`),
    `fn sunucu_yetenek_adi(alan, sunucu) -> String` (`"kalfa-mcp-{alan}-{sunucu}"`)
  - `fn sunucu_kokunu_kur`, `fn sunucu_kokunu_hazirla`, `fn korumali_kur`, `fn yalniz_izinli_kimlikler`
  - `WindowsYalitici::sunucuyu_baslat`; trait'in `baslat_etkilesimli` ve `sunucu_kokunu_kaldir`'ı

**Neden böyle (Codex bulgu 2 ve 3):**

- Boru sunucusu yalnız kendi işindeki istemciyi kabul eder
  (`kopru.rs`, `istemci_iste_mi`). Bu yüzden iş nesnesi önce kurulur. Boru
  sunucusu ona bağlanır; köprü ve asıl komut aynı işte başlar. Kabuğun
  ağlı yolu da aynı `ag_hazirla`'yı kullanır.
- `izin_ekle` mevcut ACL'i korur, `guvenli_dizin` mirası kesmez. Üst
  dizinden gelen bir paket ya da ortak araç yeteneği izni bu yüzden sunucu
  köküne inebilirdi. `mcp` ve kök `korumali_dizin_olustur` ile mirassız
  kurulur.
- Var olan `mcp`, kök, `ev`, `gecici` ve `onbellek` ilk değişiklikten önce
  denetlenir (`sunucu_kokunu_denetle`): bağlantı olmamalı, kök ve `mcp`'nin
  mirası kesik olmalı, DACL'de izinli kimlik dışında kimlik olmamalı. Ret
  dosyaları, ACL'leri ve ACE kaydını değiştirmez. Aynı denetim
  hazırlığın sonunda bir kez daha yapılır.

- [x] **Adım 1: `kap.rs`'e borulu stdin.** `Kap`'a alan eklenir:

```rust
/// Başlamış kap: süreç tutamacı ve iki çıktı borusunun okuma uçları.
pub(crate) struct Kap {
    pub surec: OwnedHandle,
    pub cikti: File,
    pub hata: File,
    /// Etkileşimli süreçte stdin'in yazma ucu; kabukta `None` (stdin `NUL`).
    pub girdi: Option<File>,
}
```

  `boru()`'nun aynası olan yardımcı (okuma ucu çocuğa geçer, yazma ucu
  Kalfa'da kalır):

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
    if unsafe { SetHandleInformation(yazma.as_raw_handle() as HANDLE, HANDLE_FLAG_INHERIT, 0) } == 0
    {
        return Err(son_hata("SetHandleInformation(girdi)"));
    }
    Ok((okuma, yazma))
}
```

  `baslat`'ın gövdesi `baslat_ortak`'a taşınır; `let girdi = nul()?;`
  satırı borulu seçimle değişir:

```rust
/// Komutu kapta başlatır ve işe atar. `is` düşünce ağaç ölür.
pub(crate) fn baslat(istek: &KapIstegi, is: &Is) -> io::Result<Kap> {
    baslat_ortak(istek, is, false)
}

/// Etkileşimli süreç: `baslat` gibi, stdin bir borudur (1.1a §4.1).
pub(crate) fn baslat_etkilesimli(istek: &KapIstegi, is: &Is) -> io::Result<Kap> {
    baslat_ortak(istek, is, true)
}

fn baslat_ortak(istek: &KapIstegi, is: &Is, borulu: bool) -> io::Result<Kap> {
    let (cikti_o, cikti_y) = boru()?;
    let (hata_o, hata_y) = boru()?;
    let (girdi, girdi_yazma) = if borulu {
        let (okuma, yazma) = girdi_borusu()?;
        (okuma, Some(yazma))
    } else {
        (nul()?, None)
    };
```

  Sondaki `Kap { … }`'a `girdi: girdi_yazma.map(File::from),` eklenir.
  Modül açıklamasındaki "standart girdi `NUL`'dur" cümlesi "Kabukta
  standart girdi `NUL`'dur; etkileşimli süreçte (1.1a §4.1) bir borudur."
  olur.

- [x] **Adım 2: `surec.rs`'e Windows yapıcısını ekleyin** (`tam_erisimle`'den önce):

```rust
#[cfg(windows)]
impl EtkilesimliSurec {
    /// Kapta başlamış ve işe atanmış etkileşimli süreç.
    pub(crate) fn kaptan(
        mut kap: crate::windows::kap::Kap,
        is: agac::Is,
    ) -> Result<Self, BaslatmaHatasi> {
        let girdi = kap
            .girdi
            .take()
            .map(|g| Box::new(tokio::fs::File::from_std(g)) as Yazici);
        Self::yeni(girdi, Cocuk::Kap(kap), Agac::isten(is))
    }
}
```

- [x] **Adım 3: Düşen kabul testlerini yazın** (`tests/yalitim/windows.rs`
  sonuna). `kurgu`, `kurgu_agli`, `TestCozucu`, `powershell_yolu`,
  `windows_koku`, `surec_aktif_mi`, `deger`, `Kurgu::{alan, calistir, temizle}`
  mevcut yardımcılardır.

```rust
// --- 1.1a-1: etkileşimli süreç (G1'in ürün başlatıcısıyla karşılıkları) ---

fn etkilesimli_istek(
    alan: &AlanBilgisi,
    sunucu: &str,
    betik: &str,
) -> kalfa_sandbox::EtkilesimliIstek {
    kalfa_sandbox::EtkilesimliIstek {
        program: powershell_yolu().display().to_string(),
        argumanlar: vec![
            "-NoLogo".into(),
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            betik.into(),
        ],
        ortam: vec![
            (
                "PATH".into(),
                windows_koku().join("System32").display().to_string(),
            ),
            ("PATHEXT".into(), ".COM;.EXE;.BAT;.CMD".into()),
        ],
        kip: kalfa_sandbox::CalistirmaKipi::Yalitimli,
        sunucu: sunucu.into(),
        alan: alan.clone(),
        ag_listesi: None,
    }
}

async fn tum_cikti(s: &mut kalfa_sandbox::EtkilesimliSurec) -> String {
    let mut m = String::new();
    tokio::io::AsyncReadExt::read_to_string(&mut s.cikti_al().unwrap(), &mut m)
        .await
        .unwrap();
    m
}

const YANKI: &str = "[Console]::InputEncoding=[Text.Encoding]::UTF8;[Console]::OutputEncoding=[Text.Encoding]::UTF8;while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine($s);[Console]::Out.Flush()}";

fn yetenek(kimlik: &str, sunucu: &str) -> Sid {
    guvenlik::yetenek_sidi(&format!("kalfa-mcp-{kimlik}-{sunucu}")).unwrap()
}

fn sid_var(yol: &Path, sid: &Sid) -> bool {
    guvenlik::aceler_oku(yol)
        .unwrap()
        .iter()
        .any(|a| a.sid.as_ref() == Some(sid))
}

/// G1.1 (ürün başlatıcısıyla): 1 000 JSON-RPC satırı, 1 MiB'lık satır ve
/// UTF-8 birebir; EOF'ta çıkış.
#[tokio::test]
async fn etkilesimli_borular_birebir_yankilanir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let alan = k.alan(&k.kok);
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "yanki", YANKI))
            .unwrap();
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
    });
    let mut alinan = Vec::new();
    while let Some(s) = cikti.next_line().await.unwrap() {
        alinan.push(s.trim_end_matches('\r').to_string());
    }
    yazici.await.unwrap();
    assert_eq!(alinan, gonderilen);
    assert_eq!(s.bitisi_bekle().await, Some(0));
    assert!(s.kapat().await.unwrap().kendiliginden);
    k.temizle(&[alan]);
}

/// G1.2 (ürün başlatıcısıyla): torun yaşar, ping'lere yanıt verilir, EOF
/// yok sayılır; `kapat` döndüğünde kök de torun da yaşamaz.
#[tokio::test]
async fn etkilesimli_uzun_yasar_kapat_agaci_durdurur() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let k = kurgu();
    let alan = k.alan(&k.kok);
    let betik = "$c = Start-Process -FilePath (Join-Path $env:SystemRoot 'System32\\timeout.exe') -ArgumentList @('/t','600','/nobreak') -PassThru -WindowStyle Hidden; \
                 [Console]::Out.WriteLine(\"pid $PID $($c.Id)\"); [Console]::Out.Flush(); \
                 while(($s=[Console]::In.ReadLine()) -ne $null){[Console]::Out.WriteLine(\"pong $s\");[Console]::Out.Flush()}; Start-Sleep 600";
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "uzun", betik))
            .unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let ilk = cikti.next_line().await.unwrap().unwrap();
    let pidler: Vec<u32> = ilk
        .split_whitespace()
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    assert_eq!(pidler.len(), 2, "{ilk}");
    for i in 1..=5 {
        girdi.write_all(format!("{i}\n").as_bytes()).await.unwrap();
        assert_eq!(
            cikti
                .next_line()
                .await
                .unwrap()
                .map(|s| s.trim_end().to_string()),
            Some(format!("pong {i}"))
        );
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    drop(girdi);
    assert!(pidler.iter().all(|p| surec_aktif_mi(*p)));
    let bas = std::time::Instant::now();
    let kapanis = s.kapat().await.unwrap();
    assert!(!kapanis.kendiliginden);
    assert!(
        bas.elapsed() < Duration::from_secs(5),
        "{:?}",
        bas.elapsed()
    );
    for p in &pidler {
        assert!(!surec_aktif_mi(*p), "kapat'tan sonra süreç yaşıyor: {p}");
    }
    k.temizle(&[alan]);
}

/// G1.3 (ürün başlatıcısıyla), Review Focus 4–5 ve Codex 3: üst dizinde
/// miraslı paket ve ortak araç yeteneği izni varken de sunucunun kökü yalnız
/// onun yetenek SID'ine açıktır. Kabuk (aynı paket, yeteneksiz), öbür
/// sunucu ve başka alanın kabuğu okuyamaz, yazamaz. A kaldırılınca B kalır;
/// alan kaldırılınca B'nin izinleri de geri alınır.
#[tokio::test]
async fn etkilesimli_sunucu_koku_yetenek_sidiyle_ayrilir_ve_izinler_geri_alinir() {
    let k = kurgu();
    let alan = k.alan(&k.kok);
    let baska = k.alan(&k.baska_kok);
    let kimlik = alan_kimligi(&k.kok);
    let paket = guvenlik::profil_sidi(&format!("Kalfa.Alan.{kimlik}")).unwrap();
    let arac = guvenlik::arac_zinciri_sidi().unwrap();
    let miraslilar = [
        Izin {
            sid: paket.clone(),
            maske: guvenlik::MASKE_DEGISTIR,
            miras: guvenlik::MIRAS,
        },
        Izin {
            sid: arac.clone(),
            maske: MASKE_OKU,
            miras: guvenlik::MIRAS,
        },
    ];
    for izin in &miraslilar {
        guvenlik::izin_ekle(&k.yalitim_dizini, izin).unwrap();
    }
    let a = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "a");
    let b = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "b");
    let dene = |hedef: &Path| {
        format!(
            "try {{ Set-Content -LiteralPath (Join-Path '{0}' 'g1.txt') 'g1' -ErrorAction Stop; 'yaz=TAMAM' }} catch {{ 'yaz=RED' }}; \
             try {{ Get-ChildItem -LiteralPath '{0}' -ErrorAction Stop | Out-Null; 'oku=TAMAM' }} catch {{ 'oku=RED' }}",
            hedef.display()
        )
    };
    // B'nin ve A'nın kökü kurulsun; A'nın ikinci başlatması önceden var olan
    // kökü kullanır.
    for ad in ["b", "a"] {
        let s =
            k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, ad, "exit 0"))
                .unwrap();
        s.kapat().await.unwrap();
    }
    for yol in [&a.kok, &a.kok.parent().unwrap().to_path_buf()] {
        assert!(guvenlik::mirasi_kesik_mi(yol).unwrap(), "{}", yol.display());
        assert!(
            !sid_var(yol, &paket) && !sid_var(yol, &arac),
            "{}",
            yol.display()
        );
    }
    let mut s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(
            &alan,
            "a",
            &format!("{}; {}", dene(&a.ev), dene(&b.ev)),
        ))
        .unwrap();
    let m = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    let satir: Vec<&str> = m.lines().map(str::trim).collect();
    assert_eq!(
        satir,
        ["yaz=TAMAM", "oku=TAMAM", "yaz=RED", "oku=RED"],
        "{m}"
    );
    for kabuk in [&alan, &baska] {
        let sonuc = k.calistir(kabuk, false, &dene(&a.ev)).await;
        assert!(
            sonuc.standart.contains("yaz=RED") && sonuc.standart.contains("oku=RED"),
            "{}",
            sonuc.standart
        );
    }
    let mcp = a.kok.parent().unwrap().to_path_buf();
    let (ya, yb) = (yetenek(&kimlik, "a"), yetenek(&kimlik, "b"));
    k.y.sunucu_kokunu_kaldir(&k.kok, "a").unwrap();
    assert!(!a.kok.exists());
    assert!(!sid_var(&mcp, &ya));
    assert!(b.ev.exists() && sid_var(&b.ev, &yb) && sid_var(&mcp, &yb));
    k.y.alani_kaldir(&alan).unwrap();
    assert!(!sid_var(&mcp, &yb) && !sid_var(&b.ev, &yb));
    for izin in &miraslilar {
        guvenlik::izni_kaldir(&k.yalitim_dizini, &izin.sid).unwrap();
    }
    k.temizle(&[baska]);
}

/// Reddin hiçbir şeyi değiştirmediğini ölçmek için: dizinlerin SDDL'i,
/// dosya içerikleri ve ACE kaydı.
fn anlik(dizinler: &[&Path], dosyalar: &[PathBuf], kayit: &Path) -> Vec<String> {
    let mut v: Vec<String> = dizinler
        .iter()
        .map(|y| guvenlik::dacl_sddl(y).unwrap())
        .collect();
    v.extend(
        dosyalar
            .iter()
            .map(|d| std::fs::read_to_string(d).unwrap_or_else(|e| format!("yok: {e}"))),
    );
    v.push(std::fs::read_to_string(kayit).unwrap());
    v
}

/// Codex 3 (r1, r2): önceden var olan kök mirası açıksa ya da korumalı ama
/// beklenmeyen bir kimliğe izin veriyorsa sunucu başlamaz. Ret, ilk
/// değişiklikten önce gelir: SDDL'ler, dosyalar, ACE kaydı ve öbür
/// sunucunun izinleri aynı kalır.
#[tokio::test]
async fn etkilesimli_guvenli_olmayan_mevcut_kok_reddedilir() {
    let k = kurgu();
    let alan = k.alan(&k.kok);
    let kimlik = alan_kimligi(&k.kok);
    let paket = guvenlik::profil_sidi(&format!("Kalfa.Alan.{kimlik}")).unwrap();
    let kayit = k.yalitim_dizini.join("ace-kaydi.jsonl");
    let d = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "d");
    let mcp = d.kok.parent().unwrap().to_path_buf();
    let s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "d", "exit 0"))
            .unwrap();
    s.kapat().await.unwrap();

    // Mirası açık kök.
    let c = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "c");
    std::fs::create_dir_all(&c.ev).unwrap();
    std::fs::write(c.ev.join(".gitconfig"), "ISARET-C").unwrap();
    let dizinler = [mcp.as_path(), &c.kok, &c.ev, &d.kok, &d.ev];
    let dosyalar = [c.ev.join(".gitconfig"), c.ev.join(".npmrc")];
    let once = anlik(&dizinler, &dosyalar, &kayit);
    let hata =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "c", "exit 0"))
            .err()
            .expect("mirası açık kök reddedilmeli");
    assert!(format!("{hata}").contains("mirası açık"), "{hata}");
    assert_eq!(anlik(&dizinler, &dosyalar, &kayit), once);

    // Korumalı ama `ev`'inde paket izni olan kök.
    let e = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &kimlik, "e");
    let s =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "e", "exit 0"))
            .unwrap();
    s.kapat().await.unwrap();
    let izin = Izin {
        sid: paket.clone(),
        maske: MASKE_OKU,
        miras: MIRAS_YOK,
    };
    guvenlik::izin_ekle(&e.ev, &izin).unwrap();
    std::fs::write(e.ev.join(".gitconfig"), "ISARET-E").unwrap();
    std::fs::write(e.ev.join("isaret.txt"), "ISARET").unwrap();
    let dizinler = [
        mcp.as_path(),
        &e.kok,
        &e.ev,
        &e.gecici,
        &e.onbellek,
        &d.kok,
        &d.ev,
    ];
    let dosyalar = [
        e.ev.join(".gitconfig"),
        e.ev.join(".npmrc"),
        e.ev.join(".curlrc"),
        e.ev.join("isaret.txt"),
    ];
    let once = anlik(&dizinler, &dosyalar, &kayit);
    let hata =
        k.y.baslat_etkilesimli(&etkilesimli_istek(&alan, "e", "exit 0"))
            .err()
            .expect("beklenmeyen izinli kök reddedilmeli");
    assert!(format!("{hata}").contains("beklenmeyen izin"), "{hata}");
    assert_eq!(anlik(&dizinler, &dosyalar, &kayit), once);
    // Tam erişim de aynı kökü reddeder.
    let mut i = etkilesimli_istek(&alan, "e", "exit 0");
    i.kip = kalfa_sandbox::CalistirmaKipi::TamErisim;
    assert!(k.y.baslat_etkilesimli(&i).is_err());
    assert_eq!(anlik(&dizinler, &dosyalar, &kayit), once);
    guvenlik::izni_kaldir(&e.ev, &paket).unwrap();
    k.temizle(&[alan]);
}

/// G1 bulgusu ve §3: ortam özel köke ve sistem değerlerine yönelir;
/// `EVREN_`/`KALFA_` adları geçmez, yapılandırılmış belirteç geçer.
#[tokio::test]
async fn etkilesimli_ortam_ozel_koke_ve_sisteme_yonelir() {
    let k = kurgu();
    let alan = k.alan(&k.kok);
    let mut i = etkilesimli_istek(
        &alan,
        "ortam",
        "foreach($a in 'USERPROFILE','LOCALAPPDATA','HOME','NPM_CONFIG_CACHE','UV_CACHE_DIR','GITHUB_TOKEN','windir','SystemDrive','SystemRoot'){ \"$a=$([Environment]::GetEnvironmentVariable($a))\" }; \
         'ayrilmis=' + ((Get-ChildItem env: | Where-Object { $_.Name -like 'EVREN_*' -or $_.Name -like 'KALFA_*' } | ForEach-Object Name) -join ',')",
    );
    i.ortam.push(("EVREN_API_KEY".into(), "evren_llm_x".into()));
    i.ortam.push(("KALFA_E2E".into(), "1".into()));
    i.ortam.push(("GITHUB_TOKEN".into(), "ghp_deneme".into()));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let m = tum_cikti(&mut s).await;
    s.kapat().await.unwrap();
    let ozel = kalfa_sandbox::sunucu_ozel(&k.yalitim_dizini, &alan_kimligi(&k.kok), "ortam");
    let es = |a: &str, b: &Path| {
        deger(&m, a)
            .trim()
            .eq_ignore_ascii_case(&b.display().to_string())
    };
    assert!(es("USERPROFILE", &ozel.ev) && es("HOME", &ozel.ev), "{m}");
    // AppContainer `LOCALAPPDATA`'yı `ev\Packages\<profil>\AC`'ye çevirir;
    // yine özel kökün içindedir.
    assert!(
        Path::new(deger(&m, "LOCALAPPDATA").trim()).starts_with(&ozel.ev),
        "{m}"
    );
    assert!(es("NPM_CONFIG_CACHE", &ozel.npm()), "{m}");
    assert!(es("UV_CACHE_DIR", &ozel.onbellek.join("uv")), "{m}");
    assert!(
        es("windir", &windows_koku()) && es("SystemRoot", &windows_koku()),
        "{m}"
    );
    assert!(!deger(&m, "SystemDrive").trim().is_empty(), "{m}");
    assert_eq!(deger(&m, "GITHUB_TOKEN").trim(), "ghp_deneme", "{m}");
    assert_eq!(deger(&m, "ayrilmis").trim(), "", "{m}");
    k.temizle(&[alan]);
}

/// §4.1: tam erişim, yalıtım hazırken yalıtımsız başlar; yalıtımlı aynı
/// sırrı okuyamaz.
#[tokio::test]
async fn etkilesimli_tam_erisim_yalitim_hazirken_yalitimsiz_baslar() {
    let k = kurgu();
    let alan = k.alan(&k.kok);
    std::fs::write(k.ev.join(".ssh").join("id_rsa"), "GIZLI-SSH").unwrap();
    let betik = format!(
        "Get-Content -LiteralPath '{}' -ErrorAction SilentlyContinue",
        k.ev.join(".ssh").join("id_rsa").display()
    );
    let mut i = etkilesimli_istek(&alan, "kip", &betik);
    i.kip = kalfa_sandbox::CalistirmaKipi::TamErisim;
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    assert!(tum_cikti(&mut s).await.contains("GIZLI-SSH"));
    s.kapat().await.unwrap();
    i.kip = kalfa_sandbox::CalistirmaKipi::Yalitimli;
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    assert!(!tum_cikti(&mut s).await.contains("GIZLI-SSH"));
    s.kapat().await.unwrap();
    k.temizle(&[alan]);
}

/// Codex 2: ağlı sunucu köprüyle ve aynı işte başlar. stdin/stdout köprüden
/// geçer; izinli hedefe vekilden ulaşılır, listede olmayan reddedilir;
/// köprünün `KALFA_KOPRU_*` değerleri sunucuya geçmez. `kapat` döndüğünde
/// iş boştur (köprü dahil) ve ağ özeti gelir.
#[tokio::test]
async fn etkilesimli_agli_sunucu_vekilden_cikar_ve_kapanista_temizlenir() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    let dinleyici = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut soket, _)) = dinleyici.accept().await {
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 1024];
            let n = soket.read(&mut buf).await.unwrap_or(0);
            if String::from_utf8_lossy(&buf[..n]).starts_with("CONNECT") {
                let _ = soket
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await;
            } else {
                let _ = soket
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nSELAM",
                    )
                    .await;
            }
        }
    });
    let mut eslemeler = std::collections::HashMap::new();
    eslemeler.insert(
        ("fake.crates.io".to_string(), port),
        vec![std::net::SocketAddr::from(([127, 0, 0, 1], port))],
    );
    let k = kurgu_agli(Some(std::sync::Arc::new(TestCozucu { eslemeler })), true);
    let alan = k.alan(&k.kok);
    let betik = format!(
        "[Console]::Out.WriteLine('pid ' + $PID + ' ayrilmis=' + ((Get-ChildItem env: | Where-Object {{ $_.Name -like 'KALFA_*' }} | ForEach-Object Name) -join ',')); [Console]::Out.Flush(); \
         while(($s=[Console]::In.ReadLine()) -ne $null){{ \
           if ($s -eq 'izinli') {{ $r = (curl.exe -p -s -S http://fake.crates.io:{port}/) -join '' }} \
           elseif ($s -eq 'yasak') {{ $r = ((curl.exe -s -i http://evil.com/) | Select-Object -First 1) }} \
           else {{ $r = 'pong ' + $s }}; \
           [Console]::Out.WriteLine($r); [Console]::Out.Flush() }}"
    );
    let mut i = etkilesimli_istek(&alan, "agli", &betik);
    i.ag_listesi = Some(kalfa_sandbox::vekil::IzinListesi::yeni(
        &[],
        &[("fake.crates.io".to_string(), port)],
    ));
    let mut s = k.y.baslat_etkilesimli(&i).unwrap();
    let mut girdi = s.girdi_al().unwrap();
    let mut cikti = BufReader::new(s.cikti_al().unwrap()).lines();
    let ilk = cikti.next_line().await.unwrap().unwrap();
    assert!(ilk.trim_end().ends_with("ayrilmis="), "{ilk}");
    let pid: u32 = ilk.split_whitespace().nth(1).unwrap().parse().unwrap();
    let mut sor = async |satir: &str| {
        girdi
            .write_all(format!("{satir}\n").as_bytes())
            .await
            .unwrap();
        cikti.next_line().await.unwrap().unwrap()
    };
    assert_eq!(sor("1").await.trim(), "pong 1");
    assert!(sor("izinli").await.contains("SELAM"));
    assert!(sor("yasak").await.contains("403"));
    let kapanis = s.kapat().await.unwrap();
    assert!(!surec_aktif_mi(pid));
    let ag = kapanis.ag.expect("ağ özeti dönmeli");
    let karar = |host: &str, k: kalfa_sandbox::vekil::AgKarari| {
        ag.baglantilar
            .iter()
            .any(|b| b.host == host && b.karar == k)
    };
    assert!(
        karar("fake.crates.io", kalfa_sandbox::vekil::AgKarari::Izinli),
        "{:?}",
        ag.baglantilar
    );
    assert!(
        karar("evil.com", kalfa_sandbox::vekil::AgKarari::ListedeYok),
        "{:?}",
        ag.baglantilar
    );
    k.temizle(&[alan]);
}
```

- [x] **Adım 4: Düştüğünü görün:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim etkilesimli -- --test-threads=1`.

- [x] **Adım 5: Ağ hazırlığını ortak işleve taşıyın** (`yalitici.rs`).
  `yalitilmis_baslat`'ın sonundaki `if istek.ag { … } else { … }`
  bloğunun tamamı şununla değişir. Köprü kopyası, vekil ve boru kodu
  değişmeden `ag_hazirla`'ya taşınır; iş nesnesi artık önce kurulur.

```rust
        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        if istek.ag {
            let (kopru_komut, kap_ortami, ag_kapatici) = ag_hazirla(
                &ayar,
                &ozel,
                &paket,
                &is,
                istek.ag_listesi.clone(),
                &komut,
                ortam,
            )?;
            let kap_istegi = KapIstegi {
                komut: &kopru_komut,
                dizin: &calisma_dizini,
                ortam: &kap_ortami,
                paket: &paket,
                yetenekler: &yetenekler,
            };
            let kap = kap::baslat(&kap_istegi, &is).map_err(yalitim_hatasi)?;
            let mut surec = Surec::kaptan(kap, is);
            surec.ag_kapat_ekle(ag_kapatici);
            Ok(surec)
        } else {
            let kap_istegi = KapIstegi {
                komut: &komut,
                dizin: &calisma_dizini,
                ortam: &ortam,
                paket: &paket,
                yetenekler: &yetenekler,
            };
            let kap = kap::baslat(&kap_istegi, &is).map_err(yalitim_hatasi)?;
            Ok(Surec::kaptan(kap, is))
        }
    }
```

```rust
/// Köprünün komutu, köprünün ortamı ve ağın kapatıcısı.
type AgHazirligi = (
    komut_satiri::KomutSatiri,
    Vec<(String, String)>,
    crate::surec::AgKapatici,
);

/// Ağlı başlatma (§9.3, 1.1a §4.1): köprü kopyası, vekil ve `is`'e bağlı
/// boru sunucusu. Boru yalnız bu işteki istemciyi kabul eder; köprü de asıl
/// komut da aynı işte başlamalıdır. Asıl komut ve boru adı köprünün ortamına
/// `KALFA_KOPRU_*` olarak girer; köprü onları çocuğa geçirmez. Dönen komut
/// köprüdür. Köprünün kopyası kabuğun özel kökündedir (`ozel`).
fn ag_hazirla(
    ayar: &WindowsAyari,
    ozel: &OzelDizinler,
    paket: &Sid,
    is: &crate::agac::Is,
    liste: Option<crate::vekil::IzinListesi>,
    komut: &komut_satiri::KomutSatiri,
    ortam: Vec<(String, String)>,
) -> Result<AgHazirligi, BaslatmaHatasi> {
    let kaynak_kopru = match ayar.kopru.as_ref() {
        Some(k) if k.is_file() => k,
        _ => {
            return Err(BaslatmaHatasi::Yalitim(
                "Ağlı yalıtımlı komut başlatılamadı: köprü bulunamadı".into(),
            ));
        }
    };
    let kopru_dizini = ozel.kok.join("kopru");
    std::fs::create_dir_all(&kopru_dizini).map_err(|e| {
        BaslatmaHatasi::Yalitim(format!(
            "Ağlı yalıtımlı komut başlatılamadı: köprü kopyalanamadı ({e})"
        ))
    })?;
    let kopru_izni_var = guvenlik::aceler_oku(&kopru_dizini)
        .map(|aceler| {
            aceler.iter().any(|a| {
                a.tur == 0 && a.maske & MASKE_OKU == MASKE_OKU && a.sid.as_ref() == Some(paket)
            })
        })
        .unwrap_or(false);
    if !kopru_izni_var {
        let izin = Izin {
            sid: paket.clone(),
            maske: MASKE_OKU,
            miras: MIRAS,
        };
        guvenlik::izin_ekle(&kopru_dizini, &izin).map_err(|e| {
            BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: köprü dizini izni verilemedi ({e})"
            ))
        })?;
    }
    let hedef_kopru = kopru_dizini.join("kalfa-kopru.exe");
    let ayni_mi = if hedef_kopru.is_file() {
        match (dosya_sha256(kaynak_kopru), dosya_sha256(&hedef_kopru)) {
            (Ok(h1), Ok(h2)) => h1 == h2,
            _ => false,
        }
    } else {
        false
    };
    if !ayni_mi {
        std::fs::copy(kaynak_kopru, &hedef_kopru).map_err(|e| {
            BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: köprü kopyalanamadı ({e})"
            ))
        })?;
    }

    let ayar_vekil = crate::vekil::VekilAyari {
        liste: liste.unwrap_or_else(|| crate::vekil::IzinListesi::yeni(&[], &[])),
        cozucu: ayar
            .vekil_cozucu
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(crate::vekil::SistemCozucu)),
        yerel_izin: ayar.vekil_yerel_izin,
        ..Default::default()
    };
    let vekil = std::sync::Arc::new(crate::vekil::Vekil::baslat(ayar_vekil));
    let gelistirici = guvenlik::kullanici_sidi().map_err(yalitim_hatasi)?;
    let boru = match super::kopru::BoruSunucusu::ac(paket, &gelistirici, is, vekil.clone(), 64) {
        Ok(b) => b,
        Err(e) => {
            return Err(BaslatmaHatasi::Yalitim(format!(
                "Ağlı yalıtımlı komut başlatılamadı: boru açılamadı ({e})"
            )));
        }
    };

    let mut kap_ortami = ortam;
    kap_ortami.push((
        "KALFA_KOPRU_UYGULAMA".into(),
        komut.uygulama.display().to_string(),
    ));
    kap_ortami.push((
        "KALFA_KOPRU_SATIR".into(),
        String::from_utf16_lossy(&komut.satir),
    ));
    kap_ortami.push(("KALFA_KOPRU_BORU".into(), boru.ad().to_string()));
    let cmd = kap::sistem_dizini().join("cmd.exe");
    let kopru_komut = komut_satiri::uret(&hedef_kopru, &[], &cmd).map_err(yalitim_hatasi)?;
    let ag_kapatici: crate::surec::AgKapatici = Box::new(move || {
        Box::pin(async move {
            boru.kapat();
            vekil.bitir().await
        })
    });
    Ok((kopru_komut, kap_ortami, ag_kapatici))
}
```

- [x] **Adım 6: Kabuk testleri yeşil kalmalı:**
  `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim -- --test-threads=1 --skip etkilesimli`.
  Beklenen: 43 test geçer (ağlı kabuk testleri dahil).

- [x] **Adım 7: Sunucu kökü ve başlatma** (`yalitici.rs`). `ag_hazirla`'dan
  sonra:

```rust
fn sunucu_kaydi(alan: &str, sunucu: &str) -> String {
    format!("{alan}/mcp/{sunucu}")
}

fn sunucu_yetenek_adi(alan: &str, sunucu: &str) -> String {
    format!("kalfa-mcp-{alan}-{sunucu}")
}

/// Sunucunun özel kökü (1.1a §3, §5.6; G1.3). `mcp` ve sunucu kökü mirassız
/// (korumalı) DACL'le kurulur: yalnız geliştirici, SYSTEM ve Administrators.
/// Üst dizinlerden gelen paket ya da ortak yetenek izni sınırı aşamaz.
/// Kökte yalnız listeleme, `ev`/`gecici`/`onbellek`'te miraslı değiştirme
/// yalnız sunucunun yetenek SID'ine verilir; ACE'ler sunucu kaydına yazılır.
/// Var olan kök ilk değişiklikten önce denetlenir: mirası açıksa ya da başka
/// bir kimliğe izin veriyorsa başlatma reddedilir ve hiçbir şey değişmez.
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
    let (mcp, ozel) = sunucu_kokunu_kur(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    ozel.hazirla(git)?;
    let listele = Izin {
        sid: yetenek.clone(),
        maske: MASKE_LISTELE,
        miras: MIRAS_YOK,
    };
    for yol in [&mcp, &ozel.kok] {
        izin_kaydet_ve_ver(ace, &kayit, yol, &listele)?;
    }
    let degistir = Izin {
        sid: yetenek.clone(),
        maske: MASKE_DEGISTIR,
        miras: MIRAS,
    };
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        izin_kaydet_ve_ver(ace, &kayit, yol, &degistir)?;
    }
    // Windows kabın TEMP'ini `LOCALAPPDATA\Packages\<paket>\AC\Temp`'e çevirir.
    profil::guvenli_dizin(
        &ozel.kok,
        Path::new("ev/Packages")
            .join(profil_adi)
            .join("AC")
            .join("Temp")
            .as_path(),
    )?;
    profil::guvenli_dizin(&ozel.kok, Path::new("ev/Roaming"))?;
    // Son denetim, araya giren bir değişikliği yakalar.
    sunucu_kokunu_denetle(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    Ok(ozel)
}

/// `mcp` ve sunucunun kökü. Önce var olanlar denetlenir; sonra eksikler
/// mirassız ve yalnız geliştirici, SYSTEM ve Administrators'la oluşturulur.
fn sunucu_kokunu_kur(
    ace: &AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    yetenek: &Sid,
) -> io::Result<(PathBuf, OzelDizinler)> {
    let (mcp, ozel) = sunucu_kokunu_denetle(ace, yalitim_dizini, alan, sunucu, yetenek)?;
    std::fs::create_dir_all(yalitim_dizini)?;
    profil::guvenli_dizin(yalitim_dizini, Path::new(alan))?;
    let temel = temel_izinler(true)?;
    for yol in [&mcp, &ozel.kok] {
        match guvenlik::korumali_dizin_olustur(yol, &temel) {
            // Var olan dizin yukarıda denetlendi.
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Ok((mcp, ozel))
}

/// Var olan `mcp`, sunucu kökü ve özel alt dizinleri değiştirmeden denetler.
/// Olmayan dizin geçer. Bağlantı, mirası açık kök ya da izinli kimlikler
/// dışında bir kimlik hatadır. `mcp`'de aynı alanın öbür sunucularının
/// kayıtlı listeleme izinleri de vardır.
fn sunucu_kokunu_denetle(
    ace: &AceKaydi,
    yalitim_dizini: &Path,
    alan: &str,
    sunucu: &str,
    yetenek: &Sid,
) -> io::Result<(PathBuf, OzelDizinler)> {
    let mcp = OzelDizinler::yol(yalitim_dizini, alan).kok.join("mcp");
    let ozel = crate::sunucu_ozel(yalitim_dizini, alan, sunucu);
    let mut izinli: Vec<Sid> = temel_izinler(true)?.into_iter().map(|i| i.sid).collect();
    izinli.push(yetenek.clone());
    mevcut_dizini_denetle(&ozel.kok, true, &izinli)?;
    for yol in [&ozel.ev, &ozel.gecici, &ozel.onbellek] {
        mevcut_dizini_denetle(yol, false, &izinli)?;
    }
    let onek = format!("{alan}/mcp/");
    for girdi in ace.girdiler() {
        if let Girdi::Izin { alan, yol, sid, .. } = girdi
            && alan.starts_with(&onek)
            && yol == &mcp
        {
            izinli.push(Sid::metinden(sid)?);
        }
    }
    mevcut_dizini_denetle(&mcp, true, &izinli)?;
    Ok((mcp, ozel))
}

fn mevcut_dizini_denetle(yol: &Path, korumali: bool, izinli: &[Sid]) -> io::Result<()> {
    match std::fs::symlink_metadata(yol) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    guvenlik::yeniden_ayristirma_izlemeden_ac(yol, true)?;
    if korumali && !guvenlik::mirasi_kesik_mi(yol)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{}: sunucu kökünün mirası açık", yol.display()),
        ));
    }
    yalniz_izinli_kimlikler(yol, izinli)
}

/// Dizinin DACL'inde `izinli` dışında kimlik yoksa `Ok`.
fn yalniz_izinli_kimlikler(yol: &Path, izinli: &[Sid]) -> io::Result<()> {
    let fazla: Vec<String> = guvenlik::aceler_oku(yol)?
        .iter()
        .filter_map(|a| a.sid.as_ref())
        .filter(|sid| !izinli.contains(sid))
        .map(Sid::metin)
        .collect();
    if fazla.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{}: sunucu kökünde beklenmeyen izin: {}",
                yol.display(),
                fazla.join(", ")
            ),
        ))
    }
}
```

  `impl WindowsYalitici`'ye, `yalitilmis_baslat`'tan sonra:

```rust
    /// Sunucuyu yalıtımda başlatır (1.1a §4.1, §5.6). ACE kilidi tutulurken
    /// çağrılır.
    fn sunucuyu_baslat(
        &self,
        istek: &crate::EtkilesimliIstek,
        ace: &mut AceKaydi,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        if let Some(neden) = self
            .okunur_izin_hatasi
            .read()
            .unwrap_or_else(|z| z.into_inner())
            .as_ref()
        {
            return Err(BaslatmaHatasi::Yalitim(neden.clone()));
        }
        if ace.bozuk_mu() {
            return Err(BaslatmaHatasi::Yalitim(
                "ACE kaydı bozuk; sunucu başlatılmadı".into(),
            ));
        }
        let kimlik = alan_kimligi(&istek.alan.kok);
        if !ace.hazir_mi(&kimlik) {
            return Err(BaslatmaHatasi::Yalitim(
                "çalışma alanı yalıtım için hazırlanmadı".into(),
            ));
        }
        let ayar = self.ayar();
        let ad = profil_adi(&kimlik);
        let paket = guvenlik::profil_olustur(&ad, "Kalfa çalışma alanı yalıtımı")
            .map_err(yalitim_hatasi)?;
        let arac = guvenlik::arac_zinciri_sidi().map_err(yalitim_hatasi)?;
        let sunucu_yetenegi = guvenlik::yetenek_sidi(&sunucu_yetenek_adi(&kimlik, &istek.sunucu))
            .map_err(yalitim_hatasi)?;
        let ozel = sunucu_kokunu_hazirla(
            ace,
            &ayar.yalitim_dizini,
            &kimlik,
            &istek.sunucu,
            &ad,
            &sunucu_yetenegi,
            &ayar.git,
        )
        .map_err(yalitim_hatasi)?;
        let alan_kok = guvenlik::sade(
            &std::fs::canonicalize(&istek.alan.kok).map_err(BaslatmaHatasi::GirisCikis)?,
        );
        let (araclar, uygun) =
            arac_yollarini_siniflandir(ace, &ayar, Some(&alan_kok)).map_err(yalitim_hatasi)?;
        if !uygun {
            return Err(BaslatmaHatasi::Yalitim(
                "araç zinciri dizini çalışma alanı ya da Kalfa'nın özel verisiyle çakışıyor".into(),
            ));
        }
        let ag = istek.ag_listesi.is_some();
        // `yalitim_ortami` kalıtılan ortamı izin listesinden geçirir; adında
        // `TOKEN`/`KEY` geçenleri de düşürür. Sunucunun ortamı yapılandırılmış
        // değerleri taşır (1.1a §5.5): yalıtımın koyduğu değerler kalır,
        // yalıtımın düşürdükleri PATH dışında geri eklenir.
        let sunucu_ortami = crate::sunucu_ortami(&istek.ortam, &ozel);
        let mut ortam = yalitim_ortami(&sunucu_ortami, &ozel, &ayar, &araclar, ag);
        for (ad, deger) in &sunucu_ortami {
            if !ad.eq_ignore_ascii_case("PATH") && deger_al(&ortam, ad).is_none() {
                ortam.push((ad.clone(), deger.clone()));
            }
        }
        let kokler = izinli_kokler(&ayar, &istek.alan, &ozel, &araclar);
        let (program, argumanlar) = program::coz(
            &KomutBicimi::Program {
                program: istek.program.clone(),
                argumanlar: istek.argumanlar.clone(),
            },
            &ortam,
        )?;
        let program = gorunen_program(&program, &kokler, &istek.alan, &ortam)?;
        let komut =
            komut_satiri::uret(&program, &argumanlar, &kap::sistem_dizini().join("cmd.exe"))
                .map_err(yalitim_hatasi)?;
        let yetenekler = vec![arac, sunucu_yetenegi];
        let is = crate::agac::Is::yeni(true).map_err(yalitim_hatasi)?;
        let (komut, ortam, ag_kapat) = if ag {
            let kabuk_ozel = OzelDizinler::yol(&ayar.yalitim_dizini, &kimlik);
            let (k, o, a) = ag_hazirla(
                &ayar,
                &kabuk_ozel,
                &paket,
                &is,
                istek.ag_listesi.clone(),
                &komut,
                ortam,
            )?;
            (k, o, Some(a))
        } else {
            (komut, ortam, None)
        };
        let kap = kap::baslat_etkilesimli(
            &KapIstegi {
                komut: &komut,
                dizin: &ozel.ev,
                ortam: &ortam,
                paket: &paket,
                yetenekler: &yetenekler,
            },
            &is,
        )
        .map_err(yalitim_hatasi)?;
        let mut surec = crate::EtkilesimliSurec::kaptan(kap, is)?;
        if let Some(k) = ag_kapat {
            surec.ag_kapat_ekle(k);
        }
        Ok(surec)
    }
```

  `use crate::{…}` listesine `KomutBicimi` eklenir.
  `impl Yalitici for WindowsYalitici`'ye, `yeniden_algila`'dan önce:

```rust
    fn baslat_etkilesimli(
        &self,
        istek: &crate::EtkilesimliIstek,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        crate::etkilesimli::sunucu_adi_dogrula(&istek.sunucu)?;
        crate::mcp_kip_uyusmasi(istek.kip, self.durum().hazir_mi())?;
        if istek.kip == crate::CalistirmaKipi::TamErisim {
            let ayar = self.ayar();
            // Kök iki kipte ortaktır; yalıtımlı başlatma onu korumalı bekler.
            let kimlik = alan_kimligi(&istek.alan.kok);
            let yetenek = guvenlik::yetenek_sidi(&sunucu_yetenek_adi(&kimlik, &istek.sunucu))
                .map_err(yalitim_hatasi)?;
            sunucu_kokunu_kur(
                &self.ace(),
                &ayar.yalitim_dizini,
                &kimlik,
                &istek.sunucu,
                &yetenek,
            )
            .map_err(yalitim_hatasi)?;
            return crate::surec::tam_erisimle(istek, &ayar.yalitim_dizini, &ayar.git);
        }
        // Kabuktaki gibi: ACE kilidi izin değişikliklerini ve başlatmayı
        // sıraya sokar.
        let mut ace = self.ace();
        self.sunucuyu_baslat(istek, &mut ace)
    }

    fn sunucu_kokunu_kaldir(&self, alan_koku: &Path, sunucu: &str) -> Result<(), String> {
        crate::etkilesimli::sunucu_adi_dogrula(sunucu).map_err(|e| e.to_string())?;
        let kimlik = alan_kimligi(alan_koku);
        let kayit = sunucu_kaydi(&kimlik, sunucu);
        let mut ace = self.ace();
        if ace.bozuk_mu() {
            return Err("ACE kaydı bozuk; sunucu kökü kaldırılmadı".into());
        }
        kayda_gore_geri_al(&mut ace, &kayit).map_err(|e| e.to_string())?;
        ace.alani_cikar(&kayit, &[]).map_err(|e| e.to_string())?;
        let kok = crate::sunucu_ozel(&self.ayar().yalitim_dizini, &kimlik, sunucu).kok;
        match std::fs::remove_dir_all(&kok) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
```

- [x] **Adım 8: `alani_kaldir_kilitli`'ye sunucu girdilerini ekleyin.**
  Bozuk olmayan kayıt dalında, `kayda_gore_geri_al(ace, kimlik)?;`
  çağrısından hemen önce:

```rust
            // Sunucuların yetenek SID'leri (1.1a §5.6) alanla birlikte gider.
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

  Bozuk kayıt yolu (`taramali_geri_al`) yetenek SID'lerini bilmez ve
  değişmez. Bozuk kayıtta `sunucu_kokunu_kaldir` da hata döner; kabuktaki
  "yalnız kanıtlananı geri al" kuralı korunur.

- [x] **Adım 9: Geçtiğini görün.** Windows'ta, kurulu makinede:
  `cargo clippy -p kalfa-sandbox --features yalitim-testleri --all-targets --locked -- -D warnings`
  ve `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim -- --test-threads=1`.
  Beklenen: clippy temiz; yeni 7 test ve bütün eski testler geçer
  (toplam 50). Ardından Görev 5 Adım 3'teki temizlik denetimini yapın.
- [x] **Adım 10: Commit'leyin:** `sandbox: Windows'ta borulu AppContainer süreci, korumalı sunucu kökü ve köprülü ağ`.

---

### Görev 5: İki platform doğrulaması ve belgeler

**Dosyalar:**
- Değiştir: `docs/GELISTIRME_DURUMU.md`, tasarım §4.1 (G1'in yanına uygulama sonucu)

- [x] **Adım 1: Windows'ta** (worktree):

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings
cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim -- --test-threads=1
cargo test -p kalfa-tools --features yalitim-testleri --locked --test uyumluluk
```

- [x] **Adım 2: WSL'de** (Ubuntu 24.04, `kalfatest`, ayrı kopya ve
  `CARGO_TARGET_DIR`):

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings
cargo build -p kalfa-sandbox --bin kalfa-kopru --locked
cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim
cargo test -p kalfa-tools --features yalitim-testleri --locked --test uyumluluk
```

- [x] **Adım 3: Temizliği doğrulayın.**
  - Windows'ta `HKCU\…\AppContainer\Mappings` altındaki `Kalfa.Alan.*`
    profillerinin, `%LOCALAPPDATA%\Packages\kalfa.alan.*` dizinlerinin ve
    `kalfa-kopru` süreçlerinin sayısı testlerden öncekiyle aynı olmalı.
    Testlerin geçici dizinleri silinir; ACE'leri onlarla gider.
  - Bu koşuda üretilen WSL kaynak kopyası silinir. Önceden var olan
    ortak `CARGO_TARGET_DIR` yeniden kullanıldıysa sahibinin denetim
    worktree'siyle birlikte korunur; yeni ayrı önbellek oluşturulmaz.
- [x] **Adım 4: Belgeleri güncelleyin.** Sonuçları ve sayıları
  `GELISTIRME_DURUMU.md`'ye ve tasarım §4.1'e yazın. `e2e`, arayüz ve
  `kalfa-mcp` bu planın kapsamında değil. Yalnız sandbox değiştiği için
  arayüz ve e2e komutları gerekmez.
- [x] **Adım 5: Commit'leyin ve PR'a yazın:** `docs: 1.1a-1 etkileşimli süreç sonucu`.

---

## Codex bulgularının karşılığı

### İlk inceleme (`INCELEME-PLAN-9d6b6b5`)

| # | Bulgu | Karar | Planda | Kabul testi |
|---|---|---|---|---|
| 1 | `kapat()` bütün ağacın durduğunu doğrulamıyor; Linux tam erişimde `setsid` torun | Katılıyorum | Görev 2: `kapat()` `io::Result` döner; `agac.rs`'e öldürme ve bitiş doğrulama arayüzü; Linux'ta kök alt süreç toplayıcısı ve torun taraması; `agac.rs` `unsafe` listesinde. Tam erişimin kalan sınırı tasarım §4.1'de açık. | `etkilesimli_kapat_donunce_torun_yasamaz`, `etkilesimli_setsid_torun_kapat_ve_birakmada_olur` (Linux), `etkilesimli_birakilan_surecin_agaci_olur`, iki platformda `etkilesimli_uzun_yasar_kapat_agaci_durdurur` |
| 2 | Windows ağlı başlatma köprüyü ve ortak Job Object'i kurmuyor | Katılıyorum | Görev 4: `ag_hazirla` köprü komutunu, köprünün ortamını ve kapatıcıyı döner; iş nesnesi önce kurulur ve boru sunucusu ile kap aynı işi kullanır. Kabuk da aynı işlevi kullanır. | İki platformda `etkilesimli_agli_sunucu_vekilden_cikar_ve_kapanista_temizlenir` |
| 3 | Windows özel kökü G1'deki korumalı DACL'i taşımıyor | Katılıyorum | Görev 4: `mcp` ve kök korumalı kurulur; var olan kök denetlenir; son DACL denetimi; kök iki kipte ortak ve korumalı. | `etkilesimli_sunucu_koku_yetenek_sidiyle_ayrilir_ve_izinler_geri_alinir` (miraslı paket ve araç yeteneği izniyle), `etkilesimli_guvenli_olmayan_mevcut_kok_reddedilir` |
| 4 | Dışarı verilen stdin yazıcısı `kapat()` tarafından kapatılamıyor | Katılıyorum | Görev 2: stdin aktarma görevi ve `duplex`; `kapat()` görevi keser. | `etkilesimli_kapat_disarida_tutulan_yaziciya_ragmen_eof_verir`, `etkilesimli_kapat_okumayan_sureci_ve_bloke_yazmayi_bitirir` |
| — | `cargo test … etkilesimli sunucu tam_erisim` geçersiz | Katılıyorum | Bütün yeni testler `etkilesimli` içerir; her komut tek süzgeç alır. | — |
| — | Windows stdin'inin `tokio::fs::File` ile sarılması | Ön doğrulamada ölçüldü | Bloke yazma, iptal, bırakma ve kapanış testleri iki platformda geçti. Bloke bir yazma, ağaç öldürülünce boru kapandığı için biter. | `etkilesimli_kapat_okumayan_sureci_ve_bloke_yazmayi_bitirir` |

### İkinci inceleme (`INCELEME-PLAN-6205095`)

| # | Bulgu | Karar | Planda | Kabul testi |
|---|---|---|---|---|
| 1 | Yarım kalan `kapat()` temizliği atlıyor; bırakma stdin görevini kesmiyor | Katılıyorum | Görev 2: `dogrulandi` yalnız doğrulanmış kapanıştan sonra; `Drop` stdin görevini her durumda keser ve doğrulanmamış ağaçta torunları ve ağacı öldürür. | `etkilesimli_kapat_iptal_edilince_agac_olur` (300 ms ve 2050 ms'de iptal; Linux'ta `setsid` torunu), `etkilesimli_birakmada_stdin_gorevi_kesilir` (görev sayısı ve sonraki yazmanın hatası) |
| 2 | Linux torun taraması eksikliği başarı sayıyor; PID sabitlenmiyor | Katılıyorum | Görev 2: `SurecKaynagi`; tarama hatası `Err`; `pidfd_open` + başlangıç doğrulaması + `pidfd_send_signal`; öldürme ve boşalma `AgacDenetimi` ile. | `agac::tests` (sahte kaynakla numara yeniden kullanımı, tarama/açma hatası; sahte `/proc` ile okunamayan, bozuk ve eksik kayıt), sahte ağaçla `etkilesimli_bosalma_son_sifirdan_once_bitmez`, `…bosalmayan_ya_da_denetlenemeyen_agac_hatadir`, `…oldurme_ve_tarama_hatasi_kapanisi_basarisiz_kilar` |
| 3 | Windows güvensiz kökü reddetmeden önce dosya ve ACL değiştiriyor | Katılıyorum | Görev 4: `sunucu_kokunu_denetle` ilk değişiklikten önce; kurulum yalnız eksikleri oluşturur. | `etkilesimli_guvenli_olmayan_mevcut_kok_reddedilir`: mirası açık ve korumalı-yabancı-SID kökler ayrı; SDDL'ler, dosyalar, ACE kaydı ve öbür sunucunun izinleri ret öncesi ve sonrası aynı; tam erişim de reddeder. |

## Ön doğrulamada plana işlenen değişiklikler

Önceki taslağa (`9d6b6b5`) göre, derleme ve testlerin gösterdiği
değişiklikler:

1. **`Yalitimsiz` değişmedi.** Taslak, tam erişim testleri için ona
   `yalitim_dizini` alanı ekliyordu. `Yalitimsiz` birim yapı olarak altı
   crate'in testlerinde değer olarak kullanılıyor. Tam erişimi zaten Linux
   ve Windows arka uçları başlatıyor. Tam erişim testleri `surec.rs` birim
   testine taşındı ve `tam_erisimle`'yi doğrudan çağırıyor.
2. **Windows'ta kök iki kipte korumalı kurulur.** Tam erişim önce
   başladığında `ozel.hazirla` `mcp`'yi mirası açık oluşturuyor ve sonraki
   yalıtımlı başlatma reddediliyordu (`etkilesimli_tam_erisim_…` testi).
3. **AppContainer `LOCALAPPDATA`'yı çevirir.** Kapta değer
   `ev\Packages\<profil>\AC` oluyor; test, değerin özel kökün içinde
   olduğunu doğrular.
4. **Windows ortamı:** Taslak, `sunucu_ortami`'nın PATH dışındaki bütün
   değerlerini yalıtımın ortamının üstüne yazıyordu. Bu, örneğin
   kullanıcının `CARGO_HOME`'unu sunucunun özel değerinin üstüne yazardı.
   Şimdi yalıtımın koyduğu değerler kalır; yalnız yalıtımın düşürdükleri
   (`TOKEN`/`KEY` içerenler gibi) geri eklenir.
5. **Linux `deneme`'si** `komut_kur`'u çağırıyordu; `KurIstegi`'ne geçirildi.
6. **Linux G1.2 testi:** bwrap'ın ve `prlimit`'in komut satırı betiğin
   metnini taşıyor. İşaretler betikte geçmesin diye `$0`'dan kurulur.
   Torun ayrıca `setsid` ile kendi oturumunda başlar.
7. **Tam erişimin git kimliği** arka ucun ayarından gelir
   (`GitKimligi::oku()` her başlatmada `git` çalıştırıyordu).
8. **Alan sırası:** `EtkilesimliSurec`'te `ag_kapat`, `agac`'tan önce
   bırakılır; Windows'ta boru sunucusu işin tutamacını kullanır.
9. **Tokio çalışma zamanı:** stdin aktarma görevi için
   `Handle::try_current()` kullanılır; çalışma zamanı yoksa hata döner.

## Kapsam dışı (1.1a-2)

- sınırlı çerçeveleyici ve stderr satır sınırı,
- `rmcp`,
- koruma özeti K ve mutlak denetim yaşı,
- W ve yazma kuşağı,
- başlatma izni,
- mühür,
- tanım denetimi,
- ortam birleştirme ve `GizliKasa`,
- uygulama yöneticisi ve `kapat()`'ın `Err`'inde kilidin tutulması,
- arayüz.

Bu planda gelenler: stdin/stdout/stderr ham akışları, ağacın doğrulanan
kapanışı, özel kök ve çalışma kipi.

## Öz denetim

- **Tasarımın karşılığı:**

  | Tasarım | Görev |
  |---|---|
  | §4.1 etkileşimli süreç ve `kapat()` sözleşmesi | G2–4 |
  | §4.1 çalışma kipi | G1–4 |
  | §4.1 ağ | G3, G4 (ağlı kabul testleri iki platformda) |
  | §3 özel kök ve ortam | G1, G3, G4 |
  | §5.6 yetenek SID'i, korumalı kök ve ACE kaydı | G4 |
  | G1'in üç ölçümü | G3 ve G4'te ürün başlatıcısıyla yeniden kurulur |

- **Tip tutarlılığı:** `EtkilesimliSurec::kaptan`, `tam_erisimle`,
  `sunucu_ozel`, `sunucu_ortami`, `KurIstegi`, `ag_hazirla` ve
  `sunucu_kokunu_kur` bütün görevlerde aynı adla geçer. Kod blokları ön
  doğrulama commit'inden üretildi.
- **Bilinen sınırlar:**
  - Linux tam erişimde, kök kendiliğinden çıktıktan sonra yaşayan
    `setsid` torunu sahiplenilemez (tasarım §4.1). Bu istisna kullanıcının
    14 Ekim'deki doğrudan yanıtıyla açık uyarı koşuluyla kabul edildi;
    §4.3'teki gizli dosya riskinden ayrı karardır.
  - `Drop` yolu bitişi doğrulamaz; iptal ve hata yolunda çağıran kilidi
    bırakmaz (1.1a-2).
  - `pidfd` Linux 5.3 gerektirir; yoksa tarama `Err` verir ve kapanış
    başarısız sayılır.

## Üçüncü incelemenin karşılığı (r4)

Ara ebeveyn PID'si yeniden kullanıldığında onun üzerinden bulunan ilgisiz
alt dala sinyal gitmesi düzeltildi. Yaprak pidfd ile sabitlenir; kendi
başlangıcı ve köke kadar bütün ara ebeveyn kimlikleri doğrulanır. Kayıp
ya da değişmiş halka bu turun alt dalını reddeder. Yeni testler
`etkilesimli_yeniden_kullanilan_ara_ebeveynin_alt_dali_sinyal_almaz` ve
`etkilesimli_kaybolan_ara_ebeveyn_eski_dali_atlar_yeni_tarama_yetimi_kapsar`:
eski seçimle ikisi de düştü, düzeltmeyle geçiyor. Bağımsız kaynak
incelemesinde yeni engelleyici bulgu yok. İki platformun zorunlu ürün
kontrolleri geçti; [ürün raporu](../bulgular/2026-10-14-kalfa-11a-1-urun.md)
negatif kanıtı, sonuçları ve temizliği taşır.
