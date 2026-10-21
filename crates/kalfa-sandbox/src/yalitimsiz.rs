//! Yalıtımsız arka uç (aşama 2 tasarımı §6.5): süreç sınırlarını uygular,
//! başka kısıt koymaz. Aşama 2a'da iki platformda da tek arka uç budur.

use crate::{
    BaslatmaHatasi, CalistirmaIstegi, Surec, Yalitici, YalitimDurumu, YalitimYoklukNedeni,
};

/// Yalıtım arka ucu olmayan platformda durum bunu ve kullanıcının ne
/// beklemesi gerektiğini söyler (aşama 2 tasarımı §9.8).
pub const TALIMAT: &str = "Bu platformda Kalfa'nın yalıtımı yok; Kalfa komut çalıştırmaz. \
Ayarlar'da yalıtımsız çalıştırmayı açarsanız her komut, çalışmadan önce yerel pencerede \
onayınıza sunulur.";

#[derive(Debug, Default)]
pub struct Yalitimsiz;

impl Yalitici for Yalitimsiz {
    fn durum(&self) -> YalitimDurumu {
        YalitimDurumu::Yok {
            neden: YalitimYoklukNedeni::Desteklenmiyor,
            talimat: TALIMAT.to_string(),
            ayrinti: "denetim=platform; yalıtım arka ucu yok".to_string(),
        }
    }

    fn baslat(&self, istek: &CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi> {
        if istek.alan.is_some() {
            return Err(BaslatmaHatasi::Yalitim(
                "komut yalıtımlı çalışmak üzere onaylandı ama yalıtım kullanılamıyor".into(),
            ));
        }
        Surec::baslat(istek)
    }

    fn baslat_kabuk_etkilesimli(
        &self,
        istek: &CalistirmaIstegi,
        terminal: Option<crate::TerminalBoyutu>,
    ) -> Result<crate::EtkilesimliSurec, BaslatmaHatasi> {
        if istek.alan.is_some() {
            return Err(BaslatmaHatasi::Yalitim(
                "komut yalıtımlı çalışmak üzere onaylandı ama yalıtım kullanılamıyor".into(),
            ));
        }
        Surec::baslat_kabuk_etkilesimli(istek, terminal)
    }
}
