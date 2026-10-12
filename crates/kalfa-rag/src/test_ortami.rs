#![allow(dead_code)]

//! Birim testlerinin bellekteki ortamı: dosyalar, depo, belirlenimli gömme ve
//! sıralama, sırası verilen yazmada ya da gömmede hata, iptal.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use kalfa_plugin_sdk::{DosyaBilgisi, Hata};
use sha2::{Digest, Sha256};

use crate::ortam::{BOYUT, Ortam};

/// `ilerleme` çağrısı: tamamlanan, toplam, ileti.
pub type Ilerleme = (u64, Option<u64>, Option<String>);

#[derive(Default)]
pub struct SahteOrtam {
    /// yol → (içerik, değişme zamanı).
    pub dosyalar: RefCell<BTreeMap<String, (String, u64)>>,
    pub depo: RefCell<BTreeMap<String, Vec<u8>>>,
    pub simdi: Cell<u64>,
    /// Her `gomme` çağrısının metinleri.
    pub gommeler: RefCell<Vec<Vec<String>>>,
    /// Her `sirala` çağrısının (sorgu, belgeler).
    pub siralamalar: RefCell<Vec<(String, Vec<String>)>>,
    pub ilerlemeler: RefCell<Vec<Ilerleme>>,
    /// Başarılı ya da başarısız her `depo_yaz` denemesi.
    pub yazma_denemesi: Cell<usize>,
    /// Bu sıradaki (1'den) `depo_yaz` bu hatayla düşer ve hiçbir şey yazmaz.
    pub yazma_hatasi: RefCell<Option<(usize, Hata)>>,
    /// Bu sıradaki (1'den) `gomme` bu hatayla düşer.
    pub gomme_hatasi: RefCell<Option<(usize, Hata)>>,
    /// `sirala` çağrılarının döneceği hata.
    pub siralama_hatasi: RefCell<Option<Hata>>,
    /// Bu kadar `gomme` çağrısından sonra `iptal_istendi` `true` döner.
    pub iptal_sonrasi: Cell<Option<usize>>,
}

impl SahteOrtam {
    pub fn yeni() -> Self {
        let o = Self::default();
        o.simdi.set(1_700_000_000_000);
        o
    }

    /// Dosyayı şimdiki zamanla yazar.
    pub fn dosya(&self, yol: &str, icerik: &str) {
        self.dosya_zamanla(yol, icerik, self.simdi.get());
    }

    pub fn dosya_zamanla(&self, yol: &str, icerik: &str, degisme: u64) {
        self.dosyalar
            .borrow_mut()
            .insert(yol.into(), (icerik.into(), degisme));
    }

    pub fn sil(&self, yol: &str) {
        self.dosyalar.borrow_mut().remove(yol);
    }

    /// Saati ilerletir.
    pub fn ilerlet(&self, ms: u64) {
        self.simdi.set(self.simdi.get() + ms);
    }

    /// Gömülen bütün metinler, sırayla.
    pub fn gomulenler(&self) -> Vec<String> {
        self.gommeler.borrow().iter().flatten().cloned().collect()
    }
}

/// Metnin SHA-256'sından türetilen, birim uzunlukta belirlenimli vektör.
pub fn sahte_vektor(metin: &str) -> Vec<f32> {
    let mut v = Vec::with_capacity(BOYUT);
    let mut blok = 0u32;
    while v.len() < BOYUT {
        let mut h = Sha256::new();
        h.update(metin.as_bytes());
        h.update(blok.to_le_bytes());
        for b in h.finalize() {
            if v.len() == BOYUT {
                break;
            }
            v.push((f32::from(b) - 127.5) / 127.5);
        }
        blok += 1;
    }
    let uzunluk = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    v.iter().map(|x| x / uzunluk).collect()
}

impl Ortam for SahteOrtam {
    fn dosya_listele(&self, _desen: &str) -> Result<Vec<DosyaBilgisi>, Hata> {
        Ok(self
            .dosyalar
            .borrow()
            .iter()
            .map(|(yol, (icerik, degisme))| DosyaBilgisi {
                yol: yol.clone(),
                boyut: icerik.len() as u64,
                degisme: *degisme,
            })
            .collect())
    }

    fn dosya_oku(&self, yol: &str) -> Result<String, Hata> {
        self.dosyalar
            .borrow()
            .get(yol)
            .map(|(icerik, _)| icerik.clone())
            .ok_or_else(|| Hata::Bulunamadi(yol.into()))
    }

    fn gomme(&self, metinler: &[String]) -> Result<Vec<Vec<f32>>, Hata> {
        self.gommeler.borrow_mut().push(metinler.to_vec());
        let sira = self.gommeler.borrow().len();
        if let Some((n, h)) = &*self.gomme_hatasi.borrow()
            && *n == sira
        {
            return Err(h.clone());
        }
        Ok(metinler.iter().map(|m| sahte_vektor(m)).collect())
    }

    fn sirala(
        &self,
        sorgu: &str,
        belgeler: &[String],
        ilk_n: u32,
    ) -> Result<Vec<(usize, f32)>, Hata> {
        self.siralamalar
            .borrow_mut()
            .push((sorgu.into(), belgeler.to_vec()));
        if let Some(h) = &*self.siralama_hatasi.borrow() {
            return Err(h.clone());
        }
        let sorgu = sorgu.to_lowercase();
        let kelimeler: Vec<&str> = sorgu
            .split_whitespace()
            .filter(|k| k.chars().count() >= 2)
            .collect();
        let mut puanlar: Vec<(usize, f32)> = belgeler
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let b = b.to_lowercase();
                let gecen = kelimeler.iter().filter(|k| b.contains(*k)).count();
                (i, gecen as f32 / (kelimeler.len() as f32 + 1.0))
            })
            .collect();
        puanlar.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        puanlar.truncate(ilk_n as usize);
        Ok(puanlar)
    }

    fn depo_oku(&self, anahtar: &str) -> Result<Option<Vec<u8>>, Hata> {
        Ok(self.depo.borrow().get(anahtar).cloned())
    }

    fn depo_yaz(&self, anahtar: &str, deger: &[u8]) -> Result<(), Hata> {
        let sira = self.yazma_denemesi.get() + 1;
        self.yazma_denemesi.set(sira);
        if let Some((n, h)) = &*self.yazma_hatasi.borrow()
            && *n == sira
        {
            return Err(h.clone());
        }
        self.depo
            .borrow_mut()
            .insert(anahtar.into(), deger.to_vec());
        Ok(())
    }

    fn depo_sil(&self, anahtar: &str) -> Result<(), Hata> {
        self.depo.borrow_mut().remove(anahtar);
        Ok(())
    }

    fn ilerleme(&self, tamamlanan: u64, toplam: Option<u64>, mesaj: Option<String>) {
        self.ilerlemeler
            .borrow_mut()
            .push((tamamlanan, toplam, mesaj));
    }

    fn iptal_istendi(&self) -> bool {
        self.iptal_sonrasi
            .get()
            .is_some_and(|n| self.gommeler.borrow().len() >= n)
    }

    fn simdi_ms(&self) -> u64 {
        self.simdi.get()
    }
}
