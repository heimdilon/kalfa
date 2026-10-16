//! MCP araç yanıtını sırları sızdırmadan Kalfa aracına dönüştürür.
//!
//! `content_alani_var`, RMCP'nin `CallToolResult` tipinin saklamadığı, sınırlı
//! taşımanın ham JSON'dan aldığı güvenilir varlık bilgisidir. `cikti_semasi`
//! yalnız keşifte derlenmiş `outputSchema` doğrulayıcısı varsa verilir.

use std::borrow::Cow;

use kalfa_core::arac::{AracCiktisi, AracHatasi};
use rmcp::model::{CallToolResult, ContentBlock, ResourceContents};
use serde_json::Value;

use crate::istemci::RpcHatasi;
use crate::maske::{GizliDeger, maskele_metin};

/// Keşifte derlenen outputSchema'yı çağrı anında uygulayan, hata metni
/// sızdırmayan doğrulayıcı. `Some` olması outputSchema bulunduğu anlamına gelir.
pub(crate) type CiktiSemaDenetleyicisi<'a> = dyn Fn(&Value) -> bool + Send + Sync + 'a;

const CIKTI_SEMASI_HATASI: &str = "MCP yapılandırılmış sonucu çıkış şemasına uymuyor.";
const DONUSUM_HATASI: &str = "MCP sonucu güvenli biçimde dönüştürülemedi.";

/// Sonucu Kalfa çıktısına çevirir.
///
/// `kaynak`, keşif kapılarından geçmiş güvenli sunucu/araç etiketidir.
/// `content_alani_var` ham JSON-RPC yanıtındaki alan varlığıdır; SDK'den
/// türetilmemelidir. `content` alanı varsa dizi boş olsa da structuredContent
/// metin dönüşümünde kullanılmaz. Output schema doğrulaması içerik
/// dönüşümünden önce, özgün `structuredContent` üzerinde yapılır.
pub(crate) fn sonucu_donustur(
    sonuc: &CallToolResult,
    content_alani_var: bool,
    kaynak: &str,
    sirlar: &[GizliDeger],
    cikti_semasi: Option<&CiktiSemaDenetleyicisi<'_>>,
) -> Result<AracCiktisi, AracHatasi> {
    let dis_hata = sonuc.is_error == Some(true);

    if !dis_hata
        && let Some(dogrulayici) = cikti_semasi
        && !sonuc.structured_content.as_ref().is_some_and(dogrulayici)
    {
        return Err(basarisiz(kaynak, CIKTI_SEMASI_HATASI, sirlar));
    }

    let icerik = if content_alani_var {
        content_metin(&sonuc.content)
    } else if let Some(yapilandirilmis) = &sonuc.structured_content {
        let Some(json) = json_metin_maskeli(yapilandirilmis, sirlar) else {
            return Err(basarisiz(kaynak, DONUSUM_HATASI, sirlar));
        };
        json
    } else {
        String::new()
    };

    let ciktisi = arac_ciktisi(kaynak, &icerik, sirlar);
    if dis_hata {
        Err(AracHatasi::Basarisiz(ciktisi))
    } else {
        Ok(ciktisi)
    }
}

/// JSON-RPC protokol hatasını dış kaynaklı araç hatasına çevirir. Ham metin
/// hiçbir `Debug`, `Display`, UI veya günlük yoluna girmeden maskelenir.
pub fn jsonrpc_hatasi_donustur(kaynak: &str, ham_metin: &str, sirlar: &[GizliDeger]) -> AracHatasi {
    basarisiz(kaynak, ham_metin, sirlar)
}

/// Uzak JSON-RPC protokol hatasını (`error.message` ve varsa `error.data`)
/// sırları sızdırmadan dış kaynaklı araç hatasına (`AracHatasi::Basarisiz`)
/// çevirir.
pub fn uzak_rpc_hatasini_donustur(
    rpc: &RpcHatasi,
    kaynak: &str,
    sirlar: &[GizliDeger],
) -> AracHatasi {
    let (_, ham_metin, ham_veri) = rpc.parcalari();
    let mut icerik = maskele_metin(ham_metin, sirlar);
    if let Some(veri) = ham_veri {
        let Some(json_veri) = json_metin_maskeli(veri, sirlar) else {
            return basarisiz(kaynak, DONUSUM_HATASI, sirlar);
        };
        if !icerik.is_empty() {
            icerik.push('\n');
        }
        icerik.push_str(&json_veri);
    }
    basarisiz(kaynak, &icerik, sirlar)
}

fn content_metin(content: &[ContentBlock]) -> String {
    let mut metin = String::new();
    let mut onceki_var = false;

    for blok in content {
        let Some(parca) = blok_metin(blok) else {
            continue;
        };
        if onceki_var {
            metin.push('\n');
        }
        metin.push_str(&parca);
        onceki_var = true;
    }

    metin
}

fn blok_metin(blok: &ContentBlock) -> Option<Cow<'_, str>> {
    match blok {
        ContentBlock::Text(metin) => Some(Cow::Borrowed(&metin.text)),
        ContentBlock::ResourceLink(kaynak) => Some(Cow::Borrowed(&kaynak.uri)),
        ContentBlock::Resource(gomulu) => match &gomulu.resource {
            ResourceContents::TextResourceContents { uri, text, .. } => {
                Some(Cow::Owned(format!("kaynak: {uri}\n{text}")))
            }
            ResourceContents::BlobResourceContents { uri, .. } => {
                Some(Cow::Owned(format!("[ikili kaynak atlandı: {uri}]")))
            }
            _ => None,
        },
        ContentBlock::Image(_) => Some(Cow::Borrowed("[görsel içerik atlandı]")),
        ContentBlock::Audio(_) => Some(Cow::Borrowed("[ses içeriği atlandı]")),
        _ => None,
    }
}

/// Value'yu kopyalamadan, yaprak metinleri JSON kaçışından önce maskeler.
/// Son maskeyi tüm JSON metnine uygular; sır birleştirilmiş anahtar/değer veya
/// alan sınırında oluşuyorsa da dışarı çıkmaz.
fn json_metin_maskeli(deger: &Value, sirlar: &[GizliDeger]) -> Option<String> {
    let mut json = String::new();
    json_yaz_maskeli(deger, sirlar, &mut json)?;
    Some(maskele_metin(&json, sirlar))
}

fn json_yaz_maskeli(deger: &Value, sirlar: &[GizliDeger], json: &mut String) -> Option<()> {
    match deger {
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            json.push_str(&serde_json::to_string(deger).ok()?);
        }
        Value::String(metin) => {
            let maskeli = maskele_metin(metin, sirlar);
            json.push_str(&serde_json::to_string(&maskeli).ok()?);
        }
        Value::Array(diziler) => {
            json.push('[');
            for (indeks, oge) in diziler.iter().enumerate() {
                if indeks > 0 {
                    json.push(',');
                }
                json_yaz_maskeli(oge, sirlar, json)?;
            }
            json.push(']');
        }
        Value::Object(nesne) => {
            json.push('{');
            for (indeks, (anahtar, alt)) in nesne.iter().enumerate() {
                if indeks > 0 {
                    json.push(',');
                }
                let maskeli_anahtar = maskele_metin(anahtar, sirlar);
                json.push_str(&serde_json::to_string(&maskeli_anahtar).ok()?);
                json.push(':');
                json_yaz_maskeli(alt, sirlar, json)?;
            }
            json.push('}');
        }
    }
    Some(())
}

fn basarisiz(kaynak: &str, icerik: &str, sirlar: &[GizliDeger]) -> AracHatasi {
    AracHatasi::Basarisiz(arac_ciktisi(kaynak, icerik, sirlar))
}

fn arac_ciktisi(kaynak: &str, icerik: &str, sirlar: &[GizliDeger]) -> AracCiktisi {
    AracCiktisi {
        icerik: maskele_metin(icerik, sirlar),
        kaynak: maskele_metin(kaynak, sirlar),
        kalfa_uretimi: false,
        ..AracCiktisi::default()
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use rmcp::model::{MetaObject, Resource};
    use serde_json::json;

    fn gizli(deger: &str) -> GizliDeger {
        GizliDeger::yeni(deger.to_owned()).expect("test sırrı geçerli")
    }

    fn ciktidan_hata(hata: AracHatasi) -> AracCiktisi {
        match hata {
            AracHatasi::Basarisiz(cikti) => cikti,
            diger => panic!("beklenmeyen araç hatası: {diger}"),
        }
    }

    fn content_alani_olmayan_yapilandirilmis(deger: Value) -> CallToolResult {
        let mut sonuc = CallToolResult::structured(deger);
        sonuc.content.clear();
        sonuc
    }

    #[test]
    fn var_olan_bos_content_yapilandirilmis_sonuca_dusmez() {
        let mut sonuc = CallToolResult::structured(json!({"deger": 7}));
        sonuc.content.clear();
        let cikti = sonucu_donustur(&sonuc, true, "mcp:sunucu:arac", &[], None)
            .expect("boş content geçerli bir araç sonucudur");
        assert_eq!(cikti.icerik, "");
    }

    #[test]
    fn content_yokken_structured_content_json_metne_doner() {
        let sonuc = content_alani_olmayan_yapilandirilmis(json!({"deger": 7}));
        let cikti = sonucu_donustur(&sonuc, false, "mcp:sunucu:arac", &[], None)
            .expect("structuredContent dönüştürülür");
        assert_eq!(cikti.icerik, r#"{"deger":7}"#);
    }

    #[test]
    fn iki_alan_varken_sadece_content_metni_kullanilir() {
        let mut sonuc = CallToolResult::structured(json!({"gorunmemeli": true}));
        sonuc.content = vec![ContentBlock::text("görünen")];
        let cikti = sonucu_donustur(&sonuc, true, "mcp:sunucu:arac", &[], None)
            .expect("content önceliklidir");
        assert_eq!(cikti.icerik, "görünen");
        assert!(!cikti.icerik.contains("gorunmemeli"));
    }

    #[test]
    fn content_alaninin_olmadigi_bos_yanit_bos_metindir() {
        let sonuc = CallToolResult::success(Vec::new());
        let cikti = sonucu_donustur(&sonuc, false, "mcp:sunucu:arac", &[], None)
            .expect("şemasız boş yanıt hata değildir");
        assert_eq!(cikti.icerik, "");
    }

    #[test]
    fn birlestirilen_content_sirini_maskeler() {
        let s = gizli("ABCD\nEFGH");
        let sonuc =
            CallToolResult::success(vec![ContentBlock::text("ABCD"), ContentBlock::text("EFGH")]);
        let cikti = sonucu_donustur(
            &sonuc,
            true,
            "mcp:sunucu:arac",
            std::slice::from_ref(&s),
            None,
        )
        .expect("araç sonucu dönüştürülür");
        assert!(!cikti.icerik.contains(s.expose_secret()));
        assert_eq!(cikti.icerik, "[gizli]");
    }

    #[test]
    fn json_kacisindan_once_anahtar_ve_deger_maskelenir() {
        let s = gizli("A1\"B2\\C3");
        let mut nesne = serde_json::Map::new();
        nesne.insert("gizli".to_owned(), json!(s.expose_secret()));
        nesne.insert(s.expose_secret().to_owned(), json!("anahtar da sır"));
        let sonuc = content_alani_olmayan_yapilandirilmis(Value::Object(nesne));
        let cikti = sonucu_donustur(
            &sonuc,
            false,
            "mcp:sunucu:arac",
            std::slice::from_ref(&s),
            None,
        )
        .expect("JSON metni sırları maskeler");
        assert!(!cikti.icerik.contains(s.expose_secret()));
        assert!(cikti.icerik.contains("[gizli]"));
    }

    #[test]
    fn resource_medya_ve_meta_yalniz_guvenli_donusuyle_gorunur() {
        let mut meta = MetaObject::new();
        meta.0
            .insert("yalniz_meta".to_owned(), json!("GIZLI_META_VERISI"));
        let mut sonuc = CallToolResult::success(vec![
            ContentBlock::resource_link(Resource::new(
                "https://ornek.invalid/yalniz-uri",
                "HASSAS_RESOURCE_ADI",
            )),
            ContentBlock::resource(ResourceContents::text(
                "kaynak metni",
                "file:///yalniz/metin",
            )),
            ContentBlock::resource(ResourceContents::blob(
                "TABAN64_GIZLI_YUK",
                "file:///yalniz/ikili",
            )),
            ContentBlock::image("RESIM_VERISI", "image/png"),
            ContentBlock::audio("SES_VERISI", "audio/wav"),
        ]);
        sonuc.meta = Some(meta);
        let cikti = sonucu_donustur(&sonuc, true, "mcp:sunucu:arac", &[], None)
            .expect("desteklenen bloklar güvenli metne çevrilir");
        assert!(cikti.icerik.contains("https://ornek.invalid/yalniz-uri"));
        assert!(
            cikti
                .icerik
                .contains("kaynak: file:///yalniz/metin\nkaynak metni")
        );
        assert!(
            cikti
                .icerik
                .contains("[ikili kaynak atlandı: file:///yalniz/ikili]")
        );
        assert!(cikti.icerik.contains("[görsel içerik atlandı]"));
        assert!(cikti.icerik.contains("[ses içeriği atlandı]"));
        assert!(!cikti.icerik.contains("TABAN64_GIZLI_YUK"));
        assert!(!cikti.icerik.contains("RESIM_VERISI"));
        assert!(!cikti.icerik.contains("SES_VERISI"));
        assert!(!cikti.icerik.contains("HASSAS_RESOURCE_ADI"));
        assert!(!cikti.icerik.contains("GIZLI_META_VERISI"));
    }

    #[test]
    fn is_error_metinini_maskeler_ve_arac_hatasina_cevirir() {
        let s = gizli("MCP-hassas-deger");
        let sonuc = CallToolResult::error(vec![ContentBlock::text(format!(
            "sunucu hata verdi: {}",
            s.expose_secret()
        ))]);
        let hata = sonucu_donustur(
            &sonuc,
            true,
            "mcp:sunucu:arac",
            std::slice::from_ref(&s),
            None,
        )
        .expect_err("isError araç hatasıdır");
        let cikti = ciktidan_hata(hata);
        assert!(cikti.icerik.contains("[gizli]"));
        assert!(!cikti.icerik.contains(s.expose_secret()));
    }

    #[test]
    fn jsonrpc_ham_hata_metnini_ve_verisini_maskeler() {
        let s1 = gizli("RPC-hassas-deger");
        let s2 = gizli("VERI\"ICINDE\\SIR");
        let hata = jsonrpc_hatasi_donustur(
            "mcp:sunucu:arac",
            &format!("uzak hata: {}", s1.expose_secret()),
            std::slice::from_ref(&s1),
        );
        let cikti = ciktidan_hata(hata);
        assert!(cikti.icerik.contains("[gizli]"));
        assert!(!cikti.icerik.contains(s1.expose_secret()));

        let rpc = RpcHatasi::yeni(
            -32000,
            format!("mesaj: {}", s1.expose_secret()),
            Some(json!({ "detay": s2.expose_secret() })),
        );
        assert!(!format!("{rpc:?}").contains(s1.expose_secret()));
        assert!(!format!("{rpc:?}").contains(s2.expose_secret()));
        let uzak_hata =
            uzak_rpc_hatasini_donustur(&rpc, "mcp:sunucu:arac", &[s1.clone(), s2.clone()]);
        let uzak_cikti = ciktidan_hata(uzak_hata);
        assert!(!uzak_cikti.icerik.contains(s1.expose_secret()));
        assert!(!uzak_cikti.icerik.contains(s2.expose_secret()));
        assert!(uzak_cikti.icerik.contains("[gizli]"));
    }

    #[test]
    fn output_schema_gecerli_structured_sonucu_kabul_eder() {
        let sonuc = content_alani_olmayan_yapilandirilmis(json!({"ok": true}));
        let sema = |deger: &Value| deger.get("ok").and_then(Value::as_bool) == Some(true);
        let cikti = sonucu_donustur(&sonuc, false, "mcp:sunucu:arac", &[], Some(&sema))
            .expect("şemaya uyan sonuç kabul edilir");
        assert!(cikti.icerik.contains("\"ok\":true"));
    }

    #[test]
    fn output_schema_icerik_olsa_da_structured_icerik_ister() {
        let sonuc = CallToolResult::success(vec![ContentBlock::text("yalnız content")]);
        let sema = |_deger: &Value| true;
        let hata = sonucu_donustur(&sonuc, true, "mcp:sunucu:arac", &[], Some(&sema))
            .expect_err("şema varken structuredContent zorunludur");
        let cikti = ciktidan_hata(hata);
        assert_eq!(cikti.icerik, CIKTI_SEMASI_HATASI);
    }

    #[test]
    fn output_schema_uyumsuz_icerigin_ham_degerini_sizdirmaz() {
        let s = gizli("semaya-girmemeli");
        let sonuc = content_alani_olmayan_yapilandirilmis(json!({"deger": s.expose_secret()}));
        let sema = |_deger: &Value| false;
        let hata = sonucu_donustur(
            &sonuc,
            false,
            "mcp:sunucu:arac",
            std::slice::from_ref(&s),
            Some(&sema),
        )
        .expect_err("uyumsuz structuredContent reddedilir");
        let cikti = ciktidan_hata(hata);
        assert_eq!(cikti.icerik, CIKTI_SEMASI_HATASI);
        assert!(!cikti.icerik.contains(s.expose_secret()));
    }
}
