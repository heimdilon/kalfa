//! Başlamış bir komut: çıktının okunması, süre, iptal ve süreç ağacının
//! sonlandırılması (aşama 2 tasarımı §6.2, §6.4).

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{Instant, sleep_until};
use tokio_util::sync::CancellationToken;

use crate::agac::{self, Agac};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalismaSonucu {
    pub bitis: Bitis,
    pub sure: Duration,
}

pub struct Surec {
    cocuk: Child,
    agac: Agac,
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
        Ok(Self { cocuk, agac })
    }

    /// Komut bitene, süre dolana ya da iptal gelene kadar bekler; çıktı
    /// parçalarını geldikçe `cikti`'ya verir. Döndüğünde süreç ağacı
    /// sonlandırılmıştır.
    pub async fn bekle(
        mut self,
        sure: Duration,
        iptal: CancellationToken,
        mut cikti: impl FnMut(Akis, &[u8]),
    ) -> CalismaSonucu {
        let baslangic = Instant::now();
        let (tx, mut rx) = mpsc::channel::<(Akis, Vec<u8>)>(16);
        let mut okuyucular = Vec::new();
        if let Some(o) = self.cocuk.stdout.take() {
            okuyucular.push(tokio::spawn(oku(o, Akis::Standart, tx.clone())));
        }
        if let Some(o) = self.cocuk.stderr.take() {
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
                durum = self.cocuk.wait(), if !cikis_alindi => {
                    cikis_alindi = true;
                    if bitis.is_none() {
                        bitis = Some(Bitis::Cikti(durum.ok().and_then(|d| d.code())));
                    }
                    // Komut bitti; geride kalan torunlar da durur.
                    self.agac.durdur();
                    oldurme.get_or_insert(Instant::now() + KIBAR_SURE);
                    bosaltma = Some(Instant::now() + BOSALTMA_SURESI);
                }
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
                _ = sleep_until(bosaltma.unwrap_or(uzak)), if bosaltma.is_some() => break,
            }
        }
        self.agac.oldur();
        for okuyucu in okuyucular {
            okuyucu.abort();
        }
        CalismaSonucu {
            bitis: bitis.unwrap_or(Bitis::Cikti(None)),
            sure: baslangic.elapsed(),
        }
    }
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
