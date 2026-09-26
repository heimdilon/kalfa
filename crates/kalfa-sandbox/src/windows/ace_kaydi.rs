//! ACE kaydı (aşama 2 tasarımı §9.4): uygulamanın eklediği her izin,
//! mirasını kestiği her nesnenin önceki DACL'i, oluşturduğu AppContainer
//! profilleri. Sağlam kayıtta geri alma bu kayda göre yapılır; bozuk kayıtta
//! yalnız çözümlenen `Izin` girdileriyle kanıtlanan ACE'ler geri alınır.
//!
//! Konum `<yalıtım dizini>\ace-kaydi.jsonl`; paket SID'lerine izin verilmez,
//! yalıtımda görünmez. Her satır bir girdidir ve işlemden **önce** yazılır:
//! sonunda satır sonu olmayan yarım son satır yapılmamış bir işlemdir ve
//! okurken atlanır. Başka her çözülemeyen satır kaydı bozuk yapar; o zaman
//! bilinmeyen DACL ve ACE'ler korunur, geri alma tamamlanmamış sayılır (§9.4).

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bütün çalışma alanlarının ortak girdileri (araç zinciri dizinleri).
pub const ORTAK: &str = "*";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case")]
pub enum Girdi {
    /// Çalışma alanının AppContainer profili.
    Profil { alan: String, ad: String },
    /// `sid`'e `yol` üzerinde verilen açık izin: maske ve devralma
    /// bayrakları (`guvenlik::MIRAS` gibi).
    Izin {
        alan: String,
        yol: PathBuf,
        sid: String,
        maske: u32,
        miras: u32,
    },
    /// `yol`'un mirası kesildi; `onceki` o anki DACL'dir (SDDL).
    MirasKesildi {
        alan: String,
        yol: PathBuf,
        onceki: String,
    },
    /// Çalışma alanının hazırlığı bitti; kök `kok`.
    Hazir { alan: String, kok: PathBuf },
}

impl Girdi {
    pub fn alan(&self) -> &str {
        match self {
            Girdi::Profil { alan, .. }
            | Girdi::Izin { alan, .. }
            | Girdi::MirasKesildi { alan, .. }
            | Girdi::Hazir { alan, .. } => alan,
        }
    }
}

#[derive(Debug)]
pub struct AceKaydi {
    yol: PathBuf,
    girdiler: Vec<Girdi>,
    bozuk: bool,
}

impl AceKaydi {
    /// Kaydı okur; yoksa boştur.
    ///
    /// - Son satır sonundan sonraki parça hiç okunmaz: dosya satır sonuyla
    ///   bitiyorsa boştur; bitmiyorsa yazılırken yarım kalmış bir girdidir ve
    ///   geçerli JSON olsa bile işlemi başlamamıştır.
    /// - Tamamlanmış her satır (boş satır dahil) bir girdi olarak
    ///   çözülmelidir; çözülemeyen satır ya da okunamayan dosya kaydı bozuk
    ///   yapar. Çözülebilen girdiler yine okunur.
    pub fn oku(yol: &Path) -> Self {
        let (metin, mut bozuk) = match std::fs::read_to_string(yol) {
            Ok(m) => (m, false),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (String::new(), false),
            Err(_) => (String::new(), true),
        };
        let mut satirlar: Vec<&str> = metin.split('\n').collect();
        satirlar.pop();
        let mut girdiler = Vec::new();
        for satir in satirlar {
            match serde_json::from_str(satir) {
                Ok(g) => girdiler.push(g),
                Err(_) => bozuk = true,
            }
        }
        Self {
            yol: yol.to_path_buf(),
            girdiler,
            bozuk,
        }
    }

    pub fn girdiler(&self) -> &[Girdi] {
        &self.girdiler
    }

    /// Kayıt bozuk mu? Bozuksa geri alma kayda güvenemez (§9.4).
    pub fn bozuk_mu(&self) -> bool {
        self.bozuk
    }

    /// Girdiyi kaydın sonuna yazar ve diske indirir; aynısı varsa yazmaz.
    pub fn ekle(&mut self, girdi: Girdi) -> io::Result<()> {
        if self.girdiler.contains(&girdi) {
            return Ok(());
        }
        if let Some(ust) = self.yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        let mut satir = serde_json::to_string(&girdi).map_err(io::Error::other)?;
        satir.push('\n');
        // Yarım son satır yapılmamış bir işlemin kaydıdır: kesilip atılır;
        // kalsaydı yeni satırla ortaya düşer ve kayıt bozuk görünürdü.
        let tam_uzunluk = tam_satirlarin_uzunlugu(&self.yol)?;
        if std::fs::metadata(&self.yol).is_ok_and(|m| m.len() != tam_uzunluk) {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&self.yol)?
                .set_len(tam_uzunluk)?;
        }
        let mut dosya = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.yol)?;
        dosya.write_all(satir.as_bytes())?;
        dosya.sync_all()?;
        self.girdiler.push(girdi);
        Ok(())
    }

    pub fn hazir_mi(&self, alan: &str) -> bool {
        self.girdiler
            .iter()
            .any(|g| matches!(g, Girdi::Hazir { alan: a, .. } if a == alan))
    }

    /// Hazırlanmış çalışma alanları: (kimlik, kök).
    pub fn hazir_alanlar(&self) -> Vec<(String, PathBuf)> {
        self.girdiler
            .iter()
            .filter_map(|g| match g {
                Girdi::Hazir { alan, kok } => Some((alan.clone(), kok.clone())),
                _ => None,
            })
            .collect()
    }

    /// Nesnenin ilk kaydedilen önceki DACL'i. Aynı nesnenin mirasını birden
    /// çok çalışma alanı kestiyse (örneğin iki worktree'nin ortak git dizini)
    /// geri yazılacak olan, Kalfa'dan önceki hâlidir.
    pub fn ilk_dacl(&self, yol: &Path) -> Option<&str> {
        self.girdiler.iter().find_map(|g| match g {
            Girdi::MirasKesildi { yol: y, onceki, .. } if y == yol => Some(onceki.as_str()),
            _ => None,
        })
    }

    /// Önceki DACL anlık görüntülerinden kaldırılmakta olan paketin SID'ini
    /// çıkarır. İç içe çalışma alanı kaldırılırken dış alanın eski SID'inin
    /// daha sonra DACL geri yüklemesiyle yeniden görünmesini önler.
    pub fn dacl_oncelerini_guncelle(
        &mut self,
        mut guncelle: impl FnMut(&str) -> io::Result<String>,
    ) -> io::Result<()> {
        let mut yeni = self.girdiler.clone();
        let mut degisti = false;
        for girdi in &mut yeni {
            if let Girdi::MirasKesildi { onceki, .. } = girdi {
                let sonraki = guncelle(onceki)?;
                degisti |= *onceki != sonraki;
                *onceki = sonraki;
            }
        }
        if !degisti {
            return Ok(());
        }

        let mut metin = String::new();
        for girdi in &yeni {
            metin.push_str(&serde_json::to_string(girdi).map_err(io::Error::other)?);
            metin.push('\n');
        }
        let gecici = self.yol.with_extension("jsonl.yeni");
        let mut dosya = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&gecici)?;
        dosya.write_all(metin.as_bytes())?;
        dosya.sync_all()?;
        drop(dosya);
        std::fs::rename(&gecici, &self.yol)?;
        self.girdiler = yeni;
        Ok(())
    }

    /// `yol`'da izni kayıtlı olan başka çalışma alanlarının SID'leri.
    pub fn baska_alanlarin_sidleri(&self, alan: &str, yol: &Path) -> Vec<String> {
        self.girdiler
            .iter()
            .filter_map(|g| match g {
                Girdi::Izin {
                    alan: a,
                    yol: y,
                    sid,
                    ..
                } if a != alan && a != ORTAK && y == yol => Some(sid.clone()),
                _ => None,
            })
            .collect()
    }

    /// Çalışma alanının girdilerini kayıttan çıkarır; `kalan` girdiler
    /// (geri alınamayanlar) kalır. Kayıt yeniden yazılır.
    pub fn alani_cikar(&mut self, alan: &str, kalan: &[Girdi]) -> io::Result<()> {
        let yeni: Vec<Girdi> = self
            .girdiler
            .iter()
            .filter(|g| g.alan() != alan || kalan.contains(g))
            .cloned()
            .collect();
        let mut metin = String::new();
        for g in &yeni {
            metin.push_str(&serde_json::to_string(g).map_err(io::Error::other)?);
            metin.push('\n');
        }
        let gecici = self.yol.with_extension("jsonl.yeni");
        std::fs::write(&gecici, metin)?;
        std::fs::rename(&gecici, &self.yol)?;
        self.girdiler = yeni;
        Ok(())
    }
}

/// Dosyanın son satır sonuna kadarki uzunluğu; dosya yoksa 0.
fn tam_satirlarin_uzunlugu(yol: &Path) -> io::Result<u64> {
    match std::fs::read(yol) {
        Ok(v) => Ok(v
            .iter()
            .rposition(|b| *b == b'\n')
            .map_or(0, |i| i as u64 + 1)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn izin(alan: &str, yol: &str, sid: &str) -> Girdi {
        Girdi::Izin {
            alan: alan.into(),
            yol: yol.into(),
            sid: sid.into(),
            maske: 0x0013_01BF,
            miras: 0x3,
        }
    }

    #[test]
    fn girdiler_eklenir_okunur_yinelenmez() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let mut k = AceKaydi::oku(&yol);
        k.ekle(izin("a", r"C:\x", "S-1-15-2-1")).unwrap();
        k.ekle(izin("a", r"C:\x", "S-1-15-2-1")).unwrap();
        k.ekle(Girdi::Hazir {
            alan: "a".into(),
            kok: r"C:\x".into(),
        })
        .unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 2);
        assert!(k.hazir_mi("a"));
        assert!(!k.hazir_mi("b"));
        assert_eq!(
            k.hazir_alanlar(),
            vec![("a".into(), PathBuf::from(r"C:\x"))]
        );
    }

    #[test]
    fn yarim_satir_atlanir_ve_sonraki_girdi_ayri_satira_yazilir() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\n{{\"tur\":\"izin\",\"al")).unwrap();
        let mut k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 1);
        assert!(!k.bozuk_mu());
        k.ekle(izin("b", r"C:\y", "S-2")).unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler().len(), 2);
        assert!(!k.bozuk_mu());
        // Yarım parça kesilip atıldı.
        let metin = std::fs::read_to_string(&yol).unwrap();
        assert!(!metin.contains("\"al\n"), "{metin}");
        assert_eq!(metin.lines().count(), 2);
    }

    /// Satır sonu yazılmamış son parça geçerli JSON olsa da okunmaz; sonraki
    /// `ekle` onu keser ve bellekteki girdiler diskteki kayıtla aynı kalır.
    #[test]
    fn satir_sonu_olmayan_gecerli_son_parca_okunmaz() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let ilk = izin("a", r"C:\x", "S-1");
        let yarim = izin("b", r"C:\y", "S-2");
        let tam = serde_json::to_string(&ilk).unwrap();
        let son = serde_json::to_string(&yarim).unwrap();
        std::fs::write(&yol, format!("{tam}\n{son}")).unwrap();
        let mut k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler(), std::slice::from_ref(&ilk));
        assert!(!k.bozuk_mu());
        // Aynı girdi yeniden yazılır; yinelenme denetimi onu atlamaz.
        k.ekle(yarim.clone()).unwrap();
        let diskteki = AceKaydi::oku(&yol);
        assert_eq!(diskteki.girdiler(), k.girdiler());
        assert_eq!(diskteki.girdiler(), &[ilk, yarim]);
        assert!(!diskteki.bozuk_mu());
        assert!(std::fs::read_to_string(&yol).unwrap().ends_with('\n'));
    }

    #[test]
    fn ortadaki_bos_satir_kaydi_bozuk_yapar() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\n\n{tam}\n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        std::fs::write(&yol, format!("{tam}\n   \n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        std::fs::write(&yol, "").unwrap();
        assert!(!AceKaydi::oku(&yol).bozuk_mu());
    }

    #[test]
    fn ortadaki_ya_da_tamamlanmis_bozuk_satir_kaydi_bozuk_yapar() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("ace-kaydi.jsonl");
        let tam = serde_json::to_string(&izin("a", r"C:\x", "S-1")).unwrap();
        std::fs::write(&yol, format!("{tam}\nbozuk\n{tam}\n")).unwrap();
        let k = AceKaydi::oku(&yol);
        assert!(k.bozuk_mu());
        assert_eq!(k.girdiler().len(), 2);
        std::fs::write(&yol, format!("{tam}\nbozuk\n")).unwrap();
        assert!(AceKaydi::oku(&yol).bozuk_mu());
        assert!(!AceKaydi::oku(&d.path().join("yok.jsonl")).bozuk_mu());
    }

    #[test]
    fn izin_girdisi_maske_ve_devralmayi_tasir() {
        let g = serde_json::to_value(izin("a", r"C:\x", "S-1")).unwrap();
        assert_eq!(g["maske"], 0x0013_01BF);
        assert_eq!(g["miras"], 3);
        assert_eq!(g["tur"], "izin");
    }

    #[test]
    fn ilk_dacl_ve_baska_alanlar() {
        let d = tempfile::tempdir().unwrap();
        let mut k = AceKaydi::oku(&d.path().join("k.jsonl"));
        for (alan, onceki) in [("a", "D:AI(ilk)"), ("b", "D:PAI(sonraki)")] {
            k.ekle(Girdi::MirasKesildi {
                alan: alan.into(),
                yol: r"C:\depo\.git\config".into(),
                onceki: onceki.into(),
            })
            .unwrap();
        }
        k.ekle(izin("b", r"C:\depo\.git\config", "S-B")).unwrap();
        k.ekle(izin(ORTAK, r"C:\depo\.git\config", "S-O")).unwrap();
        assert_eq!(
            k.ilk_dacl(Path::new(r"C:\depo\.git\config")),
            Some("D:AI(ilk)")
        );
        assert_eq!(
            k.baska_alanlarin_sidleri("a", Path::new(r"C:\depo\.git\config")),
            vec!["S-B".to_string()]
        );
        assert!(
            k.baska_alanlarin_sidleri("b", Path::new(r"C:\depo\.git\config"))
                .is_empty()
        );
    }

    #[test]
    fn alan_cikarilir_kalanlar_tutulur() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("k.jsonl");
        let mut k = AceKaydi::oku(&yol);
        let kalan = izin("a", r"C:\kalan", "S-1");
        for g in [
            izin("a", r"C:\x", "S-1"),
            kalan.clone(),
            izin("b", r"C:\y", "S-2"),
        ] {
            k.ekle(g).unwrap();
        }
        k.alani_cikar("a", std::slice::from_ref(&kalan)).unwrap();
        let k = AceKaydi::oku(&yol);
        assert_eq!(k.girdiler(), &[kalan, izin("b", r"C:\y", "S-2")]);
    }
}
