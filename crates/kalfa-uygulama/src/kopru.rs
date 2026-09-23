//! Köprü: paketleyicinin mesajlarını kanala yazar (1c tasarımı §4). Her
//! oturum için ayrı bir görevde çalışır; `sonraki` paket doldururken
//! bırakılmaya dayanıklı olmadığı için `select!` içinde çağrılmaz.

use kalfa_core::aktarim::{KanalMesaji, OlayKaynagi, Paketleyici};

use crate::arayuz::KanalYazici;

pub(crate) async fn calistir<K: OlayKaynagi>(
    mut paketleyici: Paketleyici<K>,
    yazici: Box<dyn KanalYazici>,
) {
    while let Some(mesaj) = paketleyici.sonraki().await {
        let son = matches!(mesaj, KanalMesaji::Kapandi { .. });
        if yazici.gonder(mesaj).is_err() || son {
            break;
        }
    }
}
