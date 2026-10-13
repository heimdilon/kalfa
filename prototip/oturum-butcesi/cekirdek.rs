//! 5b'nin kabul ve muhasebe modelinin bağımlılıksız prototipi; ürün kodu değildir.
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sinirlar {
    istek: u64,
    token: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engel {
    Gecersiz,
    Bilinmeyen,
    Istek,
    Token,
    Suren,
}

#[derive(Debug)]
struct Durum {
    sinirlar: Sinirlar,
    baslatilan: u64,
    token: u64,
    bilinmeyen: u64,
    onaylanan_bilinmeyen: u64,
    aktif: BTreeSet<u64>,
    biten: BTreeSet<u64>,
}

impl Durum {
    fn yeni(sinirlar: Sinirlar) -> Result<Self, Engel> {
        if sinirlar.istek == 0 || sinirlar.token == 0 {
            return Err(Engel::Gecersiz);
        }
        Ok(Self {
            sinirlar,
            baslatilan: 0,
            token: 0,
            bilinmeyen: 0,
            onaylanan_bilinmeyen: 0,
            aktif: BTreeSet::new(),
            biten: BTreeSet::new(),
        })
    }

    fn engel(&self) -> Option<Engel> {
        if self.bilinmeyen > self.onaylanan_bilinmeyen {
            Some(Engel::Bilinmeyen)
        } else if self.token >= self.sinirlar.token {
            Some(Engel::Token)
        } else if self.baslatilan >= self.sinirlar.istek {
            Some(Engel::Istek)
        } else {
            None
        }
    }

    // Üründe kayıt yazımı/kalıcılaştırma BAŞARILI olduktan sonra uygulanır;
    // bu prototip yalnız sıralı durum geçişini sınar.
    fn baslat(&mut self) -> Result<u64, Engel> {
        if let Some(engel) = self.engel() {
            return Err(engel);
        }
        let kimlik = self.baslatilan.checked_add(1).ok_or(Engel::Gecersiz)?;
        self.baslatilan = kimlik;
        self.aktif.insert(kimlik);
        Ok(kimlik)
    }

    fn bitir(&mut self, kimlik: u64, token: Option<u64>) -> Result<(), Engel> {
        if !self.aktif.remove(&kimlik) {
            return Err(Engel::Gecersiz);
        }
        self.biten.insert(kimlik);
        match token {
            Some(token) => self.token = self.token.saturating_add(token),
            None => self.bilinmeyen = self.bilinmeyen.saturating_add(1),
        }
        Ok(())
    }

    fn kurtar(&mut self) {
        self.bilinmeyen = self.bilinmeyen.saturating_add(self.aktif.len() as u64);
        self.biten.append(&mut self.aktif);
    }

    fn devam(&mut self, gozlenen_bilinmeyen: u64, sinirlar: Sinirlar) -> Result<(), Engel> {
        if !self.aktif.is_empty() {
            return Err(Engel::Suren);
        }
        if gozlenen_bilinmeyen != self.bilinmeyen
            || sinirlar.istek <= self.baslatilan
            || sinirlar.token <= self.token
        {
            return Err(Engel::Gecersiz);
        }
        self.sinirlar = sinirlar;
        self.onaylanan_bilinmeyen = self.bilinmeyen;
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    fn durum(istek: u64, token: u64) -> Durum {
        Durum::yeni(Sinirlar { istek, token }).unwrap()
    }

    #[test]
    fn sifir_sinir_sinirsiz_anlamina_gelmez() {
        assert!(matches!(
            Durum::yeni(Sinirlar { istek: 0, token: 1 }),
            Err(Engel::Gecersiz)
        ));
        assert!(matches!(
            Durum::yeni(Sinirlar { istek: 1, token: 0 }),
            Err(Engel::Gecersiz)
        ));
    }

    #[test]
    fn sinirdaki_istek_kabul_sonraki_red() {
        let mut d = durum(1, 100);
        let id = d.baslat().unwrap();
        d.bitir(id, Some(3)).unwrap();
        assert_eq!(d.baslat(), Err(Engel::Istek));
        assert_eq!((d.baslatilan, d.token), (1, 3));
    }

    #[test]
    fn sekiz_paralel_talep_dort_izni_asamaz() {
        let (tx, rx) = mpsc::channel::<mpsc::Sender<Result<u64, Engel>>>();
        let isler: Vec<_> = (0..8)
            .map(|_| {
                let tx = tx.clone();
                thread::spawn(move || {
                    let (yanit_tx, yanit_rx) = mpsc::channel();
                    tx.send(yanit_tx).unwrap();
                    yanit_rx.recv().unwrap()
                })
            })
            .collect();
        drop(tx);
        let mut d = durum(4, 100);
        for yanit in rx {
            yanit.send(d.baslat()).unwrap();
        }
        let sonuclar: Vec<_> = isler.into_iter().map(|is| is.join().unwrap()).collect();
        assert_eq!(sonuclar.iter().filter(|s| s.is_ok()).count(), 4);
        assert_eq!(
            sonuclar.iter().filter(|s| **s == Err(Engel::Istek)).count(),
            4
        );
        assert_eq!(d.baslatilan, 4);
        assert_eq!(d.aktif.len(), 4);
    }

    #[test]
    fn token_sonradan_bilinir_asimdan_sonra_yeni_istek_yok() {
        let mut d = durum(10, 100);
        let a = d.baslat().unwrap();
        let b = d.baslat().unwrap();
        d.bitir(a, Some(80)).unwrap();
        d.bitir(b, Some(60)).unwrap();
        assert_eq!(d.token, 140);
        assert_eq!(d.baslat(), Err(Engel::Token));
    }

    #[test]
    fn bilinmeyen_kullanim_yeni_istegi_keser_baslamis_olan_biter() {
        let mut d = durum(10, 100);
        let a = d.baslat().unwrap();
        let b = d.baslat().unwrap();
        d.bitir(a, None).unwrap();
        assert_eq!(d.baslat(), Err(Engel::Bilinmeyen));
        assert_eq!(d.devam(1, d.sinirlar), Err(Engel::Suren));
        d.bitir(b, Some(20)).unwrap();
        d.devam(1, d.sinirlar).unwrap();
        assert_eq!((d.token, d.bilinmeyen), (20, 1));
        assert!(d.baslat().is_ok());
    }

    #[test]
    fn onay_gecmisi_sifirlamaz_yeni_bilinmeyen_yeniden_sorar() {
        let mut d = durum(10, 100);
        let a = d.baslat().unwrap();
        d.bitir(a, None).unwrap();
        d.devam(1, d.sinirlar).unwrap();
        let b = d.baslat().unwrap();
        d.bitir(b, None).unwrap();
        assert_eq!(d.baslat(), Err(Engel::Bilinmeyen));
        assert_eq!(d.bilinmeyen, 2);
        assert_eq!(d.devam(1, d.sinirlar), Err(Engel::Gecersiz));
    }

    #[test]
    fn cokmede_baslamis_istek_kaybolmaz_kurtarma_bir_kez_sayar() {
        let mut d = durum(10, 100);
        let a = d.baslat().unwrap();
        d.kurtar();
        d.kurtar();
        assert_eq!((d.baslatilan, d.bilinmeyen), (1, 1));
        assert_eq!(d.baslat(), Err(Engel::Bilinmeyen));
        assert_eq!(d.bitir(a, Some(7)), Err(Engel::Gecersiz));
    }

    #[test]
    fn ayni_sonuc_iki_kez_token_eklemez() {
        let mut d = durum(10, 100);
        let a = d.baslat().unwrap();
        d.bitir(a, Some(7)).unwrap();
        assert_eq!(d.bitir(a, Some(7)), Err(Engel::Gecersiz));
        assert_eq!(d.bitir(999, Some(9)), Err(Engel::Gecersiz));
        assert_eq!(d.token, 7);
    }

    #[test]
    fn devam_sayaclari_sifirlamaz_yeni_mutlak_sinir_ister() {
        let mut d = durum(1, 100);
        let a = d.baslat().unwrap();
        d.bitir(a, Some(20)).unwrap();
        assert_eq!(d.devam(0, d.sinirlar), Err(Engel::Gecersiz));
        d.devam(
            0,
            Sinirlar {
                istek: 2,
                token: 100,
            },
        )
        .unwrap();
        assert_eq!((d.baslatilan, d.token), (1, 20));
        assert!(d.baslat().is_ok());
    }

    #[test]
    fn token_tasmasi_kapiyi_acamaz() {
        let mut d = durum(10, u64::MAX);
        let a = d.baslat().unwrap();
        let b = d.baslat().unwrap();
        d.bitir(a, Some(u64::MAX - 1)).unwrap();
        d.bitir(b, Some(100)).unwrap();
        assert_eq!(d.token, u64::MAX);
        assert_eq!(d.baslat(), Err(Engel::Token));
    }

    #[test]
    fn izin_alicisi_kapanirsa_sonuc_kanali_bilinmeyeni_haber_verir() {
        let mut d = durum(10, 100);
        let id = d.baslat().unwrap();
        let (sonuc_tx, sonuc_rx) = mpsc::channel::<Option<u64>>();
        let (izin_tx, izin_rx) = mpsc::channel::<mpsc::Sender<Option<u64>>>();
        drop(izin_rx);
        // İzin teslim edilemezse taşınan sahipli sonuç göndericisi de bırakılır.
        drop(izin_tx.send(sonuc_tx).unwrap_err());
        assert!(sonuc_rx.recv().is_err());
        d.bitir(id, None).unwrap();
        assert_eq!((d.baslatilan, d.bilinmeyen), (1, 1));
        assert_eq!(d.baslat(), Err(Engel::Bilinmeyen));
    }
}
