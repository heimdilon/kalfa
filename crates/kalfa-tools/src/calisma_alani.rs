//! Çalışma alanı: `cap-std` dizin tutamacı (spec §7.3).
//!
//! Her üst bileşen ayrı bir `open_dir_nofollow` çağrısıyla, son bileşen de
//! `FollowSymlinks::No` ile açılır. Böylece son dosyada no-follow kullanıp üst
//! zinciri normal açma hatasına düşülmez. v1'de bağlantı izlenmez; çalışma
//! alanının içinde kalan bağlantılar bile reddedilir.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use ignore::WalkBuilder;
use kalfa_core::arac::AracHatasi;
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::ozet_hex;
use kalfa_core::yol::CalismaYolu;
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
        let mut dizin = self.dizin.try_clone()?;
        let mut parcalar = yol.as_str().split('/').peekable();

        while let Some(ad) = parcalar.next() {
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
        let (ust, ad) = self.ust_dizin(yol, true)?;
        let mut secenek = OpenOptions::new();
        // `truncate` yerine önce no-follow açıp sonra set_len kullanılır. Son
        // bileşen son anda symlink/junction yapılırsa open_with başarısız olur.
        secenek.write(true).create(true).follow(FollowSymlinks::No);
        let mut dosya = ust.open_with(ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalara yazılabilir",
            ));
        }
        dosya.set_len(0)?;
        dosya.write_all(icerik)
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

/// İlk 8 KiB'ta NUL baytı varsa dosya ikili sayılır.
pub(crate) fn ikili_mi(veri: &[u8]) -> bool {
    veri.iter().take(8192).any(|b| *b == 0)
}
