//! Uzun oturum ölçümü için kayıt üreteci (1c tasarımı §11.4, kabul kapısı 4).
//! Turlar gerçek oturum döngüsünden sahte sağlayıcının varsayılan yanıtıyla
//! geçer; kayıt uygulamanın kendi yazdığı biçimdedir.

use std::path::PathBuf;

use kalfa_core::butce::{AZAMI_SINIR, ButceSinirlari};
use kalfa_core::kimlik::OturumId;
use kalfa_core::oturum::{self, OturumAyarlari};
use kalfa_core::protokol::{Komut, Olay};

use crate::hata::KomutHatasi;
use crate::{Uygulama, dizin, modeller};

impl Uygulama {
    /// `kok` çalışma alanında `tur` turluk bir oturum kaydı üretir ve oturumu
    /// kapatır. Şartlar kabul edilmiş olmalıdır.
    pub async fn e2e_kayit_uret(&self, kok: PathBuf, tur: u32) -> Result<OturumId, KomutHatasi> {
        let baglam = self.baglam(&kok).await?;
        std::fs::create_dir_all(&self.oturum_dizini)?;
        let oturum = OturumId::yeni();
        let mut ayarlar = OturumAyarlari::yeni(modeller::VARSAYILAN_MODEL, &kok);
        ayarlar.kayit_yolu = Some(dizin::kayit_yolu(&self.oturum_dizini, oturum));
        // Uzun kayıt varsayılan oturum bütçesini aşar; ölçüm kart beklememeli.
        ayarlar.butce = ButceSinirlari {
            istek: AZAMI_SINIR,
            token: AZAMI_SINIR,
        };
        let (tutamac, mut alici) = oturum::baslat_kimlikle(oturum, ayarlar, baglam)?;
        for i in 1..=tur {
            // Oturum listesindeki başlık ilk mesajdır; kayıtlar böyle ayrılır.
            let metin = format!("{i}. soru ({tur} turluk kayıt)");
            tutamac.gonder(Komut::KullaniciMesaji { metin }).await?;
            loop {
                match alici.recv().await.map(|z| z.yuk) {
                    Some(Olay::Tamamlandi { .. }) => break,
                    Some(_) => {}
                    None => return Err(KomutHatasi::ic("Kayıt üretilirken oturum kapandı.")),
                }
            }
        }
        drop(alici);
        tutamac.kapanmasini_bekle().await;
        Ok(oturum)
    }
}
