//! Yalıtımsız arka uç (aşama 2 tasarımı §6.5): süreç sınırlarını uygular,
//! başka kısıt koymaz. Aşama 2a'da iki platformda da tek arka uç budur.

use crate::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
};

/// 2a'da yalıtım yoktur; durum bunu ve kullanıcının ne beklemesi gerektiğini
/// söyler.
pub const TALIMAT: &str = "Bu sürümde komutlar yalıtımsız çalışıyor; yalıtım sonraki sürümde \
gelecek. Her komut, çalışmadan önce yerel pencerede onayınıza sunulur.";

#[derive(Debug, Default)]
pub struct Yalitimsiz;

impl Yalitici for Yalitimsiz {
    fn durum(&self) -> YalitimDurumu {
        YalitimDurumu::Yok {
            neden: YalitimYoklukNedeni::Desteklenmiyor,
            talimat: TALIMAT.to_string(),
        }
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        Surec::baslat(istek)
    }
}
