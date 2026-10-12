//! EVREN yolu (aşama 4 tasarımı §8.4, §9.2–§9.5, §16.1): eklentinin
//! `evren-cagir` isteği, ilk gönderim izni, gönderim kararı, kayıt ve hata
//! eşlemesi. Oturum henüz `OrtamIstegi`'ni işlemediği için talepleri bu
//! dosyadaki görev karşılar.

mod ortak;

use std::sync::Arc;
use std::time::Duration;

use kalfa_core::arac::{
    Arac, AracCiktisi, AracHatasi, AracOrtami, CiktiGonderici, GonderimTalebi, GonderimYaniti,
    Hazirlik, OrtamIstegi,
};
use kalfa_core::butce::ButceIslemId;
use kalfa_core::gonderim::{EklentiIstekOzeti, GunlukToken, IstekAsamasi, IstekSonucu};
use kalfa_core::kimlik::CagriId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_core::test_destek::{SahteSaglayici, butce_kanali, sahte_modeller};
use kalfa_evren::{EkUc, EvrenError, HamYanit, YanitMeta};
use kalfa_plugin::{BellekIzni, EvrenBaglami, GonderimIzni};
use ortak::*;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const GIZLI: &str = "GIZLI_DEGER=evren-testi-7f3a";
const GOMME: &str = r#"{"model":"qwen3-embedding-8b","input":"merhaba","dimensions":32}"#;
const SIRALAMA: &str = r#"{"model":"qwen3-reranker-8b","query":"sorgu","documents":["a","b","c"]}"#;
/// Kayıtlı gerçek EVREN yanıtları: kullanımları 5 ve 279 istemdir (5b Görev 2).
const GOMME_KAYDI: &str =
    include_str!("../../kalfa-evren/tests/kayitlar/embeddings_qwen3_1024.json");
const SIRALAMA_KAYDI: &str = include_str!("../../kalfa-evren/tests/kayitlar/rerank_qwen3.json");

/// Bir `calistir_ortamli` çağrısının sonucu, gelen talepler, ağa çıkan
/// isteklerin kayıtları ve bütçe ilişkileri.
struct Calisma {
    sonuc: Result<AracCiktisi, AracHatasi>,
    talepler: Vec<GonderimTalebi>,
    kayitlar: Vec<(EklentiIstekOzeti, IstekSonucu)>,
    /// Bütçeye sorulan izin amaçları, geliş sırasıyla.
    butce_amaclari: Vec<String>,
    /// Ağa çıkan isteklerin bütçe bitişleri: izin kimliği ve bilinen kullanım.
    bitisler: Vec<(Option<ButceIslemId>, Option<GunlukToken>)>,
}

impl Calisma {
    fn icerik(&self) -> &str {
        match &self.sonuc {
            Ok(c) => &c.icerik,
            Err(e) => panic!("araç başarısız: {e}"),
        }
    }
}

/// Sahte oturumun bir bütçe talebine cevabı. Dizi bitince izin verilir.
#[derive(Clone, Copy)]
enum Butce {
    /// İzin verir.
    Ver,
    /// Yok der: çağıran ağa çıkmaz.
    Reddet,
    /// İzni bu süre sonra verir; çağrı bu sürede kart beklemiş gibi bekler.
    Bekle(Duration),
}

/// Aracı oturumun yerine koşturur: her gönderim talebine `karar` ile yanıt
/// verir, bütçe izni istemeyen akışta izin verir.
async fn ortamli(
    arac: Arc<dyn Arac>,
    argumanlar: Value,
    hazirlik: Hazirlik,
    karar: impl FnMut(&GonderimTalebi) -> GonderimYaniti,
) -> Calisma {
    ortamli_butce(arac, argumanlar, hazirlik, karar, Vec::new()).await
}

/// `ortamli` gibi, ama bütçe talepleri `butce` dizisindeki sırayla cevaplanır.
async fn ortamli_butce(
    arac: Arc<dyn Arac>,
    argumanlar: Value,
    hazirlik: Hazirlik,
    mut karar: impl FnMut(&GonderimTalebi) -> GonderimYaniti,
    butce: Vec<Butce>,
) -> Calisma {
    let (tx, mut rx) = mpsc::channel(64);
    let (talepci, mut butce_rx) = butce_kanali();
    let ortam = AracOrtami::yeni(CiktiGonderici::bos(), CagriId::yeni(), tx, talepci);
    let gorev = tokio::spawn(async move {
        arac.calistir_ortamli(&argumanlar, &hazirlik, CancellationToken::new(), ortam)
            .await
    });
    let mut talepler = Vec::new();
    let mut kayitlar = Vec::new();
    let mut butce_amaclari = Vec::new();
    let mut bitisler = Vec::new();
    loop {
        tokio::select! {
            istek = rx.recv() => match istek {
                Some(OrtamIstegi::Gonderim { talep, yanit, .. }) => {
                    let k = karar(&talep);
                    talepler.push(talep);
                    let _ = yanit.send(k);
                }
                Some(OrtamIstegi::IstekBitti { ozet, sonuc, butce_islem, kullanim, .. }) => {
                    kayitlar.push((ozet, sonuc));
                    bitisler.push((butce_islem, kullanim));
                }
                None => break,
            },
            Some(talep) = butce_rx.recv() => {
                let sira = butce_amaclari.len();
                butce_amaclari.push(talep.amac.clone());
                let islem = ButceIslemId(sira as u64 + 1);
                match butce.get(sira).copied().unwrap_or(Butce::Ver) {
                    Butce::Ver => {
                        let _ = talep.yanit.send(Some(islem));
                    }
                    Butce::Reddet => {
                        let _ = talep.yanit.send(None);
                    }
                    Butce::Bekle(sure) => {
                        tokio::spawn(async move {
                            tokio::time::sleep(sure).await;
                            let _ = talep.yanit.send(Some(islem));
                        });
                    }
                }
            }
        }
    }
    let sonuc = gorev.await.unwrap();
    Calisma {
        sonuc,
        talepler,
        kayitlar,
        butce_amaclari,
        bitisler,
    }
}

/// Kayıtlı gerçek EVREN gövdesi, ham yanıt olarak.
fn kayitli_yanit(govde: &str) -> HamYanit {
    HamYanit {
        govde: govde.to_string(),
        model: None,
        meta: YanitMeta::default(),
    }
}

async fn hazirlikla(arac: &Arc<dyn Arac>, argumanlar: &Value) -> Hazirlik {
    arac.hazirla(argumanlar).await.unwrap()
}

fn evren_args(uc: &str, govde: &str) -> Value {
    json!({"uc": uc, "govde": govde})
}

/// Manifestte `evren` yoksa araç `izin-yok` alır; ağa hiçbir şey çıkmaz.
#[tokio::test]
async fn manifestte_evren_yoksa_izin_yok() {
    let d = Duzenek::yeni(&manifest(&["**/*"], &[], 0)).await;
    let c = d.cagir("evren", evren_args("embeddings", GOMME)).await;
    assert_eq!(
        c.icerik(),
        "1: izin-yok: evren yeteneği manifestte yok: `embeddings`"
    );
}

/// Onay taşımayan çağrı (oturum sormadan çalıştırdı): istek ağa çıkmaz.
#[tokio::test]
async fn izin_yoksa_istek_aga_cikmaz() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(false),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let c = ortamli(arac, args, Hazirlik::default(), |_| {
        panic!("izinsiz çağrıda talep gelmemeliydi")
    })
    .await;
    assert!(
        c.icerik()
            .contains("izin-yok: bu çalışma alanında EVREN'e gönderim izni yok"),
        "{}",
        c.icerik()
    );
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.talepler.is_empty() && c.kayitlar.is_empty());
}

/// İzin yokken hazırlık ilk gönderimi bildirir: dosya sayısı ve boyut, gizli
/// dosya (`.env`) sayılmadan.
#[tokio::test]
async fn ilk_gonderimde_dosya_sayisi_ve_bayt_gizli_sayilmaz() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&["**/*"], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(false),
    )
    .await;
    d.dosya("a.rs", b"fn a() {}\n")
        .dosya("docs/b.md", b"# B\n")
        .dosya(".env", GIZLI.as_bytes());
    let h = d.arac("evren").hazirla(&json!({})).await.unwrap();
    let t = h.eklenti.unwrap();
    let bayt = (b"fn a() {}\n".len() + b"# B\n".len()) as u64;
    assert!(t.ilk_gonderim);
    assert_eq!(t.dosya_sayisi, 2);
    assert_eq!(t.bayt, bayt);
    assert!(
        h.aciklama
            .contains("**kalfa-test** bu çalışma alanında ilk kez EVREN'e")
    );
    assert!(h.aciklama.contains("`**/*`"), "{}", h.aciklama);
    assert!(
        h.aciklama.contains("**2 dosya (0,0 MB)**"),
        "{}",
        h.aciklama
    );
    assert!(!h.aciklama.contains(GIZLI));
}

/// İzin varken hazırlık ilk gönderimi bildirmez.
#[tokio::test]
async fn izin_varken_ilk_gonderim_yok() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&["**/*"], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let h = d.arac("evren").hazirla(&json!({})).await.unwrap();
    let t = h.eklenti.unwrap();
    assert!(!t.ilk_gonderim && t.dosya_sayisi == 0 && t.bayt == 0);
    assert!(!h.aciklama.contains("ilk kez"));
}

/// Hazırlıkla gelen ilk onaylı çağrı izni kaydeder ve istek gider.
#[tokio::test]
async fn ilk_onayli_cagri_izni_kaydeder_ve_istek_gider() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let izin = BellekIzni::yeni(false);
    let d = Duzenek::evrenli(&manifest(&[], &["embeddings"], 0), &sahte, izin.clone()).await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    assert!(hazirlik.eklenti.as_ref().is_some_and(|t| t.ilk_gonderim));
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert!(izin.var_mi());
    assert_eq!(sahte.ek_istekleri().len(), 1);
    assert!(c.icerik().contains(r#""object":"list""#), "{}", c.icerik());
}

/// Talep, istek ağa çıkmadan gelir; `Gonder`'den sonra istek gider ve kayıt
/// başarılı yazılır.
#[tokio::test]
async fn talep_karar_gelmeden_istek_gitmez_gonder_kayit_yazar() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let s = sahte.clone();
    let c = ortamli(arac, args, hazirlik, move |talep| {
        assert!(
            s.ek_istekleri().is_empty(),
            "istek karar gelmeden ağa çıktı"
        );
        assert_eq!(talep.ozet.uc, EkUc::Gomme);
        assert_eq!(talep.ozet.model, "qwen3-embedding-8b");
        assert_eq!(talep.ozet.metin_sayisi, 1);
        GonderimYaniti::Gonder
    })
    .await;
    assert!(c.icerik().contains(r#""object":"list""#), "{}", c.icerik());
    assert_eq!(sahte.ek_istekleri().len(), 1);
    assert_eq!(c.talepler.len(), 1);
    assert!(c.talepler[0].govde.contains("merhaba"));
    assert_eq!(c.kayitlar.len(), 1);
    assert_eq!(c.kayitlar[0].1, IstekSonucu::Basarili);
    assert_eq!(c.kayitlar[0].0.model, "qwen3-embedding-8b");
}

/// Sıralama ucu da aynı yoldan gider; sorgu metin sayısına girmez.
#[tokio::test]
async fn siralama_ucu_dogru_modelle_gider() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["rerank"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let govde = r#"{"model":"qwen3-reranker-8b","query":"sorgu","documents":["a","b"]}"#;
    let args = evren_args("rerank", govde);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert!(c.sonuc.is_ok(), "{}", c.icerik());
    assert_eq!(sahte.ek_istekleri()[0].0, EkUc::Siralama);
    assert_eq!(c.talepler[0].ozet.uc, EkUc::Siralama);
    assert_eq!(c.talepler[0].ozet.metin_sayisi, 2);
}

/// Kullanıcı göndermedi: istek ağa çıkmaz, kayıt yazılmaz (kaydı oturum yazar).
#[tokio::test]
async fn gonderme_istek_gitmez_kayit_yazilmaz() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonderme).await;
    assert!(
        c.icerik().contains("izin-yok: kullanıcı göndermedi"),
        "{}",
        c.icerik()
    );
    assert_eq!(c.talepler.len(), 1);
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.kayitlar.is_empty());
}

/// Oturum sınırı dolduysa istek ağa çıkmaz; ileti eklentiye gider.
#[tokio::test]
async fn oturum_siniri_karari_istek_gitmez() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| {
        GonderimYaniti::SinirAsildi("bu oturumda eklentinin EVREN sınırı doldu".into())
    })
    .await;
    assert!(
        c.icerik()
            .contains("sinir-asildi: bu oturumda eklentinin EVREN sınırı doldu"),
        "{}",
        c.icerik()
    );
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.kayitlar.is_empty());
}

/// `once_oku` dosyası gövdeye JSON'da kaçışlı girer; talepte okunan dosya
/// listelenir.
#[tokio::test]
async fn once_oku_dosyasi_gonderilir_ve_talepte_listelenir() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&["**/*"], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    d.dosya("a.rs", b"fn a() {}\n\"tirnak\"");
    let govde = r#"{"model":"qwen3-embedding-8b","input":"{DOSYA}","dimensions":32}"#;
    let args = json!({"uc": "embeddings", "govde": govde, "once_oku": "a.rs"});
    let arac = d.arac("evren");
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert!(c.sonuc.is_ok(), "{}", c.icerik());
    assert_eq!(c.talepler[0].ozet.dosyalar, vec!["a.rs".to_string()]);
    let gonderilen = sahte.ek_istekleri()[0].1["input"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert_eq!(gonderilen, "fn a() {}\n\"tirnak\"");
}

/// S15 genişletmesi: eklenti `.env`'i okuyup göndermeye çalışır. `dosya-oku`
/// reddeder; gizli içerik hiçbir isteğe girmez.
#[tokio::test]
async fn s15_gizli_dosya_gomulemez() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&["**/*"], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    d.dosya(".env", GIZLI.as_bytes())
        .dosya("a.rs", b"fn a() {}\n");
    let govde = r#"{"model":"qwen3-embedding-8b","input":"{DOSYA}"}"#;
    let args = json!({"uc": "embeddings", "govde": govde, "once_oku": ".env"});
    let arac = d.arac("evren");
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert_eq!(c.icerik(), "izin-yok: gizli dosya");
    assert!(c.talepler.is_empty() && c.kayitlar.is_empty());
    assert!(sahte.ek_istekleri().is_empty());
    assert!(!format!("{:?}", sahte.ek_istekleri()).contains("evren-testi-7f3a"));
}

/// Model katalogda bu uca uymuyorsa istek gitmez.
#[tokio::test]
async fn yanlis_gorevli_model_reddedilir() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", r#"{"model":"glm-5.3","input":"merhaba"}"#);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert_eq!(c.icerik(), "1: izin-yok: model bu uç için değil");
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.talepler.is_empty());
}

/// Katalog alınamazsa `evren(...)`; başarılı alım önbelleğe girer, hatalı girmez.
#[tokio::test]
async fn model_katalogu_hatasi_ve_onbellek() {
    let sahte = SahteSaglayici::yeni(vec![]);
    sahte.modeller_ayarla(Err(EvrenError::GecersizYanit("bozuk".into())));
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac.clone(), args.clone(), hazirlik, |_| {
        GonderimYaniti::Gonder
    })
    .await;
    assert_eq!(c.icerik(), "1: evren: model kataloğu alınamadı");
    assert!(sahte.ek_istekleri().is_empty());
    // Hata önbelleğe girmedi: katalog düzelince aynı araç çalışır.
    sahte.modeller_ayarla(Ok(sahte_modeller()));
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert!(c.icerik().contains(r#""object":"list""#), "{}", c.icerik());
}

/// Hız sınırı `sinir-asildi`, şart `evren(...)`; ikisi de kayda hata olarak
/// girer. Ağ hatası kayıtta `Baglanti` aşamasındadır.
#[tokio::test]
async fn hata_eslemesi_hiz_sart_ag() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);

    sahte.ek_yanit_ekle(Err(EvrenError::HizSiniri {
        tekrar_dene: Some(Duration::from_secs(30)),
    }));
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac.clone(), args.clone(), hazirlik, |_| {
        GonderimYaniti::Gonder
    })
    .await;
    assert!(
        c.icerik()
            .contains("sinir-asildi: EVREN hız sınırı; 30 sn sonra yeniden deneyin"),
        "{}",
        c.icerik()
    );
    assert!(matches!(
        c.kayitlar[0].1,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Yanit,
            ..
        }
    ));

    sahte.ek_yanit_ekle(Err(EvrenError::GunlukSinir {
        sifirlanma: Some("2026-10-09T00:00:00Z".into()),
    }));
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac.clone(), args.clone(), hazirlik, |_| {
        GonderimYaniti::Gonder
    })
    .await;
    assert!(
        c.icerik()
            .contains("sinir-asildi: EVREN'in günlük token sınırına ulaşıldı"),
        "{}",
        c.icerik()
    );
    assert!(matches!(c.kayitlar[0].1, IstekSonucu::Hata { .. }));

    sahte.ek_yanit_ekle(Err(EvrenError::SartKabulGerekli { surum: Some(2) }));
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac.clone(), args.clone(), hazirlik, |_| {
        GonderimYaniti::Gonder
    })
    .await;
    assert_eq!(c.icerik(), "1: evren: EVREN şartları yeniden onay bekliyor");
    assert!(matches!(c.kayitlar[0].1, IstekSonucu::Hata { .. }));

    sahte.ek_yanit_ekle(Err(EvrenError::ag_baglanti("bağlantı kesildi")));
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert!(
        c.icerik().starts_with("1: evren: EVREN'e ulaşılamadı"),
        "{}",
        c.icerik()
    );
    assert!(matches!(
        c.kayitlar[0].1,
        IstekSonucu::Hata {
            asama: IstekAsamasi::Baglanti,
            ..
        }
    ));
}

/// Şart kapısı açık değilken istek ağa çıkmaz.
#[tokio::test]
async fn sart_kapisi_bosta_degilse_istek_gitmez() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let saglayici: Arc<dyn Saglayici> = sahte.clone();
    let kapi = Arc::new(SartKapisi::yeni(saglayici.clone()));
    // Kabul edilmemiş şart: kapı metni gösterime alır.
    kapi.ilk_durum().await.unwrap();
    let mut d = Duzenek::yeni(&manifest(&[], &["embeddings"], 0)).await;
    d.evren = Some(EvrenBaglami::yeni(saglayici, kapi));
    d.izin = BellekIzni::yeni(true);
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert_eq!(c.icerik(), "1: evren: EVREN şartları yeniden onay bekliyor");
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.talepler.is_empty() && c.kayitlar.is_empty());
}

/// Çağrı başına 1 000 istek; 1 001'incisi `sinir-asildi` alır ve ağa çıkmaz.
#[tokio::test]
async fn cagri_basina_istek_siniri() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = json!({"uc": "embeddings", "govde": GOMME, "tekrar": 1001});
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    assert_eq!(sahte.ek_istekleri().len(), 1000);
    assert_eq!(c.talepler.len(), 1000);
    assert_eq!(c.kayitlar.len(), 1000);
    let son = c.icerik().lines().last().unwrap_or_default();
    assert_eq!(
        son,
        "1001: sinir-asildi: bu çağrıda EVREN istek sınırı (1000) doldu"
    );
}

/// Manifest `evren` istediğinde ilk gönderim onayı kayıt hatasıyla başarısız
/// olursa araç çalışmaz ve ağa hiçbir şey çıkmaz.
struct KayitEdilemeyenIzin;

impl GonderimIzni for KayitEdilemeyenIzin {
    fn var_mi(&self) -> bool {
        false
    }

    fn ver(&self) -> Result<(), String> {
        Err("disk dolu".into())
    }
}

#[tokio::test]
async fn izin_kaydedilemezse_arac_calismaz() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        Arc::new(KayitEdilemeyenIzin),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let hazirlik = hazirlikla(&arac, &args).await;
    let c = ortamli(arac, args, hazirlik, |_| GonderimYaniti::Gonder).await;
    match &c.sonuc {
        Err(AracHatasi::GirisCikis(ileti)) => {
            assert_eq!(ileti, "gönderim izni kaydedilemedi: disk dolu");
        }
        diger => panic!("beklenmeyen sonuç: {diger:?}"),
    }
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.talepler.is_empty());
}

/// Bütçe izni ağdan hemen önce alınır; kayıtlı gerçek yanıtın kullanımı izin
/// kimliğiyle birlikte bitişe gider.
#[tokio::test]
async fn butce_izni_alinir_ve_kullanim_izinle_birlikte_bitise_gider() {
    let sahte = SahteSaglayici::yeni(vec![]);
    sahte.ek_yanit_ekle(Ok(kayitli_yanit(GOMME_KAYDI)));
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let c = ortamli(
        d.arac("evren"),
        evren_args("embeddings", GOMME),
        Hazirlik::default(),
        |_| GonderimYaniti::Gonder,
    )
    .await;
    assert!(c.sonuc.is_ok(), "{:?}", c.sonuc);
    assert_eq!(c.butce_amaclari, vec!["eklenti:kalfa-test".to_string()]);
    assert_eq!(
        c.bitisler,
        vec![(
            Some(ButceIslemId(1)),
            Some(GunlukToken {
                istem: 5,
                tamamlama: 0,
                dusunce: 0,
            })
        )]
    );
    assert_eq!(sahte.ek_istekleri().len(), 1);
}

/// Gömme ve sıralamanın bilinen kullanımları her ağ denemesinin kendi
/// bitişinde toplanır: 5 + 279 = 284 istem.
#[tokio::test]
async fn gomme_ve_siralama_kullanimlari_her_denemede_ayri_biter() {
    let sahte = SahteSaglayici::yeni(vec![]);
    sahte.ek_yanit_ekle(Ok(kayitli_yanit(GOMME_KAYDI)));
    sahte.ek_yanit_ekle(Ok(kayitli_yanit(SIRALAMA_KAYDI)));
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings", "rerank"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let gomme = ortamli(
        d.arac("evren"),
        evren_args("embeddings", GOMME),
        Hazirlik::default(),
        |_| GonderimYaniti::Gonder,
    )
    .await;
    let siralama = ortamli(
        d.arac("evren"),
        evren_args("rerank", SIRALAMA),
        Hazirlik::default(),
        |_| GonderimYaniti::Gonder,
    )
    .await;
    let istemler: Vec<u64> = gomme
        .bitisler
        .iter()
        .chain(&siralama.bitisler)
        .filter_map(|(_, kullanim)| kullanim.map(|t| t.istem))
        .collect();
    assert_eq!(istemler, vec![5, 279]);
    assert_eq!(istemler.iter().sum::<u64>(), 284);
}

/// Bütçe reddi ağa çıkmaz ve eklentiye sınır hatası gibi gitmez: araç
/// `iptal-edildi` alır, RAG'ın toplu küçültmesi tetiklenmez.
#[tokio::test]
async fn butce_reddi_ag_cikarmaz_ve_sinir_hatasi_gibi_gitmez() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let c = ortamli_butce(
        d.arac("evren"),
        evren_args("embeddings", GOMME),
        Hazirlik::default(),
        |_| GonderimYaniti::Gonder,
        vec![Butce::Reddet],
    )
    .await;
    assert_eq!(c.icerik(), "1: iptal-edildi");
    assert!(sahte.ek_istekleri().is_empty());
    assert!(c.kayitlar.is_empty() && c.bitisler.is_empty());
}

/// Kart beklemek çağrıyı yeniden başlatmaz: tek bütçe talebi, tek ağ isteği.
#[tokio::test]
async fn butce_beklemesi_ayni_cagriyi_surdurur() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let c = ortamli_butce(
        d.arac("evren"),
        evren_args("embeddings", GOMME),
        Hazirlik::default(),
        |_| GonderimYaniti::Gonder,
        vec![Butce::Bekle(Duration::from_millis(50))],
    )
    .await;
    assert!(c.sonuc.is_ok(), "{:?}", c.sonuc);
    assert_eq!(c.butce_amaclari.len(), 1, "bekleme yeni talep açmamalı");
    assert_eq!(sahte.ek_istekleri().len(), 1);
    assert_eq!(c.bitisler.len(), 1);
}

/// Araç iptali bekleyen bütçe talebini düşürür; sonradan gelen izin ağa
/// çıkış başlatamaz.
#[tokio::test]
async fn iptal_bekleyen_butce_talebini_birakir_ve_gec_izin_ag_cikarmaz() {
    let sahte = SahteSaglayici::yeni(vec![]);
    let d = Duzenek::evrenli(
        &manifest(&[], &["embeddings"], 0),
        &sahte,
        BellekIzni::yeni(true),
    )
    .await;
    let arac = d.arac("evren");
    let args = evren_args("embeddings", GOMME);
    let (tx, mut rx) = mpsc::channel(64);
    let (talepci, mut butce_rx) = butce_kanali();
    let ortam = AracOrtami::yeni(CiktiGonderici::bos(), CagriId::yeni(), tx, talepci);
    let iptal = CancellationToken::new();
    let gorev = tokio::spawn({
        let iptal = iptal.clone();
        async move {
            arac.calistir_ortamli(&args, &Hazirlik::default(), iptal, ortam)
                .await
        }
    });
    // Gönderim onaylanır; bütçe talebi cevapsız kalır.
    match rx.recv().await {
        Some(OrtamIstegi::Gonderim { yanit, .. }) => {
            let _ = yanit.send(GonderimYaniti::Gonder);
        }
        _ => panic!("gönderim talebi bekleniyordu"),
    }
    let talep = tokio::time::timeout(Duration::from_secs(5), butce_rx.recv())
        .await
        .expect("bütçe talebi gelmeli")
        .expect("talep kanalı açık olmalı");
    iptal.cancel();
    // Eklenti iptali `iptal-edildi` olarak görür; ağa çıkış yoktur.
    match gorev.await.unwrap() {
        Ok(cikti) => assert_eq!(cikti.icerik, "1: iptal-edildi"),
        Err(hata) => panic!("beklenmeyen sonuç: {hata}"),
    }
    // Bekleyiş düştüğü için talep kapanmış olmalı; geç gelen izin kabul edilmez.
    assert!(talep.yanit.is_closed());
    assert!(talep.yanit.send(Some(ButceIslemId(1))).is_err());
    assert!(sahte.ek_istekleri().is_empty());
}
