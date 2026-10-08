//! wasmtime motoru, eklentinin yüklenmesi ve bir aracın çağrılması (aşama 4
//! tasarımı §7).
//!
//! Uygulamada tek bir motor vardır; epoch'u bir iş parçacığı 10 ms'de bir
//! ilerletir. Her eklenti bir kez derlenir; her çağrı yeni bir `Store` ve
//! örnekle yapılır.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Weak};

use serde_json::Value;
use tokio::sync::Semaphore;
use wasmtime::component::{Component, HasSelf, Linker};
use wasmtime::{Config, Engine, Store, UpdateDeadline};

use crate::bag::kalfa::plugin::tipler::Hata;
use crate::bag::{Eklenti as Dunya, EklentiPre};
use crate::host::{CagriOrtami, Durum};
use crate::manifest::{Manifest, ManifestHatasi, arac_adi_gecerli};
use crate::sinir::{self, Sureler};

#[derive(Debug, thiserror::Error)]
pub enum YuklemeHatasi {
    #[error(transparent)]
    Manifest(#[from] ManifestHatasi),
    #[error("Eklenti çalışma zamanı başlatılamadı: {0}")]
    Motor(String),
    #[error(
        "`.wasm` bir WebAssembly bileşeni değil{0}. Eklentiyi `wasm32-wasip2` hedefi için derleyin."
    )]
    BilesenDegil(&'static str),
    #[error("Bileşen derlenemedi: {0}")]
    Derleme(String),
    #[error("{}", import_iletisi(.0))]
    Import(String),
    #[error("Bileşen Kalfa eklenti sözleşmesine (kalfa:plugin@0.1.0) uymuyor: {0}")]
    Dunya(String),
    #[error("Eklentinin araç listesi alınamadı: {0}")]
    AracListesi(String),
    #[error("Eklentinin `{arac}` aracı geçersiz: {neden}")]
    Arac { arac: String, neden: String },
}

fn import_iletisi(ad: &str) -> String {
    if ad.starts_with("wasi:sockets/") || ad.starts_with("wasi:http/") {
        format!(
            "Bu eklenti ağ erişimi (`{ad}`) istiyor. Kalfa eklentileri ağa doğrudan çıkamaz; EVREN'e yalnızca `evren-cagir` ile ulaşabilir."
        )
    } else {
        format!("Bu eklenti Kalfa'nın sunmadığı bir arayüzü (`{ad}`) istiyor; yüklenmedi.")
    }
}

/// İzinli importlar (§7.3): Kalfa'nın host'u ve WASI 0.2'nin ağsız kısmı.
/// Dosya sistemi bağlanır ama ön açık dizin yoktur.
fn izinli_import(ad: &str) -> bool {
    let (arayuz, surum) = ad.split_once('@').unwrap_or((ad, ""));
    let paket = arayuz.split('/').next().unwrap_or_default();
    match paket {
        "kalfa:plugin" => {
            surum == "0.1.0" && matches!(arayuz, "kalfa:plugin/tipler" | "kalfa:plugin/host")
        }
        "wasi:io" | "wasi:clocks" | "wasi:random" | "wasi:cli" | "wasi:filesystem" => {
            surum.starts_with("0.2.")
        }
        _ => false,
    }
}

/// Modele sunulan bir aracın tanımı.
#[derive(Debug, Clone)]
pub struct AracBilgisi {
    pub ad: String,
    pub aciklama: String,
    pub sema: Value,
}

/// Yüklenmiş, derlenmiş eklenti.
pub struct Eklenti {
    pub manifest: Arc<Manifest>,
    /// `.wasm`'ın SHA-256 özeti.
    pub ozet: String,
    pub araclar: Vec<AracBilgisi>,
    pre: EklentiPre<Durum>,
}

impl std::fmt::Debug for Eklenti {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Eklenti")
            .field("ad", &self.manifest.ad)
            .field("ozet", &self.ozet)
            .field("araclar", &self.araclar)
            .finish()
    }
}

/// Bir çağrının başarısızlığı (§7.6).
#[derive(Debug)]
pub(crate) enum CagriHatasi {
    /// Eklenti `Err(hata)` döndürdü.
    Eklenti(Hata),
    Bellek,
    Cpu,
    Sure,
    Iptal,
    /// Panik, `unreachable` ya da başka bir tuzak.
    Cokme,
}

pub struct Motor {
    engine: Engine,
    sureler: Sureler,
    eszamanli: Arc<Semaphore>,
    /// Eklenti × depo dizini başına sıra: aynı deponun iki yazarı olmaz.
    siralar: Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>,
    dur: Arc<AtomicBool>,
}

impl Drop for Motor {
    fn drop(&mut self) {
        self.dur.store(true, Ordering::SeqCst);
    }
}

impl Motor {
    pub fn yeni() -> Result<Arc<Self>, YuklemeHatasi> {
        Self::sureli(Sureler::default())
    }

    /// Kısaltılmış sürelerle motor; yalnızca testler için.
    #[cfg(feature = "test-destek")]
    pub fn test_sureleri(sureler: Sureler) -> Result<Arc<Self>, YuklemeHatasi> {
        Self::sureli(sureler)
    }

    fn sureli(sureler: Sureler) -> Result<Arc<Self>, YuklemeHatasi> {
        let mut config = Config::new();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| YuklemeHatasi::Motor(e.to_string()))?;
        let dur = Arc::new(AtomicBool::new(false));
        let (tikleyen, durdur) = (engine.clone(), dur.clone());
        std::thread::Builder::new()
            .name("kalfa-eklenti-epoch".into())
            .spawn(move || {
                while !durdur.load(Ordering::SeqCst) {
                    std::thread::sleep(sinir::TIK);
                    tikleyen.increment_epoch();
                }
            })
            .map_err(|e| YuklemeHatasi::Motor(e.to_string()))?;
        Ok(Arc::new(Self {
            engine,
            sureler,
            eszamanli: Arc::new(Semaphore::new(sinir::ESZAMANLI)),
            siralar: Mutex::default(),
            dur,
        }))
    }

    /// Bellek sınırı, epoch'la CPU bütçesi ve her tikte tokio'ya yer verme
    /// (§7.4). Bütçe yalnızca WebAssembly çalışırken tükenir; bir host
    /// çağrısında geçen süre en çok bir tik sayılır.
    fn store(&self, durum: Durum, cpu_tik: u64) -> Store<Durum> {
        let mut store = Store::new(&self.engine, durum);
        store.limiter(|d| &mut d.sinirlayici);
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(move |c| {
            let iz = &c.data().iz;
            if iz.cpu_tik.fetch_add(1, Ordering::SeqCst) + 1 > cpu_tik {
                iz.cpu.store(true, Ordering::SeqCst);
                return Err(wasmtime::format_err!("işlem süresi sınırı aşıldı"));
            }
            Ok(UpdateDeadline::YieldCustom(
                1,
                Box::pin(tokio::task::yield_now()),
            ))
        });
        store
    }

    fn sira(&self, anahtar: String) -> Arc<tokio::sync::Mutex<()>> {
        let mut siralar = self.siralar.lock().unwrap_or_else(|z| z.into_inner());
        siralar.retain(|_, s| s.strong_count() > 0);
        if let Some(s) = siralar.get(&anahtar).and_then(Weak::upgrade) {
            return s;
        }
        let s = Arc::new(tokio::sync::Mutex::new(()));
        siralar.insert(anahtar, Arc::downgrade(&s));
        s
    }

    /// Eklentiyi yükler (§7.2): manifest, bileşen, import denetimi, dünya,
    /// araç listesi.
    pub async fn yukle(&self, manifest: &str, wasm: Vec<u8>) -> Result<Eklenti, YuklemeHatasi> {
        self.yukle_ic(manifest, wasm, true).await
    }

    /// Import denetimini atlayan yükleme: ikinci katmanın (ağsız, ön açık
    /// dizinsiz WASI) testi için.
    #[cfg(feature = "test-destek")]
    pub async fn yukle_import_denetimsiz(
        &self,
        manifest: &str,
        wasm: Vec<u8>,
    ) -> Result<Eklenti, YuklemeHatasi> {
        self.yukle_ic(manifest, wasm, false).await
    }

    async fn yukle_ic(
        &self,
        manifest: &str,
        wasm: Vec<u8>,
        import_denetimi: bool,
    ) -> Result<Eklenti, YuklemeHatasi> {
        let manifest = Arc::new(Manifest::coz(manifest)?);
        let ozet = kalfa_core::muhur::ozet_hex(&wasm);
        // Bileşen sürüm 0x0d, katman 1; çekirdek modül sürüm 1, katman 0.
        if !wasm.starts_with(b"\0asm") {
            return Err(YuklemeHatasi::BilesenDegil(""));
        }
        if wasm.get(4..8) == Some(&[1, 0, 0, 0]) {
            return Err(YuklemeHatasi::BilesenDegil(" (çekirdek modül)"));
        }
        let engine = self.engine.clone();
        let component = tokio::task::spawn_blocking(move || Component::new(&engine, &wasm))
            .await
            .map_err(|e| YuklemeHatasi::Derleme(e.to_string()))?
            .map_err(|e| YuklemeHatasi::Derleme(format!("{e:#}")))?;
        if import_denetimi
            && let Some((ad, _)) = component
                .component_type()
                .imports(&self.engine)
                .find(|(ad, _)| !izinli_import(ad))
        {
            return Err(YuklemeHatasi::Import(ad.to_string()));
        }

        // İkinci katman: bütün WASI p2 bağlanır, ama `WasiCtx` ağsız ve ön
        // açık dizinsizdir (`Durum::yeni`).
        let mut linker: Linker<Durum> = Linker::new(&self.engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .map_err(|e| YuklemeHatasi::Motor(format!("{e:#}")))?;
        Dunya::add_to_linker::<Durum, HasSelf<Durum>>(&mut linker, |d| d)
            .map_err(|e| YuklemeHatasi::Motor(format!("{e:#}")))?;
        let pre = linker
            .instantiate_pre(&component)
            .and_then(EklentiPre::new)
            .map_err(|e| YuklemeHatasi::Dunya(format!("{e:#}")))?;

        let cpu_tik = Sureler {
            cpu: sinir::LISTE_SURESI,
            ..self.sureler
        }
        .cpu_tik();
        let mut store = self.store(Durum::yeni(None), cpu_tik);
        let liste = tokio::time::timeout(sinir::LISTE_SURESI, async {
            // Export'ların tipi örneklemede denetlenir: dünyaya uymayan
            // bileşen burada düşer.
            let ornek = pre
                .instantiate_async(&mut store)
                .await
                .map_err(|e| YuklemeHatasi::Dunya(format!("{e:#}")))?;
            ornek
                .kalfa_plugin_arac()
                .call_araclari_listele(&mut store)
                .await
                .map_err(|e| YuklemeHatasi::AracListesi(format!("{e:#}")))
        })
        .await
        .map_err(|_| YuklemeHatasi::AracListesi("5 sn içinde yanıt vermedi".into()))??;
        let araclar = araclari_dogrula(liste)?;
        Ok(Eklenti {
            manifest,
            ozet,
            araclar,
            pre,
        })
    }
}

fn araclari_dogrula(
    liste: Vec<crate::bag::exports::kalfa::plugin::arac::AracTanimi>,
) -> Result<Vec<AracBilgisi>, YuklemeHatasi> {
    if liste.is_empty() {
        return Err(YuklemeHatasi::AracListesi(
            "eklenti hiç araç sunmuyor".into(),
        ));
    }
    let mut araclar: Vec<AracBilgisi> = Vec::with_capacity(liste.len());
    for t in liste {
        let hata = |neden: &str| YuklemeHatasi::Arac {
            arac: kalfa_core::arac::kirp(&t.ad, 64),
            neden: neden.to_string(),
        };
        if !arac_adi_gecerli(&t.ad) {
            return Err(hata(
                "ad küçük harfle başlamalı, en çok 32 karakter olmalı, yalnızca küçük harf, rakam ve `_` içermeli; `__` içeremez",
            ));
        }
        if araclar.iter().any(|a| a.ad == t.ad) {
            return Err(hata("aynı adla iki araç var"));
        }
        // Açıklama ve şema güvenilmeyen içeriktir (§6.5).
        if t.aciklama.chars().count() > sinir::ARAC_ACIKLAMASI {
            return Err(hata("açıklama en çok 1 000 karakter olabilir"));
        }
        if t.parametre_semasi.len() > sinir::SEMA {
            return Err(hata("parametre şeması en çok 16 KiB olabilir"));
        }
        let sema: Value = serde_json::from_str(&t.parametre_semasi)
            .map_err(|e| hata(&format!("parametre şeması JSON değil: {e}")))?;
        if !sema.is_object() {
            return Err(hata("parametre şeması bir JSON nesnesi olmalı"));
        }
        jsonschema::validator_for(&sema)
            .map_err(|e| hata(&format!("parametre şeması geçersiz: {e}")))?;
        araclar.push(AracBilgisi {
            ad: t.ad,
            aciklama: t.aciklama,
            sema,
        });
    }
    Ok(araclar)
}

impl Eklenti {
    /// Bir aracı yeni bir örnekte çağırır. İkinci değer `stderr`'in sonudur.
    pub(crate) async fn calistir(
        &self,
        motor: &Motor,
        arac: &str,
        argumanlar: &str,
        ortam: CagriOrtami,
    ) -> (Result<String, CagriHatasi>, String) {
        let iptal = ortam.iptal.clone();
        let sira = motor.sira(format!(
            "{}\u{0}{}",
            self.manifest.ad,
            ortam.depo.dizin().display()
        ));
        let _sira = tokio::select! {
            s = sira.lock_owned() => s,
            _ = iptal.cancelled() => return (Err(CagriHatasi::Iptal), String::new()),
        };
        let _izin = tokio::select! {
            i = motor.eszamanli.clone().acquire_owned() => i,
            _ = iptal.cancelled() => return (Err(CagriHatasi::Iptal), String::new()),
        };
        let durum = Durum::yeni(Some(ortam));
        let iz = durum.iz.clone();
        let mut store = motor.store(durum, motor.sureler.cpu_tik());
        let pre = self.pre.clone();
        // Store çağrıyla birlikte taşınır; future bırakılınca örnek de atılır.
        let cagri = async move {
            let ornek = pre.instantiate_async(&mut store).await?;
            let sonuc = ornek
                .kalfa_plugin_arac()
                .call_calistir(&mut store, arac, argumanlar)
                .await;
            if let Some(c) = &store.data().cagri {
                c.bitir();
            }
            sonuc
        };
        // İptalden sonra eklentiye çıkması için süre verilir (§7.5).
        let iptal_bekle = async {
            iptal.cancelled().await;
            tokio::time::sleep(sinir::IPTAL_SURESI).await;
        };
        let sonuc = tokio::select! {
            r = tokio::time::timeout(motor.sureler.toplam, cagri) => r,
            () = iptal_bekle => return (Err(CagriHatasi::Iptal), iz.stderr.metin()),
        };
        let stderr = iz.stderr.metin();
        let sonuc = match sonuc {
            Err(_) => Err(CagriHatasi::Sure),
            Ok(Ok(Ok(s))) => Ok(s.icerik),
            Ok(Ok(Err(Hata::IptalEdildi))) if iptal.is_cancelled() => Err(CagriHatasi::Iptal),
            Ok(Ok(Err(h))) => Err(CagriHatasi::Eklenti(h)),
            Ok(Err(_)) if iz.bellek.load(Ordering::SeqCst) => Err(CagriHatasi::Bellek),
            Ok(Err(_)) if iz.cpu.load(Ordering::SeqCst) => Err(CagriHatasi::Cpu),
            Ok(Err(_)) if iptal.is_cancelled() => Err(CagriHatasi::Iptal),
            Ok(Err(_)) => Err(CagriHatasi::Cokme),
        };
        (sonuc, stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::izinli_import;

    #[test]
    fn import_izin_listesi() {
        for ad in [
            "kalfa:plugin/host@0.1.0",
            "kalfa:plugin/tipler@0.1.0",
            "wasi:io/streams@0.2.6",
            "wasi:cli/stderr@0.2.0",
            "wasi:filesystem/preopens@0.2.3",
            "wasi:clocks/wall-clock@0.2.6",
            "wasi:random/random@0.2.6",
        ] {
            assert!(izinli_import(ad), "{ad}");
        }
        for ad in [
            "wasi:sockets/tcp@0.2.6",
            "wasi:http/outgoing-handler@0.2.6",
            "wasi:io/streams@0.3.0",
            "wasi:io/streams",
            "kalfa:plugin/host@0.2.0",
            "kalfa:plugin/arac@0.1.0",
            "baska:paket/x@1.0.0",
            "wasi:iox/streams@0.2.0",
        ] {
            assert!(!izinli_import(ad), "{ad}");
        }
    }
}
