//! Çalışma alanı: `cap-std` dizin tutamacı (spec §7.3).
//!
//! Her üst bileşen ayrı bir `open_dir_nofollow` çağrısıyla, son bileşen de
//! `FollowSymlinks::No` ile açılır. Böylece son dosyada no-follow kullanıp üst
//! zinciri normal açma hatasına düşülmez. v1'de bağlantı izlenmez; çalışma
//! alanının içinde kalan bağlantılar bile reddedilir.
//!
//! Yazarken kökteki `.git`'in kimliği de denetlenir (güvenlik analizi
//! Bulgu 1): çekirdek `.git`'i adıyla tanıyıp korumalı sayar; git dizinine
//! başka bir adla (8.3 kısa ad gibi) yazılamaz.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use ignore::WalkBuilder;
use kalfa_core::arac::AracHatasi;
use kalfa_core::geri_alma::{DosyaDurumu, GeriYukleyici};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::yol::CalismaYolu;
use same_file::Handle;
use tokio_util::sync::CancellationToken;

use crate::baglanti;

pub struct CalismaAlani {
    kok: PathBuf,
    dizin: Dir,
    siniflandirici: Arc<Siniflandirici>,
}

impl CalismaAlani {
    /// Sınıflandırıcının köküne tek kaynaktan gelen tutamaç açar.
    pub fn ac(siniflandirici: Arc<Siniflandirici>) -> io::Result<Arc<Self>> {
        let kok = siniflandirici.kok().to_path_buf();
        let dizin = Dir::open_ambient_dir(&kok, ambient_authority())?;
        Ok(Arc::new(Self {
            kok,
            dizin,
            siniflandirici,
        }))
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    pub fn siniflandirici(&self) -> &Siniflandirici {
        &self.siniflandirici
    }

    pub fn oku(&self, yol: &CalismaYolu) -> io::Result<Vec<u8>> {
        let mut dosya = self.dosya_ac(yol)?;
        let mut veri = Vec::new();
        dosya.read_to_end(&mut veri)?;
        Ok(veri)
    }

    /// Yalnızca üst dizinler açılır; son bileşen çağıranın yapacağı dosya
    /// işlemine bırakılır. Her üst bileşen tek adla ve no-follow açılır.
    fn ust_dizin(&self, yol: &CalismaYolu, olustur: bool) -> io::Result<(Dir, String)> {
        self.ust_dizin_denetimli(yol, olustur, None)
    }

    /// `git` verilirse açılan her üst dizin onunla karşılaştırılır.
    fn ust_dizin_denetimli(
        &self,
        yol: &CalismaYolu,
        olustur: bool,
        git: Option<&Handle>,
    ) -> io::Result<(Dir, String)> {
        let mut dizin = self.dizin.try_clone()?;
        let mut parcalar = yol.as_str().split('/').peekable();
        let mut ad_onceki: Option<&str> = None;

        while let Some(ad) = parcalar.next() {
            if let (Some(g), Some(onceki)) = (git, ad_onceki) {
                git_takma_adi_reddet(g, onceki, &dizin.try_clone()?.into_std_file())?;
            }
            if parcalar.peek().is_none() {
                return Ok((dizin, ad.to_string()));
            }

            dizin = match dizin.open_dir_nofollow(ad) {
                Ok(d) => d,
                Err(e) if olustur && e.kind() == io::ErrorKind::NotFound => {
                    match dizin.create_dir(ad) {
                        Ok(()) => {}
                        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                        Err(e) => return Err(e),
                    }
                    // Kontrol ile kullanım arasında bağlantı konulsa bile bu
                    // ikinci açma takip etmez.
                    dizin.open_dir_nofollow(ad)?
                }
                Err(e) => return Err(e),
            };
            ad_onceki = Some(ad);
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Dosya yolu boş",
        ))
    }

    /// Yolun çalışma alanının içinde bir dizin olduğunu doğrular; yolun hiçbir
    /// bileşeni bağlantı olamaz. Kabuğun çalışma dizini için (aşama 2
    /// tasarımı §4.1).
    pub fn dizin_dogrula(&self, yol: &CalismaYolu) -> io::Result<()> {
        let (ust, ad) = self.ust_dizin(yol, false)?;
        ust.open_dir_nofollow(ad)?;
        Ok(())
    }

    /// Yolun henüz olmayan üst dizinleri, sığdan derine (aşama 3 tasarımı
    /// §5.3). Varlık bağlantı izlenmeden denetlenir; üst dizinlerden biri
    /// bağlantıysa hata döner (yazma zaten başarısız olurdu).
    pub fn olusacak_dizinler(&self, yol: &CalismaYolu) -> io::Result<Vec<CalismaYolu>> {
        let parcalar: Vec<&str> = yol.as_str().split('/').collect();
        let mut dizin = self.dizin.try_clone()?;
        let mut sonuc = Vec::new();
        for (sira, ad) in parcalar
            .iter()
            .enumerate()
            .take(parcalar.len().saturating_sub(1))
        {
            if sonuc.is_empty() {
                match dizin.open_dir_nofollow(ad) {
                    Ok(d) => {
                        dizin = d;
                        continue;
                    }
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e),
                }
            }
            let goreli = parcalar[..=sira].join("/");
            sonuc.push(
                CalismaYolu::coz(&goreli)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?,
            );
        }
        Ok(sonuc)
    }

    /// Son dosya bileşenini de no-follow açar ve yalnızca normal dosyaları
    /// kabul eder. `metadata` açık dosya tutamacı üzerinden alınır.
    fn dosya_ac(&self, yol: &CalismaYolu) -> io::Result<File> {
        let (ust, ad) = self.ust_dizin(yol, false)?;
        let mut secenek = OpenOptions::new();
        secenek.read(true).follow(FollowSymlinks::No);
        let dosya = ust.open_with(ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalar açılabilir",
            ));
        }
        Ok(dosya)
    }

    pub fn oku_varsa(&self, yol: &CalismaYolu) -> io::Result<Option<Vec<u8>>> {
        match self.oku(yol) {
            Ok(veri) => Ok(Some(veri)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Dosyanın SHA-256 özeti; dosya yoksa `None`.
    pub fn icerik_ozeti(&self, yol: &CalismaYolu) -> io::Result<Option<String>> {
        Ok(self.oku_varsa(yol)?.map(|veri| ozet_hex(&veri)))
    }

    /// Hard link sayısı; dosya yoksa 1.
    pub fn baglanti_sayisi(&self, yol: &CalismaYolu) -> io::Result<u64> {
        match self.dosya_ac(yol) {
            Ok(dosya) => baglanti::sayi(&dosya.into_std()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(1),
            Err(e) => Err(e),
        }
    }

    pub fn yaz(&self, yol: &CalismaYolu, icerik: &[u8]) -> io::Result<()> {
        let git = self.git_kimligi()?;
        let (ust, ad) = self.ust_dizin_denetimli(yol, true, git.as_ref())?;
        let mut secenek = OpenOptions::new();
        // `truncate` yerine önce no-follow açıp sonra set_len kullanılır. Son
        // bileşen son anda symlink/junction yapılırsa open_with başarısız olur.
        secenek.write(true).create(true).follow(FollowSymlinks::No);
        let mut dosya = ust.open_with(&ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalara yazılabilir",
            ));
        }
        // `.git` bir dosyaysa (worktree, alt modül) son bileşen de denetlenir.
        if let Some(g) = &git {
            git_takma_adi_reddet(g, &ad, &dosya.try_clone()?.into_std())?;
        }
        dosya.set_len(0)?;
        dosya.write_all(icerik)
    }

    /// Komutun oluşturduğu tehlikeli `.git`'i aynı üst dizinde
    /// `<ad>.kalfa-karantina-<damga>` adına taşır (güvenlik analizi Bulgu 1).
    /// Bağlantı izlenmez: bağlantının kendisi taşınır. Yeni yolu döner.
    pub fn karantinaya_al(&self, yol: &CalismaYolu, damga: &str) -> io::Result<CalismaYolu> {
        let (ust, ad) = self.ust_dizin(yol, false)?;
        for sira in 0..100 {
            let yeni = match sira {
                0 => format!("{ad}.kalfa-karantina-{damga}"),
                n => format!("{ad}.kalfa-karantina-{damga}-{n}"),
            };
            match ust.symlink_metadata(&yeni) {
                Ok(_) => continue,
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
            ust.rename(&ad, &ust, &yeni)?;
            let yeni_yol = match yol.ust() {
                Some(u) => format!("{u}/{yeni}"),
                None => yeni,
            };
            return CalismaYolu::coz(&yeni_yol)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()));
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "karantina adı bulunamadı",
        ))
    }

    /// Kökteki `.git`'in (dizin ya da dosya) kimliği; yoksa `None`. Bağlantı
    /// izlenmez: bağlantı olan `.git` açılamaz ve denetlenmez, git de onu
    /// izlemez.
    fn git_kimligi(&self) -> io::Result<Option<Handle>> {
        let std_dosya = match self.dizin.open_dir_nofollow(".git") {
            Ok(d) => d.into_std_file(),
            Err(_) => {
                let mut secenek = OpenOptions::new();
                secenek.read(true).follow(FollowSymlinks::No);
                match self.dizin.open_with(".git", &secenek) {
                    Ok(f) => f.into_std(),
                    // Yok ya da bağlantı: denetlenecek bir git dizini yok.
                    Err(_) => return Ok(None),
                }
            }
        };
        Handle::from_file(std_dosya).map(Some)
    }

    /// Çalışma alanındaki dosyaları döner. Yalnızca depo kökündeki
    /// `.gitignore` kuralları uygulanır; global git dışlamaları kullanılmaz.
    /// `.git` ve sembolik bağlantılar izlenmez. İşlem engelleyicidir; araçlar
    /// bunu `spawn_blocking` içinde çağırır.
    pub fn dosyalar(&self, iptal: &CancellationToken) -> Result<Vec<CalismaYolu>, AracHatasi> {
        let gezgin = WalkBuilder::new(&self.kok)
            .hidden(false)
            .parents(false)
            .require_git(false)
            .git_global(false)
            .git_ignore(true)
            .follow_links(false)
            .filter_entry(|g| !g.file_name().to_string_lossy().eq_ignore_ascii_case(".git"))
            .build();
        let mut sonuc = Vec::new();

        for giris in gezgin {
            if iptal.is_cancelled() {
                return Err(AracHatasi::Iptal);
            }
            let Ok(giris) = giris else { continue };
            if !giris.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let Ok(goreli) = giris.path().strip_prefix(&self.kok) else {
                continue;
            };
            if let Ok(yol) = CalismaYolu::coz(&goreli.to_string_lossy()) {
                sonuc.push(yol);
            }
        }

        sonuc.sort();
        Ok(sonuc)
    }
}

/// Geri alma dosya araçlarının yazma yolundan geçer (aşama 3 tasarımı §5.6):
/// bileşenler bağlantı izlenmeden açılır, `.git` takma adı reddedilir, hard
/// link'li dosyaya yazılmaz ve yazmadan hemen önce şimdiki özet denetlenir.
impl GeriYukleyici for CalismaAlani {
    fn durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>> {
        let mut dosya = match self.dosya_ac(yol) {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut veri = Vec::new();
        dosya.read_to_end(&mut veri)?;
        Ok(Some(DosyaDurumu {
            ozet: ozet_hex(&veri),
            baglanti_sayisi: baglanti::sayi(&dosya.into_std())?,
        }))
    }

    fn geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()> {
        let git = self.git_kimligi()?;
        let (ust, ad) = self.ust_dizin_denetimli(yol, false, git.as_ref())?;
        let mut secenek = OpenOptions::new();
        secenek.read(true).write(true).follow(FollowSymlinks::No);
        let mut dosya = ust.open_with(&ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalar geri yazılabilir",
            ));
        }
        if let Some(g) = &git {
            git_takma_adi_reddet(g, &ad, &dosya.try_clone()?.into_std())?;
        }
        let mut veri = Vec::new();
        dosya.read_to_end(&mut veri)?;
        if ozet_hex(&veri) != beklenen {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosya önizlemeden sonra değişti; geri yazılmadı.",
            ));
        }
        if baglanti::sayi(&dosya.try_clone()?.into_std())? > 1 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosyanın başka bir adı var (hard link); geri yazmak öteki adı da değiştirirdi.",
            ));
        }
        match icerik {
            Some(icerik) => {
                dosya.seek(SeekFrom::Start(0))?;
                dosya.set_len(0)?;
                dosya.write_all(icerik)
            }
            None => {
                drop(dosya);
                ust.remove_file(&ad)
            }
        }
    }

    fn bos_dizini_kaldir(&self, yol: &CalismaYolu) -> io::Result<bool> {
        let (ust, ad) = match self.ust_dizin(yol, false) {
            Ok(u) => u,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        match ust.remove_dir(&ad) {
            Ok(()) => Ok(true),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::DirectoryNotEmpty | io::ErrorKind::NotFound
                ) =>
            {
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }
}

/// Nesne kökteki `.git`'in kendisiyse ve ona `.git` adıyla ulaşılmadıysa
/// reddeder.
fn git_takma_adi_reddet(git: &Handle, ad: &str, nesne: &std::fs::File) -> io::Result<()> {
    if ad.eq_ignore_ascii_case(".git") {
        return Ok(());
    }
    if Handle::from_file(nesne.try_clone()?)? == *git {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Git dizinine başka bir adla erişilemez.",
        ));
    }
    Ok(())
}

/// İlk 8 KiB'ta NUL baytı varsa dosya ikili sayılır.
pub(crate) fn ikili_mi(veri: &[u8]) -> bool {
    veri.iter().take(8192).any(|b| *b == 0)
}
