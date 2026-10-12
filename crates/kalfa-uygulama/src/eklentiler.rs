//! Eklentilerin uygulamadaki yeri (aşama 4 tasarımı §13, §14): kayıt
//! (`eklentiler.json`), yerleşik eklenti, klasörden kurulum ve güncelleme,
//! kaldırma, devre dışı bırakma, çalışma alanında açma, kalıcı gönderim izni
//! ve oturuma bağlama.
//!
//! Yol kuran her yerde eklenti adı (`eklenti_adi_gecerli`) ve çalışma alanı
//! kimliği (16 küçük onaltılık karakter) doğrulanır: `../x` gibi değerler
//! `<veri>/eklentiler` ya da eklenti depolarının dışına çıkamaz.

mod kayit;
mod yukleyici;

pub(crate) use yukleyici::EklentiYoneticisi;

use std::io;
use std::path::Path;
use std::sync::Arc;

use kalfa_core::gorunum::{GORUNMEZ_UYARISI, gorunmez_mi, gorunmez_var_mi, gorunur_yap};
use kalfa_core::kimlik::{IslemId, OturumId};
use kalfa_plugin::manifest::{Manifest, eklenti_adi_gecerli};

use crate::Uygulama;
use crate::hata::KomutHatasi;
use crate::sozlesme::{
    AlanHedefi, EklentiSatiri, GunlukSinirDurumu, KurulumSonucu, OturumEklentisi,
};
use kayit::{AlanKaydi, Ayar, KuruluKaydi, kimlik_gecerli, simdi_ms};
use yukleyici::{Hedef, MANIFEST_DOSYASI, WASM_DOSYASI, klasoru_oku};

const YERLESIK_KALDIRILAMAZ: &str =
    "Kalfa'yla gelen eklenti kaldırılamaz; devre dışı bırakabilirsiniz.";

fn ad_dogrula(ad: &str) -> Result<(), KomutHatasi> {
    if eklenti_adi_gecerli(ad) {
        Ok(())
    } else {
        Err(KomutHatasi::gecersiz("Eklenti adı geçersiz."))
    }
}

fn kimlik_dogrula(kimlik: &str) -> Result<(), KomutHatasi> {
    if kimlik_gecerli(kimlik) {
        Ok(())
    } else {
        Err(KomutHatasi::gecersiz("Çalışma alanı kimliği geçersiz."))
    }
}

fn ic_hata(e: impl std::fmt::Display) -> KomutHatasi {
    KomutHatasi::ic(e.to_string())
}

/// Onay pencerelerine giren manifest alanları (açıklama, `dosya-oku`
/// kalıpları) tek satırlı ve görünür olmalı. Aksi hâlde eklenti kendi
/// metniyle yerel pencerede sahte satırlar ya da paragraflar çizebilirdi
/// ("hiçbir veri göndermez" gibi). Sorunlu alanın adını döner.
fn gosterilemez_alan(m: &Manifest) -> Option<&'static str> {
    let sorunlu = |s: &str| s.chars().any(|c| c.is_control() || gorunmez_mi(c));
    if sorunlu(&m.aciklama) {
        Some("aciklama")
    } else if m.dosya_oku.iter().any(|g| sorunlu(g)) {
        Some("dosya-oku")
    } else {
        None
    }
}

fn bilinmeyen_eklenti(ad: &str) -> KomutHatasi {
    KomutHatasi::gecersiz(format!("`{ad}` adlı bir eklenti yok."))
}

/// Dizini siler; yoksa sorun değil.
fn dizin_sil(yol: &Path) -> io::Result<()> {
    match std::fs::remove_dir_all(yol) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Doğrulanmış dosyaları `<kurulu>/<ad>.yeni/`'ye yazar, eski dizin varsa
/// siler ve yenisini yerine yeniden adlandırır (§13.2). Yarıda kalırsa
/// `.yeni` dizini temizlenir.
fn kopyala(kurulu: &Path, ad: &str, manifest: &str, wasm: &[u8]) -> io::Result<()> {
    let yeni = kurulu.join(format!("{ad}.yeni"));
    dizin_sil(&yeni)?;
    std::fs::create_dir_all(&yeni)?;
    let yerlestir = || -> io::Result<()> {
        std::fs::write(yeni.join(MANIFEST_DOSYASI), manifest)?;
        std::fs::write(yeni.join(WASM_DOSYASI), wasm)?;
        let hedef = kurulu.join(ad);
        dizin_sil(&hedef)?;
        std::fs::rename(&yeni, &hedef)
    };
    yerlestir().inspect_err(|_| {
        let _ = dizin_sil(&yeni);
    })
}

/// Aynı adlı eklentinin yerine geçen kurulumun onay penceresindeki bilgisi.
struct Guncelleme {
    eski_surum: String,
    /// Kurulu olandan daha eski bir sürüm kuruluyor.
    eskiye_donus: bool,
    /// Eskisinde olmayan yetenekler; önceki manifest okunamadıysa `None`.
    yeni_yetenekler: Option<Vec<String>>,
    /// Gönderim kapsamı değişti ya da bilinemiyor: izinler silinecek.
    izinler_silinecek: bool,
}

/// Yerel kurulum penceresinin metni (§13.2). Rust'ta üretilir, arayüzden
/// gelmez; görünmez karakterler işaretlenir (güvenlik analizi Bulgu 7).
fn kurulum_metni(
    m: &Manifest,
    sha256: &str,
    kaynak: &Path,
    araclar: &[String],
    guncelleme: Option<&Guncelleme>,
) -> String {
    let surum = match guncelleme {
        Some(g) => format!(
            "{} → {}{}",
            g.eski_surum,
            m.surum,
            if g.eskiye_donus {
                " (eski sürüm)"
            } else {
                ""
            }
        ),
        None => m.surum.to_string(),
    };
    let mut metin = format!(
        "Ad: {}\nSürüm: {surum}\nAçıklama: {}\nSHA-256: {sha256}\nKaynak klasör: {}\n\n{}\n\nAraçlar: {}",
        m.ad,
        m.aciklama,
        kaynak.display(),
        m.yetenek_cumlesi(),
        araclar.join(", ")
    );
    if let Some(g) = guncelleme {
        metin.push_str("\n\n");
        match &g.yeni_yetenekler {
            Some(yeni) if !yeni.is_empty() => {
                metin.push_str("Yeni yetenekler:");
                for y in yeni {
                    metin.push_str("\n- ");
                    metin.push_str(y);
                }
            }
            Some(_) => metin.push_str("Yeni yetenek yok."),
            None => metin
                .push_str("Önceki sürümün manifesti okunamadı; bütün yetenekler yeni sayılıyor."),
        }
        if g.izinler_silinecek {
            metin.push_str("\n\nBütün çalışma alanlarındaki EVREN gönderim izinleri silinecek.");
        }
    }
    if gorunmez_var_mi(&metin) {
        metin = format!("{}\n\n{GORUNMEZ_UYARISI}", gorunur_yap(&metin));
    }
    metin
}

impl Uygulama {
    /// Yerleşik ve kurulu eklentiler; yüklenemeyenler nedeniyle birlikte.
    pub async fn eklentileri_listele(&self) -> Result<Vec<EklentiSatiri>, KomutHatasi> {
        let bilinenler = self.eklentiler.bilinenler().await;
        let veri = self.eklentiler.veri_dizini.clone();
        tokio::task::spawn_blocking(move || {
            bilinenler.into_iter().map(|b| b.satir(&veri)).collect()
        })
        .await
        .map_err(ic_hata)
    }

    /// Yerel klasör seçici, doğrulama, yerel onay penceresi, kopyalama
    /// (§13.2).
    pub async fn eklenti_klasorden_kur(&self) -> Result<KurulumSonucu, KomutHatasi> {
        let _islem = self.eklentiler.islem_kilidi.lock().await;
        let Some(klasor) = self
            .iletisim
            .klasor_sec("Eklenti klasörünü seçin", None)
            .await
        else {
            return Ok(KurulumSonucu::Vazgecildi);
        };
        let basarisiz = |mesaj: String| Ok(KurulumSonucu::Basarisiz { mesaj });

        let okunacak = klasor.clone();
        let dosyalar = match tokio::task::spawn_blocking(move || klasoru_oku(&okunacak))
            .await
            .map_err(ic_hata)?
        {
            Ok(d) => d,
            Err(mesaj) => return basarisiz(mesaj),
        };
        // Ad ve yerleşik çakışması derlemeden önce görülür.
        let manifest = match Manifest::coz(&dosyalar.manifest) {
            Ok(m) => m,
            Err(h) => return basarisiz(h.to_string()),
        };
        if self.eklentiler.yerlesik_adlari().contains(&manifest.ad) {
            return basarisiz(format!(
                "Bu eklenti Kalfa'yla gelen bir eklentiyle aynı adı taşıyor (`{}`). Manifestteki `ad` değerini değiştirip yeniden deneyin.",
                manifest.ad
            ));
        }
        if let Some(alan) = gosterilemez_alan(&manifest) {
            return basarisiz(format!(
                "kalfa.toml'daki `{alan}` değerinde satır sonu ya da görünmez karakter var. Onay pencerelerinde yanıltıcı olabileceği için bu karakterleri kaldırıp yeniden deneyin."
            ));
        }
        let motor = match self.eklentiler.motor() {
            Ok(m) => m,
            Err(mesaj) => return basarisiz(mesaj),
        };
        let eklenti = match motor.yukle(&dosyalar.manifest, dosyalar.wasm.clone()).await {
            Ok(e) => Arc::new(e),
            Err(h) => return basarisiz(h.to_string()),
        };
        let m = eklenti.manifest.clone();

        // Güncellemede eski sürümün manifesti kapsam karşılaştırması içindir.
        let eski_surum = self
            .eklentiler
            .kayit
            .bak(|k| k.kurulu(&m.ad).map(|e| e.surum.clone()));
        let eski_manifest = eski_surum.as_ref().and_then(|_| {
            std::fs::read_to_string(
                self.eklentiler
                    .kurulu_dizin
                    .join(&m.ad)
                    .join(MANIFEST_DOSYASI),
            )
            .ok()
            .and_then(|t| Manifest::coz(&t).ok())
        });
        // Eski manifest okunamıyorsa kapsamın değişmediği bilinemez: izinler
        // silinir (güvenli yön).
        let kapsam_degisti = eski_surum.is_some()
            && eski_manifest
                .as_ref()
                .is_none_or(|e| m.gonderim_kapsami_degisti(e));
        let guncelleme = eski_surum.map(|eski_surum| Guncelleme {
            eski_surum,
            eskiye_donus: eski_manifest.as_ref().is_some_and(|e| m.surum < e.surum),
            yeni_yetenekler: eski_manifest.as_ref().map(|e| m.yeni_yetenekler(e)),
            izinler_silinecek: kapsam_degisti,
        });

        let araclar: Vec<String> = eklenti.araclar.iter().map(|a| m.model_adi(&a.ad)).collect();
        let metin = kurulum_metni(&m, &eklenti.ozet, &klasor, &araclar, guncelleme.as_ref());
        let (baslik, tamam) = if guncelleme.is_some() {
            ("Eklentiyi güncelle", "Güncelle")
        } else {
            ("Eklentiyi kur", "Kur")
        };
        if !self.iletisim.onayla(baslik, &metin, tamam, "Vazgeç").await {
            return Ok(KurulumSonucu::Vazgecildi);
        }

        // Kopyalanan, doğrulanan baytların aynısıdır.
        let (ad, surum, sha256) = (m.ad.clone(), m.surum.to_string(), eklenti.ozet.clone());
        let kurulu_dizin = self.eklentiler.kurulu_dizin.clone();
        let (manifest_metni, wasm) = (dosyalar.manifest, dosyalar.wasm);
        let yazilacak = (kurulu_dizin.clone(), ad.clone(), manifest_metni.clone());
        tokio::task::spawn_blocking(move || {
            kopyala(&yazilacak.0, &yazilacak.1, &yazilacak.2, &wasm)
        })
        .await
        .map_err(ic_hata)??;

        let kaynak = klasor.display().to_string();
        let sonuc = self.eklentiler.kayit.degistir(|k| {
            // Aynı adlı bozuk girdi yeni kayıtla değiştirilir.
            k.bozuk.retain(|b| b.ad.as_deref() != Some(ad.as_str()));
            let simdi = simdi_ms();
            if let Some(e) = k.kurulu_mut(&ad) {
                e.surum = surum.clone();
                e.sha256 = sha256.clone();
                e.kaynak = kaynak;
                e.kurulum_ms = simdi;
                if kapsam_degisti {
                    for alan in e.ayar.calisma_alanlari.values_mut() {
                        alan.gonderim_izni_ms = None;
                    }
                }
                return Ok(true);
            }
            k.kurulu.push(KuruluKaydi {
                ad: ad.clone(),
                surum: surum.clone(),
                sha256: sha256.clone(),
                kaynak,
                kurulum_ms: simdi,
                ayar: Ayar::default(),
            });
            Ok(false)
        });
        let guncellendi = match sonuc {
            Ok(g) => g,
            Err(hata) => {
                // Yeni kurulumun kayıtsız dosyaları bırakılmaz. Güncellemede
                // eski dizin gitti; sha uyuşmadığı için eklenti "yeniden
                // kurun" iletisiyle yüklenemez, ayarları kalır.
                if guncelleme.is_none() {
                    let _ = dizin_sil(&kurulu_dizin.join(&ad));
                }
                return Err(hata);
            }
        };
        self.eklentiler
            .onbellege_koy(&ad, &sha256, &manifest_metni, eklenti);
        Ok(KurulumSonucu::Kuruldu {
            ad,
            surum,
            guncelleme: guncellendi,
            izinler_silindi: guncellendi && kapsam_degisti,
        })
    }

    /// Kurulu eklentiyi ve bütün depolarını siler; yerel pencerede onaylanır
    /// (§13.3). Vazgeçilirse `false`. Yerleşik eklenti kaldırılamaz.
    pub async fn eklenti_kaldir(&self, ad: String) -> Result<bool, KomutHatasi> {
        ad_dogrula(&ad)?;
        let _islem = self.eklentiler.islem_kilidi.lock().await;
        // Kurulu kayıt (bozuk ama adı okunabilen dahil) önce aranır: kaydı
        // yerleşik bir adla çakışan eski bir kurulum yine de kaldırılabilir.
        let kayitli = self.eklentiler.kayit.bak(|k| {
            k.kurulu(&ad).is_some() || k.bozuk.iter().any(|b| b.ad.as_deref() == Some(ad.as_str()))
        });
        if !kayitli {
            return Err(if self.eklentiler.yerlesik_adlari().contains(&ad) {
                KomutHatasi::gecersiz(YERLESIK_KALDIRILAMAZ)
            } else {
                bilinmeyen_eklenti(&ad)
            });
        }
        let metin = format!(
            "{ad} eklentisi ve bütün çalışma alanlarındaki verisi kalıcı olarak silinecek. Bu geri alınamaz."
        );
        if !self
            .iletisim
            .onayla("Eklentiyi kaldır", &metin, "Kaldır", "Vazgeç")
            .await
        {
            return Ok(false);
        }
        // Önce kayıttan çıkar: araçlar hemen sunulmaz olur ("süren çağrı
        // biter", §13.3). Sonra dosyalar ve depolar silinir.
        self.eklentiler.kayit.degistir(|k| {
            k.kurulu.retain(|e| e.ad != ad);
            k.bozuk.retain(|b| b.ad.as_deref() != Some(ad.as_str()));
            Ok(())
        })?;
        self.eklentiler.onbellegi_temizle(&ad);
        let kurulu = self.eklentiler.kurulu_dizin.clone();
        let veri = self.eklentiler.veri_dizini.join(&ad);
        tokio::task::spawn_blocking(move || -> io::Result<()> {
            let dosyalar = dizin_sil(&kurulu.join(&ad));
            let yarim = dizin_sil(&kurulu.join(format!("{ad}.yeni")));
            let depolar = dizin_sil(&veri);
            dosyalar.and(yarim).and(depolar)
        })
        .await
        .map_err(ic_hata)?
        .map_err(|e| {
            KomutHatasi::ic(format!(
                "Eklenti kayıttan çıkarıldı ama dosyaları ya da verisi silinemedi: {e}"
            ))
        })?;
        Ok(true)
    }

    /// Eklentiyi bütün çalışma alanlarında devre dışı bırakır ya da yeniden
    /// etkinleştirir; ayarları kalır (§13.3).
    pub async fn eklenti_etkin_ayarla(&self, ad: String, etkin: bool) -> Result<(), KomutHatasi> {
        ad_dogrula(&ad)?;
        let yerlesik = self.eklentiler.hedef_coz(&ad)? == Hedef::Yerlesik;
        self.eklentiler.kayit.degistir(|k| {
            k.ayar_mut(&ad, yerlesik)
                .ok_or_else(|| bilinmeyen_eklenti(&ad))?
                .etkin = etkin;
            Ok(())
        })
    }

    /// Eklentiyi bir çalışma alanında açar ya da kapatır (§13.3). Kapatmak
    /// gönderim iznini de kaldırır.
    pub async fn eklenti_alani_ayarla(
        &self,
        ad: String,
        hedef: AlanHedefi,
        acik: bool,
    ) -> Result<(), KomutHatasi> {
        ad_dogrula(&ad)?;
        let (kimlik, yol) = match &hedef {
            AlanHedefi::Oturum { oturum } => self
                .ic()
                .oturumlar
                .get(oturum)
                .map(|a| {
                    (
                        a.calisma_alani_kimligi.clone(),
                        Some(a.calisma_alani.display().to_string()),
                    )
                })
                .ok_or_else(KomutHatasi::oturum_yok)?,
            AlanHedefi::Kimlik { kimlik } => {
                kimlik_dogrula(kimlik)?;
                (kimlik.clone(), None)
            }
        };
        let yerlesik = self.eklentiler.hedef_coz(&ad)? == Hedef::Yerlesik;
        self.eklentiler.kayit.degistir(|k| {
            let ayar = k
                .ayar_mut(&ad, yerlesik)
                .ok_or_else(|| bilinmeyen_eklenti(&ad))?;
            let alan = match &yol {
                Some(yol) => {
                    let alan = ayar
                        .calisma_alanlari
                        .entry(kimlik.clone())
                        .or_insert_with(|| AlanKaydi {
                            yol: yol.clone(),
                            acik: false,
                            gonderim_izni_ms: None,
                        });
                    alan.yol.clone_from(yol);
                    alan
                }
                None => ayar.calisma_alanlari.get_mut(&kimlik).ok_or_else(|| {
                    KomutHatasi::gecersiz("Bu çalışma alanı eklentinin kaydında yok.")
                })?,
            };
            alan.acik = acik;
            if !acik {
                alan.gonderim_izni_ms = None;
            }
            Ok(())
        })
    }

    /// Bir çalışma alanındaki EVREN gönderim iznini kaldırır (§8.4); sonraki
    /// ilk gönderim yeniden sorulur.
    pub async fn gonderim_iznini_kaldir(
        &self,
        ad: String,
        kimlik: String,
    ) -> Result<(), KomutHatasi> {
        ad_dogrula(&ad)?;
        kimlik_dogrula(&kimlik)?;
        let yerlesik = self.eklentiler.hedef_coz(&ad)? == Hedef::Yerlesik;
        self.eklentiler.kayit.degistir(|k| {
            k.ayar_mut(&ad, yerlesik)
                .and_then(|ayar| ayar.calisma_alanlari.get_mut(&kimlik))
                .ok_or_else(|| KomutHatasi::gecersiz("Bu çalışma alanı eklentinin kaydında yok."))?
                .gonderim_izni_ms = None;
            Ok(())
        })
    }

    /// Eklentinin bir çalışma alanındaki deposunu siler; yerel pencerede
    /// onaylanır. Vazgeçilirse `false`.
    pub async fn eklenti_verisini_sil(
        &self,
        ad: String,
        kimlik: String,
    ) -> Result<bool, KomutHatasi> {
        ad_dogrula(&ad)?;
        kimlik_dogrula(&kimlik)?;
        let yerlesik = self.eklentiler.hedef_coz(&ad)? == Hedef::Yerlesik;
        let _islem = self.eklentiler.islem_kilidi.lock().await;
        let yol = self.eklentiler.kayit.bak(|k| {
            k.ayar(&ad, yerlesik)
                .and_then(|(ayar, _)| ayar.calisma_alanlari.get(&kimlik))
                .map(|a| a.yol.clone())
        });
        let mut metin = format!(
            "{ad} eklentisinin bu çalışma alanındaki verisi (ör. arama indeksi) silinecek. Sonraki kullanımda yeniden oluşturulur."
        );
        if let Some(yol) = yol {
            metin.push_str(&format!("\n\nÇalışma alanı: {}", gorunur_yap(&yol)));
        }
        if !self
            .iletisim
            .onayla("Eklenti verisini sil", &metin, "Sil", "Vazgeç")
            .await
        {
            return Ok(false);
        }
        let depo = self.eklentiler.veri_dizini.join(&ad).join(&kimlik);
        tokio::task::spawn_blocking(move || dizin_sil(&depo))
            .await
            .map_err(ic_hata)??;
        Ok(true)
    }

    /// Oturumun çalışma alanındaki eklentiler ("Bu oturumdaki izinler", §14).
    pub async fn oturum_eklentileri(
        &self,
        oturum: OturumId,
    ) -> Result<Vec<OturumEklentisi>, KomutHatasi> {
        let kimlik = self
            .ic()
            .oturumlar
            .get(&oturum)
            .map(|a| a.calisma_alani_kimligi.clone())
            .ok_or_else(KomutHatasi::oturum_yok)?;
        Ok(self
            .eklentiler
            .bilinenler()
            .await
            .into_iter()
            .map(|b| {
                let alan = b.alanlar.get(&kimlik);
                OturumEklentisi {
                    acik: alan.is_some_and(|a| a.acik),
                    gonderim_izni_ms: alan.and_then(|a| a.gonderim_izni_ms),
                    kullanilamaz: !b.etkin || b.yuklu.is_err(),
                    yerlesik: b.yerlesik,
                    ad: b.ad,
                }
            })
            .collect())
    }

    /// Önizlemede bekleyen eklenti isteğinin gövdesi ("Metni göster", §9.4).
    pub async fn eklenti_istegi(
        &self,
        oturum: OturumId,
        islem: IslemId,
    ) -> Result<String, KomutHatasi> {
        let govde = self
            .tutamac(oturum)?
            .bekleyen_eklenti_govdesi(islem)
            .ok_or_else(|| KomutHatasi::gecersiz("Bu istek artık önizlemede beklemiyor."))?;
        Ok(
            match serde_json::from_str::<serde_json::Value>(&govde)
                .and_then(|d| serde_json::to_string_pretty(&d))
            {
                Ok(guzel) => guzel,
                Err(_) => govde.to_string(),
            },
        )
    }

    /// EVREN'in günlük token sınırı dolduysa durumu; dolmadıysa `None`.
    pub fn gunluk_sinir(&self) -> Option<GunlukSinirDurumu> {
        self.evren.gunluk_sinir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "ab12";

    fn manifest(surum: &str, globlar: &str) -> Manifest {
        Manifest::coz(&format!(
            "[eklenti]\nad = \"ornek\"\nsurum = \"{surum}\"\nsozlesme = \"0.1\"\naciklama = \"Örnek eklenti\"\n\n[yetenekler]\ndosya-oku = [{globlar}]\nevren = [\"embeddings\"]\ndepo-mb = 5\n"
        ))
        .unwrap()
    }

    #[test]
    fn yeni_kurulum_metni_bilgileri_ve_yetenek_cumlesini_gosterir() {
        let m = manifest("0.1.0", "\"**/*.md\"");
        let metin = kurulum_metni(
            &m,
            SHA,
            Path::new("C:\\kaynak\\ornek"),
            &["ornek__yanki".into(), "ornek__ara".into()],
            None,
        );
        for beklenen in [
            "Ad: ornek",
            "Sürüm: 0.1.0\n",
            "Açıklama: Örnek eklenti",
            "SHA-256: ab12",
            "Kaynak klasör: C:\\kaynak\\ornek",
            &m.yetenek_cumlesi(),
            "Araçlar: ornek__yanki, ornek__ara",
        ] {
            assert!(metin.contains(beklenen), "{beklenen:?} yok:\n{metin}");
        }
        assert!(!metin.contains("Yeni yetenek"), "{metin}");
        assert!(!metin.contains("silinecek"), "{metin}");
    }

    #[test]
    fn guncelleme_metni_yeni_yetenekleri_ve_izin_silinmesini_soyler() {
        let eski = manifest("0.1.0", "\"**/*.md\"");
        let yeni = manifest("0.2.0", "\"**/*.md\", \"**/*.py\"");
        let g = Guncelleme {
            eski_surum: "0.1.0".into(),
            eskiye_donus: false,
            yeni_yetenekler: Some(yeni.yeni_yetenekler(&eski)),
            izinler_silinecek: yeni.gonderim_kapsami_degisti(&eski),
        };
        let metin = kurulum_metni(&yeni, SHA, Path::new("k"), &[], Some(&g));
        assert!(metin.contains("Sürüm: 0.1.0 → 0.2.0\n"), "{metin}");
        assert!(
            metin.contains("Yeni yetenekler:\n- `**/*.py` ile eşleşen dosyaları okuma"),
            "{metin}"
        );
        assert!(
            metin.contains("Bütün çalışma alanlarındaki EVREN gönderim izinleri silinecek."),
            "{metin}"
        );
        assert!(!metin.contains("eski sürüm"), "{metin}");
    }

    #[test]
    fn ayni_kapsamli_guncelleme_ve_eskiye_donus_metni() {
        let eski = manifest("0.2.0", "\"**/*.md\"");
        let yeni = manifest("0.1.0", "\"**/*.md\"");
        let g = Guncelleme {
            eski_surum: "0.2.0".into(),
            eskiye_donus: true,
            yeni_yetenekler: Some(yeni.yeni_yetenekler(&eski)),
            izinler_silinecek: yeni.gonderim_kapsami_degisti(&eski),
        };
        let metin = kurulum_metni(&yeni, SHA, Path::new("k"), &[], Some(&g));
        assert!(
            metin.contains("Sürüm: 0.2.0 → 0.1.0 (eski sürüm)"),
            "{metin}"
        );
        assert!(metin.contains("Yeni yetenek yok."), "{metin}");
        assert!(!metin.contains("silinecek"), "{metin}");
    }

    #[test]
    fn onceki_manifest_okunamadiysa_metin_bunu_soyler() {
        let yeni = manifest("0.2.0", "\"**/*.md\"");
        let g = Guncelleme {
            eski_surum: "0.1.0".into(),
            eskiye_donus: false,
            yeni_yetenekler: None,
            izinler_silinecek: true,
        };
        let metin = kurulum_metni(&yeni, SHA, Path::new("k"), &[], Some(&g));
        assert!(
            metin.contains("Önceki sürümün manifesti okunamadı"),
            "{metin}"
        );
        assert!(metin.contains("izinleri silinecek."), "{metin}");
    }

    #[test]
    fn gorunmez_karakterler_isaretlenir() {
        let mut m = manifest("0.1.0", "\"**/*.md\"");
        m.aciklama = "iyi\u{202e}niyet".into();
        let metin = kurulum_metni(&m, SHA, Path::new("k"), &[], None);
        assert!(metin.contains("⟨U+202E⟩"), "{metin}");
        assert!(!metin.contains('\u{202e}'), "{metin}");
        assert!(metin.contains(GORUNMEZ_UYARISI), "{metin}");
    }

    #[test]
    fn manifestte_satir_sonu_ya_da_gorunmez_karakter_gosterilemez() {
        let temiz = manifest("0.1.0", "\"**/*.md\", \"docs/ö ü.md\"");
        assert_eq!(gosterilemez_alan(&temiz), None);
        for kotu in [
            "iyi\nSHA-256: sahte",
            "a\tb",
            "a\r",
            "iyi\u{202e}",
            "a\u{200b}",
        ] {
            let mut m = temiz.clone();
            m.aciklama = kotu.into();
            assert_eq!(gosterilemez_alan(&m), Some("aciklama"), "{kotu:?}");
            let mut m = temiz.clone();
            m.dosya_oku.push(kotu.into());
            assert_eq!(gosterilemez_alan(&m), Some("dosya-oku"), "{kotu:?}");
        }
    }

    #[test]
    fn ad_ve_kimlik_dogrulamasi_yol_gecisini_reddeder() {
        for ad in [
            "../x", "a/b", "a\\b", "..", ".", "", "A", "a", "x.yeni", "a--b",
        ] {
            assert!(ad_dogrula(ad).is_err(), "{ad:?}");
        }
        assert!(ad_dogrula("kalfa-rag").is_ok());
        for kimlik in ["../x", "a/b", "0123456789abcde", "0123456789abcdeF", ""] {
            assert!(kimlik_dogrula(kimlik).is_err(), "{kimlik:?}");
        }
        assert!(kimlik_dogrula("0123456789abcdef").is_ok());
    }

    #[test]
    fn kopyala_yeni_dizini_yerine_koyar_ve_eskiyi_siler() {
        let d = tempfile::tempdir().unwrap();
        let kurulu = d.path().join("eklentiler");
        kopyala(&kurulu, "ornek", "m1", b"w1").unwrap();
        assert_eq!(
            std::fs::read(kurulu.join("ornek/kalfa.toml")).unwrap(),
            b"m1"
        );
        assert_eq!(
            std::fs::read(kurulu.join("ornek/eklenti.wasm")).unwrap(),
            b"w1"
        );
        // Önceki yarım kalmış kopya ve eski dosyalar kalmaz.
        std::fs::create_dir_all(kurulu.join("ornek.yeni")).unwrap();
        std::fs::write(kurulu.join("ornek.yeni/artik"), b"x").unwrap();
        std::fs::write(kurulu.join("ornek/eski-dosya"), b"x").unwrap();
        kopyala(&kurulu, "ornek", "m2", b"w2").unwrap();
        assert_eq!(
            std::fs::read(kurulu.join("ornek/kalfa.toml")).unwrap(),
            b"m2"
        );
        assert!(!kurulu.join("ornek/eski-dosya").exists());
        assert!(!kurulu.join("ornek.yeni").exists());
    }
}
