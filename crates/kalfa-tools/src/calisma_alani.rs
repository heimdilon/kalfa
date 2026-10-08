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

use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, File, OpenOptions};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use kalfa_core::arac::AracHatasi;
use kalfa_core::geri_alma::{DOSYA_SINIRI, DosyaDurumu, GeriYukleyici};
use kalfa_core::izin::Siniflandirici;
use kalfa_core::muhur::akistan_ozetle;
use kalfa_core::yol::CalismaYolu;
use same_file::Handle;
use tokio_util::sync::CancellationToken;

use crate::baglanti;

/// Metin okuma ve arama tek dosyada en çok 8 MiB tutar.
pub const OKUMA_BAYT_SINIRI: usize = 8 * 1024 * 1024;

pub struct CalismaAlani {
    kok: PathBuf,
    dizin: Dir,
    siniflandirici: Arc<Siniflandirici>,
}

struct TaramaCercevesi {
    dizin: Dir,
    goreli: String,
    girisler: cap_std::fs::ReadDir,
    eski_uzunluk: [usize; 3],
}

pub(crate) struct OncekiDosya {
    pub ozet: String,
    pub bayt: u64,
    pub icerik: Option<Vec<u8>>,
    pub baglanti_sayisi: u64,
    pub gorunmez_icerik: bool,
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
        if dosya.metadata()?.len() > OKUMA_BAYT_SINIRI as u64 {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "Dosya 8 MiB okuma sınırını aşıyor; okunmadı.",
            ));
        }
        sinirli_veri_oku(&mut dosya, &CancellationToken::new())
            .map_err(|e| io::Error::other(e.to_string()))
    }

    /// Çıktı satır sınırından önce, açılan tutamacın baytlarını sınırlar.
    pub fn sinirli_oku(
        &self,
        yol: &CalismaYolu,
        iptal: &CancellationToken,
    ) -> Result<Vec<u8>, AracHatasi> {
        self.sinirli_okuma(yol, iptal, None)
    }

    pub(crate) fn sinirli_onayli_oku(
        &self,
        yol: &CalismaYolu,
        iptal: &CancellationToken,
        baglanti: u64,
    ) -> Result<Vec<u8>, AracHatasi> {
        self.sinirli_okuma(yol, iptal, Some(baglanti))
    }

    fn sinirli_okuma(
        &self,
        yol: &CalismaYolu,
        iptal: &CancellationToken,
        onayli_baglanti: Option<u64>,
    ) -> Result<Vec<u8>, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        Self::sinirli_tutamaci_oku(self.dosya_ac(yol)?.into_std(), iptal, onayli_baglanti)
    }

    fn sinirli_tutamaci_oku(
        mut dosya: std::fs::File,
        iptal: &CancellationToken,
        onayli_baglanti: Option<u64>,
    ) -> Result<Vec<u8>, AracHatasi> {
        let denetle = || -> Result<(), AracHatasi> {
            let sayi = baglanti::sayi(&dosya)?;
            if onayli_baglanti.map_or(sayi > 1, |onayli| sayi != onayli) {
                return Err(AracHatasi::OnayEskidi);
            }
            Ok(())
        };
        denetle()?;
        if dosya.metadata()?.len() > OKUMA_BAYT_SINIRI as u64 {
            return Err(okuma_siniri_hatasi());
        }
        let veri = sinirli_veri_oku(&mut dosya, iptal)?;
        let sayi = baglanti::sayi(&dosya)?;
        if onayli_baglanti.map_or(sayi > 1, |onayli| sayi != onayli) {
            return Err(AracHatasi::OnayEskidi);
        }
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

            let sonraki = match dizin.open_dir_nofollow(ad) {
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
            gercek_ad_dogrula(&dizin, ad, &sonraki.try_clone()?.into_std_file())?;
            dizin = sonraki;
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
        let dizin = ust.open_dir_nofollow(&ad)?;
        gercek_ad_dogrula(&ust, &ad, &dizin.into_std_file())?;
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
                        gercek_ad_dogrula(&dizin, ad, &d.try_clone()?.into_std_file())?;
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
        ad_degistirmeyi_engelle(&mut secenek);
        let dosya = ust.open_with(&ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalar açılabilir",
            ));
        }
        gercek_ad_dogrula(&ust, &ad, &dosya.try_clone()?.into_std())?;
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
        Ok(self.onceki_dosya(yol, false, None)?.map(|d| d.ozet))
    }

    pub(crate) fn onceki_dosya(
        &self,
        yol: &CalismaYolu,
        sakla: bool,
        iptal: Option<&CancellationToken>,
    ) -> io::Result<Option<OncekiDosya>> {
        if iptal.is_some_and(CancellationToken::is_cancelled) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "İptal edildi."));
        }
        let mut dosya = match self.dosya_ac(yol) {
            Ok(d) => d,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        Self::acik_dosyayi_ozetle(&mut dosya, sakla, iptal).map(Some)
    }

    fn acik_dosyayi_ozetle(
        dosya: &mut File,
        sakla: bool,
        iptal: Option<&CancellationToken>,
    ) -> io::Result<OncekiDosya> {
        let boyut = dosya.metadata()?.len();
        let baglanti_sayisi = baglanti::sayi(&dosya.try_clone()?.into_std())?;
        // Sürekli büyüyen dosya sonsuz özet işine dönüşmez.
        let azami = boyut
            .checked_add(1)
            .ok_or_else(|| io::Error::other("Dosya boyutu taşması."))?;
        let mut okuyucu = (&mut *dosya).take(azami);
        let okunan = akistan_ozetle(
            &mut okuyucu,
            if sakla { DOSYA_SINIRI as usize } else { 0 },
            || {
                if iptal.is_some_and(CancellationToken::is_cancelled) {
                    Err(io::Error::new(io::ErrorKind::Interrupted, "İptal edildi."))
                } else {
                    Ok(())
                }
            },
        )?;
        if okunan.bayt != boyut
            || dosya.metadata()?.len() != boyut
            || baglanti::sayi(&dosya.try_clone()?.into_std())? != baglanti_sayisi
        {
            return Err(io::Error::other(
                "Dosya okuma sırasında değişti; işlem yapılmadı.",
            ));
        }
        Ok(OncekiDosya {
            ozet: okunan.ozet,
            bayt: okunan.bayt,
            icerik: okunan.icerik,
            baglanti_sayisi,
            gorunmez_icerik: okunan.gorunmez_icerik,
        })
    }

    /// Dosyanın boyutu ve son değişme zamanı; dosya okumanın yolundan açılır
    /// (bağlantı izlenmez, yalnızca normal dosya). Eklentinin dosya listesi
    /// için (aşama 4 tasarımı §6.3).
    pub fn dosya_bilgisi(&self, yol: &CalismaYolu) -> io::Result<(u64, std::time::SystemTime)> {
        let bilgi = self.dosya_ac(yol)?.metadata()?;
        Ok((bilgi.len(), bilgi.modified()?.into_std()))
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
        ad_degistirmeyi_engelle(&mut secenek);
        let mut dosya = ust.open_with(&ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalara yazılabilir",
            ));
        }
        gercek_ad_dogrula(&ust, &ad, &dosya.try_clone()?.into_std())?;
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
        #[cfg(windows)]
        gercek_ad_bul(&ust, &ad)?;
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

    /// Çalışma alanındaki dosyaları döner. Yerel `.ignore`, `.gitignore` ve
    /// `.git/info/exclude` yalnız denetimli tutamaçtan okunur; global git
    /// dışlamaları kullanılmaz.
    /// `.git` ve sembolik bağlantılar izlenmez. İşlem engelleyicidir; araçlar
    /// bunu `spawn_blocking` içinde çağırır.
    pub fn dosyalar(&self, iptal: &CancellationToken) -> Result<Vec<CalismaYolu>, AracHatasi> {
        let mut sonuc = Vec::new();
        let mut kurallar = [Vec::new(), Vec::new(), Vec::new()];
        let kok =
            self.tarama_cercevesi(self.dizin.try_clone()?, String::new(), &mut kurallar, iptal)?;
        let mut cerceveler = vec![kok];
        while let Some(cerceve) = cerceveler.last_mut() {
            if iptal.is_cancelled() {
                return Err(AracHatasi::Iptal);
            }
            let Some(giris) = cerceve.girisler.next() else {
                let uzunluk = cerceve.eski_uzunluk;
                cerceveler.pop();
                for (aile, eski) in kurallar.iter_mut().zip(uzunluk) {
                    aile.truncate(eski);
                }
                continue;
            };
            let Ok(giris) = giris else { continue };
            let ad = giris.file_name();
            let Some(ad) = ad.to_str() else { continue };
            if ad.eq_ignore_ascii_case(".git") {
                continue;
            }
            let Ok(tur) = giris.file_type() else { continue };
            if !tur.is_file() && !tur.is_dir() {
                continue;
            }
            let goreli_yol = if cerceve.goreli.is_empty() {
                ad.to_string()
            } else {
                format!("{}/{ad}", cerceve.goreli)
            };
            let Ok(yol) = CalismaYolu::coz(&goreli_yol) else {
                continue;
            };
            let tam = self.kok.join(&goreli_yol);
            // Öncelik: .ignore, .gitignore, info/exclude; her türde en yakın dizin.
            let eslesme = kurallar.iter().find_map(|aile| {
                aile.iter()
                    .rev()
                    .map(|k| k.matched(&tam, tur.is_dir()))
                    .find(|m| !m.is_none())
            });
            if eslesme.is_some_and(|m| m.is_ignore()) {
                continue;
            }
            if tur.is_dir() {
                let Ok(alt) = cerceve.dizin.open_dir_nofollow(ad) else {
                    continue;
                };
                gercek_ad_dogrula(&cerceve.dizin, ad, &alt.try_clone()?.into_std_file())?;
                let alt = self.tarama_cercevesi(alt, goreli_yol, &mut kurallar, iptal)?;
                cerceveler.push(alt);
            } else {
                sonuc.push(yol);
            }
        }
        sonuc.sort();
        Ok(sonuc)
    }

    fn dislama_kurali(
        &self,
        dizin: &str,
        ad: &str,
        iptal: &CancellationToken,
    ) -> Result<Option<Gitignore>, AracHatasi> {
        let goreli = if dizin.is_empty() {
            ad.to_string()
        } else {
            format!("{dizin}/{ad}")
        };
        let yol = CalismaYolu::coz(&goreli)?;
        let dosya = match self.dosya_ac(&yol) {
            Ok(d) => d.into_std(),
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(None);
            }
            Err(_) => return Err(dislama_hatasi()),
        };
        if self.siniflandirici.gizli_mi(&yol) {
            return Err(dislama_hatasi());
        }
        let veri = Self::sinirli_tutamaci_oku(dosya, iptal, None).map_err(|e| {
            if e == AracHatasi::Iptal {
                e
            } else {
                dislama_hatasi()
            }
        })?;
        let kok = self.kok.join(dizin);
        let mut yapici = GitignoreBuilder::new(kok);
        for (no, satir) in io::Cursor::new(veri).lines().enumerate() {
            let Ok(satir) = satir else { break };
            let satir = if no == 0 {
                satir.trim_start_matches('\u{feff}')
            } else {
                &satir
            };
            // Eski gezgin gibi geçersiz kuralları atlar; içerik hata metnine girmez.
            let _ = yapici.add_line(None, satir);
        }
        yapici.build().map(Some).map_err(|_| dislama_hatasi())
    }

    fn tarama_cercevesi(
        &self,
        dizin: Dir,
        goreli: String,
        kurallar: &mut [Vec<Gitignore>; 3],
        iptal: &CancellationToken,
    ) -> Result<TaramaCercevesi, AracHatasi> {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let eski_uzunluk = kurallar.each_ref().map(Vec::len);
        for (tur, ad) in [(0, ".ignore"), (1, ".gitignore"), (2, ".git/info/exclude")] {
            if let Some(kural) = self.dislama_kurali(&goreli, ad, iptal)? {
                kurallar[tur].push(kural);
            }
        }
        Ok(TaramaCercevesi {
            girisler: dizin.entries()?,
            dizin,
            goreli,
            eski_uzunluk,
        })
    }
}

fn dislama_hatasi() -> AracHatasi {
    AracHatasi::Gecersiz("Dışlama dosyası güvenle okunamadı; listeleme ve arama yapılmadı.".into())
}

/// Geri alma dosya araçlarının yazma yolundan geçer (aşama 3 tasarımı §5.6):
/// bileşenler bağlantı izlenmeden açılır, `.git` takma adı reddedilir, hard
/// link'li dosyaya yazılmaz ve yazmadan hemen önce şimdiki özet denetlenir.
impl GeriYukleyici for CalismaAlani {
    fn durum(&self, yol: &CalismaYolu) -> io::Result<Option<DosyaDurumu>> {
        Ok(self.onceki_dosya(yol, false, None)?.map(|d| DosyaDurumu {
            ozet: d.ozet,
            baglanti_sayisi: d.baglanti_sayisi,
        }))
    }

    fn geri_yaz(&self, yol: &CalismaYolu, icerik: Option<&[u8]>, beklenen: &str) -> io::Result<()> {
        let git = self.git_kimligi()?;
        let (ust, ad) = self.ust_dizin_denetimli(yol, false, git.as_ref())?;
        let mut secenek = OpenOptions::new();
        secenek.read(true).write(true).follow(FollowSymlinks::No);
        ad_degistirmeyi_engelle(&mut secenek);
        let mut dosya = ust.open_with(&ad, &secenek)?;
        if !dosya.metadata()?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Yalnızca normal dosyalar geri yazılabilir",
            ));
        }
        gercek_ad_dogrula(&ust, &ad, &dosya.try_clone()?.into_std())?;
        if let Some(g) = &git {
            git_takma_adi_reddet(g, &ad, &dosya.try_clone()?.into_std())?;
        }
        if Self::acik_dosyayi_ozetle(&mut dosya, false, None)?.ozet != beklenen {
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
        #[cfg(windows)]
        {
            let dizin = match ust.open_dir_nofollow(&ad) {
                Ok(d) => d,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
                Err(e) => return Err(e),
            };
            gercek_ad_dogrula(&ust, &ad, &dizin.try_clone()?.into_std_file())?;
            drop(dizin);
        }
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

/// Açılan nesneye ulaşan yazım, gerçek dizin girdisinin adı olmalıdır.
/// Kısa adın biçimini tahmin etmek yerine uzun adı ve tutamaç kimliğini bağlar.
fn gercek_ad_dogrula(ust: &Dir, ad: &str, nesne: &std::fs::File) -> io::Result<()> {
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        // Hızlı yol: tutamacın kendi uzun adı. Dizin taraması her dosyada
        // yinelenince arama dizin başına karesel yavaşlıyordu.
        if let Ok(gercek) = baglanti::gercek_ad(nesne) {
            let gercek = gercek.to_string_lossy();
            if gercek == ad || gercek.to_lowercase() == ad.to_lowercase() {
                return Ok(());
            }
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosyaya gerçek adı dışında bir adla erişilemez.",
            ));
        }
        // Normalleştirilmiş adı vermeyen dosya sistemlerinde dizin taranır.
        let gercek = gercek_ad_bul(ust, ad)?;
        let acilan = if nesne.metadata()?.is_dir() {
            ust.open_dir_nofollow(&gercek)?.into_std_file()
        } else {
            let mut secenek = OpenOptions::new();
            // Kimlik için veri okuma yetkisi gerekmez; kör yazma korunur.
            secenek.access_mode(0).follow(FollowSymlinks::No);
            ust.open_with(&gercek, &secenek)?.into_std()
        };
        if Handle::from_file(acilan)? != Handle::from_file(nesne.try_clone()?)? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Dosya adı ile açılan nesne uyuşmuyor.",
            ));
        }
    }
    #[cfg(not(windows))]
    let _ = (ust, ad, nesne);
    Ok(())
}

#[cfg(windows)]
fn gercek_ad_bul(ust: &Dir, ad: &str) -> io::Result<std::ffi::OsString> {
    let mut harf_farki = None;
    for giris in ust.entries()? {
        let ad_girdisi = giris?.file_name();
        if let Some(g) = ad_girdisi.to_str() {
            // Windows'ta harfe duyarlı dizinlerde iki ayrı nesne bulunabilir.
            if g == ad {
                return Ok(ad_girdisi);
            }
            if g.to_lowercase() == ad.to_lowercase() {
                harf_farki = Some(ad_girdisi);
            }
        }
    }
    if let Some(ad) = harf_farki {
        return Ok(ad);
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "Dosyaya gerçek adı dışında bir adla erişilemez.",
    ))
}

fn ad_degistirmeyi_engelle(secenek: &mut OpenOptions) {
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};
        // Ad ile kimlik doğrulandıktan sonra açık dosya yeniden adlandırılamaz.
        secenek.share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    #[cfg(not(windows))]
    let _ = secenek;
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

fn okuma_siniri_hatasi() -> AracHatasi {
    AracHatasi::Gecersiz("Dosya 8 MiB okuma sınırını aşıyor; okunmadı.".into())
}

fn sinirli_veri_oku(
    okuyucu: &mut impl Read,
    iptal: &CancellationToken,
) -> Result<Vec<u8>, AracHatasi> {
    let mut veri = Vec::new();
    let mut tampon = [0; 64 * 1024];
    loop {
        if iptal.is_cancelled() {
            return Err(AracHatasi::Iptal);
        }
        let adet = okuyucu.read(&mut tampon)?;
        if adet == 0 {
            return Ok(veri);
        }
        if adet > OKUMA_BAYT_SINIRI - veri.len() {
            return Err(okuma_siniri_hatasi());
        }
        veri.extend_from_slice(&tampon[..adet]);
    }
}

#[cfg(test)]
mod okuma_siniri_testleri {
    use super::*;

    #[test]
    fn buyuyen_kaynak_metadata_olmadan_da_sinirlanir() {
        let iptal = CancellationToken::new();
        assert!(
            sinirli_veri_oku(&mut io::repeat(b'a').take(OKUMA_BAYT_SINIRI as u64), &iptal).is_ok()
        );
        assert!(sinirli_veri_oku(&mut io::repeat(b'a'), &iptal).is_err());
    }

    #[test]
    fn okuma_sirasinda_iptal_tamponlar_arasinda_islenir() {
        struct IptalEden(CancellationToken);
        impl Read for IptalEden {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                b.fill(b'a');
                self.0.cancel();
                Ok(b.len())
            }
        }
        let iptal = CancellationToken::new();
        assert!(matches!(
            sinirli_veri_oku(&mut IptalEden(iptal.clone()), &iptal),
            Err(AracHatasi::Iptal)
        ));
    }
}

#[cfg(all(test, windows))]
mod gercek_ad_testleri {
    use super::*;

    #[test]
    fn ayni_adla_degisik_nesne_onaylanan_tutamaci_degistiremez() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("normal.txt"), b"normal").unwrap();
        std::fs::write(d.path().join(".env"), b"sir").unwrap();
        let ust = Dir::open_ambient_dir(d.path(), ambient_authority()).unwrap();
        let normal = ust.open("normal.txt").unwrap().into_std();
        let gizli = ust.open(".env").unwrap().into_std();
        assert!(gercek_ad_dogrula(&ust, "normal.txt", &normal).is_ok());
        assert!(gercek_ad_dogrula(&ust, "NORMAL.TXT", &normal).is_ok());
        // Hard link kendi adıyla açılınca o bağlantının adı döner.
        std::fs::hard_link(d.path().join("normal.txt"), d.path().join("ikinci.txt")).unwrap();
        let ikinci = ust.open("ikinci.txt").unwrap().into_std();
        assert!(gercek_ad_dogrula(&ust, "ikinci.txt", &ikinci).is_ok());
        assert_eq!(
            gercek_ad_dogrula(&ust, "normal.txt", &gizli)
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
