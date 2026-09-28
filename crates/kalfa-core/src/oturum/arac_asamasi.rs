//! Araç aşaması: planlama (doğrulama + izin) → onay → kilitli paralel
//! yürütme (spec §6, §6.1, §7.4–§7.6).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use kalfa_evren::tipler::{ChatMessage, ToolCall};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::{Etki, Oturum};
use crate::arac::{Arac, AracCiktisi, AracHatasi, CiktiGonderici, CiktiParcasi, Hazirlik};
use crate::izin::{Islem, Karar, Kip, Siniflandirici, YolSinifi, kabuk_karari, karar_ver};
use crate::kayit::KayitOgesi;
use crate::kimlik::{CagriId, IslemId};
use crate::muhur::{OnayMuhru, muhur_hesapla};
use crate::protokol::{AracDurumu, BitisNedeni, Komut, Olay, OnayKarari, OnayOgesi};
use crate::yol::CalismaYolu;

/// Araçlardan oturuma gelen ara çıktı kanalının kapasitesi (parça).
const CIKTI_KANALI: usize = 64;

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
    /// Başarısızlık, aynı çağrının üst üste başarısızlık sınırına sayılmaz
    /// (kabukta sıfırdan farklı çıkış kodu; aşama 2 tasarımı §4.5).
    hata_sayilmaz: bool,
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
            hata_sayilmaz: false,
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
                Ok((plan, Karar::Onayli { uyarilar })) => onayli.push((plan, uyarilar, false)),
                Ok((plan, Karar::YerelOnayli { uyarilar })) => onayli.push((plan, uyarilar, true)),
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
                        hata_sayilmaz: false,
                    };
                    self.sonuc_isle(s, hata_sayaci).await;
                }
            }
        }

        // Hazırlıkta iptal, önceden planlanan ve henüz ele alınmayan
        // çağrıları da kapatır; sonraki model isteğinde eksik tool yanıtı kalmaz.
        if self.iptal.is_cancelled() {
            for p in serbest.iter().chain(onayli.iter().map(|(p, _, _)| p)) {
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
            .partition(|(p, _, _)| p.politika == politika);
        eski.extend(eski_onayli.into_iter().map(|(p, _, _)| p));
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
        if !arac.sunulur_mu() {
            return Err(format!("`{ad}` aracı şu an kullanılamıyor."));
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
        if let Some(kabuk) = &hazirlik.kabuk {
            karar = karar.birlestir(kabuk_karari(kip, kabuk));
        }
        karar
    }

    async fn onay_topla(&mut self, bekleyen: Vec<(Planlanan, Vec<String>, bool)>) -> OnayToplama {
        let ogeler: Vec<OnayOgesi> = bekleyen
            .iter()
            .map(|(p, uyarilar, yerel)| OnayOgesi {
                cagri: p.cagri,
                arac: p.arac.ad().to_string(),
                aciklama: p.hazirlik.aciklama.clone(),
                hedefler: hedefler(&p.hazirlik),
                diff: p.hazirlik.diff.clone(),
                uyarilar: uyarilar.clone(),
                muhur: p.muhur.clone(),
                kabuk: p.hazirlik.kabuk.clone(),
                yerel_onay: *yerel,
            })
            .collect();
        // Yerel iletişim kutusu komutu çekirdekten okur, webview'dan değil
        // (aşama 2 tasarımı §5.2).
        *self
            .bekleyen_onaylar
            .lock()
            .unwrap_or_else(|z| z.into_inner()) =
            ogeler.iter().map(|o| (o.cagri, o.clone())).collect();
        let yerel: HashSet<CagriId> = bekleyen
            .iter()
            .filter(|(_, _, yerel)| *yerel)
            .map(|(p, _, _)| p.cagri)
            .collect();
        let mut kalan: HashMap<CagriId, Planlanan> =
            bekleyen.into_iter().map(|(p, _, _)| (p.cagri, p)).collect();
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
                    Some(true) if karar == OnayKarari::Onayla && yerel.contains(&cagri) => {
                        self.hata("Bu komut yalnızca yerel pencerede onaylanabilir.")
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
                Komut::YerelOnay { cagri, muhur } => {
                    match kalan.get(&cagri).map(|p| p.muhur == muhur) {
                        None => {
                            self.hata("Bu onay artık geçerli değil; bekleyen bir işlem yok.")
                                .await
                        }
                        Some(false) => {
                            self.hata("Onay mührü eşleşmedi; onay kartı güncel değil.")
                                .await
                        }
                        Some(true) if !yerel.contains(&cagri) => {
                            self.hata("Bu işlem yerel onay gerektirmiyor.").await
                        }
                        Some(true) => {
                            if let Some(p) = kalan.remove(&cagri) {
                                t.onaylanan.push(p);
                            }
                        }
                    }
                }
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
        self.bekleyen_onaylar
            .lock()
            .unwrap_or_else(|z| z.into_inner())
            .clear();
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
        let (cikti_tx, mut cikti_rx) = mpsc::channel::<CiktiParcasi>(CIKTI_KANALI);
        let mut isler = FuturesUnordered::new();
        // Araç tarafında atılan (oturuma hiç ulaşmayan) baytların sayaçları.
        let mut arac_sayaclari = HashMap::new();
        for p in calisacak {
            // Kilitler çalışma alanı düzeyinde ortaktır.
            let kilitler = self.baglam.kilitler.clone();
            let politika = self.politika.clone();
            let siniflandirici = self.baglam.siniflandirici.clone();
            let iptal = self.iptal.child_token();
            let cikti = CiktiGonderici::yeni(p.cagri, cikti_tx.clone());
            arac_sayaclari.insert(p.cagri, cikti.atlanan_sayaci());
            isler.push(async move {
                let sonuc =
                    tek_calistir(&p, &kilitler, &politika, &siniflandirici, iptal, cikti).await;
                (p, sonuc)
            });
        }
        drop(cikti_tx);

        let mut iptal_nedeni = None;
        let mut cikti_kapali = false;
        // Arayüze iletilemeyen ara çıktının çağrı başına bayt sayısı.
        let mut atlanan: HashMap<CagriId, u64> = HashMap::new();
        loop {
            tokio::select! {
                biased;
                _ = self.iptal.cancelled(), if iptal_nedeni.is_none() => iptal_nedeni = Some(BitisNedeni::Iptal),
                // Ara çıktı sonuçtan önce işlenir: araç parçayı sonucu
                // döndürmeden önce gönderir.
                parca = cikti_rx.recv(), if !cikti_kapali => match parca {
                    Some(parca) => self.cikti_ilet(parca, &mut atlanan).await,
                    None => cikti_kapali = true,
                },
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
                        if let Some(sayac) = arac_sayaclari.remove(&p.cagri) {
                            let bayt = sayac.swap(0, Ordering::SeqCst);
                            *atlanan.entry(p.cagri).or_default() += bayt;
                        }
                        self.atlanani_bildir(p.cagri, &mut atlanan).await;
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

    /// Ara çıktı parçasını arayüze iletir. Olay kuyruğu yarıdan fazla doluysa
    /// parça atılır ve sayılır: çıktı seli kuyruğu doldurup turu iptal
    /// ettirmemeli (aşama 2 tasarımı §4.4).
    async fn cikti_ilet(&mut self, parca: CiktiParcasi, atlanan: &mut HashMap<CagriId, u64>) {
        let onceki = atlanan.remove(&parca.cagri).unwrap_or(0) + parca.atlanan_bayt;
        if self.olaylar.yer_var(parca.metin.len()) {
            self.yay(Olay::AracCiktisiParcasi {
                cagri: parca.cagri,
                akis: parca.akis,
                metin: parca.metin,
                atlanan_bayt: onceki,
            })
            .await;
        } else {
            atlanan.insert(parca.cagri, onceki + parca.metin.len() as u64);
        }
    }

    /// Çağrı bitti: atılmış ama bildirilmemiş çıktı varsa boş bir parçayla
    /// bildirilir. Bu parça atılmaz; çağrı başına en çok bir tane olur ve
    /// kuyruğun yarısı düşürülemeyen olaylara ayrılmıştır.
    async fn atlanani_bildir(&mut self, cagri: CagriId, atlanan: &mut HashMap<CagriId, u64>) {
        let Some(bayt) = atlanan.remove(&cagri) else {
            return;
        };
        if bayt > 0 {
            self.yay(Olay::AracCiktisiParcasi {
                cagri,
                akis: crate::protokol::CiktiAkisi::Standart,
                metin: String::new(),
                atlanan_bayt: bayt,
            })
            .await;
        }
    }

    /// Sonucu bağlama, kayda ve arayüze işler. Mesaj `AracBitti`'den önce
    /// yazılır: çökme arada olursa kurtarma sonucu bilir.
    async fn sonuc_isle(&mut self, s: CagriSonucu, hata_sayaci: &mut HashMap<String, u32>) {
        if s.hata_sayilmaz {
            // Sayaç ne artar ne sıfırlanır.
        } else if s.durum == AracDurumu::Basarisiz {
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

/// Taze hazırlığın kabuk tarifini onaylanana göre düzeltir
/// ([`crate::kabuk::KabukTarifi::onayla_kapsanan`]).
fn onayla_kapsanan(mut taze: Hazirlik, onaylanan: &Hazirlik) -> Hazirlik {
    if let Some(o) = &onaylanan.kabuk {
        taze.kabuk = taze.kabuk.map(|t| t.onayla_kapsanan(o));
    }
    taze
}

async fn tek_calistir(
    p: &Planlanan,
    kilitler: &crate::kilit::KaynakKilitleri,
    politika: &AtomicU64,
    siniflandirici: &Siniflandirici,
    iptal: CancellationToken,
    cikti: CiktiGonderici,
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
    // Kabukta tükenen tek seferlik koşullar onaylı sayılır; araç da
    // onaylanan tarife göre çalışır.
    let taze = onayla_kapsanan(taze, &p.hazirlik);
    if muhur_hesapla(p.cagri, p.arac.ad(), &p.argumanlar, &taze) != p.muhur {
        return Err(match (&p.hazirlik.kabuk, &taze.kabuk) {
            (Some(onaylanan), Some(t)) => {
                AracHatasi::KomutKosuluDegisti(onaylanan.degisiklik_nedeni(t).to_string())
            }
            _ => AracHatasi::OnayEskidi,
        });
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
    p.arac
        .calistir_akisli(&p.argumanlar, &taze, iptal, cikti)
        .await
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
        Err(AracHatasi::Basarisiz(c)) => {
            let icerik = kirp(format!("[kaynak: {}]\n{}", c.kaynak, c.icerik), azami);
            CagriSonucu {
                hata_sayilmaz: true,
                ..CagriSonucu::plandan(p, AracDurumu::Basarisiz, icerik)
            }
        }
        Err(AracHatasi::Iptal) => {
            CagriSonucu::plandan(p, AracDurumu::IptalEdildi, "[iptal edildi]".into())
        }
        Err(e @ (AracHatasi::PolitikaDegisti | AracHatasi::KomutKosuluDegisti(_))) => {
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
