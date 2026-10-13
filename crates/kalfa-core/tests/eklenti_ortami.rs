//! Eklenti aracının EVREN isteği (aşama 4 tasarımı §8.4, §9.3–§9.5): ilk
//! gönderim onayı, göndermeden önce gösterme, oturum sınırı, günlük ve kurtarma.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, AracOrtami, EklentiTarifi, GonderimTalebi, GonderimYaniti,
    Hazirlik,
};
use kalfa_core::butce::{ButceEngeli, ButceGorunumu, ButceIslemId, ButceSinirlari};
use kalfa_core::gonderim::{
    EKLENTI_BAYT_SINIRI, EKLENTI_ISTEK_SINIRI, EklentiIstekOzeti, EklentiKullanimi, GunlukToken,
    IstekSonucu, gunluk_uret,
};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, IslemId, OturumId};
use kalfa_core::oturum::{OturumAyarlari, devam_et};
use kalfa_core::protokol::{
    AracDurumu, BitisNedeni, ButceKarari, Komut, Olay, OnayKarari, OnayOgesi, OnizlemeNedeni,
    OnizlemeOgesi, SEMA_SURUMU, VeriCikisi, Zarf,
};
use kalfa_core::test_destek::{
    SahteSaglayici, SahteYanit, Surucu, sanal_siniflandirici, test_baglami, test_oturumu,
};
use kalfa_evren::{EkUc, KrediGozlemi, YanitMeta};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const ARAC: &str = "kalfa_rag__ara";
const EKLENTI: &str = "kalfa-rag";
const SINIR_NEDENI: &str = "bu oturumda eklentinin EVREN sınırı doldu";
const MEBI: u64 = 1024 * 1024;
const EKLENTI_AMACI: &str = "eklenti:kalfa-rag";

/// Eklentinin EVREN'e istek atan sahte aracı. Argümanlar: `sayi` (istek adedi,
/// varsayılan 1), `bayt` (her isteğin bildirilen gövde baytı, varsayılan 10),
/// `kredi` (`true` ise yanıt kredi ve istek kimliği taşır).
struct SahteEklenti {
    izin: Arc<AtomicBool>,
    calisma: AtomicU32,
    bitti: AtomicU32,
}

impl SahteEklenti {
    /// `izin` çalışma alanında gönderim izni verildi mi; ilk gönderim onayı
    /// bunun tersidir.
    fn yeni(izin: Arc<AtomicBool>) -> Arc<Self> {
        Arc::new(Self {
            izin,
            calisma: AtomicU32::new(0),
            bitti: AtomicU32::new(0),
        })
    }

    fn calisma(&self) -> u32 {
        self.calisma.load(Ordering::SeqCst)
    }

    fn bitti(&self) -> u32 {
        self.bitti.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Arac for SahteEklenti {
    fn ad(&self) -> &str {
        ARAC
    }

    fn aciklama(&self) -> &str {
        "sahte eklenti aracı"
    }

    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        let ilk_gonderim = !self.izin.load(Ordering::SeqCst);
        Ok(Hazirlik {
            aciklama: format!("{EKLENTI} eklentisinin `{ARAC}` aracını çalıştırmak istiyor."),
            veri_cikisi: Some(VeriCikisi::EklentiCiktisi),
            eklenti: Some(EklentiTarifi {
                ad: EKLENTI.into(),
                surum: "0.1.0".into(),
                ozet: "a".repeat(64),
                globlar: vec!["**/*.rs".into()],
                evren_uclari: vec!["embeddings".into()],
                ilk_gonderim,
                dosya_sayisi: if ilk_gonderim { 3 } else { 0 },
                bayt: if ilk_gonderim { 1024 } else { 0 },
            }),
            kilitsiz: true,
            ..Default::default()
        })
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Err(AracHatasi::Gecersiz("eklenti aracı ortamla çalışır".into()))
    }

    async fn calistir_ortamli(
        &self,
        argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
        ortam: AracOrtami,
    ) -> Result<AracCiktisi, AracHatasi> {
        self.calisma.fetch_add(1, Ordering::SeqCst);
        let sayi = argumanlar["sayi"].as_u64().unwrap_or(1);
        let bayt = argumanlar["bayt"].as_u64().unwrap_or(10);
        let kredili = argumanlar["kredi"].as_bool() == Some(true);
        let kullanim = argumanlar["kullanim"].as_u64().unwrap_or(10);
        let kullanimsiz = argumanlar["kullanimsiz"].as_bool() == Some(true);
        let butce_bekleme = argumanlar["butce_bekleme_ms"]
            .as_u64()
            .map(Duration::from_millis);
        let mut icerik = String::new();
        for _ in 0..sayi {
            let ozet = istek_ozeti(bayt);
            let talep = GonderimTalebi {
                ozet: ozet.clone(),
                govde: Arc::from("metin"),
            };
            match ortam.gonderim_izni(talep).await {
                GonderimYaniti::Gonder => {
                    // Ağdan önce bütçe izni (5b tasarımı §7). Reddedilen ya da
                    // düşen talep ağa çıkmaz.
                    let izin = match butce_bekleme {
                        Some(sure) => {
                            match tokio::time::timeout(sure, ortam.butce_izni(EKLENTI_AMACI)).await
                            {
                                Ok(izin) => izin,
                                Err(_) => {
                                    return Err(AracHatasi::Gecersiz("zaman aşımı".into()));
                                }
                            }
                        }
                        None => ortam.butce_izni(EKLENTI_AMACI).await,
                    };
                    let islem = match izin {
                        Ok(islem) => islem,
                        Err(()) => return Err(AracHatasi::Iptal),
                    };
                    let meta = if kredili {
                        kredi_meta()
                    } else {
                        YanitMeta::default()
                    };
                    let bilinen = (!kullanimsiz).then_some(GunlukToken {
                        istem: kullanim,
                        tamamlama: 0,
                        dusunce: 0,
                    });
                    ortam
                        .istek_bitti(ozet, meta, IstekSonucu::Basarili, islem, bilinen)
                        .await;
                    self.bitti.fetch_add(1, Ordering::SeqCst);
                    icerik.push_str("tamam\n");
                }
                GonderimYaniti::Gonderme => {
                    return Err(AracHatasi::Gecersiz(
                        "izin-yok: kullanıcı göndermedi".into(),
                    ));
                }
                GonderimYaniti::SinirAsildi(neden) => return Err(AracHatasi::Gecersiz(neden)),
                GonderimYaniti::Iptal => return Err(AracHatasi::Iptal),
            }
        }
        Ok(AracCiktisi {
            icerik,
            kaynak: format!("eklenti:{EKLENTI}:{ARAC}"),
            ..Default::default()
        })
    }
}

/// Çalışma alanının gönderim iznini `deger` yapar: başka bir çağrının izni
/// değiştirmesini taklit eder. Kendisi serbesttir.
struct IzinAyarla {
    ad: &'static str,
    izin: Arc<AtomicBool>,
    deger: bool,
}

#[async_trait]
impl Arac for IzinAyarla {
    fn ad(&self) -> &str {
        self.ad
    }

    fn aciklama(&self) -> &str {
        "gönderim iznini değiştirir"
    }

    fn parametre_semasi(&self) -> Value {
        json!({"type": "object"})
    }

    fn yazar_mi(&self) -> bool {
        false
    }

    async fn hazirla(&self, _argumanlar: &Value) -> Result<Hazirlik, AracHatasi> {
        self.izin.store(self.deger, Ordering::SeqCst);
        Ok(Hazirlik::default())
    }

    async fn calistir(
        &self,
        _argumanlar: &Value,
        _hazirlik: &Hazirlik,
        _iptal: CancellationToken,
    ) -> Result<AracCiktisi, AracHatasi> {
        Ok(AracCiktisi {
            icerik: "tamam".into(),
            kaynak: format!("sahte:{}", self.ad),
            ..Default::default()
        })
    }
}

fn a<T: Arac + 'static>(arac: &Arc<T>) -> Arc<dyn Arac> {
    arac.clone()
}

fn istek_ozeti(bayt: u64) -> EklentiIstekOzeti {
    EklentiIstekOzeti {
        eklenti: EKLENTI.into(),
        uc: EkUc::Gomme,
        model: "qwen3-embedding-8b".into(),
        metin_sayisi: 1,
        bayt,
        ozet: "0".repeat(64),
        dosyalar: vec!["src/a.rs".into()],
        dosya_fazlasi: 0,
    }
}

fn kredi_meta() -> YanitMeta {
    YanitMeta {
        istek_id: Some("evren-istek-1".into()),
        yonlendirilen_model: Some("qwen/qwen3-embedding-8b".into()),
        kredi: Some(KrediGozlemi {
            kalan: None,
            tutulan: None,
            istek_id: Some("evren-istek-1".into()),
            zaman_ms: 77,
        }),
    }
}

fn selam() -> Komut {
    Komut::KullaniciMesaji {
        metin: "selam".into(),
    }
}

fn kayitli(d: &Path, kip: Kip) -> (OturumAyarlari, PathBuf) {
    let yol = d.join("oturum.jsonl");
    let mut ayar = OturumAyarlari::yeni("glm-5.3", d);
    ayar.kayit_yolu = Some(yol.clone());
    ayar.kip = kip;
    (ayar, yol)
}

fn arac_cagrisi(sayi: u64, bayt: u64) -> Value {
    json!({"sayi": sayi, "bayt": bayt})
}

/// Her eklenti isteğinin bilinen kullanımı `kullanim` istemdir.
fn arac_cagrisi_kullanimli(sayi: u64, kullanim: u64) -> Value {
    json!({"sayi": sayi, "bayt": 10, "kullanim": kullanim})
}

fn eklenti_kaydi(sonuc: IstekSonucu, bayt: u64) -> KayitOgesi {
    KayitOgesi::EklentiIstegi {
        cagri: CagriId::yeni(),
        istek: istek_ozeti(bayt),
        istek_id: None,
        kredi: None,
        yonlendirilen_model: None,
        sonuc,
        butce_islem: None,
    }
}

fn acilis() -> KayitOgesi {
    KayitOgesi::OturumAcildi {
        model: "glm-5.3".into(),
        kip: Kip::Serbest,
        calisma_alani: None,
    }
}

/// `yuklar`'ı yeni bir kayda sırayla yazar; ilk öğe açılış kaydı olmalıdır.
fn kayit_yaz(yol: &Path, yuklar: Vec<KayitOgesi>) {
    let oturum = OturumId::yeni();
    let mut kayit = OturumKaydi::olustur(yol).unwrap();
    for (i, yuk) in yuklar.into_iter().enumerate() {
        kayit
            .yaz(&Zarf {
                sema: SEMA_SURUMU,
                oturum,
                tur: None,
                sira: i as u64 + 1,
                yuk,
            })
            .unwrap();
    }
}

/// Kayıttaki eklenti isteklerinin sonuçları, yazılış sırasıyla.
fn eklenti_sonuclari(yol: &Path) -> Vec<IstekSonucu> {
    OturumKaydi::oku(yol)
        .unwrap()
        .into_iter()
        .filter_map(|z| match z.yuk {
            KayitOgesi::EklentiIstegi { sonuc, .. } => Some(sonuc),
            _ => None,
        })
        .collect()
}

async fn onay_bekle(d: &mut Surucu) -> Vec<OnayOgesi> {
    match d
        .bekle(|o| matches!(o, Olay::OnayGerekiyor { .. }))
        .await
        .yuk
    {
        Olay::OnayGerekiyor { ogeler, .. } => ogeler,
        _ => unreachable!(),
    }
}

async fn onizleme_bekle(d: &mut Surucu) -> (IslemId, OnizlemeNedeni, Vec<OnizlemeOgesi>) {
    match d
        .bekle(|o| matches!(o, Olay::GonderimOnizlemesi { .. }))
        .await
        .yuk
    {
        Olay::GonderimOnizlemesi {
            islem,
            ogeler,
            neden,
        } => (islem, neden, ogeler),
        _ => unreachable!(),
    }
}

#[tokio::test]
async fn ilk_gonderim_her_kipte_yerel_onay_ister() {
    for kip in [Kip::Serbest, Kip::Proje, Kip::Siki, Kip::Inceleme] {
        let d = tempfile::tempdir().unwrap();
        let (ayar, _yol) = kayitli(d.path(), kip);
        let s = SahteSaglayici::yeni(vec![
            SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
            SahteYanit::metin("tamam"),
        ]);
        let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(false)));
        let mut surucu = test_oturumu(
            test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
            ayar,
        );
        surucu.gonder(selam()).await;

        let ogeler = onay_bekle(&mut surucu).await;
        assert_eq!(ogeler.len(), 1, "{kip:?}");
        assert!(ogeler[0].yerel_onay, "{kip:?}: yerel onay istenmeli");
        assert!(ogeler[0].kisayol.is_none(), "{kip:?}: kısayol sunulmamalı");
        surucu
            .gonder(Komut::YerelOnay {
                cagri: ogeler[0].cagri,
                muhur: ogeler[0].muhur.clone(),
            })
            .await;

        assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal, "{kip:?}");
        assert_eq!(arac.calisma(), 1, "{kip:?}");
    }
}

#[tokio::test]
async fn ilk_gonderim_onayi_yalnizca_yerel_pencereden_gecer() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(false)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    let ogeler = onay_bekle(&mut surucu).await;
    let (cagri, muhur) = (ogeler[0].cagri, ogeler[0].muhur.clone());

    surucu
        .gonder(Komut::AracOnayi {
            cagri,
            karar: OnayKarari::Onayla,
            muhur: muhur.clone(),
            kisayol: false,
        })
        .await;
    let Olay::Hata { mesaj } = surucu.bekle(|o| matches!(o, Olay::Hata { .. })).await.yuk else {
        unreachable!()
    };
    assert!(mesaj.contains("yalnızca yerel pencerede"), "{mesaj}");
    assert_eq!(arac.calisma(), 0, "reddedilen onayla araç çalışmamalı");

    surucu.gonder(Komut::YerelOnay { cagri, muhur }).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.calisma(), 1);
}

#[tokio::test]
async fn ilk_gonderim_reddedilince_calismaz() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(false)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    let ogeler = onay_bekle(&mut surucu).await;

    surucu
        .gonder(Komut::AracOnayi {
            cagri: ogeler[0].cagri,
            karar: OnayKarari::Reddet { gerekce: None },
            muhur: ogeler[0].muhur.clone(),
            kisayol: false,
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.calisma(), 0);
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::AracSonucu {
            durum: AracDurumu::Reddedildi,
            ..
        }
    )));
}

#[tokio::test]
async fn ilk_gonderim_yokken_serbestte_kartsiz_calisir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    // İzin zaten var: ilk gönderim yok.
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert!(
        !surucu
            .olaylar()
            .iter()
            .any(|o| matches!(o, Olay::OnayGerekiyor { .. }))
    );
    assert_eq!(arac.calisma(), 1);
}

#[tokio::test]
async fn onizleme_kapaliyken_talep_gonderilir_ve_gunlukte_gorunur() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert_eq!(arac.bitti(), 1);
    assert_eq!(eklenti_sonuclari(&yol), vec![IstekSonucu::Basarili]);
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::IstekBilgisi { amac, basarili: true, .. } if amac.as_str() == "eklenti:kalfa-rag"
    )));

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let satirlar = gunluk_uret(&kayitlar);
    let satir = satirlar
        .iter()
        .find(|s| s.amac.starts_with("eklenti:"))
        .expect("eklenti satırı günlükte olmalı");
    assert_eq!(satir.amac, "eklenti:kalfa-rag");
    assert_eq!(
        satir.eklenti.as_ref().map(|e| e.eklenti.as_str()),
        Some(EKLENTI)
    );
    assert_eq!(satir.sonuc, IstekSonucu::Basarili);
    assert_eq!(satir.yeni[0].kaynak, "eklenti:kalfa-rag:gomme");
}

#[tokio::test]
async fn onizleme_acikken_karar_gelmeden_ilerlemez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, neden, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(neden, OnizlemeNedeni::Eklenti);
    assert_eq!(ogeler.len(), 1);
    assert_eq!(ogeler[0].kaynak, "eklenti:kalfa-rag:gomme");
    assert_eq!(ogeler[0].eklenti.as_ref().map(|e| e.bayt), Some(10));

    // Karar gelmeden araç ağa çıkmaz.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(arac.bitti(), 0);
    assert!(eklenti_sonuclari(&yol).is_empty());
    // "Metni göster": gövde karar gelene kadar tutamaçtan okunur.
    assert_eq!(
        surucu.tutamac.bekleyen_eklenti_govdesi(islem).as_deref(),
        Some("metin")
    );

    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: Vec::new(),
            hep_gonderilecekler: Vec::new(),
        })
        .await;
    // Aracın kendi sonucu da modele gitmeden önce gösterilir.
    let (islem2, neden2, _) = onizleme_bekle(&mut surucu).await;
    assert_eq!(
        surucu.tutamac.bekleyen_eklenti_govdesi(islem),
        None,
        "karar verilince gövde bırakılır"
    );
    assert_eq!(neden2, OnizlemeNedeni::Onizleme);
    surucu
        .gonder(Komut::GonderimKarari {
            islem: islem2,
            gonderilmeyecekler: Vec::new(),
            hep_gonderilecekler: Vec::new(),
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 1);
    assert_eq!(eklenti_sonuclari(&yol), vec![IstekSonucu::Basarili]);
}

#[tokio::test]
async fn onizleme_gonderilmeyeni_kayda_yazilir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, _neden, ogeler) = onizleme_bekle(&mut surucu).await;
    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: vec![ogeler[0].cagri],
            hep_gonderilecekler: Vec::new(),
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 0, "gönderilmeyen istek ağa çıkmaz");
    assert_eq!(
        eklenti_sonuclari(&yol),
        vec![IstekSonucu::Gonderilmedi {
            neden: "kullanıcı göndermedi".into()
        }]
    );
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::IstekBilgisi {
            basarili: false,
            ..
        }
    )));
}

#[tokio::test]
async fn hep_gonder_ayni_cagride_kart_acmadan_gider() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(2, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, neden, ogeler) = onizleme_bekle(&mut surucu).await;
    assert_eq!(neden, OnizlemeNedeni::Eklenti);
    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: Vec::new(),
            hep_gonderilecekler: vec![ogeler[0].cagri],
        })
        .await;

    // İkinci istek kart açmadan gider; sıradaki kart yalnızca aracın sonucudur.
    let (islem2, neden2, _) = onizleme_bekle(&mut surucu).await;
    assert_eq!(neden2, OnizlemeNedeni::Onizleme);
    surucu
        .gonder(Komut::GonderimKarari {
            islem: islem2,
            gonderilmeyecekler: Vec::new(),
            hep_gonderilecekler: Vec::new(),
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 2);
    let eklenti_kartlari = surucu
        .olaylar()
        .iter()
        .filter(|o| {
            matches!(
                o,
                Olay::GonderimOnizlemesi {
                    neden: OnizlemeNedeni::Eklenti,
                    ..
                }
            )
        })
        .count();
    assert_eq!(eklenti_kartlari, 1);
}

#[tokio::test]
async fn onizleme_beklerken_tur_iptali() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;
    let (islem, _, _) = onizleme_bekle(&mut surucu).await;
    assert!(surucu.tutamac.bekleyen_eklenti_govdesi(islem).is_some());

    surucu.tutamac.iptal_et();

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(
        surucu.tutamac.bekleyen_eklenti_govdesi(islem),
        None,
        "iptalde gövde bırakılır"
    );
    assert_eq!(arac.bitti(), 0);
    assert!(eklenti_sonuclari(&yol).is_empty());
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::AracSonucu {
            durum: AracDurumu::IptalEdildi,
            ..
        }
    )));
}

#[tokio::test]
async fn bayt_siniri_asilinca_istek_gonderilmez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(2, 200 * MEBI)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 1, "ilk istek 200 MiB, ikincisi sınırı aşar");
    assert_eq!(
        eklenti_sonuclari(&yol),
        vec![
            IstekSonucu::Basarili,
            IstekSonucu::Gonderilmedi {
                neden: SINIR_NEDENI.into()
            },
        ]
    );
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::IstekBilgisi {
            basarili: false,
            amac,
            ..
        } if amac.as_str() == "eklenti:kalfa-rag"
    )));
}

#[test]
fn eklenti_sinirlari_saf_karar() {
    let sinirda = EklentiKullanimi {
        istek: EKLENTI_ISTEK_SINIRI - 1,
        bayt: 0,
    };
    assert!(!sinirda.sinir_asilir_mi(0), "5000. istek hâlâ serbest");
    let dolu = EklentiKullanimi {
        istek: EKLENTI_ISTEK_SINIRI,
        bayt: 0,
    };
    assert!(dolu.sinir_asilir_mi(0), "5001. istek sınırı aşar");

    let bayt_sinirinda = EklentiKullanimi {
        istek: 0,
        bayt: EKLENTI_BAYT_SINIRI - 5,
    };
    assert!(!bayt_sinirinda.sinir_asilir_mi(5), "tam sınır serbest");
    assert!(
        bayt_sinirinda.sinir_asilir_mi(6),
        "bir bayt fazlası reddedilir"
    );

    let mut kullanim = EklentiKullanimi::default();
    kullanim.say(7);
    assert_eq!(kullanim, EklentiKullanimi { istek: 1, bayt: 7 });
}

#[tokio::test]
async fn kurtarmada_sayac_kayittan_yeniden_kurulur() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut yuklar = vec![acilis()];
    yuklar.extend((0..EKLENTI_ISTEK_SINIRI).map(|_| eklenti_kaydi(IstekSonucu::Basarili, 10)));
    kayit_yaz(&yol, yuklar);

    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut ayar = OturumAyarlari::yeni("glm-5.3", d.path());
    ayar.kayit_yolu = Some(yol.clone());
    let (tutamac, olaylar, _) = devam_et(
        &yol,
        ayar,
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
    )
    .unwrap();
    let mut surucu = Surucu::yeni(tutamac, olaylar);
    surucu.gonder(selam()).await;
    // 5b öncesi eklenti istekleri bilinmeyen tüketimdir; ilk model isteğinden
    // önce karar gerekir. Kayıttaki 5000 istek varsayılan 200 sınırını aştığı için
    // istek sınırı yükseltilir.
    surucu
        .butce_devam_et(Some(ButceSinirlari {
            istek: 10_000,
            token: 5_000_000,
        }))
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 0, "sınır dolmuş oturumda istek ağa çıkmaz");
    assert_eq!(
        eklenti_sonuclari(&yol).last(),
        Some(&IstekSonucu::Gonderilmedi {
            neden: SINIR_NEDENI.into()
        })
    );
}

#[tokio::test]
async fn kurtarmada_gonderilmeyen_istek_sayilmaz() {
    let d = tempfile::tempdir().unwrap();
    let yol = d.path().join("oturum.jsonl");
    let mut yuklar = vec![acilis()];
    yuklar.extend((0..EKLENTI_ISTEK_SINIRI - 1).map(|_| eklenti_kaydi(IstekSonucu::Basarili, 10)));
    // Ağa çıkmamış bu istek ne sayıya ne bayta girer.
    yuklar.push(eklenti_kaydi(
        IstekSonucu::Gonderilmedi {
            neden: "kullanıcı göndermedi".into(),
        },
        EKLENTI_BAYT_SINIRI,
    ));
    kayit_yaz(&yol, yuklar);

    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut ayar = OturumAyarlari::yeni("glm-5.3", d.path());
    ayar.kayit_yolu = Some(yol.clone());
    let (tutamac, olaylar, _) = devam_et(
        &yol,
        ayar,
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
    )
    .unwrap();
    let mut surucu = Surucu::yeni(tutamac, olaylar);
    surucu.gonder(selam()).await;
    // 5b öncesi kayıttaki bilinmeyen eklenti tüketimi için ilk istekten önce karar.
    surucu
        .butce_devam_et(Some(ButceSinirlari {
            istek: 10_000,
            token: 5_000_000,
        }))
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 1, "5000. istek hâlâ sınırın içinde");
    assert_eq!(eklenti_sonuclari(&yol).last(), Some(&IstekSonucu::Basarili));
}

#[tokio::test]
async fn tur_ozeti_eklenti_isteklerini_sayar() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    let ozet = match surucu
        .bekle(|o| matches!(o, Olay::TurOzeti { .. }))
        .await
        .yuk
    {
        Olay::TurOzeti { ozet } => ozet,
        _ => unreachable!(),
    };
    // İki model isteği ve bir eklenti isteği.
    assert_eq!(ozet.istekler.sayi, 3);
}

#[tokio::test]
async fn kredi_ve_istek_kimligi_kayda_ve_gunluge_girer() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, json!({"sayi": 1, "bayt": 10, "kredi": true})),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::KrediGozlemi { gozlem } if gozlem.zaman_ms == 77
    )));
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::IstekBilgisi { istek_id: Some(k), basarili: true, .. } if k.as_str() == "evren-istek-1"
    )));

    let kayitlar = OturumKaydi::oku(&yol).unwrap();
    let satir = gunluk_uret(&kayitlar)
        .into_iter()
        .find(|s| s.amac.starts_with("eklenti:"))
        .expect("eklenti satırı");
    assert_eq!(satir.zaman, Some(77));
    assert_eq!(satir.istek_id.as_deref(), Some("evren-istek-1"));
}

#[tokio::test]
async fn onaylanan_ilk_gonderim_izni_sonradan_verilirse_gecerli_kalir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let izin = Arc::new(AtomicBool::new(false));
    let eklenti = SahteEklenti::yeni(izin.clone());
    let veren = Arc::new(IzinAyarla {
        ad: "izin_ver",
        izin: izin.clone(),
        deger: true,
    });
    // Eklenti önce planlanır (izin yok: ilk gönderim onayı), sonra başka bir
    // çağrı izni verir; onaylanan çağrı yine çalışır.
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![(ARAC, arac_cagrisi(1, 10)), ("izin_ver", json!({}))]),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&eklenti), a(&veren)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;

    let ogeler = onay_bekle(&mut surucu).await;
    assert_eq!(ogeler.len(), 1, "kart yalnız eklentinin çağrısını gösterir");
    surucu
        .gonder(Komut::YerelOnay {
            cagri: ogeler[0].cagri,
            muhur: ogeler[0].muhur.clone(),
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(eklenti.calisma(), 1);
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::AracSonucu { arac, durum: AracDurumu::Basarili, .. } if arac.as_str() == ARAC
    )));
}

#[tokio::test]
async fn izin_sonradan_geri_alinirsa_calistirilmaz() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let izin = Arc::new(AtomicBool::new(true));
    let eklenti = SahteEklenti::yeni(izin.clone());
    let geri_alan = Arc::new(IzinAyarla {
        ad: "izin_geri_al",
        izin: izin.clone(),
        deger: false,
    });
    // Eklenti izinli planlanır; sonra başka bir çağrı izni kaldırır. Sıkılaştırma
    // onay istemeden çağrıyı durdurur.
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            (ARAC, arac_cagrisi(1, 10)),
            ("izin_geri_al", json!({})),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&eklenti), a(&geri_alan)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(eklenti.calisma(), 0);
    assert!(surucu.olaylar().iter().any(|o| matches!(
        o,
        Olay::AracSonucu { arac, durum: AracDurumu::Basarisiz, .. } if arac.as_str() == ARAC
    )));
}

#[test]
fn gunluk_eklenti_satiri_sohbet_kullanimini_almaz() {
    let oturum = OturumId::yeni();
    let z = |sira, yuk| Zarf {
        sema: SEMA_SURUMU,
        oturum,
        tur: None,
        sira,
        yuk,
    };
    let kayitlar = vec![
        z(
            1,
            KayitOgesi::Istek {
                amac: "ana".into(),
                istek_id: None,
                kredi: None,
                yonlendirilen_model: None,
                model: Some("glm-5.3".into()),
                yeni: Vec::new(),
                onceki: 0,
                sonuc: IstekSonucu::Basarili,
                butce_islem: None,
            },
        ),
        z(
            2,
            KayitOgesi::Kullanim {
                istem: 1,
                tamamlama: 2,
                dusunce: 0,
            },
        ),
        z(3, eklenti_kaydi(IstekSonucu::Basarili, 10)),
        z(
            4,
            KayitOgesi::Kullanim {
                istem: 5,
                tamamlama: 6,
                dusunce: 0,
            },
        ),
        z(
            5,
            KayitOgesi::IstekKoptu {
                mesaj: "kopma".into(),
            },
        ),
    ];

    let satirlar = gunluk_uret(&kayitlar);
    assert_eq!(satirlar.len(), 2);
    assert_eq!(
        satirlar[0].token,
        Some(GunlukToken {
            istem: 1,
            tamamlama: 2,
            dusunce: 0,
        })
    );
    assert!(satirlar[1].eklenti.is_some());
    assert_eq!(satirlar[1].token, None, "kullanım eklenti satırına girmez");
    assert_eq!(
        satirlar[1].sonuc,
        IstekSonucu::Basarili,
        "kopma eklenti satırına yazılmaz"
    );
}

/// Bütçe kartını bekler ve kartın sayaçlarını döner.
async fn kart_bekle(d: &mut Surucu) -> ButceGorunumu {
    match d
        .bekle(|o| matches!(o, Olay::ButceKarariGerekli { .. }))
        .await
        .yuk
    {
        Olay::ButceKarariGerekli { durum } => durum,
        diger => unreachable!("{diger:?}"),
    }
}

/// Oturumdaki son bütçe görünümü.
fn son_butce(d: &Surucu) -> ButceGorunumu {
    d.olaylar()
        .iter()
        .rev()
        .find_map(|o| match o {
            Olay::ButceDurumu { durum } => Some(*durum),
            _ => None,
        })
        .expect("bütçe durumu olmalı")
}

fn butce_ayari(mut ayar: OturumAyarlari, istek: u64) -> OturumAyarlari {
    ayar.butce = ButceSinirlari {
        istek,
        token: 5_000_000,
    };
    ayar
}

fn durdur() -> Komut {
    Komut::ButceKarari {
        karar: ButceKarari::Durdur,
    }
}

/// `DevamEt` kararı; gözlenen sayaçlar kartın sayaçlarıdır.
fn devam(istek: u64, baslatilan: u64, bilinmeyen: u64) -> Komut {
    Komut::ButceKarari {
        karar: ButceKarari::DevamEt {
            sinirlar: ButceSinirlari {
                istek,
                token: 5_000_000,
            },
            gozlenen_baslatilan: baslatilan,
            gozlenen_bilinmeyen: bilinmeyen,
        },
    }
}

/// İki paralel eklenti çağrısı ile model isteği aynı oturum sayacını paylaşır:
/// sınır üçte dolar ve ikinci model isteği kartla bekler.
#[tokio::test]
async fn paralel_eklenti_cagrilari_ayni_oturum_sayacini_paylasir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![
            (ARAC, arac_cagrisi(1, 10)),
            (ARAC, arac_cagrisi(1, 10)),
        ]),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        butce_ayari(ayar, 3),
    );
    surucu.gonder(selam()).await;

    let kart = kart_bekle(&mut surucu).await;
    assert_eq!((kart.baslatilan, kart.engel), (3, Some(ButceEngeli::Istek)));
    assert_eq!(
        s.istekler().len(),
        1,
        "ikinci model isteği karardan önce gitmemeli"
    );
    assert_eq!(arac.bitti(), 2);
    surucu.gonder(durdur()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Iptal);
}

/// Ayrı bir oturumun sayacı diğerinin sınırını paylaşmaz: biri kartta beklerken
/// öteki kendi ilk isteğini gönderir.
#[tokio::test]
async fn ayri_oturumlar_butce_sayacini_paylasmaz() {
    let d = tempfile::tempdir().unwrap();
    let s_a = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("son"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut a_surucu = test_oturumu(
        test_baglami(s_a, vec![a(&arac)], sanal_siniflandirici()),
        butce_ayari(OturumAyarlari::yeni("glm-5.3", d.path()), 1),
    );
    a_surucu.gonder(selam()).await;
    assert_eq!(kart_bekle(&mut a_surucu).await.baslatilan, 1);

    let s_b = SahteSaglayici::yeni(vec![SahteYanit::metin("tamam")]);
    let mut b_surucu = test_oturumu(
        test_baglami(s_b.clone(), vec![], sanal_siniflandirici()),
        butce_ayari(OturumAyarlari::yeni("glm-5.3", d.path()), 1),
    );
    b_surucu.gonder(selam()).await;
    assert_eq!(b_surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(
        s_b.istekler().len(),
        1,
        "ikinci oturumun ilk isteği serbest"
    );

    a_surucu.gonder(durdur()).await;
    assert_eq!(a_surucu.tur_sonu().await, BitisNedeni::Iptal);
}

/// "Gönderme" kararı bütçe tüketmez: yalnız iki model isteği izin harcar.
#[tokio::test]
async fn onizlemede_gonderme_butce_tuketmez() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(Komut::OnizlemeAyarla { acik: true }).await;
    surucu.gonder(selam()).await;

    let (islem, _neden, ogeler) = onizleme_bekle(&mut surucu).await;
    surucu
        .gonder(Komut::GonderimKarari {
            islem,
            gonderilmeyecekler: vec![ogeler[0].cagri],
            hep_gonderilecekler: Vec::new(),
        })
        .await;

    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 0);
    assert_eq!(
        son_butce(&surucu).baslatilan,
        2,
        "yalnız iki model isteği izin harcar"
    );
}

/// Eklenti isteklerinin bilinen kullanımı bilinen tokena eklenir ve her deneme
/// kendi izin kimliğiyle kayda girer.
#[tokio::test]
async fn eklenti_kullanimi_bilinen_tokena_eklenir_ve_kayitta_izinle_baglanir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi_kullanimli(2, 5)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s, vec![a(&arac)], sanal_siniflandirici()),
        ayar,
    );
    surucu.gonder(selam()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);

    // Modeller 15 + 15 (10 istem, 5 tamamlama); iki eklenti isteği 5 + 5.
    assert_eq!(son_butce(&surucu).bilinen_token, 40);
    let kimlikler: Vec<Option<ButceIslemId>> = OturumKaydi::oku(&yol)
        .unwrap()
        .into_iter()
        .filter_map(|z| match z.yuk {
            KayitOgesi::EklentiIstegi { butce_islem, .. } => Some(butce_islem),
            _ => None,
        })
        .collect();
    assert_eq!(
        kimlikler,
        vec![Some(ButceIslemId(2)), Some(ButceIslemId(3))]
    );
}

/// Kart beklerken çağrı yeniden başlamaz: `DevamEt` aynı eklenti çağrısını
/// sürdürür ve araç bir kez çalışır.
#[tokio::test]
async fn butce_devami_ayni_eklenti_cagrisini_surdurur() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        butce_ayari(ayar, 1),
    );
    surucu.gonder(selam()).await;

    let kart = surucu
        .butce_devam_et(Some(ButceSinirlari {
            istek: 10,
            token: 5_000_000,
        }))
        .await;
    assert_eq!(kart.baslatilan, 1);
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.calisma(), 1, "devam aracı yeniden çalıştırmaz");
    assert_eq!(arac.bitti(), 1);
    assert_eq!(s.istekler().len(), 2);
    assert_eq!(son_butce(&surucu).baslatilan, 3);
}

/// `Durdur` bekleyen eklenti isteğini iptal eder: araç ağa çıkmaz ve model
/// isteği bir daha gönderilmez.
#[tokio::test]
async fn durdur_bekleyen_eklenti_istegini_iptal_eder() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, arac_cagrisi(1, 10)),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        butce_ayari(ayar, 1),
    );
    surucu.gonder(selam()).await;
    kart_bekle(&mut surucu).await;

    surucu.gonder(durdur()).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Iptal);
    assert_eq!(arac.calisma(), 1);
    assert_eq!(arac.bitti(), 0, "durdurulan eklenti isteği ağa çıkmaz");
    assert_eq!(s.istekler().len(), 1, "durdurmadan sonra model isteği yok");
}

/// Araç zaman aşımına uğrayınca bekleyen bütçe talebi düşer; sonradan gelen
/// devam kararı onu yeniden başlatmaz.
#[tokio::test]
async fn zaman_asimi_bekleyen_butce_talebini_birakir() {
    let d = tempfile::tempdir().unwrap();
    let (ayar, _yol) = kayitli(d.path(), Kip::Serbest);
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::arac(ARAC, json!({"sayi": 1, "bayt": 10, "butce_bekleme_ms": 20})),
        SahteYanit::metin("tamam"),
    ]);
    let arac = SahteEklenti::yeni(Arc::new(AtomicBool::new(true)));
    let mut surucu = test_oturumu(
        test_baglami(s.clone(), vec![a(&arac)], sanal_siniflandirici()),
        butce_ayari(ayar, 1),
    );
    surucu.gonder(selam()).await;
    assert_eq!(kart_bekle(&mut surucu).await.baslatilan, 1);

    // Araç 20 ms sonra vazgeçer; ikinci model isteği, düşen talebin arkasında
    // kartla bekler. Sonra karar verilir.
    surucu.bekle(|o| matches!(o, Olay::AracSonucu { .. })).await;
    surucu.gonder(devam(10, 1, 0)).await;
    assert_eq!(surucu.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(arac.bitti(), 0, "düşen talep ağa çıkış başlatmaz");
    assert_eq!(s.istekler().len(), 2);
    assert_eq!(
        son_butce(&surucu).baslatilan,
        2,
        "yalnız iki model isteği başladı"
    );
}
