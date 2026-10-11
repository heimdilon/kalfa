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
use kalfa_core::gonderim::{EklentiIstekOzeti, IstekAsamasi, IstekSonucu};
use kalfa_core::kimlik::CagriId;
use kalfa_core::saglayici::Saglayici;
use kalfa_core::sart_kapisi::SartKapisi;
use kalfa_core::test_destek::{SahteSaglayici, sahte_modeller};
use kalfa_evren::{EkUc, EvrenError};
use kalfa_plugin::{BellekIzni, EvrenBaglami, GonderimIzni};
use ortak::*;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const GIZLI: &str = "GIZLI_DEGER=evren-testi-7f3a";
const GOMME: &str = r#"{"model":"qwen3-embedding-8b","input":"merhaba","dimensions":32}"#;

/// Bir `calistir_ortamli` çağrısının sonucu, gelen talepler ve ağa çıkan
/// isteklerin kayıtları.
struct Calisma {
    sonuc: Result<AracCiktisi, AracHatasi>,
    talepler: Vec<GonderimTalebi>,
    kayitlar: Vec<(EklentiIstekOzeti, IstekSonucu)>,
}

impl Calisma {
    fn icerik(&self) -> &str {
        match &self.sonuc {
            Ok(c) => &c.icerik,
            Err(e) => panic!("araç başarısız: {e}"),
        }
    }
}

/// Aracı oturumun yerine koşturur: her gönderim talebine `karar` ile yanıt
/// verir, ağa çıkan isteklerin kayıtlarını toplar.
async fn ortamli(
    arac: Arc<dyn Arac>,
    argumanlar: Value,
    hazirlik: Hazirlik,
    mut karar: impl FnMut(&GonderimTalebi) -> GonderimYaniti,
) -> Calisma {
    let (tx, mut rx) = mpsc::channel(64);
    let ortam = AracOrtami::yeni(CiktiGonderici::bos(), CagriId::yeni(), tx);
    let gorev = tokio::spawn(async move {
        arac.calistir_ortamli(&argumanlar, &hazirlik, CancellationToken::new(), ortam)
            .await
    });
    let mut talepler = Vec::new();
    let mut kayitlar = Vec::new();
    while let Some(istek) = rx.recv().await {
        match istek {
            OrtamIstegi::Gonderim { talep, yanit, .. } => {
                let k = karar(&talep);
                talepler.push(talep);
                let _ = yanit.send(k);
            }
            OrtamIstegi::IstekBitti { ozet, sonuc, .. } => kayitlar.push((ozet, sonuc)),
        }
    }
    let sonuc = gorev.await.unwrap();
    Calisma {
        sonuc,
        talepler,
        kayitlar,
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
