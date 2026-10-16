use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::{
    BOSTA_KAPANMA_SURESI, CalisanKusak, MUTLAK_KORUMA_YASI, McpAnahtari, McpSunucuDurumu,
    McpYoneticisi, YoneticiHatasi,
};

fn sureyi_ms(sure: Duration) -> u64 {
    u64::try_from(sure.as_millis()).unwrap_or(u64::MAX)
}

fn ayni_kusak_aktif_mi(
    yonetici: &McpYoneticisi,
    anahtar: &McpAnahtari,
    kusak: &Arc<CalisanKusak>,
) -> bool {
    let d = yonetici.durum();
    d.yuvalar
        .get(anahtar)
        .and_then(|yuva| yuva.calisan.as_ref())
        .is_some_and(|aktif| Arc::ptr_eq(aktif, kusak))
}

fn bosta_kapanma_vakti_geldi_mi(kusak: &CalisanKusak, simdi_ms: u64, bosta_ms: u64) -> bool {
    kusak.etkin_cagri_sayisi.load(Ordering::SeqCst) == 0
        && !kusak.kesif_suruyor.load(Ordering::SeqCst)
        && !kusak.onay_bekliyor.load(Ordering::SeqCst)
        && simdi_ms
            >= kusak
                .son_cagri_ani_ms
                .load(Ordering::SeqCst)
                .saturating_add(bosta_ms)
}

impl McpYoneticisi {
    pub(super) fn yeniden_baslatmayi_zamanla(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: kalfa_core::arac::McpBaslatmaSayaci,
        revizyon: u64,
        an_ms: u64,
        kok: PathBuf,
    ) {
        let yonetici = Arc::downgrade(self);
        let bekleme = Duration::from_millis(an_ms.saturating_sub(self.saat.simdi_ms()));
        tokio::spawn(async move {
            tokio::time::sleep(bekleme).await;
            let Some(yonetici) = yonetici.upgrade() else {
                return;
            };
            let gecerli = {
                let d = yonetici.durum();
                d.yuvalar.get(&anahtar).is_some_and(|y| {
                    y.son_kusak == kusak
                        && y.baslatma_revizyonu == revizyon
                        && y.sonraki_cokme_izni_ani_ms == Some(an_ms)
                })
            };
            if gecerli {
                let _ = yonetici
                    .baslat_veya_dogrula(&anahtar.alan_kimligi, &kok, &anahtar.sunucu)
                    .await;
            }
        });
    }

    /// Her çalışan kuşak için bağımsız mutlak yaş ve boşta kapanma zamanlayıcısı başlatır.
    ///
    /// Zamanlayıcı uyurken yöneticinin güçlü başvurusunu tutmaz. Her tetiklemede
    /// kuşağın hâlâ yöneticiye ait olduğunu doğrular; bir sunucunun taraması ya da
    /// kapanışı başka bir sunucunun zamanlayıcısını bekletmez.
    pub(super) fn kusak_zamanlayicisi_baslat(
        self: &Arc<Self>,
        anahtar: McpAnahtari,
        kusak: Arc<CalisanKusak>,
        kok: PathBuf,
    ) {
        let yonetici = Arc::downgrade(self);
        let saat = Arc::clone(&self.saat);
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };

        handle.spawn(async move {
            let t_k_yasi_ms = sureyi_ms(MUTLAK_KORUMA_YASI);
            let bosta_sure_ms = sureyi_ms(BOSTA_KAPANMA_SURESI);

            loop {
                // Önce kaydol, sonra atomik durumları oku. Böylece bildirim
                // okuma ile uyku arasına düşüp kaybolamaz.
                let bildirim = kusak.zamanlayici_bildirimi.notified();
                tokio::pin!(bildirim);
                bildirim.as_mut().enable();

                if kusak.iptal.is_cancelled() {
                    break;
                }

                let simdi_ms = saat.simdi_ms();
                let t_k_deadline_ms = kusak
                    .t_k_ani_ms
                    .load(Ordering::SeqCst)
                    .saturating_add(t_k_yasi_ms);
                let idle_deadline_ms = (kusak.etkin_cagri_sayisi.load(Ordering::SeqCst) == 0
                    && !kusak.kesif_suruyor.load(Ordering::SeqCst)
                    && !kusak.onay_bekliyor.load(Ordering::SeqCst))
                .then(|| {
                    kusak
                        .son_cagri_ani_ms
                        .load(Ordering::SeqCst)
                        .saturating_add(bosta_sure_ms)
                });

                let idle_due = idle_deadline_ms.is_some_and(|deadline| simdi_ms >= deadline);
                if idle_due {
                    let Some(yonetici) = yonetici.upgrade() else {
                        break;
                    };

                    let kapanacak = {
                        let mut d = yonetici.durum();
                        let ayni_kusak = d
                            .yuvalar
                            .get(&anahtar)
                            .and_then(|yuva| yuva.calisan.as_ref())
                            .is_some_and(|aktif| Arc::ptr_eq(aktif, &kusak));
                        if ayni_kusak
                            && bosta_kapanma_vakti_geldi_mi(&kusak, saat.simdi_ms(), bosta_sure_ms)
                        {
                            Self::yuvadan_sureci_ayir(
                                &mut d,
                                &anahtar,
                                YoneticiHatasi::SunucuDurduruldu,
                                McpSunucuDurumu::BostaKapandi,
                            )
                        } else {
                            None
                        }
                    };

                    if let Some((anahtar, kapanan, kok_yolu)) = kapanacak {
                        let kok_yolu = kok_yolu.or_else(|| Some(kok.clone()));
                        let _ = yonetici.kapat_ve_denetle(anahtar, kapanan, kok_yolu).await;
                    }
                    continue;
                }

                if simdi_ms >= t_k_deadline_ms {
                    let Some(yonetici) = yonetici.upgrade() else {
                        break;
                    };

                    let denetle = ayni_kusak_aktif_mi(&yonetici, &anahtar, &kusak)
                        && saat.simdi_ms()
                            >= kusak
                                .t_k_ani_ms
                                .load(Ordering::SeqCst)
                                .saturating_add(t_k_yasi_ms);
                    if denetle {
                        let sonuc = yonetici
                            .sunucu_korumasini_denetle(&anahtar.alan_kimligi, &anahtar.sunucu, &kok)
                            .await;
                        if matches!(sonuc, Err(YoneticiHatasi::BaslatmaYarisi)) {
                            tokio::select! {
                                _ = kusak.iptal.cancelled() => break,
                                _ = bildirim.as_mut() => {},
                            }
                        }
                    }
                    continue;
                }

                let sonraki_deadline_ms =
                    idle_deadline_ms.map_or(t_k_deadline_ms, |idle| idle.min(t_k_deadline_ms));
                let uyku = Duration::from_millis(sonraki_deadline_ms.saturating_sub(simdi_ms));

                tokio::select! {
                    biased;
                    _ = kusak.iptal.cancelled() => break,
                    _ = bildirim.as_mut() => {},
                    _ = tokio::time::sleep(uyku) => {},
                }
            }
        });
    }
}
