//! Kayıttan oturum durumunu kurma (spec §6.6).
//!
//! Sonucu bilinmeyen yan etkili işlem **asla** yeniden çalıştırılmaz: yalnızca
//! "sonucu bilinmiyor" olarak modele ve kullanıcıya bildirilir.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use kalfa_evren::tipler::{ChatMessage, Role, ToolDef};

use crate::denetci::DenetimSonucu;
use crate::geri_alma::GeriAlmaDizini;
use crate::gonderim::{EklentiKullanimi, IstekAsamasi, IstekSonucu};
use crate::izin::Kip;
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, OturumId};
use crate::protokol::Zarf;

pub const BILINMEYEN_SONUC: &str = "[sonucu bilinmiyor: Kalfa bu araç çalışırken kapandı; işlem yapılmış da olabilir, yapılmamış da]";
pub const CALISTIRILMADI: &str = "[çalıştırılmadı: Kalfa bu çağrıdan önce kapandı]";

pub const ONIZLEME_KAPANISI: &str =
    "[sonuç gönderilmedi: Kalfa gönderim önizlemesini beklerken kapandı]";

pub const ONIZLEME_IPTALI: &str =
    "[sonuç gönderilmedi: tur, gönderim önizlemesi beklenirken durduruldu]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BilinmeyenCagri {
    pub cagri: CagriId,
    pub arac: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KurtarilmisDurum {
    pub oturum: OturumId,
    pub model: String,
    pub kip: Kip,
    pub calisma_alani: Option<PathBuf>,
    /// Kayıtta bulunan bağlam mesajları.
    pub mesajlar: Vec<ChatMessage>,
    /// Yanıtı olmayan araç çağrıları için üretilen, henüz kayda yazılmamış
    /// `tool` mesajları; `mesajlar`ın sonuna eklenir.
    pub tamamlayicilar: Vec<ChatMessage>,
    pub bilinmeyenler: Vec<BilinmeyenCagri>,
    pub son_sira: u64,
    /// Bu oturumda daha önce bir istekle gönderilmiş mesaj sayısı (aşama 3b).
    pub gonderilen: usize,
    /// Mesajların kayıt sırası ve kaynak bilgisi.
    pub mesaj_bilgileri: Vec<(Option<u64>, Option<String>)>,
    /// Kayıttaki son sistem istemi (sıra, metin).
    pub son_sistem_istemi: Option<(Option<u64>, String)>,
    /// Kayıttaki son araç tanımları (sıra, tanımlar).
    pub son_arac_tanimlari: Option<(Option<u64>, Vec<ToolDef>)>,
    /// Göndermeden önce göster ayarı (aşama 3b).
    pub onizleme_acik: bool,
    /// Kayıttan okunan güvenlik denetimi sonuçları (aşama 3c).
    pub denetimler: HashMap<CagriId, DenetimSonucu>,
    /// Turların geri alma bilgisi (aşama 3 tasarımı §5.7). Depoda bunun
    /// kullanmadığı dosyalar kayda ulaşmamış yakalamalardır ve silinir (§5.2).
    pub geri_alma: GeriAlmaDizini,
    /// Eklentilerin oturumdaki EVREN kullanımı (aşama 4 tasarımı §9.5),
    /// kayıttaki `EklentiIstegi` öğelerinden. Sınırın sayacı buradan sürer.
    pub eklenti_kullanimi: HashMap<String, EklentiKullanimi>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KurtarmaHatasi {
    #[error("Oturum kaydı boş.")]
    Bos,
    #[error("Oturum kaydı bir açılış kaydıyla başlamıyor.")]
    AcilisYok,
}

pub fn kurtar(kayitlar: &[Zarf<KayitOgesi>]) -> Result<KurtarilmisDurum, KurtarmaHatasi> {
    let ilk = kayitlar.first().ok_or(KurtarmaHatasi::Bos)?;
    let KayitOgesi::OturumAcildi {
        model,
        kip,
        calisma_alani,
    } = &ilk.yuk
    else {
        return Err(KurtarmaHatasi::AcilisYok);
    };
    let (mut model, mut kip) = (model.clone(), *kip);
    let mut mesajlar = Vec::new();
    let mut mesaj_bilgileri = Vec::new();
    let mut gonderilen = 0usize;
    let mut son_sistem_istemi: Option<(Option<u64>, String)> = None;
    let mut son_arac_tanimlari: Option<(Option<u64>, Vec<ToolDef>)> = None;
    let mut bekleyen_sistem_istemi: Option<(Option<u64>, String)> = None;
    let mut bekleyen_arac_tanimlari: Option<(Option<u64>, Vec<ToolDef>)> = None;
    let mut baslayan: HashMap<String, BilinmeyenCagri> = HashMap::new();
    let mut biten: HashSet<CagriId> = HashSet::new();
    let mut onizleme_acik = false;
    let mut denetimler = HashMap::new();
    let mut eklenti_kullanimi: HashMap<String, EklentiKullanimi> = HashMap::new();

    for zarf in kayitlar {
        match &zarf.yuk {
            KayitOgesi::Mesaj { mesaj, kaynak } => {
                mesajlar.push(mesaj.clone());
                mesaj_bilgileri.push((Some(zarf.sira), kaynak.clone()));
            }
            // Geri almanın notu bağlama kullanıcı rolünde girer (§5.9).
            KayitOgesi::KalfaNotu { metin } => {
                mesajlar.push(ChatMessage::user(metin.clone()));
                mesaj_bilgileri.push((Some(zarf.sira), Some("kalfa:not".to_string())));
            }
            KayitOgesi::SistemIstemi { metin } => {
                bekleyen_sistem_istemi = Some((Some(zarf.sira), metin.clone()));
            }
            KayitOgesi::AracTanimlari { tanimlar } => {
                bekleyen_arac_tanimlari = Some((Some(zarf.sira), tanimlar.clone()));
            }
            KayitOgesi::OnizlemeDegisti { acik } => {
                onizleme_acik = *acik;
            }
            KayitOgesi::Istek {
                onceki,
                yeni,
                sonuc,
                ..
            } => {
                let sayilacak = matches!(
                    sonuc,
                    IstekSonucu::Basarili
                        | IstekSonucu::Hata {
                            asama: IstekAsamasi::Yanit,
                            ..
                        }
                );
                if sayilacak {
                    if let Some(s) = bekleyen_sistem_istemi.take() {
                        son_sistem_istemi = Some(s);
                    }
                    if let Some(a) = bekleyen_arac_tanimlari.take() {
                        son_arac_tanimlari = Some(a);
                    }
                    if *onceki > 0 || !yeni.is_empty() {
                        let yeni_mesaj_sayisi = yeni
                            .iter()
                            .filter(|o| o.kaynak != "kalfa:sistem" && o.kaynak != "kalfa:araçlar")
                            .count();
                        gonderilen = *onceki as usize + yeni_mesaj_sayisi;
                    } else {
                        gonderilen = mesajlar.len();
                    }
                } else {
                    bekleyen_sistem_istemi = son_sistem_istemi.clone();
                    bekleyen_arac_tanimlari = son_arac_tanimlari.clone();
                }
            }
            KayitOgesi::KipDegisti { kip: k } => kip = *k,
            KayitOgesi::ModelDegisti { model: m } => model = m.clone(),
            KayitOgesi::AracBasladi {
                cagri,
                model_cagri_id,
                arac,
                ..
            } => {
                baslayan.insert(
                    model_cagri_id.clone(),
                    BilinmeyenCagri {
                        cagri: *cagri,
                        arac: arac.clone(),
                    },
                );
            }
            KayitOgesi::AracBitti { cagri, .. } => {
                biten.insert(*cagri);
            }
            KayitOgesi::Denetim { cagri, sonuc, .. } => {
                denetimler.insert(*cagri, sonuc.clone());
            }
            // Gönderilmeyen istek ağa çıkmadı; sınırı tüketmez (§9.5). Sohbet
            // mantığına girmez: `gonderilen` ve sistem istemi buna bakmaz.
            // ponytail: sonucu yazılmadan çökülen istek sayılmaz (çağrı başına en
            // çok bir); izin anında yazılan bir kayıt bu boşluğu kapatır.
            KayitOgesi::EklentiIstegi { istek, sonuc, .. }
                if !matches!(sonuc, IstekSonucu::Gonderilmedi { .. }) =>
            {
                eklenti_kullanimi
                    .entry(istek.eklenti.clone())
                    .or_default()
                    .say(istek.bayt);
            }
            _ => {}
        }
    }

    // Serbest kip yalnızca oturum boyunca geçerlidir; kurtarılan oturum Sıkı açılır.
    if kip == Kip::Serbest {
        kip = Kip::Siki;
    }

    // Her araç çağrısının bir yanıtı olmalı; yoksa bağlam geçersizdir.
    let yanitlanan: HashSet<&str> = mesajlar
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.as_deref())
        .collect();
    let mut tamamlayicilar = Vec::new();
    let mut bilinmeyenler = Vec::new();
    for mesaj in mesajlar.iter().filter(|m| m.role == Role::Assistant) {
        for tc in mesaj.tool_calls.iter().flatten() {
            if yanitlanan.contains(tc.id.as_str()) {
                continue;
            }
            match baslayan.get(&tc.id) {
                Some(b) if !biten.contains(&b.cagri) => {
                    bilinmeyenler.push(b.clone());
                    tamamlayicilar.push(ChatMessage::tool(tc.id.clone(), BILINMEYEN_SONUC));
                }
                Some(b) if biten.contains(&b.cagri) => {
                    tamamlayicilar.push(ChatMessage::tool(tc.id.clone(), ONIZLEME_KAPANISI));
                }
                _ => tamamlayicilar.push(ChatMessage::tool(tc.id.clone(), CALISTIRILMADI)),
            }
        }
    }

    Ok(KurtarilmisDurum {
        oturum: ilk.oturum,
        model,
        kip,
        calisma_alani: calisma_alani.clone(),
        mesajlar,
        tamamlayicilar,
        bilinmeyenler,
        son_sira: kayitlar.last().map_or(0, |z| z.sira),
        gonderilen,
        mesaj_bilgileri,
        son_sistem_istemi,
        son_arac_tanimlari,
        onizleme_acik,
        denetimler,
        geri_alma: GeriAlmaDizini::kayittan(kayitlar),
        eklenti_kullanimi,
    })
}
