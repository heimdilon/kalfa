//! Başlamış bir komut: çıktının okunması, süre, iptal ve süreç ağacının
//! sonlandırılması (aşama 2 tasarımı §6.2, §6.4).

use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::agac::{self, Agac, AgacDenetimi};
use crate::{BaslatmaHatasi, CalistirmaIstegi, program};

/// `SIGTERM`'den sonra `SIGKILL`'e kadar beklenen süre.
const KIBAR_SURE: Duration = Duration::from_secs(2);
/// Komut bittikten sonra çıktı borularının kapanması için beklenen en uzun
/// süre. Boruyu tutan torun süreçler ağaçla birlikte sonlandırılır.
const BOSALTMA_SURESI: Duration = Duration::from_secs(2);
const OKUMA_PARCASI: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Akis {
    Standart,
    Hata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bitis {
    /// Komut kendisi çıktı. Sinyalle ölen süreçte kod yoktur.
    Cikti(Option<i32>),
    ZamanAsimi,
    Iptal,
}

pub type AgKapatici =
    Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = crate::vekil::AgOzeti> + Send>> + Send>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalismaSonucu {
    pub bitis: Bitis,
    pub sure: Duration,
    pub ag: Option<crate::vekil::AgOzeti>,
}

pub struct Surec {
    cocuk: Cocuk,
    agac: Agac,
    ag_kapat: Option<AgKapatici>,
}

enum Cocuk {
    Tokio(Box<Child>),
    /// Windows kabı: Kalfa'nın `CreateProcessW` ile başlattığı süreç (§9.3).
    #[cfg(windows)]
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
                let hata = k
                    .hata
                    .map(|h| Box::new(tokio::fs::File::from_std(h)) as Okuyucu);
                let surec = k.surec;
                (
                    Some(cikti),
                    hata,
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

impl Surec {
    /// İsteği yalıtımsız başlatır: yalnızca süreç sınırları uygulanır.
    pub(crate) fn baslat(istek: &CalistirmaIstegi) -> Result<Self, BaslatmaHatasi> {
        let (yol, argumanlar) = program::coz(&istek.komut, &istek.ortam)?;
        let mut komut = Command::new(yol);
        komut
            .args(argumanlar)
            .env_clear()
            .envs(istek.ortam.iter().map(|(a, d)| (a, d)))
            .current_dir(&istek.dizin);
        Self::komutla(komut)
    }

    /// Kurulmuş bir komutu (örneğin bwrap) ağacın kökü olarak başlatır:
    /// standart girdi kapalıdır, çıktılar okunur.
    pub(crate) fn komutla(mut komut: Command) -> Result<Self, BaslatmaHatasi> {
        komut
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let (cocuk, agac) = agac::baslat(&mut komut)?;
        Ok(Self {
            cocuk: Cocuk::Tokio(Box::new(cocuk)),
            agac,
            ag_kapat: None,
        })
    }

    /// Kapta başlamış ve işe atanmış süreç.
    #[cfg(windows)]
    pub(crate) fn kaptan(kap: crate::windows::kap::Kap, is: agac::Is) -> Self {
        Self {
            cocuk: Cocuk::Kap(kap),
            agac: Agac::isten(is),
            ag_kapat: None,
        }
    }

    /// Komut bittiğinde vekili ve köprüyü kapatıp ağ özetini döndürecek işlevi ekler.
    pub fn ag_kapat_ekle(&mut self, kapat: AgKapatici) {
        self.ag_kapat = Some(kapat);
    }

    /// Komut bitene, süre dolana ya da iptal gelene kadar bekler; çıktı
    /// parçalarını geldikçe `cikti`'ya verir. Döndüğünde süreç ağacı
    /// sonlandırılmıştır.
    pub async fn bekle(
        self,
        sure: Duration,
        iptal: CancellationToken,
        mut cikti: impl FnMut(Akis, &[u8]),
    ) -> CalismaSonucu {
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
        drop(tx);

        let son = baslangic + sure;
        let mut bitis: Option<Bitis> = None;
        let mut cikis_alindi = false;
        let mut boru_kapali = false;
        let mut oldurme: Option<Instant> = None;
        let mut bosaltma: Option<Instant> = None;
        let uzak = baslangic + Duration::from_secs(365 * 24 * 3600);

        loop {
            if cikis_alindi && boru_kapali {
                break;
            }
            tokio::select! {
                biased;
                parca = rx.recv(), if !boru_kapali => match parca {
                    Some((akis, veri)) => cikti(akis, &veri),
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
                }
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
                _ = sleep_until(bosaltma.unwrap_or(uzak)), if bosaltma.is_some() => break,
            }
        }
        agac.oldur();
        for okuyucu in okuyucular {
            okuyucu.abort();
        }
        let ag = match self.ag_kapat {
            Some(k) => Some(k().await),
            None => None,
        };
        CalismaSonucu {
            bitis: bitis.unwrap_or(Bitis::Cikti(None)),
            sure: baslangic.elapsed(),
            ag,
        }
    }
}

type Yazici = Box<dyn tokio::io::AsyncWrite + Send + Unpin>;

/// Ağaç öldürüldükten sonra bitişin doğrulanması için tanınan en uzun süre.
const OLUM_BEKLEMESI: Duration = Duration::from_secs(5);
/// Kalfa'nın stdin'e aktarmayı beklettiği en çok veri.
const GIRDI_TAMPONU: usize = 64 * 1024;
const YOKLAMA: Duration = Duration::from_millis(10);

/// Açık bir sözde terminalin (Linux PTY / Windows ConPTY) boyutlandırma ve
/// kapatma denetimi.
#[derive(Clone)]
pub struct TerminalDenetimi {
    ic: std::sync::Arc<std::sync::Mutex<Option<TerminalTutamaci>>>,
}

#[cfg(unix)]
struct TerminalTutamaci(std::os::fd::OwnedFd);

#[cfg(windows)]
struct TerminalTutamaci(windows_sys::Win32::System::Console::HPCON);

#[cfg(windows)]
// SAFETY: `HPCON` konsol oturumu tutamacıdır; `Mutex` altında senkronize erişilir.
unsafe impl Send for TerminalTutamaci {}

#[cfg(windows)]
impl Drop for TerminalTutamaci {
    fn drop(&mut self) {
        // SAFETY: `self.0` `CreatePseudoConsole` ile oluşturuldu ve yalnız bir kez kapatılır.
        unsafe {
            windows_sys::Win32::System::Console::ClosePseudoConsole(self.0);
        }
    }
}

#[cfg(not(any(unix, windows)))]
struct TerminalTutamaci;

impl TerminalDenetimi {
    #[cfg(unix)]
    pub(crate) fn unix_yeni(master: std::os::fd::OwnedFd) -> Self {
        Self {
            ic: std::sync::Arc::new(std::sync::Mutex::new(Some(TerminalTutamaci(master)))),
        }
    }

    #[cfg(windows)]
    pub(crate) fn win_yeni(
        hpcon: windows_sys::Win32::System::Console::HPCON,
    ) -> (Self, windows_sys::Win32::System::Console::HPCON) {
        (
            Self {
                ic: std::sync::Arc::new(std::sync::Mutex::new(Some(TerminalTutamaci(hpcon)))),
            },
            hpcon,
        )
    }

    pub fn boyut_degistir(&self, boyut: crate::TerminalBoyutu) -> std::io::Result<()> {
        boyut
            .dogrula()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
        let kilit = self.ic.lock().unwrap_or_else(|z| z.into_inner());
        let Some(tutamac) = kilit.as_ref() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "terminal kapandı",
            ));
        };
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let ws = libc::winsize {
                ws_row: boyut.satir,
                ws_col: boyut.sutun,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            // SAFETY: `tutamac.0` açık PTY master tanımlayıcısıdır; `ws` geçerli `winsize` yapısıdır.
            if unsafe { libc::ioctl(tutamac.0.as_raw_fd(), libc::TIOCSWINSZ, &ws) } == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        }
        #[cfg(windows)]
        {
            let coord = windows_sys::Win32::System::Console::COORD {
                X: i16::try_from(boyut.sutun).unwrap_or(i16::MAX),
                Y: i16::try_from(boyut.satir).unwrap_or(i16::MAX),
            };
            // SAFETY: `tutamac.0` açık ConPTY tutamacıdır; `coord` doğrulanmış boyuttur.
            let hr = unsafe {
                windows_sys::Win32::System::Console::ResizePseudoConsole(tutamac.0, coord)
            };
            if hr != 0 {
                return Err(std::io::Error::from_raw_os_error(hr));
            }
            Ok(())
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = tutamac;
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "bu platformda terminal boyutlandırma desteklenmiyor",
            ))
        }
    }

    pub fn kapat(&self) {
        let mut kilit = self.ic.lock().unwrap_or_else(|z| z.into_inner());
        let _ = kilit.take();
    }

    fn eof_baytlari(&self, satir_basi: bool) -> Vec<u8> {
        let kilit = self.ic.lock().unwrap_or_else(|z| z.into_inner());
        let Some(tutamac) = kilit.as_ref() else {
            return Vec::new();
        };
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            // SAFETY: `termios` yalnız sayısal alanlardan oluşur; sıfırlanması geçerlidir.
            let mut termios: libc::termios = unsafe { std::mem::zeroed() };
            // SAFETY: `tutamac.0` açık PTY master tanımlayıcısıdır.
            let veof = if unsafe { libc::tcgetattr(tutamac.0.as_raw_fd(), &mut termios) } == 0
                && termios.c_cc[libc::VEOF] != 0
            {
                termios.c_cc[libc::VEOF]
            } else {
                0x04
            };
            if satir_basi {
                vec![veof]
            } else {
                vec![veof, veof]
            }
        }
        #[cfg(windows)]
        {
            let _ = tutamac;
            if satir_basi {
                b"\x1a\r\n".to_vec()
            } else {
                b"\r\n\x1a\r\n".to_vec()
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = (tutamac, satir_basi);
            Vec::new()
        }
    }
}

type KokBitisi = Pin<Box<dyn Future<Output = ()> + Send>>;

async fn girdi_aktar(
    mut ic: tokio::io::DuplexStream,
    mut boru: Yazici,
    terminal: Option<TerminalDenetimi>,
) {
    let mut tampon = [0u8; 8192];
    let mut son_bayt: Option<u8> = None;
    loop {
        match tokio::io::AsyncReadExt::read(&mut ic, &mut tampon).await {
            Ok(0) => break,
            Ok(n) => {
                son_bayt = Some(tampon[n - 1]);
                if tokio::io::AsyncWriteExt::write_all(&mut boru, &tampon[..n])
                    .await
                    .is_err()
                {
                    return;
                }
                let _ = tokio::io::AsyncWriteExt::flush(&mut boru).await;
            }
            Err(_) => return,
        }
    }
    if let Some(term) = terminal {
        let satir_basi = matches!(son_bayt, None | Some(b'\n') | Some(b'\r'));
        let eof_dizisi = term.eof_baytlari(satir_basi);
        if !eof_dizisi.is_empty() {
            let _ = tokio::io::AsyncWriteExt::write_all(&mut boru, &eof_dizisi).await;
            let _ = tokio::io::AsyncWriteExt::flush(&mut boru).await;
        }
        #[cfg(windows)]
        std::future::pending::<()>().await;
        #[cfg(unix)]
        {
            let tek = term.eof_baytlari(true);
            if !tek.is_empty() {
                loop {
                    tokio::time::sleep(Duration::from_millis(25)).await;
                    if tokio::io::AsyncWriteExt::write_all(&mut boru, &tek)
                        .await
                        .is_err()
                    {
                        break;
                    }
                    let _ = tokio::io::AsyncWriteExt::flush(&mut boru).await;
                }
            }
        }
    } else {
        let _ = tokio::io::AsyncWriteExt::shutdown(&mut boru).await;
    }
}

/// Uzun yaşayan, borulu ya da terminalli süreç (1.1a §4.1, G2b/G2c). Bırakılırsa
/// (örneğin panik sırasında) ağaç yine öldürülür; bitişi ise yalnız `kapat`
/// doğrular.
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
    /// Linux'ta çapa alt süreç toplayıcısının (`R`) toplanmasını bekleyen gelecek.
    /// Kök komut (`C`) kendiliğinden çıktığında `bitisi` hemen tamamlanır, ancak
    /// `R` ancak `agaci_bitir` içinde torunlar öldürüldükten sonra toplanır.
    kok_bitisi: Option<KokBitisi>,
    terminal: Option<TerminalDenetimi>,
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

#[cfg(target_os = "linux")]
fn durum_borusundan_bitis(
    durum_okuma: std::os::fd::OwnedFd,
    mut cocuk: Child,
) -> std::io::Result<(Bitisi, Option<KokBitisi>)> {
    let mut alici = tokio::net::unix::pipe::Receiver::from_owned_fd(durum_okuma)?;
    let bitisi: Bitisi = Box::pin(async move {
        let mut baytlar = [0u8; 4];
        match alici.read_exact(&mut baytlar).await {
            Ok(_) => {
                let kod = i32::from_ne_bytes(baytlar);
                if kod == i32::MIN { None } else { Some(kod) }
            }
            Err(_) => None,
        }
    });
    let kok_bitisi: KokBitisi = Box::pin(async move {
        let _ = cocuk.wait().await;
    });
    Ok((bitisi, Some(kok_bitisi)))
}

impl EtkilesimliSurec {
    fn yeni_ic(
        girdi: Option<Yazici>,
        cikti: Option<Okuyucu>,
        hata: Option<Okuyucu>,
        bitisi: Bitisi,
        kok_bitisi: Option<KokBitisi>,
        agac: Agac,
        terminal: Option<TerminalDenetimi>,
    ) -> Result<Self, BaslatmaHatasi> {
        let terminal_kopya = terminal.clone();
        let (girdi, girdi_gorevi) = match girdi {
            Some(boru) => {
                let calisma = tokio::runtime::Handle::try_current().map_err(|_| {
                    BaslatmaHatasi::Yalitim(
                        "etkileşimli süreç tokio çalışma zamanının dışında başlatılamaz".into(),
                    )
                })?;
                let (dis, ic) = tokio::io::duplex(GIRDI_TAMPONU);
                let gorev = calisma.spawn(girdi_aktar(ic, boru, terminal_kopya));
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
            kok_bitisi,
            terminal,
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

    pub fn terminal_denetimi(&self) -> Option<TerminalDenetimi> {
        self.terminal.clone()
    }

    pub fn boyut_degistir(&self, boyut: crate::TerminalBoyutu) -> std::io::Result<()> {
        match &self.terminal {
            Some(t) => t.boyut_degistir(boyut),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "bu süreç sözde terminalle başlatılmadı",
            )),
        }
    }

    pub fn ag_kapat_ekle(&mut self, kapat: AgKapatici) {
        self.ag_kapat = Some(kapat);
    }

    /// Kök süreç kendiliğinden bitmiş mi diye beklemeden yoklar.
    pub fn bitisi_yokla(&mut self) -> Option<Option<i32>> {
        if let Some(kod) = self.cikis {
            return Some(kod);
        }
        let bitisi = self.bitisi.as_mut()?;
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match bitisi.as_mut().poll(&mut cx) {
            std::task::Poll::Ready(kod) => {
                self.bitisi = None;
                self.cikis = Some(kod);
                Some(kod)
            }
            std::task::Poll::Pending => None,
        }
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
    pub async fn kapat(self) -> std::io::Result<KapanisSonucu> {
        self.kapat_kadar(Instant::now() + crate::etkilesimli::KAPANMA_PAYI)
            .await
    }

    /// MCP'nin nazik iptal/flush aşamasıyla aynı kapanma payını paylaşır.
    /// Verilen son an en çok normal pay kadar olabilir; geçmişse doğrudan
    /// zorunlu öldürme ve doğrulamaya geçilir. Bu yol da stdin'i keser ve
    /// ağacın durduğunu doğrulamadan başarı dönmez.
    pub async fn kapat_kadar(mut self, son: Instant) -> std::io::Result<KapanisSonucu> {
        let bas = std::time::Instant::now();
        let son = son.min(Instant::now() + crate::etkilesimli::KAPANMA_PAYI);
        self.girdiyi_kes();
        let kendiliginden = self.cikis.is_some()
            || tokio::time::timeout_at(son, self.bitisi_bekle())
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

    fn terminali_kapat(&mut self) {
        if let Some(t) = self.terminal.take() {
            t.kapat();
        }
    }

    async fn agaci_bitir(&mut self) -> std::io::Result<()> {
        let son = Instant::now() + OLUM_BEKLEMESI;
        // Linux'ta çapa süreç (R) henüz toplanmadığı için (`kok_bitisi.is_some()`),
        // kök komut (C) erken çıkmış olsa bile torunlar R'ye bağlıdır ve taranır.
        let kok_toplanmadi = self.kok_bitisi.is_some() || self.cikis.is_none();
        let oldurme = agaci_oldur(&self.agac, kok_toplanmadi, son).await;
        // G2a (G2.5): ağaç öldürüldükten hemen sonra sözde terminal kapatılır;
        // böylece ConPTY tam çıktı altında bloklanmaz ve çıktı borusu EOF alır.
        self.terminali_kapat();
        oldurme?;
        if let Some(kok) = self.kok_bitisi.take() {
            tokio::time::timeout_at(son, kok)
                .await
                .map_err(|_| zaman_asimi("süreç ağacı çapası öldürüldükten sonra bitmedi"))?;
        }
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
            self.terminali_kapat();
            return;
        }
        if self.kok_bitisi.is_some() || self.cikis.is_none() {
            let son = std::time::Instant::now() + Duration::from_millis(500);
            while matches!(self.agac.torunlari_oldur(), Ok(true)) && std::time::Instant::now() < son
            {
                std::thread::sleep(YOKLAMA);
            }
        }
        let _ = self.agac.oldur_denetimli();
        self.terminali_kapat();
    }
}

#[cfg(unix)]
fn pty_ac(
    boyut: crate::TerminalBoyutu,
) -> std::io::Result<(std::os::fd::OwnedFd, std::os::fd::OwnedFd)> {
    use std::os::fd::FromRawFd;
    boyut
        .dogrula()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string()))?;
    let mut master: libc::c_int = -1;
    let mut slave: libc::c_int = -1;
    let ws = libc::winsize {
        ws_row: boyut.satir,
        ws_col: boyut.sutun,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: `openpty` geçerli yerel tam sayı ve `winsize` göstericileri alır;
    // dönen tanımlayıcılar hemen `OwnedFd` ile sahiplenilir.
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &ws,
        )
    } == -1
    {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: `master` ve `slave` `openpty` tarafından yeni açıldı.
    let master = unsafe { std::os::fd::OwnedFd::from_raw_fd(master) };
    let slave = unsafe { std::os::fd::OwnedFd::from_raw_fd(slave) };
    // Master tarafı çocuğa sızmasın.
    // SAFETY: `master` geçerli tanımlayıcıdır.
    unsafe {
        use std::os::fd::AsRawFd;
        let bayrak = libc::fcntl(master.as_raw_fd(), libc::F_GETFD);
        if bayrak >= 0 {
            let _ = libc::fcntl(master.as_raw_fd(), libc::F_SETFD, bayrak | libc::FD_CLOEXEC);
        }
    }
    Ok((master, slave))
}

pub(crate) fn terminal_ortami_ekle(ortam: &mut Vec<(String, String)>) {
    if let Some((_, deger)) = ortam
        .iter_mut()
        .find(|(a, _)| a.eq_ignore_ascii_case("TERM"))
    {
        if deger.is_empty() || deger.eq_ignore_ascii_case("dumb") {
            *deger = "xterm-256color".to_string();
        }
    } else {
        ortam.push(("TERM".to_string(), "xterm-256color".to_string()));
    }
}

#[cfg(unix)]
type KokAyrimi = (
    Option<Yazici>,
    Option<Okuyucu>,
    Option<Okuyucu>,
    Bitisi,
    Option<KokBitisi>,
    Agac,
);

#[cfg(unix)]
fn kokten_ayir(kok: agac::EtkilesimliKok) -> std::io::Result<KokAyrimi> {
    let mut cocuk = kok.cocuk;
    let girdi = cocuk.stdin.take().map(|g| Box::new(g) as Yazici);
    #[cfg(target_os = "linux")]
    if let Some(durum_okuma) = kok.durum_okuma {
        let cikti = cocuk.stdout.take().map(|o| Box::new(o) as Okuyucu);
        let hata = cocuk.stderr.take().map(|o| Box::new(o) as Okuyucu);
        let (bitisi, kok_bitisi) = durum_borusundan_bitis(durum_okuma, cocuk)?;
        return Ok((girdi, cikti, hata, bitisi, kok_bitisi, kok.agac));
    }
    let (cikti, hata, bitisi) = Cocuk::Tokio(Box::new(cocuk)).ayir();
    Ok((girdi, cikti, hata, bitisi, None, kok.agac))
}

impl Surec {
    /// İsteği yalıtımsız etkileşimli kabuk süreci (borulu ya da PTY/ConPTY)
    /// olarak başlatır.
    pub(crate) fn baslat_kabuk_etkilesimli(
        istek: &CalistirmaIstegi,
        terminal: Option<crate::TerminalBoyutu>,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        let mut ortam = istek.ortam.clone();
        if let Some(boyut) = terminal {
            boyut.dogrula()?;
            terminal_ortami_ekle(&mut ortam);
            let (yol, argumanlar) = program::coz(&istek.komut, &ortam)?;
            #[cfg(unix)]
            {
                let mut komut = Command::new(yol);
                komut
                    .args(argumanlar)
                    .env_clear()
                    .envs(ortam.iter().map(|(a, d)| (a, d)))
                    .current_dir(&istek.dizin);
                return Self::komutla_pty(komut, boyut, true);
            }
            #[cfg(windows)]
            {
                let cmd = crate::windows::kap::sistem_dizini().join("cmd.exe");
                let komut = crate::windows::komut_satiri::uret(&yol, &argumanlar, &cmd)?;
                let is = agac::Is::yeni(false)?;
                let (kap, terminal_denetimi) = crate::windows::kap::baslat_yalitimsiz_terminal(
                    &komut,
                    &istek.dizin,
                    &ortam,
                    &is,
                    boyut,
                )?;
                let mut surec =
                    EtkilesimliSurec::kaptan_terminalli(kap, is, Some(terminal_denetimi))?;
                // İlk yoklama bekleme görevini başlatır.
                let _ = surec.bitisi_yokla();
                return Ok(surec);
            }
            #[cfg(not(any(unix, windows)))]
            {
                let _ = (yol, argumanlar);
                return Err(BaslatmaHatasi::Yalitim(
                    "bu platformda sözde terminal desteklenmiyor".into(),
                ));
            }
        }
        let (yol, argumanlar) = program::coz(&istek.komut, &ortam)?;
        let mut komut = Command::new(yol);
        komut
            .args(argumanlar)
            .env_clear()
            .envs(ortam.iter().map(|(a, d)| (a, d)))
            .current_dir(&istek.dizin);
        Self::komutla_etkilesimli(komut)
    }

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
        let kok = agac::baslat_etkilesimli(&mut komut)?;
        #[cfg(unix)]
        let (girdi, cikti, hata, bitisi, kok_bitisi, agac) = kokten_ayir(kok)?;
        #[cfg(not(unix))]
        let (girdi, cikti, hata, bitisi, kok_bitisi, agac) = {
            let mut cocuk = kok.cocuk;
            let girdi = cocuk.stdin.take().map(|g| Box::new(g) as Yazici);
            let (cikti, hata, bitisi) = Cocuk::Tokio(Box::new(cocuk)).ayir();
            (girdi, cikti, hata, bitisi, None, kok.agac)
        };
        let mut surec =
            EtkilesimliSurec::yeni_ic(girdi, cikti, hata, bitisi, kok_bitisi, agac, None)?;
        let _ = surec.bitisi_yokla();
        Ok(surec)
    }

    /// Kurulmuş bir komutu Unix PTY ile başlatır. `dogrudan_oturum` `true` ise
    /// `setsid()` + `ioctl(TIOCSCTTY)` `pre_exec`'te çalışır; `false` ise
    /// (örneğin `bwrap --new-session`) oturum ve `TIOCSCTTY` kap içindeki
    /// `/usr/bin/setsid -c` sarmalayıcısına bırakılır.
    #[cfg(unix)]
    pub(crate) fn komutla_pty(
        mut komut: Command,
        boyut: crate::TerminalBoyutu,
        dogrudan_oturum: bool,
    ) -> Result<EtkilesimliSurec, BaslatmaHatasi> {
        let (master, slave) = pty_ac(boyut)?;
        let slave_out = slave.try_clone()?;
        let slave_err = slave.try_clone()?;
        let master_yazma = master.try_clone()?;
        let master_okuma = master.try_clone()?;
        let terminal = TerminalDenetimi::unix_yeni(master);
        komut
            .stdin(Stdio::from(slave))
            .stdout(Stdio::from(slave_out))
            .stderr(Stdio::from(slave_err))
            .kill_on_drop(true);
        let kok = if dogrudan_oturum {
            agac::baslat_etkilesimli_pty(&mut komut)?
        } else {
            agac::baslat_etkilesimli(&mut komut)?
        };
        let (_, _, _, bitisi, kok_bitisi, agac) = kokten_ayir(kok)?;
        let girdi =
            Box::new(tokio::fs::File::from_std(std::fs::File::from(master_yazma))) as Yazici;
        let cikti =
            Box::new(tokio::fs::File::from_std(std::fs::File::from(master_okuma))) as Okuyucu;
        let mut surec = EtkilesimliSurec::yeni_ic(
            Some(girdi),
            Some(cikti),
            None,
            bitisi,
            kok_bitisi,
            agac,
            Some(terminal),
        )?;
        let _ = surec.bitisi_yokla();
        Ok(surec)
    }
}

#[cfg(windows)]
impl EtkilesimliSurec {
    /// Kapta başlamış ve işe atanmış etkileşimli süreç.
    pub(crate) fn kaptan(
        kap: crate::windows::kap::Kap,
        is: agac::Is,
    ) -> Result<Self, BaslatmaHatasi> {
        Self::kaptan_terminalli(kap, is, None)
    }

    /// Kapta başlamış ve (varsa) ConPTY denetimine bağlı etkileşimli süreç.
    pub(crate) fn kaptan_terminalli(
        mut kap: crate::windows::kap::Kap,
        is: agac::Is,
        terminal: Option<TerminalDenetimi>,
    ) -> Result<Self, BaslatmaHatasi> {
        let girdi = kap
            .girdi
            .take()
            .map(|g| Box::new(tokio::fs::File::from_std(g)) as Yazici);
        let (cikti, hata, bitisi) = Cocuk::Kap(kap).ayir();
        let mut surec = Self::yeni_ic(girdi, cikti, hata, bitisi, None, Agac::isten(is), terminal)?;
        let _ = surec.bitisi_yokla();
        Ok(surec)
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

async fn oku(mut kaynak: impl AsyncRead + Unpin, akis: Akis, tx: mpsc::Sender<(Akis, Vec<u8>)>) {
    let mut tampon = vec![0u8; OKUMA_PARCASI];
    loop {
        match kaynak.read(&mut tampon).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                if tx.send((akis, tampon[..n].to_vec())).await.is_err() {
                    return;
                }
            }
        }
    }
}

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

    /// MCP iptal yazımı payı tüketmişse EOF sonrası gecikmeli doğal çıkış
    /// için yeni pay açılmaz. Öldürmenin OS süresine eşik konmaz.
    #[tokio::test]
    async fn etkilesimli_kapat_kadar_tukenmis_kapanma_payini_yenilemez() {
        let d = tempfile::tempdir().unwrap();
        let (program, argumanlar): (&str, Vec<&str>) = if cfg!(windows) {
            (
                "powershell",
                vec![
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Console]::Out.WriteLine($PID);[Console]::Out.Flush();[Console]::In.ReadToEnd() > $null;Start-Sleep -Milliseconds 600",
                ],
            )
        } else {
            ("sh", vec!["-c", "echo $$; cat >/dev/null; sleep 0.6"])
        };
        let mut surec = baslat(d.path(), program, &argumanlar);
        let pid: u32 = ilk_satir(&mut surec).await.parse().unwrap();
        let _temizlik = Temizlik(vec![pid]);
        let sonuc = surec
            .kapat_kadar(tokio::time::Instant::now() - Duration::from_millis(1))
            .await
            .unwrap();
        assert!(!sonuc.kendiliginden);
        assert!(!yasiyor(pid), "süre dolsa da süreç ağacı doğrulanmalı");
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

    #[tokio::test]
    async fn kabuk_etkilesimli_terminal_boyut_degistirme_ve_kapanis() {
        let d = tempfile::tempdir().unwrap();
        let istek = crate::CalistirmaIstegi {
            komut: crate::KomutBicimi::Betik {
                metin: if cfg!(windows) {
                    "$w = [Console]::WindowWidth; $h = [Console]::WindowHeight; [Console]::Out.WriteLine(\"BOYUT:${w}x${h}\"); [Console]::Out.Flush(); $satir = [Console]::In.ReadLine(); $w2 = [Console]::WindowWidth; $h2 = [Console]::WindowHeight; [Console]::Out.WriteLine(\"YENI:${satir}:${w2}x${h2}\"); [Console]::Out.Flush()".into()
                } else {
                    "stty size | tr ' ' 'x' | sed 's/^/BOYUT:/'; read satir; stty size | tr ' ' 'x' | sed \"s/^/YENI:${satir}:/\"".into()
                },
            },
            dizin: d.path().to_path_buf(),
            ortam: crate::ortam::uret(std::env::vars()),
            ag: false,
            ag_listesi: None,
            alan: None,
        };
        let boyut = crate::TerminalBoyutu::yeni(90, 30).unwrap();
        let mut surec = super::Surec::baslat_kabuk_etkilesimli(&istek, Some(boyut)).unwrap();
        assert!(surec.terminal_denetimi().is_some());
        let mut girdi = surec.girdi_al().unwrap();
        let mut cikti = surec.cikti_al().unwrap();
        let mut biriken = Vec::new();
        let mut tampon = [0u8; 1024];
        let son = Instant::now() + Duration::from_secs(5);
        while !String::from_utf8_lossy(&biriken).contains("BOYUT:") && Instant::now() < son {
            if let Ok(Ok(n)) =
                tokio::time::timeout(Duration::from_millis(200), cikti.read(&mut tampon)).await
            {
                if n == 0 {
                    break;
                }
                biriken.extend_from_slice(&tampon[..n]);
            }
        }
        let ilk = String::from_utf8_lossy(&biriken).into_owned();
        if cfg!(windows) {
            assert!(ilk.contains("BOYUT:90x30"), "ilk çıktı: {ilk:?}");
        } else {
            assert!(ilk.contains("BOYUT:30x90"), "ilk çıktı: {ilk:?}");
        }

        let yeni_boyut = crate::TerminalBoyutu::yeni(110, 36).unwrap();
        surec.boyut_degistir(yeni_boyut).unwrap();
        assert!(
            surec
                .boyut_degistir(crate::TerminalBoyutu {
                    sutun: 0,
                    satir: 24
                })
                .is_err()
        );
        girdi.write_all(b"devam\r\n").await.unwrap();
        girdi.flush().await.unwrap();

        while !String::from_utf8_lossy(&biriken).contains("YENI:devam:") && Instant::now() < son {
            if let Ok(Ok(n)) =
                tokio::time::timeout(Duration::from_millis(200), cikti.read(&mut tampon)).await
            {
                if n == 0 {
                    break;
                }
                biriken.extend_from_slice(&tampon[..n]);
            }
        }
        let son_metin = String::from_utf8_lossy(&biriken).into_owned();
        if cfg!(windows) {
            assert!(
                son_metin.contains("YENI:devam:110x36"),
                "son çıktı: {son_metin:?}"
            );
        } else {
            assert!(
                son_metin.contains("YENI:devam:36x110"),
                "son çıktı: {son_metin:?}"
            );
        }

        let denetim = surec.terminal_denetimi().unwrap();
        assert_eq!(surec.bitisi_bekle().await, Some(0));
        let kapanis = surec.kapat().await.unwrap();
        assert!(kapanis.kendiliginden);
        assert_eq!(kapanis.cikis, Some(0));
        // Kapanıştan sonra boyut değiştirme reddedilir.
        assert!(denetim.boyut_degistir(boyut).is_err());
    }

    /// ST-01: Kök kabuk doğal olarak çıktığında arkada bıraktığı `setsid`
    /// torunu `kapat` `Ok` döndüğünde yaşamamalıdır.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn kendiliginden_cikan_kokun_setsid_torunu_kapanista_yasamaz() {
        use crate::Yalitici;
        let d = tempfile::tempdir().unwrap();
        let istek = crate::CalistirmaIstegi {
            komut: crate::KomutBicimi::Betik {
                metin: "setsid sleep 30 </dev/null >/dev/null 2>&1 & echo $!; sleep 0.2".into(),
            },
            dizin: d.path().to_path_buf(),
            ortam: crate::ortam::uret(std::env::vars()),
            ag: false,
            ag_listesi: None,
            alan: None,
        };

        let mut surec = crate::Yalitimsiz
            .baslat_kabuk_etkilesimli(&istek, None)
            .unwrap();
        let torun_pid: u32 = ilk_satir(&mut surec).await.parse().unwrap();
        let _temizlik = Temizlik(vec![torun_pid]);

        assert_eq!(surec.bitisi_bekle().await, Some(0));
        let kapanis = surec.kapat().await.unwrap();
        assert!(kapanis.kendiliginden);

        let durum = std::fs::read_to_string(format!("/proc/{torun_pid}/stat")).unwrap_or_default();
        assert!(
            durum.is_empty() || durum.contains(") Z ") || durum.contains(") X "),
            "kapat Ok döndüğü halde setsid torunu yaşıyor: pid={torun_pid}, stat={durum:?}"
        );
    }

    /// ST-02: PTY/ConPTY girdi yazıcısı bırakılınca (`eof=true` / `drop`)
    /// standart girdiden EOF bekleyen program sınırlı sürede çıkmalıdır.
    #[tokio::test]
    async fn pty_girdi_kapatilinca_eof_bekleyen_program_biter() {
        use crate::Yalitici;
        let d = tempfile::tempdir().unwrap();
        let istek = crate::CalistirmaIstegi {
            komut: crate::KomutBicimi::Betik {
                metin: if cfg!(windows) {
                    "[Console]::In.ReadToEnd() > $null".into()
                } else {
                    "cat >/dev/null".into()
                },
            },
            dizin: d.path().to_path_buf(),
            ortam: crate::ortam::uret(std::env::vars()),
            ag: false,
            ag_listesi: None,
            alan: None,
        };

        let mut surec = crate::Yalitimsiz
            .baslat_kabuk_etkilesimli(&istek, Some(crate::TerminalBoyutu::default()))
            .unwrap();
        drop(surec.girdi_al());

        let cikis = tokio::time::timeout(Duration::from_secs(2), surec.bitisi_bekle()).await;
        let _ = surec.kapat().await;
        assert_eq!(
            cikis.expect("PTY girdi kapatılınca program EOF alıp çıkmalı"),
            Some(0)
        );
    }
}
