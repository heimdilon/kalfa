//! Ham MCP tanımından güvenli ve sürümlenmiş araç kartı üretir.
//!
//! İlk işlem sır taramasıdır. Reddetme yalnız 1 tabanlı sıra numarası taşır;
//! ham RMCP tanımı ve hata ayrıntıları bu modülden çıkmaz.

use std::sync::Arc;

use kalfa_core::arac::{McpAracTanimOzeti, McpJsonSemasi, McpKartEtiketi};
use kalfa_core::gorunum::gorunmez_var_mi;
use serde_json::Value;

use crate::arac::{McpAracRpcHataDonusturucu, McpAracSonucDonusturucu, McpAracSunumu};
use crate::maske::GizliDeger;
use crate::tarif::arac_tanim_ozeti;

const ACIKLAMA_AZAMI_BAYT: usize = 1024;

/// Red edilen araç için yalnız ham listedeki 1 tabanlı sıra.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AracTanimReddi {
    sira: usize,
}

/// Yalnız tanım kapılarından geçmiş alanları tutar. Ham Tool, ham şema hata
/// metni veya serileştirilebilir sır listesi bu tipte bulunmaz.
#[derive(Clone)]
pub struct DenetlenmisAracTanimi {
    sunucu_etiketi: McpKartEtiketi,
    arac_etiketi: McpKartEtiketi,
    sunum: McpAracSunumu,
    d: McpAracTanimOzeti,
    girdi_semasi: Arc<McpJsonSemasi>,
    cikti_semasi: Option<Arc<McpJsonSemasi>>,
    cikti_semasi_degeri: Option<Value>,
    denetim_metni: String,
    sirlar: Vec<GizliDeger>,
}

impl DenetlenmisAracTanimi {
    pub fn sunum(&self) -> &McpAracSunumu {
        &self.sunum
    }

    pub fn sunum_kopyasi(&self) -> McpAracSunumu {
        self.sunum.clone()
    }

    pub fn sunucu_etiketi(&self) -> &McpKartEtiketi {
        &self.sunucu_etiketi
    }

    pub fn arac_etiketi(&self) -> &McpKartEtiketi {
        &self.arac_etiketi
    }

    pub fn d(&self) -> McpAracTanimOzeti {
        self.d
    }

    pub fn girdi_semasi(&self) -> Arc<McpJsonSemasi> {
        Arc::clone(&self.girdi_semasi)
    }

    pub fn cikti_semasi_degeri(&self) -> Option<&Value> {
        self.cikti_semasi_degeri.as_ref()
    }

    pub fn argumanlar_uygun_mu(&self, argumanlar: &Value) -> bool {
        self.girdi_semasi.uygun_mu(argumanlar)
    }

    /// Yalnız inputSchema'nın metinlerini ve anahtarlarını içerir. Çıkış
    /// şeması D'ye girer; denetçinin input tarifi gibi sunulmaz.
    pub fn denetim_metni(&self) -> &str {
        &self.denetim_metni
    }

    /// G6 çağrı kartının zorunlu, sır maskeli sonuç dönüştürücüsünü üretir.
    pub fn sonuc_donusturucusu(&self) -> McpAracSonucDonusturucu {
        let kaynak = self.sunum.ad.clone();
        let sirlar = self.sirlar.clone();
        let cikti_semasi = self.cikti_semasi.clone();

        Arc::new(move |ham_sonuc, content_alani_var| {
            if let Some(sema) = &cikti_semasi {
                let dogrulayici = |deger: &Value| sema.uygun_mu(deger);
                crate::sonuc::sonucu_donustur(
                    &ham_sonuc,
                    content_alani_var,
                    &kaynak,
                    &sirlar,
                    Some(&dogrulayici),
                )
            } else {
                crate::sonuc::sonucu_donustur(&ham_sonuc, content_alani_var, &kaynak, &sirlar, None)
            }
        })
    }

    /// Çağrı kartının zorunlu, sır maskeli JSON-RPC protokol hatası dönüştürücüsünü üretir.
    pub fn rpc_hata_donusturucusu(&self) -> McpAracRpcHataDonusturucu {
        let kaynak = self.sunum.ad.clone();
        let sirlar = self.sirlar.clone();

        Arc::new(move |rpc| crate::sonuc::uzak_rpc_hatasini_donustur(rpc, &kaynak, &sirlar))
    }
}

/// Bilinen sırları bütün ham tanım alanlarında arar, sonra ad/schema kapılarını
/// açar. Çağıran her ham araç için bu işlevi bir kez çalıştırıp ham listeyi
/// hemen bırakmalıdır.
pub(crate) fn tanimi_denetle(
    sira: usize,
    sunucu_model_adi: &str,
    uzak_ad: &str,
    uzak_aciklama: Option<&str>,
    girdi_semasi: &Value,
    cikti_semasi: Option<&Value>,
    sirlar: &[GizliDeger],
) -> Result<DenetlenmisAracTanimi, AracTanimReddi> {
    let red = || AracTanimReddi { sira };

    // Bu taramadan önce D, audit metni, log, UI/model/cache DTO'su üretilmez.
    if metin_sir_iceriyor(sunucu_model_adi, sirlar)
        || metin_sir_iceriyor(uzak_ad, sirlar)
        || uzak_aciklama.is_some_and(|metin| metin_sir_iceriyor(metin, sirlar))
        || sema_sir_iceriyor(girdi_semasi, sirlar)
        || cikti_semasi.is_some_and(|sema| sema_sir_iceriyor(sema, sirlar))
    {
        return Err(red());
    }

    if !model_arac_adi_gecerli(sunucu_model_adi) || !model_arac_adi_gecerli(uzak_ad) {
        return Err(red());
    }

    let sunucu_etiketi = McpKartEtiketi::yeni(sunucu_model_adi).map_err(|_| red())?;
    let arac_etiketi = McpKartEtiketi::yeni(uzak_ad).map_err(|_| red())?;

    let birlesik_ad = format!("mcp__{sunucu_model_adi}__{uzak_ad}");
    if metin_sir_iceriyor(&birlesik_ad, sirlar)
        || birlesik_ad.is_empty()
        || birlesik_ad.chars().count() > 64
        || gorunmez_var_mi(&birlesik_ad)
        || !model_arac_adi_gecerli(&birlesik_ad)
    {
        return Err(red());
    }

    // Görünmez ya da yön değiştiren karakter modele/denetçiye/arayüze
    // gördüğünden farklı metin taşıyabilir; açıklama ve şemalarda da araç kapanır.
    if uzak_aciklama.is_some_and(gorunmez_var_mi)
        || semada_metin_var(girdi_semasi, &gorunmez_var_mi)
        || cikti_semasi.is_some_and(|sema| semada_metin_var(sema, &gorunmez_var_mi))
    {
        return Err(red());
    }

    let aciklama = uzak_aciklama.unwrap_or_default();
    let aciklama = utf8_bayt_sinirinda_kirp(aciklama, ACIKLAMA_AZAMI_BAYT).to_owned();

    // Model girdisindeki alan sınırları ve audit birleştirmesi de sırrı
    // parçalardan yeniden kurabilir; iki türetilmiş metin de D'den önce taranır.
    let tanim_metni = serde_json::to_string(&serde_json::json!({
        "name": &birlesik_ad,
        "description": &aciklama,
        "inputSchema": girdi_semasi,
        "outputSchema": cikti_semasi,
    }))
    .map_err(|_| red())?;
    if metin_sir_iceriyor(&tanim_metni, sirlar) {
        return Err(red());
    }
    let denetim_metni = denetim_metni_olustur(&birlesik_ad, &aciklama, girdi_semasi);
    if metin_sir_iceriyor(&denetim_metni, sirlar) {
        return Err(red());
    }

    // Şema derleme hataları core'dan ham kütüphane ayrıntısı çıkarmaz.
    let derlenmis_girdi = Arc::new(McpJsonSemasi::derle(girdi_semasi).map_err(|_| red())?);
    let derlenmis_cikti = cikti_semasi
        .map(McpJsonSemasi::derle)
        .transpose()
        .map_err(|_| red())?
        .map(Arc::new);

    let d = arac_tanim_ozeti(&birlesik_ad, &aciklama, girdi_semasi, cikti_semasi);

    Ok(DenetlenmisAracTanimi {
        sunucu_etiketi,
        arac_etiketi,
        sunum: McpAracSunumu {
            ad: birlesik_ad,
            aciklama,
            parametre_semasi: girdi_semasi.clone(),
        },
        d,
        girdi_semasi: derlenmis_girdi,
        cikti_semasi: derlenmis_cikti,
        cikti_semasi_degeri: cikti_semasi.cloned(),
        denetim_metni,
        sirlar: sirlar.to_vec(),
    })
}

fn model_arac_adi_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

fn utf8_bayt_sinirinda_kirp(metin: &str, azami_bayt: usize) -> &str {
    if metin.len() <= azami_bayt {
        return metin;
    }

    let mut son = azami_bayt;
    while !metin.is_char_boundary(son) {
        son -= 1;
    }
    &metin[..son]
}

fn metin_sir_iceriyor(metin: &str, sirlar: &[GizliDeger]) -> bool {
    sirlar.iter().any(|sir| metin.contains(sir.expose_secret()))
}

fn sema_sir_iceriyor(sema: &Value, sirlar: &[GizliDeger]) -> bool {
    semada_metin_var(sema, &|metin| metin_sir_iceriyor(metin, sirlar))
}

/// Şemadaki her yazı ve nesne anahtarı için `kosul`'u dener.
fn semada_metin_var(sema: &Value, kosul: &dyn Fn(&str) -> bool) -> bool {
    match sema {
        Value::String(metin) => kosul(metin),
        Value::Array(degerler) => degerler.iter().any(|deger| semada_metin_var(deger, kosul)),
        Value::Object(nesne) => nesne
            .iter()
            .any(|(anahtar, deger)| kosul(anahtar) || semada_metin_var(deger, kosul)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

fn denetim_metni_olustur(ad: &str, aciklama: &str, girdi_semasi: &Value) -> String {
    let mut parcalar = vec![ad.to_owned(), aciklama.to_owned()];
    sema_metinlerini_ekle(girdi_semasi, &mut parcalar);
    parcalar.join("\n")
}

fn sema_metinlerini_ekle(sema: &Value, parcalar: &mut Vec<String>) {
    match sema {
        Value::String(metin) => parcalar.push(metin.clone()),
        Value::Array(degerler) => {
            for deger in degerler {
                sema_metinlerini_ekle(deger, parcalar);
            }
        }
        Value::Object(nesne) => {
            for (anahtar, deger) in nesne {
                parcalar.push(anahtar.clone());
                sema_metinlerini_ekle(deger, parcalar);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

#[cfg(test)]
mod testler {
    use kalfa_core::arac::McpJsonSemasi;
    use serde_json::{Value, json};

    use crate::maske::GizliDeger;

    use super::{AracTanimReddi, tanimi_denetle, utf8_bayt_sinirinda_kirp};

    const SUNUCU: &str = "yerel_srv";
    const AD: &str = "ara";
    const GIZLI: &str = "SIR-TEST-123456";

    fn sirlar() -> Vec<GizliDeger> {
        vec![GizliDeger::yeni(GIZLI.to_owned()).unwrap()]
    }

    fn tanim(
        ad: &str,
        aciklama: Option<&str>,
        girdi: &Value,
        cikti: Option<&Value>,
    ) -> Result<super::DenetlenmisAracTanimi, AracTanimReddi> {
        tanimi_denetle(3, SUNUCU, ad, aciklama, girdi, cikti, &sirlar())
    }

    #[test]
    fn sir_ad_aciklama_schema_anahtari_ve_schema_yazisinda_tanimi_kapatir() {
        let bos_sema = json!({"type":"object"});
        let mut gizli_anahtarli_sema = json!({"type":"object"});
        gizli_anahtarli_sema
            .as_object_mut()
            .unwrap()
            .insert(GIZLI.to_owned(), json!({"type":"string"}));
        let gizli_yazili_girdi = json!({"description": GIZLI});
        let gizli_anahtarli_cikti = json!({(GIZLI): {"type":"string"}});
        let durumlar = [
            tanim(GIZLI, None, &bos_sema, None).err(),
            tanim(AD, Some(GIZLI), &bos_sema, None).err(),
            tanim(AD, None, &gizli_anahtarli_sema, None).err(),
            tanim(AD, None, &gizli_yazili_girdi, None).err(),
            tanim(AD, None, &bos_sema, Some(&json!({"description": GIZLI}))).err(),
            tanim(AD, None, &bos_sema, Some(&gizli_anahtarli_cikti)).err(),
        ];

        for red in durumlar.into_iter().flatten() {
            assert_eq!(red.sira, 3);
            assert!(!format!("{red:?}").contains(GIZLI));
        }

        let sunucu_sirli = tanimi_denetle(3, GIZLI, AD, None, &bos_sema, None, &sirlar())
            .err()
            .unwrap();
        assert_eq!(sunucu_sirli.sira, 3);
        assert!(!format!("{sunucu_sirli:?}").contains(GIZLI));
    }

    #[test]
    fn tanim_ve_schema_hatalari_yalniz_sira_tasarir() {
        let red = tanim(
            AD,
            None,
            &json!({"$ref":"https://ornek.invalid/gizli"}),
            None,
        )
        .err()
        .unwrap();

        assert_eq!(red.sira, 3);
        assert!(!format!("{red:?}").contains("ornek.invalid"));
    }

    #[test]
    fn audit_birlesiminde_yeniden_olusabilecek_sir_denetime_cikmaz() {
        let bolunen_sir = GizliDeger::yeni("bolunen\ngizli".to_owned()).unwrap();
        let girdi = json!({"examples":["bolunen", "gizli"]});
        let red = tanimi_denetle(1, SUNUCU, AD, None, &girdi, None, &[bolunen_sir])
            .err()
            .unwrap();

        assert_eq!(red.sira, 1);
    }

    #[test]
    fn birlesik_ad_kurali_64_karakteri_ve_gorunmez_unicodeyi_reddeder() {
        let uzun = "a".repeat(55);
        assert!(tanim(&uzun, None, &json!({"type":"object"}), None).is_err());
        assert!(tanim("ara\u{200b}", None, &json!({"type":"object"}), None).is_err());
        assert!(tanim("ara/oku", None, &json!({"type":"object"}), None).is_err());
        assert!(tanim("", None, &json!({"type":"object"}), None).is_err());
    }

    #[test]
    fn aciklama_ve_semalarda_gorunmez_unicode_araci_kapatir() {
        let nesne = json!({"type":"object"});
        // Normal satır sonu ve sekme reddedilmez.
        assert!(tanim(AD, Some("iki\r\nsatır\tsekme"), &nesne, None).is_ok());

        let gizli = "ara\u{202e}tluser";
        assert!(tanim(AD, Some(gizli), &nesne, None).is_err());
        let deger = json!({"type":"object","description": gizli});
        assert!(tanim(AD, None, &deger, None).is_err());
        let anahtar = json!({"type":"object","properties":{"a\u{200b}":{"type":"string"}}});
        assert!(tanim(AD, None, &anahtar, None).is_err());
        let dizi = json!({"type":"object","examples":["\u{e0041}"]});
        assert!(tanim(AD, None, &nesne, Some(&dizi)).is_err());
    }

    #[test]
    fn aciklama_1024_baytta_utf8_karakterini_bolmadan_kesilir() {
        let metin = format!("{}şson", "a".repeat(1023));
        let kirpilmis = utf8_bayt_sinirinda_kirp(&metin, 1024);

        assert_eq!(kirpilmis.len(), 1023);
        assert!(!kirpilmis.ends_with('ş'));
    }

    #[test]
    fn denetim_metni_input_schema_anahtar_ve_yazilarini_alir_cikisi_almaz() {
        let girdi = json!({
            "properties": {
                "aranan": {
                    "type": "string",
                    "description": "kullanıcı girdisi",
                    "examples": ["örnek"]
                }
            }
        });
        let cikti = json!({"description":"yalnız çıkış"});
        let tanim = tanim(AD, Some("ara"), &girdi, Some(&cikti)).unwrap();

        assert!(tanim.denetim_metni().contains("aranan"));
        assert!(tanim.denetim_metni().contains("kullanıcı girdisi"));
        assert!(tanim.denetim_metni().contains("örnek"));
        assert!(!tanim.denetim_metni().contains("yalnız çıkış"));
    }

    #[test]
    fn output_schema_d_ozetinin_parcasidir_ve_derlenmis_girdi_kullanilir() {
        let girdi = json!({"type":"object"});
        let cikti_metni = json!({"type":"string"});
        let cikti_sayi = json!({"type":"integer"});
        let metin = tanim(AD, None, &girdi, Some(&cikti_metni)).unwrap();
        let sayi = tanim(AD, None, &girdi, Some(&cikti_sayi)).unwrap();

        assert_ne!(metin.d().sha256(), sayi.d().sha256());
        assert!(metin.argumanlar_uygun_mu(&json!({})));
    }

    #[test]
    fn input_ve_output_schemata_bozuksa_tanim_yayimlanmaz() {
        let bozuk = json!({"type": 42});
        assert!(tanim(AD, None, &bozuk, None).is_err());
        assert!(tanim(AD, None, &json!({"type":"object"}), Some(&bozuk)).is_err());
    }

    #[test]
    fn core_sema_api_hatasi_opaque_kalir() {
        let hata = McpJsonSemasi::derle(&json!({"$ref":"https://private.invalid/x"})).unwrap_err();

        assert!(!format!("{hata:?}").contains("private.invalid"));
    }
}
