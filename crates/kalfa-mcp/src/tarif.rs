//! Sır taşımayan canonical MCP özetleri.
//!
//! T ve R burada üretilir. Π/K/P üreticileri ile süreç başına artan N sayacı
//! Görev 4/5 yöneticisine aittir; eksik görüntüden K üretilemez.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use kalfa_core::arac::{
    McpAracTanimOzeti, McpBaslatmaSayaci, McpKorumaOzeti, McpPolitikaOzeti, McpProfilOzeti,
    McpTanimOzeti, McpTarifi, McpYalitimProfiliOzeti,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::tanim::{CozulmusTanim, McpTanim, TanimHatasi, UzakMcpTanim, UzakYetkiTuru};

/// Onaydan sonra okunan dosyanın yalnız yolu ve SHA-256'sı tutulur.
/// Ham dosya içeriği bu tipe alınmaz; Debug/serde uygulanmaz.
pub struct DosyaIcerikOzeti {
    yol: PathBuf,
    sha256: [u8; 32],
}

impl DosyaIcerikOzeti {
    pub fn yeni(yol: PathBuf, sha256: [u8; 32]) -> Self {
        Self { yol, sha256 }
    }

    pub fn yol(&self) -> &std::path::Path {
        &self.yol
    }
}

fn alan(h: &mut Sha256, veri: &[u8]) {
    h.update((veri.len() as u64).to_le_bytes());
    h.update(veri);
}

/// Ham değer yalnızca SHA-256 akışına girer; canonical tarifte içerik yerine özeti taşınır.
pub fn dosya_icerik_ozeti(icerik: &[u8]) -> [u8; 32] {
    Sha256::digest(icerik).into()
}

fn gizli_deger_ozeti(deger: &str) -> [u8; 32] {
    Sha256::digest(deger.as_bytes()).into()
}

/// İçeriği tek Vec'e almadan, sabit 32 KiB bloklarla SHA-256 üretir.
pub fn dosya_akisi_ozeti(mut okuyucu: impl Read) -> io::Result<[u8; 32]> {
    let mut ozet = Sha256::new();
    let mut tampon = [0_u8; 32 * 1024];
    loop {
        let adet = match okuyucu.read(&mut tampon) {
            Ok(0) => break,
            Ok(adet) => adet,
            Err(hata) if hata.kind() == io::ErrorKind::Interrupted => continue,
            Err(hata) => return Err(hata),
        };
        ozet.update(&tampon[..adet]);
    }
    Ok(ozet.finalize().into())
}

fn yol_metni(yol: &Path) -> Result<&str, TanimHatasi> {
    yol.to_str().ok_or(TanimHatasi::YolCozumlenemedi)
}

/// T = ad, çözülmüş ikili ve içeriği, sıraya duyarlı argümanlar, son ortam,
/// kip/ağ, özel çalışma dizini, alan kimliği ve onay sonrası dosya özetleri.
pub fn tanim_ozeti(
    tanim: &McpTanim,
    cozulmus: &CozulmusTanim,
    dosyalar: &[DosyaIcerikOzeti],
    alan_kimligi: &str,
) -> Result<McpTanimOzeti, TanimHatasi> {
    let mut beklenen: Vec<_> = cozulmus.dosya_adaylari().iter().map(|d| d.yol()).collect();
    let mut gelen: Vec<_> = dosyalar.iter().map(|d| d.yol.as_path()).collect();
    beklenen.sort();
    gelen.sort();
    if beklenen != gelen {
        return Err(TanimHatasi::DosyaOkunamadi);
    }

    let ikili = dosyalar
        .iter()
        .find(|f| f.yol == cozulmus.program())
        .ok_or(TanimHatasi::DosyaOkunamadi)?;
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-tanim-v1");
    alan(&mut h, tanim.ad().as_bytes());
    alan(&mut h, &[cozulmus.komut_tipi()]);
    alan(&mut h, yol_metni(cozulmus.program())?.as_bytes());
    alan(&mut h, &ikili.sha256);
    alan(&mut h, yol_metni(cozulmus.calisma_dizini())?.as_bytes());
    alan(&mut h, alan_kimligi.as_bytes());
    alan(&mut h, b"argumanlar");
    alan(&mut h, &(cozulmus.argumanlar().len() as u64).to_le_bytes());
    for arg in cozulmus.argumanlar() {
        alan(&mut h, arg.as_bytes());
    }
    let mut ortam: Vec<_> = cozulmus.ortam().ham_degerler().collect();
    ortam.sort_by(|a, b| {
        let a = if cfg!(windows) {
            a.0.to_ascii_lowercase()
        } else {
            a.0.to_string()
        };
        let b = if cfg!(windows) {
            b.0.to_ascii_lowercase()
        } else {
            b.0.to_string()
        };
        a.cmp(&b)
    });
    alan(&mut h, b"ortam");
    alan(&mut h, &(ortam.len() as u64).to_le_bytes());
    for (ad, deger, gizli) in ortam {
        let ad = if cfg!(windows) {
            ad.to_ascii_lowercase()
        } else {
            ad.to_owned()
        };
        alan(&mut h, ad.as_bytes());
        alan(&mut h, &[u8::from(gizli)]);
        alan(&mut h, &gizli_deger_ozeti(deger));
    }
    alan(
        &mut h,
        &[match cozulmus.kip() {
            kalfa_sandbox::CalistirmaKipi::Yalitimli => 1,
            kalfa_sandbox::CalistirmaKipi::TamErisim => 2,
        }],
    );
    let mut ag = cozulmus.ag().to_vec();
    ag.sort();
    alan(&mut h, b"ag");
    alan(&mut h, &(ag.len() as u64).to_le_bytes());
    for kural in ag {
        alan(&mut h, kural.host().as_bytes());
        alan(&mut h, &kural.port().to_le_bytes());
        alan(&mut h, &[u8::from(kural.ozel_ag())]);
    }
    alan(&mut h, &cozulmus.timeoutlar().0.to_le_bytes());
    alan(&mut h, &cozulmus.timeoutlar().1.to_le_bytes());
    let mut dosyalar: Vec<_> = dosyalar.iter().collect();
    dosyalar.sort_by(|a, b| a.yol.cmp(&b.yol));
    alan(&mut h, b"dosyalar");
    alan(&mut h, &(dosyalar.len() as u64).to_le_bytes());
    for dosya in dosyalar {
        alan(&mut h, yol_metni(&dosya.yol)?.as_bytes());
        alan(&mut h, &dosya.sha256);
    }
    Ok(McpTanimOzeti::ozetten(h.finalize().into()))
}

/// Π: Çalışma alanı dışındaki yalıtım arka ucu / sürümü, gizli kalıp kuralları
/// ve ek okunur dizinleri kanonik olarak özetler.
pub fn politika_ozeti(
    yalitim_durumu: &kalfa_sandbox::YalitimDurumu,
    gizli_ayarlari: &kalfa_core::izin::GizliAyarlari,
    ek_okunur_dizinler: &[PathBuf],
) -> McpPolitikaOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-politika-v1");
    match yalitim_durumu {
        kalfa_sandbox::YalitimDurumu::Hazir { arka_uc, surum } => {
            alan(&mut h, &[1]);
            alan(&mut h, arka_uc.as_bytes());
            alan(&mut h, surum.as_bytes());
        }
        kalfa_sandbox::YalitimDurumu::Yok {
            neden,
            talimat,
            ayrinti,
        } => {
            alan(&mut h, &[0]);
            alan(&mut h, format!("{neden:?}").as_bytes());
            alan(&mut h, talimat.as_bytes());
            alan(&mut h, ayrinti.as_bytes());
        }
    }
    let mut ek_kaliplar = gizli_ayarlari.ek_kaliplar();
    ek_kaliplar.sort();
    ek_kaliplar.dedup();
    alan(&mut h, b"ek_gizli_kaliplar");
    alan(&mut h, &(ek_kaliplar.len() as u64).to_le_bytes());
    for kalip in &ek_kaliplar {
        alan(&mut h, kalip.as_bytes());
    }
    let mut istisnalar = gizli_ayarlari.istisnalar();
    istisnalar.sort();
    istisnalar.dedup();
    alan(&mut h, b"gizli_sayilmayanlar");
    alan(&mut h, &(istisnalar.len() as u64).to_le_bytes());
    for istisna in &istisnalar {
        alan(&mut h, istisna.as_bytes());
    }
    let mut sirali_okunur: Vec<String> = ek_okunur_dizinler
        .iter()
        .map(|d| d.to_string_lossy().into_owned())
        .collect();
    sirali_okunur.sort();
    sirali_okunur.dedup();
    alan(&mut h, b"okunur_dizinler");
    alan(&mut h, &(sirali_okunur.len() as u64).to_le_bytes());
    for dizin in &sirali_okunur {
        alan(&mut h, dizin.as_bytes());
    }
    McpPolitikaOzeti::ozetten(h.finalize().into())
}

/// Çalışma kipini, yalıtıcı profil özetini ve çalışma alanı profil özetini
/// `McpYalitimProfiliOzeti` içine bağlar.
pub fn yalitim_profili_ozeti(
    yalitici: &dyn kalfa_sandbox::Yalitici,
    kip: kalfa_sandbox::CalistirmaKipi,
    alan_bilgisi: &kalfa_sandbox::AlanBilgisi,
    ag_ozeti: Option<&str>,
) -> McpYalitimProfiliOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-yalitim-profili-v1");
    alan(
        &mut h,
        &[match kip {
            kalfa_sandbox::CalistirmaKipi::Yalitimli => 1,
            kalfa_sandbox::CalistirmaKipi::TamErisim => 2,
        }],
    );
    let yalitici_ozeti = yalitici
        .profil_ozeti(ag_ozeti.is_some(), ag_ozeti)
        .unwrap_or_default();
    alan(&mut h, yalitici_ozeti.as_bytes());
    alan(&mut h, alan_bilgisi.kok.as_os_str().as_encoded_bytes());
    if let Some(ag) = ag_ozeti {
        alan(&mut h, &[1]);
        alan(&mut h, ag.as_bytes());
    } else {
        alan(&mut h, &[0]);
    }
    McpYalitimProfiliOzeti::ozetten(h.finalize().into())
}

/// P = Π + K + yalıtım profili. Görev 4/5 doğrulanmış girdileri sağlayacaktır;
/// bu fonksiyon mevcut özetleri tek canonical profile bağlar.
pub fn profil_ozeti(
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    yalitim_profili: McpYalitimProfiliOzeti,
) -> McpProfilOzeti {
    McpProfilOzeti::birlestir(politika, koruma, yalitim_profili)
}

/// R = T + P + N. Π/K girdilerinin üretim doğrulaması Görev 4/5'tedir.
pub fn calisma_tarifi(
    tanim: McpTanimOzeti,
    politika: McpPolitikaOzeti,
    koruma: McpKorumaOzeti,
    yalitim_profili: McpYalitimProfiliOzeti,
    baslatma: McpBaslatmaSayaci,
) -> McpTarifi {
    McpTarifi::yeni(tanim, politika, koruma, yalitim_profili, baslatma)
}

/// D, araca modele sunulan tam ad/açıklama/girdi/çıkış şeması içindir.
/// Açıklamanın kırpılmış değeri çağıran tarafından verilmelidir.
pub fn arac_tanim_ozeti(
    ad: &str,
    aciklama: &str,
    girdi_semasi: &Value,
    cikti_semasi: Option<&Value>,
) -> McpAracTanimOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-arac-tanimi-v1");
    alan(&mut h, ad.as_bytes());
    alan(&mut h, aciklama.as_bytes());
    kanonik_json(&mut h, girdi_semasi);
    if let Some(sema) = cikti_semasi {
        alan(&mut h, b"cikti-var");
        kanonik_json(&mut h, sema);
    } else {
        alan(&mut h, b"cikti-yok");
    }
    McpAracTanimOzeti::ozetten(h.finalize().into())
}

/// Uzak MCP sunucusu için T özeti (`1.1b`).
///
/// Yerel `kalfa-mcp-tanim-v1` alan etiketinden ayrı `kalfa-mcp-uzak-tanim-v1`
/// kullanır; sahte PID, yerel ikili yolu ya da yalıtım kipi üretmez.
/// Kanonik uç URL'si, normalize origin, alan kimliği, tanım revizyonu, kimlik
/// revizyonu, yetki türü (yalnız opak kimlikler ve kapsamlar; ham token girmez),
/// izinli başlık kimlikleri, zaman aşımları ve bağlam bütçesi özetlenir.
pub fn uzak_tanim_ozeti(
    tanim: &UzakMcpTanim,
    alan_kimligi: &str,
    tanim_revizyonu: u64,
    kimlik_revizyonu: u64,
) -> McpTanimOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-uzak-tanim-v1");
    alan(&mut h, tanim.ad().as_bytes());
    alan(&mut h, tanim.uc_url().as_bytes());
    alan(&mut h, tanim.origin().as_bytes());
    alan(&mut h, alan_kimligi.as_bytes());
    alan(&mut h, &tanim_revizyonu.to_le_bytes());
    alan(&mut h, &kimlik_revizyonu.to_le_bytes());
    match tanim.yetki_turu() {
        UzakYetkiTuru::Yetkisiz => {
            alan(&mut h, b"yetkisiz");
        }
        UzakYetkiTuru::SabitBaslikGizli {
            baslik_adi,
            gizli_kimlik,
        } => {
            alan(&mut h, b"sabit-baslik-gizli");
            alan(&mut h, baslik_adi.as_bytes());
            alan(&mut h, gizli_kimlik.as_bytes());
        }
        UzakYetkiTuru::OAuth {
            istemci_kimligi,
            kapsamlar,
            kaynak_url,
            yetki_kimligi,
        } => {
            alan(&mut h, b"oauth");
            alan(
                &mut h,
                istemci_kimligi.as_deref().unwrap_or_default().as_bytes(),
            );
            alan(&mut h, &(kapsamlar.len() as u64).to_le_bytes());
            for kapsam in kapsamlar {
                alan(&mut h, kapsam.as_bytes());
            }
            alan(&mut h, kaynak_url.as_deref().unwrap_or_default().as_bytes());
            alan(
                &mut h,
                yetki_kimligi.as_deref().unwrap_or_default().as_bytes(),
            );
        }
    }
    alan(&mut h, b"izinli-baslik-kimlikleri");
    alan(
        &mut h,
        &(tanim.izinli_baslik_kimlikleri().len() as u64).to_le_bytes(),
    );
    for kimlik in tanim.izinli_baslik_kimlikleri() {
        alan(&mut h, kimlik.as_bytes());
    }
    let (baslatma_sn, cagri_sn) = tanim.timeoutlar();
    alan(&mut h, &baslatma_sn.to_le_bytes());
    alan(&mut h, &cagri_sn.to_le_bytes());
    alan(&mut h, &[u8::from(tanim.test_fiksturu_loopback())]);
    let butce = tanim.baglam_butcesi();
    alan(&mut h, &(butce.aday_arac_siniri() as u64).to_le_bytes());
    alan(&mut h, &(butce.etkin_arac_siniri() as u64).to_le_bytes());
    alan(&mut h, &(butce.sema_bayt_siniri() as u64).to_le_bytes());
    McpTanimOzeti::ozetten(h.finalize().into())
}

/// Uzak MCP bağlantısı için Π (politika) özeti.
///
/// Yerel AppContainer / bwrap yalıtımı varmış gibi davranmaz; yalnız gizli kalıp
/// kurallarını ve uzak origin ağ politikasını bağlar.
pub fn uzak_politika_ozeti(
    gizli_ayarlari: &kalfa_core::izin::GizliAyarlari,
    origin: &str,
    test_fiksturu_loopback: bool,
) -> McpPolitikaOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-uzak-politika-v1");
    alan(&mut h, origin.as_bytes());
    alan(&mut h, &[u8::from(test_fiksturu_loopback)]);
    let mut ek_kaliplar = gizli_ayarlari.ek_kaliplar();
    ek_kaliplar.sort();
    ek_kaliplar.dedup();
    alan(&mut h, &(ek_kaliplar.len() as u64).to_le_bytes());
    for kalip in &ek_kaliplar {
        alan(&mut h, kalip.as_bytes());
    }
    let mut istisnalar = gizli_ayarlari.istisnalar();
    istisnalar.sort();
    istisnalar.dedup();
    alan(&mut h, &(istisnalar.len() as u64).to_le_bytes());
    for istisna in &istisnalar {
        alan(&mut h, istisna.as_bytes());
    }
    McpPolitikaOzeti::ozetten(h.finalize().into())
}

/// Uzak MCP bağlantısı için K (bağlantı koruma) özeti.
///
/// Sahte yerel kod kökü veya PID üretmez; kanonik uç URL'si, müzakere edilen
/// protokol sürümü ve kimlik revizyonunu bağlar.
pub fn uzak_baglanti_koruma_ozeti(
    uc_url: &str,
    protokol_surumu: &str,
    kimlik_revizyonu: u64,
) -> McpKorumaOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-uzak-baglanti-koruma-v1");
    alan(&mut h, uc_url.as_bytes());
    alan(&mut h, protokol_surumu.as_bytes());
    alan(&mut h, &kimlik_revizyonu.to_le_bytes());
    McpKorumaOzeti::ozetten(h.finalize().into())
}

/// Uzak MCP bağlantısı için ağ profili özeti (`McpYalitimProfiliOzeti` yuvası).
///
/// Yerel yalıtım profiliyle (`kalfa-mcp-yalitim-profili-v1`) asla çakışmaz;
/// uzak origin ve izinli başlık kimliklerini bağlar.
pub fn uzak_ag_profili_ozeti(
    origin: &str,
    izinli_baslik_kimlikleri: &[String],
) -> McpYalitimProfiliOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-uzak-ag-profili-v1");
    alan(&mut h, origin.as_bytes());
    alan(
        &mut h,
        &(izinli_baslik_kimlikleri.len() as u64).to_le_bytes(),
    );
    for kimlik in izinli_baslik_kimlikleri {
        alan(&mut h, kimlik.as_bytes());
    }
    McpYalitimProfiliOzeti::ozetten(h.finalize().into())
}

/// Uzak MCP sunucusu için R (`McpTarifi`) özeti.
///
/// Sunucu kimliği, kanonik uç/origin, tanım revizyonu, kimlik revizyonu,
/// müzakere edilen protokol sürümü ve bağlantı nesil sayacından biri değişirse
/// R değişir ve eski onay mühürleri geçersiz kalır.
pub fn uzak_calisma_tarifi(
    tanim: &UzakMcpTanim,
    alan_kimligi: &str,
    gizli_ayarlari: &kalfa_core::izin::GizliAyarlari,
    tanim_revizyonu: u64,
    kimlik_revizyonu: u64,
    protokol_surumu: &str,
    baglanti_sayaci: McpBaslatmaSayaci,
) -> McpTarifi {
    let t = uzak_tanim_ozeti(tanim, alan_kimligi, tanim_revizyonu, kimlik_revizyonu);
    let origin = tanim.origin();
    let pi = uzak_politika_ozeti(gizli_ayarlari, &origin, tanim.test_fiksturu_loopback());
    let k = uzak_baglanti_koruma_ozeti(tanim.uc_url(), protokol_surumu, kimlik_revizyonu);
    let ag_profili = uzak_ag_profili_ozeti(&origin, tanim.izinli_baslik_kimlikleri());
    calisma_tarifi(t, pi, k, ag_profili, baglanti_sayaci)
}

/// Uzak MCP aracı için D (`McpAracTanimOzeti`) özeti.
///
/// Sunucu kimliği, normalize origin, araç tanım revizyonu, kırpılmış açıklama,
/// `inputSchema` ve varsa `outputSchema` değerlerini bağlar.
pub fn uzak_arac_tanim_ozeti(
    sunucu_adi: &str,
    origin: &str,
    arac_revizyonu: u64,
    ad: &str,
    aciklama: &str,
    girdi_semasi: &Value,
    cikti_semasi: Option<&Value>,
) -> McpAracTanimOzeti {
    let mut h = Sha256::new();
    alan(&mut h, b"kalfa-mcp-uzak-arac-tanimi-v1");
    alan(&mut h, sunucu_adi.as_bytes());
    alan(&mut h, origin.as_bytes());
    alan(&mut h, &arac_revizyonu.to_le_bytes());
    alan(&mut h, ad.as_bytes());
    alan(&mut h, aciklama.as_bytes());
    kanonik_json(&mut h, girdi_semasi);
    if let Some(sema) = cikti_semasi {
        alan(&mut h, b"cikti-var");
        kanonik_json(&mut h, sema);
    } else {
        alan(&mut h, b"cikti-yok");
    }
    McpAracTanimOzeti::ozetten(h.finalize().into())
}

fn kanonik_json(h: &mut Sha256, deger: &Value) {
    match deger {
        Value::Null => alan(h, b"null"),
        Value::Bool(v) => alan(h, if *v { b"true" } else { b"false" }),
        Value::Number(v) => {
            alan(h, b"number");
            alan(h, v.to_string().as_bytes());
        }
        Value::String(v) => {
            alan(h, b"string");
            alan(h, v.as_bytes());
        }
        Value::Array(degerler) => {
            alan(h, b"array");
            alan(h, &(degerler.len() as u64).to_le_bytes());
            for oge in degerler {
                kanonik_json(h, oge);
            }
        }
        Value::Object(degerler) => {
            alan(h, b"object");
            alan(h, &(degerler.len() as u64).to_le_bytes());
            let mut anahtarlar: Vec<_> = degerler.keys().collect();
            anahtarlar.sort();
            for anahtar in anahtarlar {
                alan(h, anahtar.as_bytes());
                if let Some(oge) = degerler.get(anahtar) {
                    kanonik_json(h, oge);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ortam::OrtamDegeri,
        tanim::{AgIzinKurali, KodGuveni, Komut, SeciliYurutulebilirKimligi, YazilabilirKokler},
    };
    use kalfa_sandbox::{CalistirmaKipi, OzelDizinler};

    #[derive(Clone, Copy)]
    enum KodYeri {
        Sunucu,
        BoslukluSunucu,
        Proje,
        Kabuk,
    }
    #[derive(Clone, Copy)]
    enum Degisim {
        Yok,
        Ad,
        DuzOrtam,
        GizliOrtam,
        NodeOptions,
        Path,
        Kip,
        Ag,
        Arguman,
        BaslatmaZamani,
        CagriZamani,
    }

    fn hazir_tanim(
        kok: &Path,
        yer: KodYeri,
        degisim: Degisim,
    ) -> (McpTanim, CozulmusTanim, PathBuf) {
        let proje = kok.join("proje");
        let yalitim = kok.join("yalitim");
        let ozel = kalfa_sandbox::sunucu_ozel(&yalitim, "alan", "ornek");
        let kabuk = OzelDizinler::yol(&yalitim, "alan");
        let bin = kok.join("bin");
        let bin2 = kok.join("bin2");
        for dizin in [
            &proje,
            &ozel.ev,
            &ozel.gecici,
            &ozel.onbellek,
            &kabuk.ev,
            &kabuk.gecici,
            &kabuk.onbellek,
            &bin,
            &bin2,
        ] {
            std::fs::create_dir_all(dizin).unwrap();
        }
        let ikili = bin.join(if cfg!(windows) {
            "deneme.exe"
        } else {
            "deneme"
        });
        std::fs::write(&ikili, b"ikili").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&ikili, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let (betik, arguman) = match yer {
            KodYeri::Sunucu => (ozel.ev.join("sunucu.js"), "sunucu.js".to_string()),
            KodYeri::BoslukluSunucu => {
                (ozel.ev.join("sunucu kodu.js"), "sunucu kodu.js".to_string())
            }
            KodYeri::Proje => (
                proje.join("sunucu.js"),
                "${CALISMA_ALANI}/sunucu.js".to_string(),
            ),
            KodYeri::Kabuk => {
                let yol = kabuk.ev.join("sunucu.js");
                let arguman = yol.to_str().unwrap().to_owned();
                (yol, arguman)
            }
        };
        std::fs::write(&betik, b"aaaa").unwrap();
        let mut argumanlar = vec![arguman];
        if matches!(degisim, Degisim::Arguman) {
            argumanlar.push("--renk=evet".into());
        }
        let tanim = McpTanim::yeni(
            if matches!(degisim, Degisim::Ad) {
                "ornek-iki"
            } else {
                "ornek"
            }
            .into(),
            Komut::SeciliYurutulebilir {
                kimlik: SeciliYurutulebilirKimligi::yeni(7),
                argumanlar,
            },
            vec![
                OrtamDegeri::gizli(
                    "GITHUB_TOKEN",
                    if matches!(degisim, Degisim::GizliOrtam) {
                        "sir-degisti"
                    } else {
                        "sir-ozgun"
                    },
                ),
                OrtamDegeri::duz(
                    "DUZ",
                    if matches!(degisim, Degisim::DuzOrtam) {
                        "iki"
                    } else {
                        "bir"
                    },
                ),
                OrtamDegeri::duz(
                    "NODE_OPTIONS",
                    if matches!(degisim, Degisim::NodeOptions) {
                        "--trace-warnings"
                    } else {
                        "--no-warnings"
                    },
                ),
            ],
            if matches!(degisim, Degisim::Kip) {
                CalistirmaKipi::TamErisim
            } else {
                CalistirmaKipi::Yalitimli
            },
            if matches!(degisim, Degisim::Ag) {
                Some(vec![AgIzinKurali::yeni("example.com:443", false).unwrap()])
            } else {
                None
            },
            if matches!(degisim, Degisim::BaslatmaZamani) {
                31
            } else {
                30
            },
            if matches!(degisim, Degisim::CagriZamani) {
                121
            } else {
                120
            },
        )
        .unwrap();
        let path = std::env::join_paths(if matches!(degisim, Degisim::Path) {
            vec![bin, bin2]
        } else {
            vec![bin]
        })
        .unwrap();
        let sistem = vec![("PATH".into(), path.to_str().unwrap().to_owned())];
        let kokler = YazilabilirKokler::yeni(proje, kabuk);
        let cozucu = move |_: SeciliYurutulebilirKimligi| Ok(ikili.clone());
        let cozulmus = tanim.hazirla(&sistem, &ozel, &kokler, &cozucu).unwrap();
        (tanim, cozulmus, betik)
    }

    fn icerikleri_ozetle(cozulmus: &CozulmusTanim) -> Vec<DosyaIcerikOzeti> {
        cozulmus
            .dosya_adaylari()
            .iter()
            .map(|aday| {
                DosyaIcerikOzeti::yeni(
                    aday.yol().to_owned(),
                    dosya_akisi_ozeti(std::fs::File::open(aday.yol()).unwrap()).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn ayni_boyutlu_ve_bosluklu_kod_dosyasi_degisiminde_t_degisir() {
        for yer in [KodYeri::Sunucu, KodYeri::BoslukluSunucu] {
            let kok = tempfile::tempdir().unwrap();
            let (tanim, cozulmus, betik) = hazir_tanim(kok.path(), yer, Degisim::Yok);
            let ilk =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            std::fs::write(&betik, b"bbbb").unwrap();
            let son =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            assert_ne!(ilk, son);
        }
    }

    #[test]
    fn tanim_ozeti_ortam_kip_ag_path_alan_ve_komut_ayrintilarina_baglidir() {
        let kok = tempfile::tempdir().unwrap();
        let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, Degisim::Yok);
        let ilk = tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
        for degisim in [
            Degisim::Ad,
            Degisim::DuzOrtam,
            Degisim::GizliOrtam,
            Degisim::NodeOptions,
            Degisim::Path,
            Degisim::Kip,
            Degisim::Ag,
            Degisim::Arguman,
            Degisim::BaslatmaZamani,
            Degisim::CagriZamani,
        ] {
            let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, degisim);
            let son =
                tanim_ozeti(&tanim, &cozulmus, &icerikleri_ozetle(&cozulmus), "alan").unwrap();
            assert_ne!(ilk, son);
        }
        let farkli_alan = tanim_ozeti(
            &tanim,
            &cozulmus,
            &icerikleri_ozetle(&cozulmus),
            "baska-alan",
        )
        .unwrap();
        assert_ne!(ilk, farkli_alan);
    }

    #[test]
    fn w_proje_ve_kabuk_koklerini_kapsar_sunucunun_ozel_koku_disarida_kalir() {
        for (yer, beklenen) in [
            (KodYeri::Proje, KodGuveni::WdeKodlu),
            (KodYeri::Kabuk, KodGuveni::WdeKodlu),
            (KodYeri::Sunucu, KodGuveni::WDisindaKodlu),
        ] {
            let kok = tempfile::tempdir().unwrap();
            let (_, cozulmus, betik) = hazir_tanim(kok.path(), yer, Degisim::Yok);
            assert_eq!(cozulmus.kod_guveni(), beklenen);
            if matches!(yer, KodYeri::Sunucu) {
                assert_eq!(
                    cozulmus.calisma_dizini(),
                    betik.parent().unwrap().canonicalize().unwrap()
                );
            }
        }
    }

    #[test]
    fn eksik_dosya_ozeti_kismi_tanim_uretmez() {
        let kok = tempfile::tempdir().unwrap();
        let (tanim, cozulmus, _) = hazir_tanim(kok.path(), KodYeri::Sunucu, Degisim::Yok);
        assert!(matches!(
            tanim_ozeti(&tanim, &cozulmus, &[], "alan"),
            Err(TanimHatasi::DosyaOkunamadi)
        ));
    }

    #[test]
    fn d_nesne_sirasindan_etkilenmez_cikti_semasi_ve_aciklama_degisiminden_etkilenir() {
        let a: Value = serde_json::from_str(
            r#"{"type":"object","properties":{"x":{"type":"string"},"y":{"type":"integer"}}}"#,
        )
        .unwrap();
        let b: Value = serde_json::from_str(
            r#"{"properties":{"y":{"type":"integer"},"x":{"type":"string"}},"type":"object"}"#,
        )
        .unwrap();
        let ilk = arac_tanim_ozeti("arac", "aciklama", &a, None);
        assert_eq!(ilk, arac_tanim_ozeti("arac", "aciklama", &b, None));
        assert_ne!(ilk, arac_tanim_ozeti("arac", "farkli", &a, None));
        assert_ne!(
            ilk,
            arac_tanim_ozeti("arac", "aciklama", &a, Some(&Value::Null))
        );
        assert_ne!(ilk, arac_tanim_ozeti("baska", "aciklama", &a, None));
    }

    #[test]
    fn akis_ozeti_sha256_ile_ayni_olur_okuma_hatasi_yutulmaz() {
        let veri = vec![b'x'; 100_001];
        assert_eq!(
            dosya_akisi_ozeti(std::io::Cursor::new(&veri)).unwrap(),
            dosya_icerik_ozeti(&veri)
        );
        struct Hata;
        impl Read for Hata {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("sahte okuma hatası"))
            }
        }
        assert!(dosya_akisi_ozeti(Hata).is_err());
    }

    #[test]
    fn uzak_tarif_ve_arac_ozeti_host_kimlik_revizyonu_ve_output_schema_degisimine_baglidir() {
        let gizli = kalfa_core::izin::GizliAyarlari::yeni(&[], &[]).unwrap();
        let baz_tanim = UzakMcpTanim::varsayilan_zaman_asimlari(
            "uzak-sunucu".into(),
            "https://api.example.com/mcp".into(),
            UzakYetkiTuru::OAuth {
                istemci_kimligi: Some("istemci-1".into()),
                kapsamlar: vec!["read".into()],
                kaynak_url: Some("https://api.example.com/mcp".into()),
                yetki_kimligi: Some("oauth-kimlik-1".into()),
            },
        )
        .unwrap();

        let baz_r = uzak_calisma_tarifi(
            &baz_tanim,
            "alan-1",
            &gizli,
            1,
            10,
            "2026-07-28",
            McpBaslatmaSayaci::yeni(1),
        );

        // Tanım revizyonu, kimlik revizyonu, protokol sürümü veya bağlantı nesli değişirse R değişmeli.
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &baz_tanim,
                "alan-1",
                &gizli,
                2,
                10,
                "2026-07-28",
                McpBaslatmaSayaci::yeni(1),
            )
        );
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &baz_tanim,
                "alan-1",
                &gizli,
                1,
                11,
                "2026-07-28",
                McpBaslatmaSayaci::yeni(1),
            )
        );
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &baz_tanim,
                "alan-1",
                &gizli,
                1,
                10,
                "2025-11-25",
                McpBaslatmaSayaci::yeni(1),
            )
        );
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &baz_tanim,
                "alan-1",
                &gizli,
                1,
                10,
                "2026-07-28",
                McpBaslatmaSayaci::yeni(2),
            )
        );

        // Host veya kapsam değişirse T ve R değişmeli.
        let baska_host = UzakMcpTanim::varsayilan_zaman_asimlari(
            "uzak-sunucu".into(),
            "https://baska.example.com/mcp".into(),
            baz_tanim.yetki_turu().clone(),
        )
        .unwrap();
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &baska_host,
                "alan-1",
                &gizli,
                1,
                10,
                "2026-07-28",
                McpBaslatmaSayaci::yeni(1),
            )
        );

        let genis_kapsam = UzakMcpTanim::varsayilan_zaman_asimlari(
            "uzak-sunucu".into(),
            "https://api.example.com/mcp".into(),
            UzakYetkiTuru::OAuth {
                istemci_kimligi: Some("istemci-1".into()),
                kapsamlar: vec!["read".into(), "write".into()],
                kaynak_url: Some("https://api.example.com/mcp".into()),
                yetki_kimligi: Some("oauth-kimlik-1".into()),
            },
        )
        .unwrap();
        assert_ne!(
            baz_r,
            uzak_calisma_tarifi(
                &genis_kapsam,
                "alan-1",
                &gizli,
                1,
                10,
                "2026-07-28",
                McpBaslatmaSayaci::yeni(1),
            )
        );

        // Uzak araç tanımı özeti (D): sunucu, origin, araç revizyonu ve outputSchema'ya bağlıdır;
        // yerel arac_tanim_ozeti ile çakışmaz.
        let girdi: Value = serde_json::from_str(r#"{"type":"object"}"#).unwrap();
        let cikti_1: Value =
            serde_json::from_str(r#"{"type":"object","properties":{"ok":{"type":"boolean"}}}"#)
                .unwrap();
        let cikti_2: Value =
            serde_json::from_str(r#"{"type":"object","properties":{"ok":{"type":"string"}}}"#)
                .unwrap();

        let yerel_d = arac_tanim_ozeti("sorgula", "açıklama", &girdi, Some(&cikti_1));
        let uzak_d = uzak_arac_tanim_ozeti(
            "uzak-sunucu",
            "https://api.example.com",
            1,
            "sorgula",
            "açıklama",
            &girdi,
            Some(&cikti_1),
        );
        assert_ne!(yerel_d, uzak_d);
        assert_ne!(
            uzak_d,
            uzak_arac_tanim_ozeti(
                "uzak-sunucu",
                "https://api.example.com",
                2,
                "sorgula",
                "açıklama",
                &girdi,
                Some(&cikti_1),
            )
        );
        assert_ne!(
            uzak_d,
            uzak_arac_tanim_ozeti(
                "uzak-sunucu",
                "https://api.example.com",
                1,
                "sorgula",
                "açıklama",
                &girdi,
                Some(&cikti_2),
            )
        );
        assert_ne!(
            uzak_d,
            uzak_arac_tanim_ozeti(
                "uzak-sunucu",
                "https://api.example.com",
                1,
                "sorgula",
                "açıklama",
                &girdi,
                None,
            )
        );
    }
}
