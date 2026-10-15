//! Native kayıt onayı ve onaydan sonraki kod özeti. Bu Rust API'si IPC ucu değildir.

use crate::arayuz::Iletisim;
use kalfa_core::arac::McpTanimOzeti;
use kalfa_mcp::{
    tanim::{
        CozulmusTanim, DosyaAdayi, McpTanim, SeciliYurutulebilirCozucu, TanimHatasi,
        YazilabilirKokler,
    },
    tarif::{self, DosyaIcerikOzeti},
};
use std::{fs::File, future::Future};

struct YerelOnayKodu;

/// Alanlar yalnız native onay kapısından oluşturulur. Sır taşıdığı için Debug/serde yoktur.
pub struct OnayliTanim {
    _onay: YerelOnayKodu,
    tanim: McpTanim,
    cozulmus: CozulmusTanim,
    ozet: McpTanimOzeti,
}

impl OnayliTanim {
    pub const fn tanim_ozeti(&self) -> McpTanimOzeti {
        self.ozet
    }
    pub fn tanim(&self) -> &McpTanim {
        &self.tanim
    }
    pub fn cozulmus(&self) -> &CozulmusTanim {
        &self.cozulmus
    }
}

/// Arka uç sahipliği: webview dosya yolu veya onay token'ı oluşturamaz.
pub struct KayitBaglami<'a> {
    pub sistem_ortami: &'a [(String, String)],
    pub ozel: &'a kalfa_sandbox::OzelDizinler,
    pub yazilabilir_kokler: &'a YazilabilirKokler,
    pub cozucu: &'a dyn SeciliYurutulebilirCozucu,
    pub alan_kimligi: &'a str,
}

/// Önce metadata/yol ve native onay; içerik ancak kabulden sonra sabit bloklarla okunur.
pub async fn kaydi_onayla_ve_ozetle(
    tanim: McpTanim,
    iletisim: &dyn Iletisim,
    baglam: KayitBaglami<'_>,
) -> Result<Option<OnayliTanim>, TanimHatasi> {
    let cozulmus = tanim.hazirla(
        baglam.sistem_ortami,
        baglam.ozel,
        baglam.yazilabilir_kokler,
        baglam.cozucu,
    )?;
    let mesaj = cozulmus.yerel_onay_metni();
    let onay = iletisim.onayla("MCP sunucusunu kaydet", &mesaj, "Kaydet", "Vazgeç");
    let alan = baglam.alan_kimligi.to_owned();
    yerel_onaydan_sonra(onay, move || {
        let mut ozetler = Vec::with_capacity(cozulmus.dosya_adaylari().len());
        for aday in cozulmus.dosya_adaylari() {
            ozetler.push(DosyaIcerikOzeti::yeni(
                aday.yol().to_owned(),
                dosyayi_ozetle(aday)?,
            ));
        }
        let ozet = tarif::tanim_ozeti(&tanim, &cozulmus, &ozetler, &alan)?;
        Ok(OnayliTanim {
            _onay: YerelOnayKodu,
            tanim,
            cozulmus,
            ozet,
        })
    })
    .await
}

async fn yerel_onaydan_sonra<Fut, Islem, Sonuc>(
    onay: Fut,
    islem: Islem,
) -> Result<Option<Sonuc>, TanimHatasi>
where
    Fut: Future<Output = bool>,
    Islem: FnOnce() -> Result<Sonuc, TanimHatasi> + Send + 'static,
    Sonuc: Send + 'static,
{
    if !onay.await {
        return Ok(None);
    }
    // Disk okuması ortak async runtime'ı ve koruma zamanlayıcısını durdurmaz.
    tokio::task::spawn_blocking(islem)
        .await
        .map_err(|_| TanimHatasi::DosyaOkunamadi)?
        .map(Some)
}

fn dosyayi_ozetle(aday: &DosyaAdayi) -> Result<[u8; 32], TanimHatasi> {
    let mut dosya = File::open(aday.yol()).map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    let once = dosya.metadata().map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    if !once.is_file()
        || !aday
            .kimlik_uyuyor(&dosya)
            .map_err(|_| TanimHatasi::CiktiYoluDegisti)?
    {
        return Err(TanimHatasi::CiktiYoluDegisti);
    }
    let onceki_zaman = once.modified().map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    let ozet = tarif::dosya_akisi_ozeti(&mut dosya).map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    let sonra = dosya.metadata().map_err(|_| TanimHatasi::DosyaOkunamadi)?;
    if once.len() != sonra.len()
        || onceki_zaman != sonra.modified().map_err(|_| TanimHatasi::DosyaOkunamadi)?
        || !aday
            .kimlik_uyuyor(&dosya)
            .map_err(|_| TanimHatasi::CiktiYoluDegisti)?
    {
        return Err(TanimHatasi::CiktiYoluDegisti);
    }
    Ok(ozet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        task::Poll,
    };

    fn sayan_okuma(
        yol: PathBuf,
        sayac: Arc<AtomicUsize>,
    ) -> impl FnOnce() -> Result<[u8; 32], TanimHatasi> + Send + 'static {
        move || {
            sayac.fetch_add(1, Ordering::SeqCst);
            tarif::dosya_akisi_ozeti(File::open(yol).map_err(|_| TanimHatasi::DosyaOkunamadi)?)
                .map_err(|_| TanimHatasi::DosyaOkunamadi)
        }
    }

    #[tokio::test]
    async fn yerel_onay_reddinde_gecerli_dosyanin_icerigi_hic_okunmaz() {
        let kok = tempfile::tempdir().unwrap();
        let yol = kok.path().join("arguman.json");
        std::fs::write(&yol, b"gizli icerik").unwrap();
        let sayac = Arc::new(AtomicUsize::new(0));
        let sonuc = yerel_onaydan_sonra(std::future::ready(false), sayan_okuma(yol, sayac.clone()))
            .await
            .unwrap();
        assert_eq!(sayac.load(Ordering::SeqCst), 0);
        assert!(sonuc.is_none());
    }

    #[tokio::test]
    async fn yerel_onay_bekleyisi_iptal_edilince_icerik_okunmaz() {
        let kok = tempfile::tempdir().unwrap();
        let yol = kok.path().join("arguman.json");
        std::fs::write(&yol, b"gizli icerik").unwrap();
        let sayac = Arc::new(AtomicUsize::new(0));
        let mut bekleyen = Box::pin(yerel_onaydan_sonra(
            std::future::pending(),
            sayan_okuma(yol, sayac.clone()),
        ));
        std::future::poll_fn(|cx| {
            assert!(bekleyen.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        drop(bekleyen);
        assert_eq!(sayac.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn onaydan_sonra_ozet_alinir_ve_ham_icerik_donmez() {
        let kok = tempfile::tempdir().unwrap();
        let yol = kok.path().join("arguman.json");
        std::fs::write(&yol, b"abcd").unwrap();
        let sayac = Arc::new(AtomicUsize::new(0));
        let sonuc = yerel_onaydan_sonra(std::future::ready(true), sayan_okuma(yol, sayac.clone()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(sonuc, tarif::dosya_icerik_ozeti(b"abcd"));
        assert_eq!(sayac.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn onay_beklerken_ayni_yola_baska_dosya_gelirse_ozet_reddedilir() {
        let kok = tempfile::tempdir().unwrap();
        let yol = kok.path().join("arguman.json");
        std::fs::write(&yol, b"aaaa").unwrap();
        let aday = DosyaAdayi::yakala(yol.canonicalize().unwrap()).unwrap();
        std::fs::rename(&yol, kok.path().join("eski.json")).unwrap();
        std::fs::write(&yol, b"bbbb").unwrap();
        assert!(matches!(
            dosyayi_ozetle(&aday),
            Err(TanimHatasi::CiktiYoluDegisti)
        ));
    }
}
