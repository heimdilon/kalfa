//! 5a tasarımı §9–§10: bağlam özetinin kayıttan kurtarılması.

use kalfa_core::gonderim::{GonderilenOge, IstekSonucu, oge_metni};
use kalfa_core::izin::Kip;
use kalfa_core::kayit::{KayitOgesi, OzetYontemi};
use kalfa_core::kimlik::OturumId;
use kalfa_core::kurtarma::{KurtarmaHatasi, kurtar};
use kalfa_core::protokol::{SEMA_SURUMU, Zarf};
use kalfa_evren::tipler::{ChatMessage, FunctionCall, ToolCall};

fn zarflar(ogeler: Vec<KayitOgesi>) -> Vec<Zarf<KayitOgesi>> {
    let oturum = OturumId::yeni();
    ogeler
        .into_iter()
        .enumerate()
        .map(|(i, yuk)| Zarf {
            sema: SEMA_SURUMU,
            oturum,
            tur: None,
            sira: i as u64 + 1,
            yuk,
        })
        .collect()
}

fn oge(kaynak: &str) -> GonderilenOge {
    GonderilenOge {
        kaynak: kaynak.into(),
        sira: None,
        bayt: 1,
    }
}

fn istek(amac: &str, onceki: u32, yeni: Vec<GonderilenOge>) -> KayitOgesi {
    KayitOgesi::Istek {
        amac: amac.into(),
        istek_id: None,
        kredi: None,
        yonlendirilen_model: None,
        model: None,
        yeni,
        onceki,
        sonuc: IstekSonucu::Basarili,
        butce_islem: None,
    }
}

fn mesaj(m: ChatMessage, kaynak: &str) -> KayitOgesi {
    KayitOgesi::Mesaj {
        mesaj: m,
        kaynak: Some(kaynak.into()),
    }
}

/// Sıra 1–5: açılış, kullanıcı, araç çağrısı, araç sonucu (sıra 4), ana istek.
fn temel() -> Vec<KayitOgesi> {
    let cagri = ToolCall {
        id: "c1".into(),
        kind: "function".into(),
        function: FunctionCall {
            name: "oku".into(),
            arguments: "{}".into(),
        },
    };
    vec![
        KayitOgesi::OturumAcildi {
            model: "glm-5.3".into(),
            kip: Kip::Siki,
            calisma_alani: None,
        },
        mesaj(ChatMessage::user("oku"), "mesaj:kullanıcı"),
        mesaj(ChatMessage::assistant(None, vec![cagri]), "model"),
        mesaj(ChatMessage::tool("c1", "uzun çıktı"), "sahte:oku"),
        istek(
            "ana",
            0,
            vec![
                oge("kalfa:sistem"),
                oge("kalfa:araçlar"),
                oge("mesaj:kullanıcı"),
                oge("model"),
                oge("sahte:oku"),
            ],
        ),
    ]
}

fn ozet(mesaj_sira: u64) -> KayitOgesi {
    KayitOgesi::BaglamOzeti {
        mesaj_sira,
        kaynak: "özet:sahte:oku".into(),
        icerik: "özet".into(),
        yontem: OzetYontemi::Ozet,
    }
}

#[test]
fn ozet_mesajin_yerine_gecer_ve_gonderilmeyi_bekler() {
    let mut k = temel();
    k.push(ozet(4));
    let d = kurtar(&zarflar(k)).unwrap();
    assert_eq!(d.mesajlar[2].content.as_deref(), Some("özet"));
    assert_eq!(d.mesajlar[2].tool_call_id.as_deref(), Some("c1"));
    assert_eq!(
        d.mesaj_bilgileri[2],
        (Some(6), Some("özet:sahte:oku".to_string()))
    );
    assert_eq!(
        d.bekleyen_ozetler.iter().copied().collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(d.gonderilen, 3);
}

#[test]
fn sonraki_ana_istek_bekleyeni_temizler_ve_ozeti_saymaz() {
    let mut k = temel();
    k.push(ozet(4));
    k.push(istek("ana", 3, vec![oge("özet:sahte:oku")]));
    let d = kurtar(&zarflar(k)).unwrap();
    assert!(d.bekleyen_ozetler.is_empty());
    assert_eq!(d.gonderilen, 3);
}

#[test]
fn ana_olmayan_istek_gonderileni_degistirmez() {
    let mut k = temel();
    k.push(istek("denetci", 0, vec![oge("sahte:oku")]));
    k.push(istek("özetleme", 1, Vec::new()));
    let d = kurtar(&zarflar(k)).unwrap();
    assert_eq!(d.gonderilen, 3);
}

#[test]
fn arac_mesajina_baglanmayan_ozet_hata_verir() {
    for sira in [2, 99] {
        let mut k = temel();
        k.push(ozet(sira));
        assert_eq!(
            kurtar(&zarflar(k)).unwrap_err(),
            KurtarmaHatasi::BozukOzet { sira: 6 },
            "{sira}"
        );
    }
}

#[test]
fn gunlugun_metni_ozeti_ve_ozgunu_ayri_gosterir() {
    let mut k = temel();
    k.push(ozet(4));
    let z = zarflar(k);
    assert_eq!(oge_metni(&z, 6).as_deref(), Some("özet"));
    assert_eq!(oge_metni(&z, 4).as_deref(), Some("uzun çıktı"));
}
