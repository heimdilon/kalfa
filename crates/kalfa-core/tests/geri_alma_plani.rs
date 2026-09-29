//! Geri almanın planı, mührü ve yürütmesi (aşama 3 tasarımı §5.6, §5.7,
//! §5.9, §5.2'deki depo sınırı).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use kalfa_core::arac::{Arac, DegisiklikTuru, DosyaDegisiklikleri};
use kalfa_core::gecmis::{GorunumOgesi, OturumDurumu, gorunum_uret, oturum_ozeti};
use kalfa_core::geri_alma::*;
use kalfa_core::izin::{KORUMALI_UYARISI, Kip, YAPILANDIRMA_UYARISI};
use kalfa_core::kayit::{KayitOgesi, OturumKaydi};
use kalfa_core::kimlik::{CagriId, TurId};
use kalfa_core::muhur::{OnayMuhru, ozet_hex};
use kalfa_core::oturum::{OturumAyarlari, OturumBaglami, devam_et};
use kalfa_core::protokol::{AracDurumu, BitisNedeni, Komut, Olay, SEMA_SURUMU, Zarf};
use kalfa_core::test_destek::*;
use kalfa_core::tur_ozeti::KomutOzeti;
use kalfa_core::yol::CalismaYolu;
use kalfa_evren::tipler::Role;
use serde_json::json;

fn y(s: &str) -> CalismaYolu {
    CalismaYolu::coz(s).unwrap()
}

fn saklandi(icerik: &[u8]) -> OncekiHal {
    OncekiHal::Saklandi {
        ozet: ozet_hex(icerik),
    }
}

fn yedek(yol: &str, onceki: OncekiHal, sonraki: &[u8]) -> DosyaYedegi {
    DosyaYedegi {
        yol: y(yol),
        onceki,
        sonraki: Some(ozet_hex(sonraki)),
        olusan_dizinler: Vec::new(),
        eklenen: 1,
        silinen: 1,
        diff: Some(format!(
            "--- a/{yol}\n+++ b/{yol}\n@@ -1 +1 @@\n-eski\n+yeni"
        )),
    }
}

fn cagri(dizin: &mut GeriAlmaDizini, tur: TurId, durum: AracDurumu, yedekler: Vec<DosyaYedegi>) {
    let c = CagriId::yeni();
    dizin.arac_basladi(tur, c, yedekler);
    if durum != AracDurumu::SonucuBilinmiyor {
        dizin.arac_bitti(c, durum);
    }
}

struct Plan {
    _d: tempfile::TempDir,
    depo: OncekiHalDeposu,
    dosyalar: Arc<SahteGeriYukleyici>,
    dizin: GeriAlmaDizini,
}

impl Plan {
    fn yeni() -> Self {
        let d = tempfile::tempdir().unwrap();
        let depo = OncekiHalDeposu::ac(&d.path().join("depo")).unwrap();
        Self {
            _d: d,
            depo,
            dosyalar: SahteGeriYukleyici::yeni(),
            dizin: GeriAlmaDizini::default(),
        }
    }

    fn sakla(&self, icerik: &[u8]) {
        self.depo.sakla(&ozet_hex(icerik), icerik).unwrap();
    }

    fn cikar(&self, tur: TurId) -> GeriAlmaPlani {
        plan_cikar(
            tur,
            &self.dizin,
            &self.depo,
            self.dosyalar.as_ref(),
            &sanal_siniflandirici(),
        )
        .expect("tur dizinde")
    }
}

fn eylem<'a>(plan: &'a GeriAlmaPlani, yol: &str) -> &'a PlanOgesi {
    plan.ogeler
        .iter()
        .find(|o| o.yol.as_str() == yol)
        .unwrap_or_else(|| panic!("{yol} planda yok: {plan:?}"))
}

#[test]
fn plan_tablosu() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "birinci tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![
            yedek("a.txt", saklandi(b"a0"), b"a1"),
            yedek("b.txt", OncekiHal::Yoktu, b"b1"),
            yedek("c.txt", saklandi(b"c0"), b"c1"),
            yedek(
                "d.bin",
                OncekiHal::Buyuk {
                    ozet: ozet_hex(b"d0"),
                    bayt: 9,
                },
                b"d1",
            ),
            yedek("e.txt", saklandi(b"e0"), b"e1"),
            yedek("f.txt", saklandi(b"f0"), b"f1"),
        ],
    );
    // g.txt: A → B çağrısı başarısız oldu, sonra A → C yazıldı.
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarisiz,
        vec![yedek("g.txt", saklandi(b"g0"), b"gB")],
    );
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("g.txt", saklandi(b"g0"), b"gC")],
    );
    for icerik in [&b"a0"[..], b"c0", b"e0", b"f0", b"g0"] {
        p.sakla(icerik);
    }
    let fs = &p.dosyalar;
    fs.yaz("a.txt", b"a1");
    fs.yaz("b.txt", b"b1");
    fs.yaz("c.txt", b"c0");
    fs.yaz("d.bin", b"d1");
    fs.yaz("e.txt", b"kullanici");
    fs.yaz("f.txt", b"f1");
    fs.baglanti_ekle("f.txt");
    // Kullanıcı modelin başarısız olan değişikliğini kendisi yaptı.
    fs.yaz("g.txt", b"gB");

    let plan = p.cikar(t);
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"a0")
        }
    );
    assert_eq!(eylem(&plan, "b.txt").eylem, GeriAlmaEylemi::Sil);
    assert_eq!(eylem(&plan, "c.txt").eylem, GeriAlmaEylemi::ZatenOnceki);
    assert_eq!(
        eylem(&plan, "d.bin").eylem,
        GeriAlmaEylemi::GeriAlinamaz {
            neden: GeriAlinamazNedeni::Buyuk
        }
    );
    assert_eq!(
        eylem(&plan, "e.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::Elle
        }
    );
    assert_eq!(
        eylem(&plan, "f.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::HardLink
        }
    );
    assert_eq!(
        eylem(&plan, "g.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::BasarisizCagri
        },
        "başarısız çağrının hâli Kalfa'nın yazdığı sayılmaz"
    );
    assert!(plan.yapilacak_var_mi());
    assert_eq!(eylem(&plan, "a.txt").simdiki, Some(ozet_hex(b"a1")));
    assert_eq!(
        eylem(&plan, "a.txt").diff.as_deref(),
        Some("--- b/a.txt\n+++ a/a.txt\n@@ -1 +1 @@\n+eski\n-yeni"),
        "geri alma yönündeki diff"
    );
    assert_eq!(eylem(&plan, "e.txt").diff, None, "çakışmada diff yok");
}

#[test]
fn verisi_silinen_tur_geri_alinamaz_ama_olusturdugu_dosya_silinir() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![
            yedek("a.txt", saklandi(b"a0"), b"a1"),
            yedek("yeni.txt", OncekiHal::Yoktu, b"n"),
        ],
    );
    p.sakla(b"a0");
    p.dizin.veri_silindi(&[t]);
    p.dosyalar.yaz("a.txt", b"a1");
    p.dosyalar.yaz("yeni.txt", b"n");
    let plan = p.cikar(t);
    assert!(plan.veri_silindi);
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::GeriAlinamaz {
            neden: GeriAlinamazNedeni::VeriSilindi
        }
    );
    assert_eq!(eylem(&plan, "yeni.txt").eylem, GeriAlmaEylemi::Sil);
}

#[test]
fn ara_hal_de_geri_alinir() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"B"), b"C")],
    );
    p.sakla(b"A");
    p.sakla(b"B");
    p.dosyalar.yaz("a.txt", b"B");
    assert_eq!(
        eylem(&p.cikar(t), "a.txt").eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"A")
        }
    );
}

#[test]
fn sonucu_bilinmeyen_cagrinin_hali_uyariyla_geri_alinir() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::SonucuBilinmiyor,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    p.sakla(b"A");
    p.dosyalar.yaz("a.txt", b"B");
    let plan = p.cikar(t);
    let o = eylem(&plan, "a.txt");
    assert_eq!(
        o.eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"A")
        }
    );
    assert!(
        o.uyarilar.iter().any(|u| u == BILINMEYEN_YAZMA_UYARISI),
        "{:?}",
        o.uyarilar
    );
}

#[test]
fn iptal_edilen_cagri_onceki_hali_belirlemez() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::IptalEdildi,
        vec![yedek("a.txt", saklandi(b"eskimis"), b"X")],
    );
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    p.sakla(b"A");
    p.dosyalar.yaz("a.txt", b"B");
    assert_eq!(
        eylem(&p.cikar(t), "a.txt").eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"A")
        }
    );
    // İptal edilen çağrının hâli de Kalfa'nın yazdığı sayılmaz.
    p.dosyalar.yaz("a.txt", b"X");
    assert_eq!(
        eylem(&p.cikar(t), "a.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::Elle
        }
    );
}

#[test]
fn sonraki_turla_cakisma_nedeni() {
    let mut p = Plan::yeni();
    let (t1, t2) = (TurId::yeni(), TurId::yeni());
    p.dizin.tur_basladi(t1, "bir");
    cagri(
        &mut p.dizin,
        t1,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    p.dizin.tur_basladi(t2, "iki");
    cagri(
        &mut p.dizin,
        t2,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"B"), b"C")],
    );
    p.sakla(b"A");
    p.sakla(b"B");
    p.dosyalar.yaz("a.txt", b"C");
    assert_eq!(
        eylem(&p.cikar(t1), "a.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::SonrakiTur { tur: t2 }
        }
    );
    assert_eq!(
        eylem(&p.cikar(t2), "a.txt").eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"B")
        }
    );
}

fn komut(yollar: &[&str]) -> KomutOzeti {
    KomutOzeti {
        cagri: CagriId::yeni(),
        komut: "make".into(),
        durum: AracDurumu::Basarili,
        degisiklikler: Some(DosyaDegisiklikleri::olustur(
            yollar
                .iter()
                .map(|y| ((*y).to_string(), DegisiklikTuru::Degisti)),
            false,
        )),
    }
}

/// Aşama 3 tasarımı §5.7: bir komutun değiştirdiği dosya çakışmadır ve
/// nedeni söylenir; komut planda salt liste olarak görünür.
#[test]
fn komutun_degistirdigi_dosya_cakisma_nedeni() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "bir");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    p.dizin.komut_degisti(t, komut(&["a.txt", "b.txt"]));
    // Dosya değiştirmeyen komut kaydedilmez.
    p.dizin.komut_degisti(t, komut(&[]));
    p.sakla(b"A");
    p.dosyalar.yaz("a.txt", b"komutun yazdigi");
    let plan = p.cikar(t);
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::Komut
        }
    );
    assert_eq!(plan.komutlar.len(), 1);
    assert_eq!(plan.komutlar[0].komut, "make");
    // Komutlar mühre girmez; dosya aynıyken mühür aynıdır.
    let mut komutsuz = plan.clone();
    komutsuz.komutlar.clear();
    assert_eq!(plan_muhru(t, &plan), plan_muhru(t, &komutsuz));
}

#[test]
fn korumali_yapilandirma_ve_gizli_dosyalar_isaretlenir() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    let mut gizli = yedek(".env", saklandi(b"S=1"), b"S=2");
    gizli.diff = None;
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![
            yedek(".git/config", saklandi(b"g0"), b"g1"),
            yedek(".vscode/settings.json", saklandi(b"v0"), b"v1"),
            gizli,
        ],
    );
    for icerik in [&b"g0"[..], b"v0", b"S=1"] {
        p.sakla(icerik);
    }
    p.dosyalar.yaz(".git/config", b"g1");
    p.dosyalar.yaz(".vscode/settings.json", b"v1");
    p.dosyalar.yaz(".env", b"S=2");
    let plan = p.cikar(t);
    assert!(
        eylem(&plan, ".git/config")
            .uyarilar
            .contains(&KORUMALI_UYARISI.to_string())
    );
    assert!(
        eylem(&plan, ".vscode/settings.json")
            .uyarilar
            .contains(&YAPILANDIRMA_UYARISI.to_string())
    );
    let env = eylem(&plan, ".env");
    assert!(env.uyarilar.contains(&GIZLI_GERI_ALMA_UYARISI.to_string()));
    assert_eq!(env.diff, None);
    assert!(matches!(env.eylem, GeriAlmaEylemi::GeriYukle { .. }));
}

#[test]
fn muhur_plana_ve_dosyalarin_simdiki_haline_baglidir() {
    let mut p = Plan::yeni();
    let t = TurId::yeni();
    p.dizin.tur_basladi(t, "tur");
    cagri(
        &mut p.dizin,
        t,
        AracDurumu::Basarili,
        vec![yedek("a.txt", saklandi(b"A"), b"B")],
    );
    p.sakla(b"A");
    p.dosyalar.yaz("a.txt", b"B");
    let m1 = plan_muhru(t, &p.cikar(t));
    assert_eq!(m1, plan_muhru(t, &p.cikar(t)), "deterministik");
    assert_ne!(m1, plan_muhru(TurId::yeni(), &p.cikar(t)), "tura bağlı");
    p.dosyalar.yaz("a.txt", b"kullanici");
    assert_ne!(
        m1,
        plan_muhru(t, &p.cikar(t)),
        "dosya değişince mühür değişir"
    );
}

#[test]
fn bilinmeyen_tur_planlanmaz() {
    let p = Plan::yeni();
    assert!(
        plan_cikar(
            TurId::yeni(),
            &p.dizin,
            &p.depo,
            p.dosyalar.as_ref(),
            &sanal_siniflandirici()
        )
        .is_none()
    );
}

#[test]
fn diff_ters_cevrilir() {
    let d = "--- a/x\n+++ b/x\n@@ -1,3 +1,4 @@ fn x\n bir\n-iki\n+ikiA\n+ikiB\n uc";
    assert_eq!(
        ters_diff(d),
        "--- b/x\n+++ a/x\n@@ -1,4 +1,3 @@ fn x\n bir\n+iki\n-ikiA\n-ikiB\n uc"
    );
    assert_eq!(ters_diff(&ters_diff(d)), d);
}

#[test]
fn kalfa_notu_sonuclari_anlatir() {
    let s = |yol: &str, eylem: GeriAlmaEylemi, hata: Option<&str>| GeriAlmaSonucu {
        yol: y(yol),
        eylem,
        hata: hata.map(str::to_owned),
    };
    let metin = kalfa_notu(
        "README'yi güncelle",
        &[
            s("a.rs", GeriAlmaEylemi::GeriYukle { ozet: "x".into() }, None),
            s("b.rs", GeriAlmaEylemi::Sil, None),
            s(
                "c.rs",
                GeriAlmaEylemi::Cakisma {
                    neden: CakismaNedeni::Elle,
                },
                None,
            ),
            s(
                "d.rs",
                GeriAlmaEylemi::GeriYukle { ozet: "y".into() },
                Some("yazılamadı"),
            ),
            s("e.rs", GeriAlmaEylemi::ZatenOnceki, None),
        ],
        false,
    );
    assert_eq!(
        metin,
        "[Kalfa] Kullanıcı \"README'yi güncelle\" turunun dosya değişikliklerini geri aldı.\n\
         Önceki hâline dönen: a.rs.\n\
         Silinen (turda oluşturulmuştu): b.rs.\n\
         Geri alınmayan (sonradan değişmiş ya da geri alınamaz): c.rs, d.rs."
    );
    // Turda dosya değiştiren bir komut varsa not bunu da söyler.
    let kabuklu = kalfa_notu("t", &[s("a.rs", GeriAlmaEylemi::Sil, None)], true);
    assert!(
        kabuklu.ends_with("\nKabukla yapılan değişiklikler geri alınmadı."),
        "{kabuklu}"
    );
}

// --- Oturum üzerinden ---------------------------------------------------

struct Ortam {
    d: tempfile::TempDir,
    dosyalar: Arc<SahteGeriYukleyici>,
}

impl Ortam {
    fn yeni() -> Self {
        Self {
            d: tempfile::tempdir().unwrap(),
            dosyalar: SahteGeriYukleyici::yeni(),
        }
    }

    fn kayit(&self) -> PathBuf {
        self.d.path().join("kayitlar/oturum.jsonl")
    }

    fn depo_dizini(&self) -> PathBuf {
        self.d.path().join("geri-alma/oturum")
    }

    fn ayarlar(&self, kip: Kip) -> OturumAyarlari {
        let mut a = OturumAyarlari::yeni("glm-5.3", Path::new("calisma"));
        a.kip = kip;
        a.kayit_yolu = Some(self.kayit());
        a.geri_alma_dizini = Some(self.depo_dizini());
        a
    }

    fn baglam(&self, s: Arc<SahteSaglayici>) -> OturumBaglami {
        let yaz = Arc::new(SahteArac::yazan("yaz").dosyalar(self.dosyalar.clone()));
        let mut baglam = test_baglami(s, vec![yaz as Arc<dyn Arac>], sanal_siniflandirici());
        baglam.geri_yukleyici = Some(self.dosyalar.clone());
        baglam
    }

    fn surucu(&self, s: Arc<SahteSaglayici>, ayarlar: OturumAyarlari) -> Surucu {
        test_oturumu(self.baglam(s), ayarlar)
    }

    fn kayitlar(&self) -> Vec<Zarf<KayitOgesi>> {
        OturumKaydi::oku(&self.kayit()).unwrap()
    }
}

fn mesaj(m: &str) -> Komut {
    Komut::KullaniciMesaji { metin: m.into() }
}

fn yaz(yol: &str, icerik: &str) -> (&'static str, serde_json::Value) {
    ("yaz", json!({"yol": yol, "icerik": icerik}))
}

fn turlar(d: &Surucu) -> Vec<TurId> {
    d.gecmis
        .iter()
        .filter(|z| matches!(z.yuk, Olay::TurBasladi))
        .filter_map(|z| z.tur)
        .collect()
}

async fn onizle(d: &mut Surucu, tur: TurId) -> (GeriAlmaPlani, OnayMuhru) {
    d.gonder(Komut::GeriAlmaOnizle { tur }).await;
    match d
        .bekle(|o| matches!(o, Olay::GeriAlmaPlani { .. } | Olay::Hata { .. }))
        .await
        .yuk
    {
        Olay::GeriAlmaPlani {
            geri_alinan_tur,
            plan,
            muhur,
        } => {
            assert_eq!(geri_alinan_tur, tur);
            (plan, muhur)
        }
        diger => panic!("plan yerine: {diger:?}"),
    }
}

async fn geri_al(d: &mut Surucu, tur: TurId, muhur: OnayMuhru) -> Vec<GeriAlmaSonucu> {
    d.gonder(Komut::TuruGeriAl { tur, muhur }).await;
    match d
        .bekle(|o| matches!(o, Olay::TurGeriAlindi { .. } | Olay::Hata { .. }))
        .await
        .yuk
    {
        Olay::TurGeriAlindi { sonuc, .. } => sonuc,
        diger => panic!("geri alma yerine: {diger:?}"),
    }
}

#[tokio::test]
async fn tur_onizlenip_muhurle_geri_alinir() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"v0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1"), yaz("yeni/b.txt", "b")]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, o.ayarlar(Kip::Proje));
    d.gonder(mesaj("iki dosyaya yaz")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"v1");
    let tur = turlar(&d)[0];

    let (plan, muhur) = onizle(&mut d, tur).await;
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::GeriYukle {
            ozet: ozet_hex(b"v0")
        }
    );
    assert_eq!(eylem(&plan, "yeni/b.txt").eylem, GeriAlmaEylemi::Sil);
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"v1", "önizleme yazmaz");

    let sonuc = geri_al(&mut d, tur, muhur).await;
    assert!(sonuc.iter().all(|s| s.hata.is_none()), "{sonuc:?}");
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"v0");
    assert_eq!(o.dosyalar.oku("yeni/b.txt"), None);
    d.kapat().await;

    let kayit = o.kayitlar();
    assert!(kayit.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::GeriAlmaBasladi { geri_alinan_tur, .. } if *geri_alinan_tur == tur
    )));
    assert!(kayit.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::GeriAlindi { geri_alinan_tur, .. } if *geri_alinan_tur == tur
    )));
    let gorunum = gorunum_uret(&kayit);
    assert!(gorunum.iter().any(|g| matches!(
        g,
        GorunumOgesi::GeriAlma { geri_alinan_tur, yarim: false, sonuc, .. }
            if *geri_alinan_tur == tur && sonuc.len() == 2
    )));
}

#[tokio::test]
async fn onizlemeden_sonra_degisen_dosya_geri_almayi_durdurur() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"v0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1")]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, o.ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let tur = turlar(&d)[0];
    let (_, muhur) = onizle(&mut d, tur).await;
    o.dosyalar.yaz("a.txt", b"kullanici");

    d.gonder(Komut::TuruGeriAl { tur, muhur }).await;
    let yeni = match d
        .bekle(|o| matches!(o, Olay::GeriAlmaPlani { .. }))
        .await
        .yuk
    {
        Olay::GeriAlmaPlani { plan, .. } => plan,
        _ => unreachable!(),
    };
    assert_eq!(
        eylem(&yeni, "a.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::Elle
        }
    );
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("önizlemeden sonra değişti")))
        .await;
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"kullanici");
    d.kapat().await;
    assert!(
        !o.kayitlar()
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::GeriAlmaBasladi { .. })),
        "mühür tutmayınca başladı kaydı da yazılmaz"
    );
}

#[tokio::test]
async fn tur_surerken_geri_alma_reddedilir() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![SahteYanit::araclar(vec![yaz("a.txt", "v1")])]);
    let mut d = o.surucu(s, o.ayarlar(Kip::Siki));
    d.gonder(mesaj("yaz")).await;
    d.bekle(|o| matches!(o, Olay::OnayGerekiyor { .. })).await;
    d.gonder(Komut::GeriAlmaOnizle { tur: TurId::yeni() }).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("Önceki tur sürüyor")))
        .await;
    let tur = d.tutamac.aktif_tur().unwrap();
    d.gonder(Komut::Iptal { tur }).await;
    d.tur_sonu().await;
}

#[tokio::test]
async fn yapilacak_bir_sey_yoksa_geri_alma_yazmaz() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"v0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1")]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, o.ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let tur = turlar(&d)[0];
    o.dosyalar.yaz("a.txt", b"v0");
    let (plan, muhur) = onizle(&mut d, tur).await;
    assert!(!plan.yapilacak_var_mi());
    d.gonder(Komut::TuruGeriAl { tur, muhur }).await;
    d.bekle(
        |o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("Geri alınacak bir değişiklik yok")),
    )
    .await;
    d.kapat().await;
    assert!(
        !o.kayitlar()
            .iter()
            .any(|z| matches!(z.yuk, KayitOgesi::GeriAlmaBasladi { .. }))
    );
}

#[tokio::test]
async fn kayitsiz_oturumda_geri_alma_yok() {
    let o = Ortam::yeni();
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1")]),
        SahteYanit::metin("tamam"),
    ]);
    let mut ayarlar = o.ayarlar(Kip::Proje);
    ayarlar.kayit_yolu = None;
    let mut d = o.surucu(s, ayarlar);
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let tur = turlar(&d)[0];
    d.gonder(Komut::GeriAlmaOnizle { tur }).await;
    d.bekle(|o| matches!(o, Olay::Hata { mesaj } if mesaj.contains("kayıtsız oturum")))
        .await;
}

#[tokio::test]
async fn geri_alma_sonrasi_kalfa_notu_baglamda_ve_kayitta() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"v0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1")]),
        SahteYanit::metin("tamam"),
        SahteYanit::metin("anlaşıldı"),
    ]);
    let mut d = o.surucu(s.clone(), o.ayarlar(Kip::Proje));
    d.gonder(mesaj("a.txt'yi değiştir")).await;
    d.tur_sonu().await;
    let tur = turlar(&d)[0];
    let (_, muhur) = onizle(&mut d, tur).await;
    geri_al(&mut d, tur, muhur).await;
    d.gonder(mesaj("devam et")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    d.kapat().await;

    // Not, yeni mesajdan hemen önce kullanıcı rolünde bağlamdadır.
    let istek = s.istekler().last().unwrap().clone();
    let n = istek.messages.len();
    let not = &istek.messages[n - 2];
    assert_eq!(not.role, Role::User);
    let metin = not.content.clone().unwrap_or_default();
    assert!(
        metin.starts_with("[Kalfa] Kullanıcı \"a.txt'yi değiştir\" turunun"),
        "{metin}"
    );
    assert!(metin.contains("Önceki hâline dönen: a.txt."), "{metin}");
    assert_eq!(istek.messages[n - 1].content.as_deref(), Some("devam et"));

    let kayit = o.kayitlar();
    assert!(
        kayit
            .iter()
            .any(|z| matches!(&z.yuk, KayitOgesi::KalfaNotu { metin: m } if *m == metin))
    );
    let gorunum = gorunum_uret(&kayit);
    assert!(
        gorunum
            .iter()
            .any(|g| matches!(g, GorunumOgesi::KalfaNotu { .. }))
    );
    assert_eq!(
        gorunum
            .iter()
            .filter(|g| matches!(g, GorunumOgesi::KullaniciMesaji { .. }))
            .count(),
        2,
        "not kullanıcı balonu değildir"
    );
    let ozet = oturum_ozeti(&o.kayit());
    assert_eq!(ozet.baslik.as_deref(), Some("a.txt'yi değiştir"));
    assert_eq!(ozet.durum, OturumDurumu::Tamamlandi);
}

#[tokio::test]
async fn yarim_kalan_geri_alma_yeniden_istenebilir() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"a0");
    o.dosyalar.yaz("c.txt", b"c0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "a1"), yaz("c.txt", "c1")]),
        SahteYanit::metin("tamam"),
    ]);
    let mut d = o.surucu(s, o.ayarlar(Kip::Proje));
    d.gonder(mesaj("yaz")).await;
    d.tur_sonu().await;
    let tur = turlar(&d)[0];
    let (_, muhur) = onizle(&mut d, tur).await;
    d.kapat().await;

    // Geri alma a.txt'yi yazdıktan sonra Kalfa kapanmış gibi.
    o.dosyalar.yaz("a.txt", b"a0");
    let kayitlar = o.kayitlar();
    let son = kayitlar.last().unwrap();
    {
        let mut k = OturumKaydi::ac(&o.kayit()).unwrap();
        k.yaz_kalici(&Zarf {
            sema: SEMA_SURUMU,
            oturum: son.oturum,
            tur: None,
            sira: son.sira + 1,
            yuk: KayitOgesi::GeriAlmaBasladi {
                geri_alinan_tur: tur,
                muhur: muhur.clone(),
            },
        })
        .unwrap();
    }
    let gorunum = gorunum_uret(&o.kayitlar());
    assert!(gorunum.iter().any(|g| matches!(
        g,
        GorunumOgesi::GeriAlma { yarim: true, geri_alinan_tur, .. } if *geri_alinan_tur == tur
    )));

    let (t, alici, _) = devam_et(
        &o.kayit(),
        o.ayarlar(Kip::Proje),
        o.baglam(SahteSaglayici::yeni(vec![])),
    )
    .unwrap();
    let mut d = Surucu::yeni(t, alici);
    let (plan, muhur) = onizle(&mut d, tur).await;
    assert_eq!(eylem(&plan, "a.txt").eylem, GeriAlmaEylemi::ZatenOnceki);
    assert!(matches!(
        eylem(&plan, "c.txt").eylem,
        GeriAlmaEylemi::GeriYukle { .. }
    ));
    geri_al(&mut d, tur, muhur).await;
    assert_eq!(o.dosyalar.oku("c.txt").unwrap(), b"c0");
    d.kapat().await;
    let gorunum = gorunum_uret(&o.kayitlar());
    assert!(gorunum.iter().any(|g| matches!(
        g,
        GorunumOgesi::GeriAlma { yarim: false, geri_alinan_tur, .. } if *geri_alinan_tur == tur
    )));
}

fn sinirli(o: &Ortam, sinir: u64, alt: u64) -> OturumAyarlari {
    let mut a = o.ayarlar(Kip::Proje);
    a.sinirlar.geri_alma_oturum = sinir;
    a.sinirlar.geri_alma_oturum_alt = alt;
    a
}

/// Üç tur; üçüncünün sonunda depo sınırı aşılır. Silme olayı turun
/// `Tamamlandi`'sından sonra gelir.
async fn uc_tur(
    o: &Ortam,
    ayarlar: OturumAyarlari,
    birinci: Vec<(&'static str, serde_json::Value)>,
) -> (Surucu, Vec<TurId>) {
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(birinci),
        SahteYanit::metin("1"),
        SahteYanit::araclar(vec![yaz("b.txt", "b")]),
        SahteYanit::metin("2"),
        SahteYanit::araclar(vec![yaz("c.txt", "c")]),
        SahteYanit::metin("3"),
    ]);
    let mut d = o.surucu(s, ayarlar);
    for m in ["bir", "iki", "üç"] {
        d.gonder(mesaj(m)).await;
        d.tur_sonu().await;
    }
    d.bekle(|o| matches!(o, Olay::GeriAlmaVerisiSilindi { .. }))
        .await;
    let t = turlar(&d);
    (d, t)
}

fn silinen_turlar(d: &Surucu) -> Vec<TurId> {
    d.gecmis
        .iter()
        .filter_map(|z| match &z.yuk {
            Olay::GeriAlmaVerisiSilindi { turlar } => Some(turlar.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[tokio::test]
async fn oturum_siniri_asilinca_en_eski_turun_verisi_silinir_paylasilan_kalir() {
    let o = Ortam::yeni();
    // Birinci tur S'yi ve D'yi, ikinci tur yine S'yi, üçüncü tur C'yi saklar:
    // 10 + 10 + 12 = 32 > 30. Birinci turun silinmesi yalnızca D'yi serbest
    // bırakır (22 <= 25); S ikinci tura kalır.
    o.dosyalar.yaz("a.txt", &[b'S'; 10]);
    o.dosyalar.yaz("d.txt", &[b'D'; 10]);
    o.dosyalar.yaz("b.txt", &[b'S'; 10]);
    o.dosyalar.yaz("c.txt", &[b'C'; 12]);
    let (mut d, t) = uc_tur(
        &o,
        sinirli(&o, 30, 25),
        vec![yaz("a.txt", "a"), yaz("d.txt", "d")],
    )
    .await;
    assert_eq!(silinen_turlar(&d), vec![t[0]]);
    let depo = OncekiHalDeposu::ac(&o.depo_dizini()).unwrap();
    assert!(depo.var_mi(&ozet_hex(&[b'S'; 10])), "ortak içerik kalır");
    assert!(!depo.var_mi(&ozet_hex(&[b'D'; 10])));
    assert!(depo.var_mi(&ozet_hex(&[b'C'; 12])));

    let (plan, _) = onizle(&mut d, t[0]).await;
    assert!(plan.veri_silindi);
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::GeriAlinamaz {
            neden: GeriAlinamazNedeni::VeriSilindi
        }
    );
    let (plan, _) = onizle(&mut d, t[1]).await;
    assert!(matches!(
        eylem(&plan, "b.txt").eylem,
        GeriAlmaEylemi::GeriYukle { .. }
    ));
    d.kapat().await;
    assert!(o.kayitlar().iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::GeriAlmaVerisiSilindi { turlar } if *turlar == vec![t[0]]
    )));
}

#[tokio::test]
async fn sinir_asilinca_kullanilmayan_icerik_silinir() {
    let o = Ortam::yeni();
    // S birinci ve ikinci turda ortak, C üçüncüde: 10 + 12 = 22 > 20. Birinci
    // tur S'yi serbest bırakmaz; ikinci tur da silinir (12 <= 15).
    o.dosyalar.yaz("a.txt", &[b'S'; 10]);
    o.dosyalar.yaz("b.txt", &[b'S'; 10]);
    o.dosyalar.yaz("c.txt", &[b'C'; 12]);
    let (d, t) = uc_tur(&o, sinirli(&o, 20, 15), vec![yaz("a.txt", "a")]).await;
    assert_eq!(silinen_turlar(&d), vec![t[0], t[1]]);
    let depo = OncekiHalDeposu::ac(&o.depo_dizini()).unwrap();
    assert!(!depo.var_mi(&ozet_hex(&[b'S'; 10])));
    assert!(depo.var_mi(&ozet_hex(&[b'C'; 12])));
    d.kapat().await;
}

/// Kabuğun değiştirdiği dosyalar kayda girer; geri alma onları listeler ama
/// geri almaz ve Kalfa notu bunu söyler. Kayıttan açılışta da aynıdır.
#[tokio::test]
async fn kabuk_degisiklikleri_geri_alinmaz_listelenir() {
    let o = Ortam::yeni();
    o.dosyalar.yaz("a.txt", b"v0");
    let s = SahteSaglayici::yeni(vec![
        SahteYanit::araclar(vec![yaz("a.txt", "v1"), yaz("b.txt", "b")]),
        SahteYanit::arac("kabuk", json!({"yol": "."})),
        SahteYanit::metin("tamam"),
    ]);
    let yaz_arac = Arc::new(SahteArac::yazan("yaz").dosyalar(o.dosyalar.clone()));
    let kabuk = Arc::new(
        SahteArac::kabuk("kabuk")
            .dosyalar(o.dosyalar.clone())
            .degistirdigi(&["a.txt"]),
    );
    let mut baglam = test_baglami(
        s,
        vec![yaz_arac as Arc<dyn Arac>, kabuk as Arc<dyn Arac>],
        sanal_siniflandirici(),
    );
    baglam.geri_yukleyici = Some(o.dosyalar.clone());
    let mut d = test_oturumu(baglam, o.ayarlar(Kip::Serbest));
    d.gonder(mesaj("yaz ve derle")).await;
    assert_eq!(d.tur_sonu().await, BitisNedeni::Normal);
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"kabuk-kabuk");
    let tur = turlar(&d)[0];

    let (plan, muhur) = onizle(&mut d, tur).await;
    assert_eq!(
        eylem(&plan, "a.txt").eylem,
        GeriAlmaEylemi::Cakisma {
            neden: CakismaNedeni::Komut
        }
    );
    assert_eq!(eylem(&plan, "b.txt").eylem, GeriAlmaEylemi::Sil);
    assert_eq!(plan.komutlar.len(), 1);
    geri_al(&mut d, tur, muhur).await;
    assert_eq!(o.dosyalar.oku("a.txt").unwrap(), b"kabuk-kabuk");
    assert_eq!(o.dosyalar.oku("b.txt"), None);
    d.kapat().await;

    let kayit = o.kayitlar();
    assert!(kayit.iter().any(|z| matches!(
        &z.yuk,
        KayitOgesi::KabukDegisiklikleri { degisiklikler, .. } if degisiklikler.toplam == 1
    )));
    let not = kayit
        .iter()
        .find_map(|z| match &z.yuk {
            KayitOgesi::KalfaNotu { metin } => Some(metin.clone()),
            _ => None,
        })
        .unwrap();
    assert!(
        not.contains("Kabukla yapılan değişiklikler geri alınmadı."),
        "{not}"
    );
    let dizin = GeriAlmaDizini::kayittan(&kayit);
    assert_eq!(dizin.tur(tur).unwrap().komutlar.len(), 1);
    assert_eq!(
        dizin.tur(tur).unwrap().komutlar[0].durum,
        AracDurumu::Basarili
    );
}
