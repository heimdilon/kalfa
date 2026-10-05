//! Oturum kaydı: yalnızca sona eklenen JSONL (spec §6.6).
//!
//! Depo içinde değil, uygulama veri dizininde tutulur (yolu uygulama verir).
//! Yan etkili bir araçtan önce yazılan "başladı" kaydı diske ulaşmadan
//! dönülmez (`yaz_kalici`). Çökmede yarım kalan son satır, kayıt devam etmek
//! üzere açılırken kesilir; yoksa yeni kayıtlar o yarım satıra yapışır ve bir
//! sonraki açılışta kayıt ortasından bozuk görünürdü.
//!
//! Aynı kayda aynı anda tek yazıcı açılır: kaydın yanındaki `<kayıt>.kilit`
//! dosyasında işletim sistemi kilidi tutulur (1c tasarımı §5.2). Kilit kaydın
//! kendisine konmaz; Windows'ta dosya kilidi aynı süreçteki okumaları da
//! engellerdi. Çoklu hardlink'i olan kayıtlar açık hata ile reddedilir; bu
//! alias'lar için ortak bir sidecar adı üretilemez.
//!
//! Kilit bırakılırken tanıtıcı kapatılmadan önce açıkça çözülür
//! (`KayitKilidi`). Unix'te `flock` açık dosya tanımına bağlıdır: başka bir iş
//! parçacığının başlattığı alt süreç `fork` ile `exec` arasında tanıtıcının
//! kopyasını taşır ve yalnızca kapatılan kilit o kopya kapanana kadar
//! çözülmezdi. Kilidi kapatan oturum aynı süreçte hemen yeniden açılamaz ya da
//! silinemezdi.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use same_file::Handle;

use kalfa_evren::KrediGozlemi;
use kalfa_evren::tipler::{ChatMessage, ToolDef};
use serde::{Deserialize, Serialize};

use crate::arac::DosyaDegisiklikleri;
use crate::geri_alma::{DosyaYedegi, GeriAlmaSonucu};
use crate::gonderim::{GonderilenOge, IstekSonucu};
use crate::izin::Kip;
use crate::kimlik::{CagriId, TurId};
use crate::muhur::OnayMuhru;
use crate::protokol::{AracDurumu, BitisNedeni, Komut, Zarf};
use crate::tur_ozeti::TurOzeti;

/// Komut kaydının wire türü. Geçmişteki şart kabul komutu yalnızca kayıt
/// uyumluluğu için okunur; bu kayıt EVREN'in kabulü onayladığını göstermez.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum KayitKomutu {
    Guncel(Komut),
    EskiSartKabulIstegi(EskiSartKabulKomutu),
}

/// Kayıtlarda tutulmuş eski şart kabul isteği; yeni `Komut` API'sinin parçası
/// değildir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum EskiSartKabulKomutu {
    #[serde(rename = "SartlariKabulEt", skip_serializing)]
    SartKabulIstegi { surum: u32 },
}

impl From<Komut> for KayitKomutu {
    fn from(komut: Komut) -> Self {
        Self::Guncel(komut)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum KayitOgesi {
    OturumAcildi {
        model: String,
        kip: Kip,
        /// Oturumun açıldığı çalışma alanı; kayıttan açarken araçlar buna göre
        /// kurulur. Bu alanı olmayan eski kayıtlar yalnızca geçmiş gösterir.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        calisma_alani: Option<PathBuf>,
    },
    Komut {
        komut: KayitKomutu,
    },
    Mesaj {
        mesaj: ChatMessage,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kaynak: Option<String>,
    },
    AracBasladi {
        cagri: CagriId,
        model_cagri_id: String,
        arac: String,
        /// Yazan dosya aracının dosya başına geri alma bilgisi (aşama 3
        /// tasarımı §5.3). Önceki hâller bu kayıt yazılmadan önce depoya
        /// ulaşır.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        yedekler: Vec<DosyaYedegi>,
    },
    AracBitti {
        cagri: CagriId,
        durum: AracDurumu,
    },
    /// Aracın bildirdiği güvenlik uyarısı; geçmişte de gösterilir.
    GuvenlikUyarisi {
        cagri: CagriId,
        mesaj: String,
    },
    Istek {
        amac: String,
        istek_id: Option<String>,
        kredi: Option<KrediGozlemi>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        yonlendirilen_model: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        yeni: Vec<GonderilenOge>,
        #[serde(default)]
        onceki: u32,
        #[serde(default)]
        sonuc: IstekSonucu,
    },
    /// Akış açıldıktan sonra kopan istek (aşama 3b).
    IstekKoptu {
        mesaj: String,
    },
    /// Model isteğinde gönderilen sistem istemi (aşama 3b).
    SistemIstemi {
        metin: String,
    },
    /// Model isteğinde gönderilen araç tanımları (aşama 3b).
    AracTanimlari {
        tanimlar: Vec<ToolDef>,
    },
    /// Bir model isteğinin token kullanımı; kayıttan açılan oturumun durum
    /// çubuğu bundan toplanır.
    Kullanim {
        istem: u64,
        tamamlama: u64,
        dusunce: u64,
    },
    KipDegisti {
        kip: Kip,
    },
    /// Göndermeden önce gösterme ayarı değişti (aşama 3b).
    OnizlemeDegisti {
        acik: bool,
    },
    ModelDegisti {
        model: String,
    },
    TurBitti {
        neden: BitisNedeni,
    },
    /// Oturum, kullanıcının şart penceresinde kabul ettiği sürümle sürdü.
    SartKabulEdildi {
        surum: u32,
    },
    /// Geri alma yazmaya başlamadan önce, kalıcı (aşama 3 tasarımı §5.6).
    /// `GeriAlindi`'si olmayan kayıt "geri alma yarım kaldı" demektir.
    GeriAlmaBasladi {
        geri_alinan_tur: TurId,
        muhur: OnayMuhru,
    },
    GeriAlindi {
        geri_alinan_tur: TurId,
        sonuc: Vec<GeriAlmaSonucu>,
    },
    /// Geri almanın modele bildirimi (§5.9). Bağlama kullanıcı rolünde girer
    /// ama kullanıcının mesajı değildir: başlığı, oturum listesindeki durumu
    /// ve kullanıcı balonunu etkilemez.
    KalfaNotu {
        metin: String,
    },
    /// Depo sınırı yüzünden bu turların önceki hâlleri silindi (§5.2).
    GeriAlmaVerisiSilindi {
        turlar: Vec<TurId>,
    },
    /// Kabuk çağrısının çalışırken değişen dosyaları (aşama 3 tasarımı
    /// §5.4); `AracBitti`'den sonra. Geri alınmaz, listelenir.
    KabukDegisiklikleri {
        cagri: CagriId,
        komut: String,
        degisiklikler: DosyaDegisiklikleri,
    },
    /// Turun özeti (§5.5); `TurBitti`'den sonra, aynı tur kimliğiyle.
    TurOzeti {
        ozet: TurOzeti,
    },
}

/// Kaydın yazıldığı yer. Dosya dışındaki hedefler, testlerde yazma hatası
/// benzetmek içindir.
pub trait KayitHedefi: Send {
    /// Satırı sona ekler ve işletim sistemine iletir.
    fn ekle(&mut self, satir: &[u8]) -> io::Result<()>;
    /// Şimdiye kadar eklenenleri kalıcı depolamaya yazar.
    fn kalici_yap(&mut self) -> io::Result<()>;
}

impl KayitHedefi for File {
    fn ekle(&mut self, satir: &[u8]) -> io::Result<()> {
        self.write_all(satir)?;
        self.flush()
    }

    fn kalici_yap(&mut self) -> io::Result<()> {
        self.sync_data()
    }
}

pub struct OturumKaydi {
    hedef: Box<dyn KayitHedefi>,
    /// Kayıt yanındaki sidecar kilidi, kayıt açık kaldığı sürece tutulur.
    _kilit: Option<KayitKilidi>,
}

impl OturumKaydi {
    /// Yeni kayıt oluşturur; dosya zaten varsa hata döner.
    pub fn olustur(yol: &Path) -> io::Result<Self> {
        kilit_uzantisi_kontrol(yol)?;
        if let Some(ust) = yol.parent() {
            fs::create_dir_all(ust)?;
        }
        let kilitli = kilitle(yol)?;
        let dosya = yeni_kayit_dosyasi(&kilitli.yol)?;
        dosya_kimligini_dogrula(&dosya, &kilitli.yol, kilitli.beklenen.as_ref())?;
        Ok(Self {
            hedef: Box::new(dosya),
            _kilit: Some(kilitli.dosya),
        })
    }

    /// Var olan kaydı devam etmek üzere açar ve onarır: çökmede yarım kalan
    /// son satır kesilir, son geçerli satırın satır sonu eksikse eklenir.
    pub fn ac(yol: &Path) -> io::Result<Self> {
        Self::devam_icin_ac(yol).map(|(kayit, _)| kayit)
    }

    /// Önce kilidi alır, sonra kaydı okur ve onarır. Kilit okumadan önce
    /// tutulduğu için arada başka bir yazıcı ekleme yapamaz.
    pub fn devam_icin_ac(yol: &Path) -> io::Result<(Self, Vec<Zarf<KayitOgesi>>)> {
        // Olmayan kayıt için kilit dosyası bırakmamak için önce varlığı sına.
        fs::metadata(yol)?;
        let kilitli = kilitle(yol)?;
        let mut dosya = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&kilitli.yol)?;
        dosya_kimligini_dogrula(&dosya, &kilitli.yol, kilitli.beklenen.as_ref())?;
        dosya.seek(SeekFrom::Start(0))?;
        let mut veri = Vec::new();
        dosya.read_to_end(&mut veri)?;
        let (kayitlar, gecerli) = coz(&veri)?;
        dosya.set_len(gecerli as u64)?;
        dosya.seek(SeekFrom::End(0))?;
        if gecerli > 0 && veri[gecerli - 1] != b'\n' {
            dosya.write_all(b"\n")?;
        }
        dosya.sync_data()?;
        Ok((
            Self {
                hedef: Box::new(dosya),
                _kilit: Some(kilitli.dosya),
            },
            kayitlar,
        ))
    }

    pub fn hedefle(hedef: Box<dyn KayitHedefi>) -> Self {
        Self {
            hedef,
            _kilit: None,
        }
    }

    pub fn yaz(&mut self, zarf: &Zarf<KayitOgesi>) -> io::Result<()> {
        let mut satir = serde_json::to_vec(zarf).map_err(io::Error::other)?;
        satir.push(b'\n');
        self.hedef.ekle(&satir)
    }

    pub fn yaz_kalici(&mut self, zarf: &Zarf<KayitOgesi>) -> io::Result<()> {
        self.yaz(zarf)?;
        self.hedef.kalici_yap()
    }

    /// Kaydı ve kilit dosyasını siler (aşama 3 tasarımı §5.10). Kilit
    /// alınamazsa (kayıt başka bir yerde açık) `ResourceBusy` döner ve hiçbir
    /// şey silinmez. Kilit altında önce `once` çalışır (oturumun geri alma
    /// verisi); o başarısız olursa kayıt kalır ve silme yinelenebilir.
    pub fn sil(yol: &Path, once: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        fs::metadata(yol)?;
        let kilitli = kilitle(yol)?;
        once()?;
        fs::remove_file(&kilitli.yol)?;
        // Kilit tutulurken silinir: arada başka bir yazıcı kilidi alamaz.
        let kilit = sidecar_yolu(&kilitli.yol);
        let sonuc = fs::remove_file(&kilit);
        drop(kilitli);
        match sonuc {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    /// Kaydı okur. Çökme sırasında yarım yazılmış **son** satır atlanır;
    /// ortadaki bozuk bir satır kaydın güvenilmez olduğunu gösterir ve hatadır.
    pub fn oku(yol: &Path) -> io::Result<Vec<Zarf<KayitOgesi>>> {
        Ok(coz(&fs::read(yol)?)?.0)
    }
}

/// Tutulan sidecar kilidi. Bırakılırken tanıtıcı kapatılmadan önce kilit
/// açıkça çözülür: alt süreçlerin `exec` öncesi taşıdığı tanıtıcı kopyaları
/// kilidi tutmaya devam etmesin (modül belgesi).
struct KayitKilidi(File);

impl Drop for KayitKilidi {
    fn drop(&mut self) {
        // Çözme başarısız olursa kilit tanıtıcı kapanınca yine çözülür.
        let _ = self.0.unlock();
    }
}

struct KilitliKayitYolu {
    dosya: KayitKilidi,
    yol: PathBuf,
    beklenen: Option<Handle>,
}

/// Kayıt kilit dosyasının yolu. Mevcut symlink/junction ve mevcut olmayan
/// dosyanın üst dizini mümkün olduğunda kanonikleştirilir; hata durumunda
/// API'nin `PathBuf` imzası için girdinin yan yolu döndürülür.
pub fn kilit_yolu(kayit: &Path) -> PathBuf {
    let yol = kayit_yolunu_coz(kayit).unwrap_or_else(|_| kayit.to_path_buf());
    sidecar_yolu(&yol)
}

fn kilit_uzantisi_kontrol(kayit: &Path) -> io::Result<()> {
    if kayit
        .extension()
        .is_some_and(|uzanti| uzanti.to_string_lossy().eq_ignore_ascii_case("kilit"))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Kayıt adı .kilit ile bitemez; bu uzantı kayıt kilidi dosyasına ayrılmıştır.",
        ));
    }
    Ok(())
}

/// Mevcut kayıt için canonical path; yeni kayıt için canonical üst dizin ve
/// özgün dosya adı. Böylece symlink/junction alias'ları aynı sidecar'ı bulur.
fn kayit_yolunu_coz(kayit: &Path) -> io::Result<PathBuf> {
    match fs::canonicalize(kayit) {
        Ok(yol) => Ok(yol),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let ad = kayit.file_name().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "Kayıt dosya adı gerekli.")
            })?;
            let ust = kayit
                .parent()
                .filter(|ust| !ust.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            Ok(fs::canonicalize(ust)?.join(ad))
        }
        Err(e) => Err(e),
    }
}

fn sidecar_yolu(kayit: &Path) -> PathBuf {
    let mut ad = kayit.as_os_str().to_owned();
    ad.push(".kilit");
    PathBuf::from(ad)
}

/// Kayıt kilidini alır; kilit başka bir yazıcıdaysa `ResourceBusy` döner.
/// Kilit sonrası açılan kayıt file handle'ı canonicalization öncesi kimlikle
/// karşılaştırılır; arada hedef değiştirilmişse açma başarısız olur.
fn kilitle(kayit: &Path) -> io::Result<KilitliKayitYolu> {
    kilit_uzantisi_kontrol(kayit)?;
    let yol = kayit_yolunu_coz(kayit)?;
    kilit_uzantisi_kontrol(&yol)?;
    let beklenen = match fs::metadata(&yol) {
        Ok(_) => Some(Handle::from_path(&yol)?),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let mut secenek = OpenOptions::new();
    secenek.create(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        secenek.mode(0o600);
    }
    let dosya = secenek.open(sidecar_yolu(&yol))?;
    match dosya.try_lock() {
        Ok(()) => Ok(KilitliKayitYolu {
            dosya: KayitKilidi(dosya),
            yol,
            beklenen,
        }),
        Err(TryLockError::WouldBlock) => Err(io::Error::new(
            io::ErrorKind::ResourceBusy,
            "Bu oturum kaydı başka bir yerde açık.",
        )),
        Err(TryLockError::Error(e)) => Err(e),
    }
}

fn yeni_kayit_dosyasi(yol: &Path) -> io::Result<File> {
    let mut secenek = OpenOptions::new();
    secenek.create_new(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        secenek.mode(0o600);
    }
    secenek.open(yol)
}

fn dosya_kimligini_dogrula(dosya: &File, yol: &Path, beklenen: Option<&Handle>) -> io::Result<()> {
    let acilan = Handle::from_file(dosya.try_clone()?)?;
    if beklenen.is_some_and(|beklenen| beklenen != &acilan) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Kayıt dosyası kilit alınırken değişti.",
        ));
    }
    if Handle::from_path(yol)? != acilan {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Açılan kayıt yolu başka bir dosyayı gösteriyor.",
        ));
    }
    tek_baglantili_kayit(dosya)
}

fn tek_baglantili_kayit(dosya: &File) -> io::Result<()> {
    #[cfg(unix)]
    let baglanti_sayisi: u64 = {
        use std::os::unix::fs::MetadataExt;
        dosya.metadata()?.nlink()
    };
    #[cfg(windows)]
    let baglanti_sayisi = {
        use std::mem::MaybeUninit;
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };

        let mut bilgi = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
        // Windows MetadataExt::number_of_links() henüz unstable. Aynı alanı
        // dosyanın açık handle'ı üzerinden Win32 API ile güvenle alıyoruz.
        let basarili =
            unsafe { GetFileInformationByHandle(dosya.as_raw_handle(), bilgi.as_mut_ptr()) };
        if basarili == 0 {
            return Err(io::Error::last_os_error());
        }
        u64::from(unsafe { bilgi.assume_init().nNumberOfLinks })
    };
    #[cfg(not(any(unix, windows)))]
    let baglanti_sayisi = {
        let _ = dosya;
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Bu platformda hardlink sayısı doğrulanamıyor.",
        ));
    };

    if baglanti_sayisi > 1 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Birden fazla hardlink'i olan kayıtlar desteklenmiyor.",
        ));
    }
    Ok(())
}

/// Kaydı çözer. İkinci değer, son geçerli satırın bittiği bayttır (satır sonu
/// dahil); `ac` dosyayı bu uzunluğa keser.
pub(crate) fn coz(veri: &[u8]) -> io::Result<(Vec<Zarf<KayitOgesi>>, usize)> {
    let mut kayitlar = Vec::new();
    let mut gecerli = 0usize;
    let mut bas = 0usize;
    let mut satir_no = 0usize;
    while bas < veri.len() {
        let (satir, sonraki) = match veri[bas..].iter().position(|b| *b == b'\n') {
            Some(i) => (&veri[bas..bas + i], bas + i + 1),
            None => (&veri[bas..], veri.len()),
        };
        if !satir.iter().all(u8::is_ascii_whitespace) {
            satir_no += 1;
            match serde_json::from_slice(satir) {
                Ok(zarf) => {
                    kayitlar.push(zarf);
                    gecerli = sonraki;
                }
                // Çökmede yarım kalan son satır: atlanır, kayıt burada biter.
                Err(_) if veri[sonraki..].iter().all(u8::is_ascii_whitespace) => break,
                Err(e) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("oturum kaydının {satir_no}. satırı bozuk: {e}"),
                    ));
                }
            }
        }
        bas = sonraki;
    }
    Ok((kayitlar, gecerli))
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Alt sürecin `fork` ile `exec` arasında taşıdığı tanıtıcı kopyası
    /// `try_clone` ile taklit edilir: kopya aynı açık dosya tanımını paylaşır.
    fn kilit_kopyasi(kayit: &OturumKaydi) -> File {
        kayit._kilit.as_ref().unwrap().0.try_clone().unwrap()
    }

    #[test]
    fn kilit_tanitici_kopyasi_kapanan_kaydi_tutmaz() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("oturum.jsonl");
        let kayit = OturumKaydi::olustur(&yol).unwrap();
        let kopya = kilit_kopyasi(&kayit);
        drop(kayit);
        let kayit = OturumKaydi::ac(&yol).expect("kopya kilidi tutmamalı");
        assert_eq!(
            OturumKaydi::ac(&yol).err().map(|e| e.kind()),
            Some(io::ErrorKind::ResourceBusy),
            "açık kayıt yine korunur"
        );
        drop(kopya);
        drop(kayit);
    }

    #[test]
    fn kilit_tanitici_kopyasi_kapanan_kaydin_silinmesini_engellemez() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("oturum.jsonl");
        let kayit = OturumKaydi::olustur(&yol).unwrap();
        let kopya = kilit_kopyasi(&kayit);
        drop(kayit);
        OturumKaydi::sil(&yol, || Ok(())).expect("kopya silmeyi engellememeli");
        assert!(!yol.exists());
        assert!(!kilit_yolu(&yol).exists());
        drop(kopya);
    }
}
