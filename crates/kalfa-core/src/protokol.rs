//! `Komut` / `Olay` sınırı (spec §4.1). Tauri IPC'ye, WebSocket'e, stdio'ya ve
//! birim testine eşit uzaklıktadır.

use kalfa_evren::KrediGozlemi;
use serde::{Deserialize, Serialize};

use crate::izin::Kip;
use crate::kimlik::{CagriId, IslemId, OturumId, TurId};
use crate::muhur::OnayMuhru;

pub const SEMA_SURUMU: u32 = 1;

/// Her komut ve olay bu zarfla taşınır. `sira` oturum başına tekdüze artar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zarf<T> {
    pub sema: u32,
    pub oturum: OturumId,
    pub tur: Option<TurId>,
    pub sira: u64,
    pub yuk: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum Komut {
    KullaniciMesaji {
        metin: String,
    },
    Iptal {
        tur: TurId,
    },
    AracOnayi {
        cagri: CagriId,
        karar: OnayKarari,
        muhur: OnayMuhru,
    },
    ModelDegistir {
        model: String,
    },
    IzinKipiDegistir {
        kip: Kip,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum OnayKarari {
    Onayla,
    Reddet { gerekce: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AracDurumu {
    Basarili,
    Basarisiz,
    Reddedildi,
    IptalEdildi,
    SonucuBilinmiyor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnayOgesi {
    pub cagri: CagriId,
    pub arac: String,
    pub aciklama: String,
    pub hedefler: Vec<String>,
    pub diff: Option<String>,
    pub uyarilar: Vec<String>,
    pub muhur: OnayMuhru,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum BitisNedeni {
    Normal,
    Iptal,
    Sinir { aciklama: String },
    Hata,
}

/// Olay aktarımının neden durduğu (1c tasarımı §4.4). Arayüz gösterilecek
/// metni buna göre seçer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AktarimDurmaNedeni {
    /// Tüketici kuyruğu boşaltamadı: olay ya da bayt bütçesi doldu.
    TuketiciYetismedi,
    /// Tek bir olay bütçeden büyüktü.
    OlayCokBuyuk,
    /// Olay serileştirilemedi ya da alıcı kapandı.
    Diger,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tip")]
pub enum Olay {
    TurBasladi,
    TokenParcasi {
        metin: String,
    },
    DusunceParcasi {
        metin: String,
    },
    AracCagrisiBasladi {
        cagri: CagriId,
        arac: String,
        aciklama: String,
    },
    AracSonucu {
        cagri: CagriId,
        arac: String,
        durum: AracDurumu,
        icerik: String,
    },
    OnayGerekiyor {
        islem: IslemId,
        ogeler: Vec<OnayOgesi>,
    },
    /// Bir EVREN isteğinin kimliği ve yönlendirilen model (üst belge §6.4).
    IstekBilgisi {
        istek_id: Option<String>,
        yonlendirilen_model: Option<String>,
    },
    KrediGozlemi {
        gozlem: KrediGozlemi,
    },
    TokenKullanimi {
        istem: u64,
        tamamlama: u64,
        dusunce: u64,
    },
    SartOnayiGerekiyor {
        surum: u32,
        metin: String,
    },
    Hata {
        mesaj: String,
    },
    Tamamlandi {
        neden: BitisNedeni,
    },
    /// Olay kuyruğu durdu ve oturum kapanıyor; kuyruğun son olayıdır.
    AktarimDurdu {
        neden: AktarimDurmaNedeni,
    },
}
