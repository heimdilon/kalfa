//! Araç aşaması: planlama (doğrulama + izin) → onay → kilitli paralel
//! yürütme (spec §6, §6.1, §7.4–§7.6).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use kalfa_evren::tipler::{ChatMessage, ToolCall};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::{Etki, Oturum};
use crate::arac::{Arac, AracCiktisi, AracHatasi, Hazirlik};
use crate::izin::{Islem, Karar, Kip, Siniflandirici, YolSinifi, karar_ver};
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, IslemId};
use crate::muhur::{OnayMuhru, muhur_hesapla};
use crate::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use crate::yol::CalismaYolu;

struct Planlanan {
    cagri: CagriId,
    model_id: String,
    anahtar: String,
    arac: Arc<dyn Arac>,
    argumanlar: Value,
    hazirlik: Hazirlik,
    muhur: OnayMuhru,
    /// Planlandığı andaki politika sürümü (spec §7.6).
    politika: u64,
    kip: Kip,
    karar: Karar,
}

struct CagriSonucu {
    cagri: CagriId,
    model_id: String,
    arac: String,
    /// Tekrar koruması anahtarı: araç adı + ham argüman.
    anahtar: String,
    durum: AracDurumu,
    icerik: String,
}

impl CagriSonucu {
    fn plandan(p: &Planlanan, durum: AracDurumu, icerik: String) -> Self {
        Self {
            cagri: p.cagri,
            model_id: p.model_id.clone(),
            arac: p.arac.ad().to_string(),
            anahtar: p.anahtar.clone(),
            durum,
            icerik,
        }
    }
}

struct OnayToplama {
    onaylanan: Vec<Planlanan>,
    reddedilen: Vec<CagriSonucu>,
    yanitsiz: Vec<Planlanan>,
    iptal: Option<BitisNedeni>,
    /// Onay beklenirken izin kipi değişti: bu aşamanın hiçbir çağrısı yürütülmez.
    politika_degisti: bool,
}

pub(super) enum AsamaSonucu {
    Devam,
    Bitir(BitisNedeni),
}

fn tekrar_anahtari(tc: &ToolCall) -> String {
    format!("{}\u{0}{}", tc.function.name, tc.function.arguments)
}

impl Oturum {
    pub(super) async fn arac_asamasi(
        &mut self,
        cagrilar: Vec<ToolCall>,
        hata_sayaci: &mut HashMap<String, u32>,
    ) -> AsamaSonucu {
        let mut serbest = Vec::new();
        let mut onayli = Vec::new();
        for tc in cagrilar {
            let cagri = CagriId::yeni();
            let plan = if self.iptal.is_cancelled() {
                Err("tur iptal edildi".into())
            } else {
                self.planla(cagri, &tc).await
            };
            match plan {
                Ok((plan, Karar::Serbest)) => serbest.push(plan),
                Ok((plan, Karar::Onayli { uyarilar })) => onayli.push((plan, uyarilar)),
                Ok((plan, Karar::Yasak { neden })) => {
                    let s = CagriSonucu::plandan(
                        &plan,
                        AracDurumu::Reddedildi,
                        format!("[izin verilmedi] {neden}"),
                    );
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                Err(mesaj) => {
                    let durum = if self.iptal.is_cancelled() {
                        AracDurumu::IptalEdildi
                    } else {
                        AracDurumu::Basarisiz
                    };
                    let icerik = if durum == AracDurumu::IptalEdildi {
                        "[çalıştırılmadı: tur iptal edildi]".to_string()
                    } else {
                        format!("[hata] {mesaj}")
                    };
                    let s = CagriSonucu {
                        cagri,
                        model_id: tc.id.clone(),
                        arac: tc.function.name.clone(),
                        anahtar: tekrar_anahtari(&tc),
                        durum,
                        icerik,
                    };
                    self.sonuc_isle(s, hata_sayaci).await;
                }
            }
        }

        // Hazırlıkta iptal, önceden planlanan ve henüz ele alınmayan
        // çağrıları da kapatır; sonraki model isteğinde eksik tool yanıtı kalmaz.
        if self.iptal.is_cancelled() {
            for p in serbest.iter().chain(onayli.iter().map(|(p, _)| p)) {
                self.sonuc_isle(
                    CagriSonucu::plandan(
                        p,
                        AracDurumu::IptalEdildi,
                        "[çalıştırılmadı: tur iptal edildi]".into(),
                    ),
                    hata_sayaci,
                )
                .await;
            }
            return AsamaSonucu::Bitir(BitisNedeni::Iptal);
        }

        // Daha sonraki bir hazırlık beklenirken önceki planların kipi de
        // değişmiş olabilir. Geçersiz planlar için onay kartı gösterme.
        let politika = self.politika.load(Ordering::SeqCst);
        let (mut serbest, mut eski): (Vec<_>, Vec<_>) =
            serbest.into_iter().partition(|p| p.politika == politika);
        let (onayli, eski_onayli): (Vec<_>, Vec<_>) = onayli
            .into_iter()
            .partition(|(p, _)| p.politika == politika);
        eski.extend(eski_onayli.into_iter().map(|(p, _)| p));
        for p in eski {
            self.sonuc_isle(
                CagriSonucu::plandan(
                    &p,
                    AracDurumu::IptalEdildi,
                    format!("[çalıştırılmadı] {}", AracHatasi::PolitikaDegisti),
                ),
                hata_sayaci,
            )
            .await;
        }

        if !onayli.is_empty() {
            let toplama = self.onay_topla(onayli).await;
            for s in toplama.reddedilen {
                self.sonuc_isle(s, hata_sayaci).await;
            }
            if let Some(neden) = toplama.iptal {
                for p in serbest
                    .iter()
                    .chain(&toplama.onaylanan)
                    .chain(&toplama.yanitsiz)
                {
                    let s = CagriSonucu::plandan(
                        p,
                        AracDurumu::IptalEdildi,
                        "[çalıştırılmadı: tur iptal edildi]".into(),
                    );
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                return AsamaSonucu::Bitir(neden);
            }
            if toplama.politika_degisti {
                self.hata(
                    "İzin kipi değişti; bekleyen onaylar geçersiz sayıldı ve bu adımdaki işlemler yapılmadı.",
                )
                .await;
                for p in serbest
                    .iter()
                    .chain(&toplama.onaylanan)
                    .chain(&toplama.yanitsiz)
                {
                    let icerik =
                        "[çalıştırılmadı: izin kipi değişti; gerekirse yeniden dene]".to_string();
                    let s = CagriSonucu::plandan(p, AracDurumu::IptalEdildi, icerik);
                    self.sonuc_isle(s, hata_sayaci).await;
                }
                return AsamaSonucu::Devam;
            }
            serbest.extend(toplama.onaylanan);
        }

        if let Some(neden) = self.yurut(serbest, hata_sayaci).await {
            return AsamaSonucu::Bitir(neden);
        }
        let esik = self.ayarlar.sinirlar.ayni_hata_esigi;
        if hata_sayaci.values().any(|n| *n >= esik) {
            return AsamaSonucu::Bitir(BitisNedeni::Sinir {
                aciklama: format!(
                    "Aynı araç çağrısı üst üste {esik} kez başarısız oldu; döngü durduruldu."
                ),
            });
        }
        AsamaSonucu::Devam
    }

    async fn planla(
        &mut self,
        cagri: CagriId,
        tc: &ToolCall,
    ) -> Result<(Planlanan, Karar), String> {
        let ad = tc.function.name.as_str();
        let arac = self
            .baglam
            .araclar
            .bul(ad)
            .ok_or_else(|| format!("`{ad}` adlı bir araç yok."))?;
        let kip = self.ayarlar.kip;
        if kip == Kip::Inceleme && arac.yazar_mi() {
            return Err("İnceleme kipinde bu araç kullanılamaz; bu kip yalnızca okur.".into());
        }
        let argumanlar = self.baglam.araclar.dogrula(ad, &tc.function.arguments)?;
        let siniflandirici = self.baglam.siniflandirici.clone();
        // Hazırlık beklerken komut kanalını da işle. Politika sürümünü
        // beklemeden önce al: hazırlık sırasında değişen kip eski işlemi yetkilendirmez.
        let politika = self.politika.load(Ordering::SeqCst);
        let hazirlik = self
            .iptal_edilebilir(arac.hazirla(&argumanlar))
            .await
            .map_err(|_| "tur iptal edildi".to_string())?
            .map_err(|e| e.to_string())?;
        let karar = Self::karar(kip, &siniflandirici, &hazirlik);
        let muhur = muhur_hesapla(cagri, ad, &argumanlar, &hazirlik);
        let plan = Planlanan {
            cagri,
            model_id: tc.id.clone(),
            anahtar: tekrar_anahtari(tc),
            arac,
            argumanlar,
            hazirlik,
            muhur,
            politika,
            kip,
            karar: karar.clone(),
        };
        Ok((plan, karar))
    }

    fn karar(kip: Kip, s: &Siniflandirici, hazirlik: &Hazirlik) -> Karar {
        let mut karar = Karar::Serbest;
        for yol in &hazirlik.okunan {
            let sinif = if s.gizli_mi(yol) {
                YolSinifi::Gizli
            } else {
                YolSinifi::Normal
            };
            karar = karar.birlestir(karar_ver(kip, Islem::Okuma, sinif, false));
        }
        for hedef in &hazirlik.yazilan {
            let yeni = karar_ver(
                kip,
                Islem::Yazma,
                s.sinifla(&hedef.yol),
                hedef.baglanti_sayisi > 1,
            );
            karar = karar.birlestir(yeni);
        }
        karar
    }

    async fn onay_topla(&mut self, bekleyen: Vec<(Planlanan, Vec<String>)>) -> OnayToplama {
        let ogeler = bekleyen
            .iter()
            .map(|(p, uyarilar)| OnayOgesi {
                cagri: p.cagri,
                arac: p.arac.ad().to_string(),
                aciklama: p.hazirlik.aciklama.clone(),
                hedefler: hedefler(&p.hazirlik),
                diff: p.hazirlik.diff.clone(),
                uyarilar: uyarilar.clone(),
                muhur: p.muhur.clone(),
            })
            .collect();
        let mut kalan: HashMap<CagriId, Planlanan> =
            bekleyen.into_iter().map(|(p, _)| (p.cagri, p)).collect();
        let mut t = OnayToplama {
            onaylanan: Vec::new(),
            reddedilen: Vec::new(),
            yanitsiz: Vec::new(),
            iptal: None,
            politika_degisti: false,
        };
        let baslangic_politikasi = self.politika.load(Ordering::SeqCst);
        self.yay(Olay::OnayGerekiyor {
            islem: IslemId::yeni(),
            ogeler,
        })
        .await;

        while !kalan.is_empty() {
            let Some(komut) = self.tur_komutu_al().await else {
                t.iptal = Some(BitisNedeni::Iptal);
                break;
            };
            match komut {
                Komut::AracOnayi {
                    cagri,
                    karar,
                    muhur,
                } => match kalan.get(&cagri).map(|p| p.muhur == muhur) {
                    None => {
                        self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                            .await
                    }
                    Some(false) => {
                        self.hata("Onay mührü eşleşmedi; onay kartı güncel değil.")
                            .await
                    }
                    Some(true) => {
                        if let Some(p) = kalan.remove(&cagri) {
                            match karar {
                                OnayKarari::Onayla => t.onaylanan.push(p),
                                OnayKarari::Reddet { gerekce } => {
                                    let g = gerekce
                                        .filter(|g| !g.trim().is_empty())
                                        .unwrap_or_else(|| "gerekçe belirtilmedi".into());
                                    let icerik = format!("[kullanıcı reddetti] {g}");
                                    t.reddedilen.push(CagriSonucu::plandan(
                                        &p,
                                        AracDurumu::Reddedildi,
                                        icerik,
                                    ));
                                }
                            }
                        }
                    }
                },
                diger => {
                    if self.genel_komut(diger).await == Etki::Iptal {
                        t.iptal = Some(BitisNedeni::Iptal);
                        break;
                    }
                    if self.politika.load(Ordering::SeqCst) != baslangic_politikasi {
                        t.politika_degisti = true;
                        break;
                    }
                }
            }
        }
        t.yanitsiz = kalan.into_values().collect();
        t
    }

    /// Planları paralel yürütür; iptal geldiyse ya da kayıt yazılamadıysa
    /// nedeni döner. Sonuçlar, araçların iptal belirtecini görmesi için
    /// yine toplanır.
    async fn yurut(
        &mut self,
        planlar: Vec<Planlanan>,
        hata_sayaci: &mut HashMap<String, u32>,
    ) -> Option<BitisNedeni> {
        let mut calisacak = Vec::with_capacity(planlar.len());
        let mut kaydi_yazilamayan = Vec::new();
        for p in planlar {
            let oge = KayitOgesi::AracBasladi {
                cagri: p.cagri,
                model_cagri_id: p.model_id.clone(),
                arac: p.arac.ad().to_string(),
            };
            // Yan etkili aracın "başladı" kaydı diske ulaşmadan araç çalışmaz.
            let yazildi = if p.arac.yazar_mi() {
                self.kaydet_kalici(oge).await
            } else {
                self.kaydet(oge).await
            };
            if !yazildi {
                kaydi_yazilamayan.push(p);
                continue;
            }
            let olay = Olay::AracCagrisiBasladi {
                cagri: p.cagri,
                arac: p.arac.ad().to_string(),
                aciklama: p.hazirlik.aciklama.clone(),
            };
            self.yay(olay).await;
            calisacak.push(p);
        }
        // Henüz hiçbir future poll edilmedi. Grubun sonraki kaydı bozulduysa
        // önceki başarılı başlangıç kayıtları da yürütülmez.
        if self.kayit_hatasi.is_some() {
            kaydi_yazilamayan.append(&mut calisacak);
        }
        for p in &kaydi_yazilamayan {
            let icerik = "[çalıştırılmadı: oturum kaydı yazılamadı]".to_string();
            self.sonuc_isle(
                CagriSonucu::plandan(p, AracDurumu::IptalEdildi, icerik),
                hata_sayaci,
            )
            .await;
        }

        let azami = self.ayarlar.sinirlar.azami_arac_ciktisi;
        let mut isler = FuturesUnordered::new();
        for p in calisacak {
            // Kilitler çalışma alanı düzeyinde ortaktır.
            let kilitler = self.baglam.kilitler.clone();
            let politika = self.politika.clone();
            let siniflandirici = self.baglam.siniflandirici.clone();
            let iptal = self.iptal.child_token();
            isler.push(async move {
                let sonuc = tek_calistir(&p, &kilitler, &politika, &siniflandirici, iptal).await;
                (p, sonuc)
            });
        }

        let mut iptal_nedeni = None;
        loop {
            tokio::select! {
                biased;
                _ = self.iptal.cancelled(), if iptal_nedeni.is_none() => iptal_nedeni = Some(BitisNedeni::Iptal),
                komut = self.komutlar.recv(), if iptal_nedeni.is_none() => {
                    match self.komut_geldi(komut).await {
                        Some(k) => {
                            if self.genel_komut(k).await == Etki::Iptal {
                                iptal_nedeni = Some(BitisNedeni::Iptal);
                            }
                        }
                        None => iptal_nedeni = Some(BitisNedeni::Iptal),
                    }
                }
                sonraki = isler.next() => match sonraki {
                    Some((p, sonuc)) => {
                        let s = sonuc_olustur(&p, sonuc, azami);
                        self.sonuc_isle(s, hata_sayaci).await;
                    }
                    None => break,
                },
            }
        }
        if self.kayit_hatasi.is_some() {
            Some(BitisNedeni::Hata)
        } else {
            iptal_nedeni
        }
    }

    /// Sonucu bağlama, kayda ve arayüze işler. Mesaj `AracBitti`'den önce
    /// yazılır: çökme arada olursa kurtarma sonucu bilir.
    async fn sonuc_isle(&mut self, s: CagriSonucu, hata_sayaci: &mut HashMap<String, u32>) {
        if s.durum == AracDurumu::Basarisiz {
            *hata_sayaci.entry(s.anahtar.clone()).or_default() += 1;
        } else {
            hata_sayaci.remove(&s.anahtar);
        }
        self.mesaj_ekle(ChatMessage::tool(s.model_id.clone(), s.icerik.clone()))
            .await;
        self.kaydet(KayitOgesi::AracBitti {
            cagri: s.cagri,
            durum: s.durum,
        })
        .await;
        self.yay(Olay::AracSonucu {
            cagri: s.cagri,
            arac: s.arac,
            durum: s.durum,
            icerik: s.icerik,
        })
        .await;
    }
}

fn hedefler(h: &Hazirlik) -> Vec<String> {
    h.okunan
        .iter()
        .map(|y| y.to_string())
        .chain(h.yazilan.iter().map(|y| y.yol.to_string()))
        .collect()
}

async fn tek_calistir(
    p: &Planlanan,
    kilitler: &crate::kilit::KaynakKilitleri,
    politika: &AtomicU64,
    siniflandirici: &Siniflandirici,
    iptal: CancellationToken,
) -> Result<AracCiktisi, AracHatasi> {
    let yazilan: Vec<CalismaYolu> = p.hazirlik.yazilan.iter().map(|h| h.yol.clone()).collect();
    let _kilit = tokio::select! {
        k = kilitler.al(&p.hazirlik.okunan, &yazilan, p.hazirlik.tum_calisma_alani) => k,
        _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
    };
    // İzin kipi planlamadan sonra değişti ise yeni kipte yeniden
    // yetkilendirilmeden işlem yapılmaz.
    if politika.load(Ordering::SeqCst) != p.politika {
        return Err(AracHatasi::PolitikaDegisti);
    }
    // Kilit altında hedef yeniden hazırlanır: onaydan sonra değiştiyse
    // mührü tutmaz ve işlem yapılmaz.
    let taze = tokio::select! {
        biased;
        _ = iptal.cancelled() => return Err(AracHatasi::Iptal),
        h = p.arac.hazirla(&p.argumanlar) => h?,
    };
    if muhur_hesapla(p.cagri, p.arac.ad(), &p.argumanlar, &taze) != p.muhur {
        return Err(AracHatasi::OnayEskidi);
    }
    // Etkin git koruması değişmiş olabilir; yeni karar eski onayla geçmez.
    if Oturum::karar(p.kip, siniflandirici, &taze) != p.karar {
        return Err(AracHatasi::PolitikaDegisti);
    }
    if iptal.is_cancelled() {
        return Err(AracHatasi::Iptal);
    }
    // `hazirla` await içerir; bu sırada kip değişmiş olabilir.
    if politika.load(Ordering::SeqCst) != p.politika {
        return Err(AracHatasi::PolitikaDegisti);
    }
    p.arac.calistir(&p.argumanlar, &taze, iptal).await
}

fn sonuc_olustur(
    p: &Planlanan,
    sonuc: Result<AracCiktisi, AracHatasi>,
    azami: usize,
) -> CagriSonucu {
    match sonuc {
        Ok(c) => {
            let icerik = kirp(format!("[kaynak: {}]\n{}", c.kaynak, c.icerik), azami);
            CagriSonucu::plandan(p, AracDurumu::Basarili, icerik)
        }
        Err(AracHatasi::Iptal) => {
            CagriSonucu::plandan(p, AracDurumu::IptalEdildi, "[iptal edildi]".into())
        }
        Err(e @ AracHatasi::PolitikaDegisti) => {
            CagriSonucu::plandan(p, AracDurumu::IptalEdildi, format!("[çalıştırılmadı] {e}"))
        }
        Err(e) => CagriSonucu::plandan(p, AracDurumu::Basarisiz, format!("[hata] {e}")),
    }
}

fn kirp(metin: String, azami: usize) -> String {
    if metin.len() <= azami {
        return metin;
    }
    let mut kes = azami;
    while !metin.is_char_boundary(kes) {
        kes -= 1;
    }
    format!(
        "{}\n[çıktı {} bayttan {} bayta kırpıldı]",
        &metin[..kes],
        metin.len(),
        kes
    )
}
