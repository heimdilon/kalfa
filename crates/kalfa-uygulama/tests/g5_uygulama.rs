//! G5 v2: gerçek uygulama/oturum yolu; yalnız dış model ve sunucu verisi sabittir.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_evren::tipler::{ChatRequest, Role};
use kalfa_uygulama::mcp::komutlar::{UzakMcpSunucuFormu, UzakMcpYetkiFormGirdisi};
use kalfa_uygulama::sozlesme::OnayGirdisi;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedReceiver;

mod g5_mcp_fixture;

struct Ortam {
    _veri: TempDir,
    alan: TempDir,
    t: TestUygulamasi,
}

impl Ortam {
    async fn yeni() -> Self {
        let veri = tempfile::tempdir().unwrap();
        let alan = tempfile::tempdir().unwrap();
        let saglayici = SahteSaglayici::yeni(Vec::new());
        saglayici.sart_kabul(1).await.unwrap();
        let t = test_uygulamasi(veri.path(), saglayici, Some("evren_llm_g5"));
        t.uygulama.baslat().await;
        t.iletisim.klasor_ver(alan.path());
        t.iletisim.onay_ver(true);
        Self {
            _veri: veri,
            alan,
            t,
        }
    }

    async fn oturum(&self) -> OturumId {
        let alan = self.t.uygulama.calisma_alani_sec().await.unwrap();
        self.t
            .uygulama
            .oturum_baslat(alan.kimlik, "glm-5.3".into())
            .await
            .unwrap()
            .oturum
    }

    async fn tur(&self, id: OturumId, adimlar: Vec<SahteYanit>) -> Vec<Olay> {
        self.tur_kanalli(id, adimlar).await.0
    }

    async fn tur_kanalli(
        &self,
        id: OturumId,
        adimlar: Vec<SahteYanit>,
    ) -> (Vec<Olay>, UnboundedReceiver<KanalMesaji>) {
        self.tur_kararli(id, adimlar, |_| OnayKarari::Onayla).await
    }

    async fn tur_kararli(
        &self,
        id: OturumId,
        adimlar: Vec<SahteYanit>,
        karar: fn(&kalfa_core::protokol::OnayOgesi) -> OnayKarari,
    ) -> (Vec<Olay>, UnboundedReceiver<KanalMesaji>) {
        let (kanal, mut rx) = SahteKanal::yeni();
        self.t.uygulama.oturumu_dinle(id, kanal).unwrap();
        for adim in adimlar {
            self.t.saglayici.senaryoya_ekle(adim);
        }
        self.t
            .uygulama
            .mesaj_gonder(id, "G5 sabit kabul görevini yürütün.".into())
            .await
            .unwrap();
        let mut olaylar = Vec::new();
        loop {
            let paket = tokio::time::timeout(Duration::from_secs(20), rx.recv())
                .await
                .expect("G5 olay zaman aşımı")
                .expect("G5 kanalı kapandı");
            let KanalMesaji::Olaylar { no, zarflar } = paket else {
                panic!("G5 kanalı kapandı")
            };
            self.t.uygulama.paketi_isledim(id, no).unwrap();
            for zarf in zarflar {
                if let Olay::OnayGerekiyor { ref ogeler, .. } = zarf.yuk {
                    let kararlar = ogeler
                        .iter()
                        .map(|g| OnayGirdisi {
                            cagri: g.cagri,
                            muhur: g.muhur.clone(),
                            karar: karar(g),
                            kisayol: false,
                        })
                        .collect();
                    self.t.uygulama.arac_onayi(id, kararlar).await.unwrap();
                }
                if let Olay::KullaniciSorusuGerekiyor { ref soru } = zarf.yuk {
                    self.t
                        .uygulama
                        .kullanici_sorusu_yaniti(
                            id,
                            soru.tur,
                            soru.soru_kimligi.clone(),
                            kalfa_core::soru::KullaniciSorusuYaniti::Secenek {
                                secenek_kimligi: "toml".into(),
                            },
                        )
                        .await
                        .unwrap();
                }
                if let Olay::Tamamlandi { ref neden } = zarf.yuk {
                    assert_eq!(
                        *neden,
                        kalfa_core::protokol::BitisNedeni::Normal,
                        "limit, hata ve iptal başarılı kabul değildir"
                    );
                }
                let son = matches!(zarf.yuk, Olay::Tamamlandi { .. });
                olaylar.push(zarf.yuk);
                if son {
                    return (olaylar, rx);
                }
            }
        }
    }
}

fn son_arac_sonucu(istek: &ChatRequest) -> &str {
    istek
        .messages
        .iter()
        .rev()
        .find(|m| m.role == Role::Tool)
        .and_then(|m| m.content.as_deref())
        .expect("model gerçek araç sonucunu görmeli")
}

fn model_is_kimligi(istek: &ChatRequest) -> &str {
    istek
        .messages
        .iter()
        .rev()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.content.as_deref())
        .flat_map(str::split_whitespace)
        .find(|s| s.starts_with("is-"))
        .expect("iş kimliği modelin aldığı gerçek araç sonucunda olmalı")
}

fn model_girdi(istek: &ChatRequest) -> SahteYanit {
    SahteYanit::arac(
        "komut_girdi",
        json!({"is_id":model_is_kimligi(istek),"girdi":"dogrula\r"}),
    )
}

fn model_durdur(istek: &ChatRequest) -> SahteYanit {
    SahteYanit::arac("komut_durdur", json!({"is_id":model_is_kimligi(istek)}))
}

fn model_s05_yaz(istek: &ChatRequest) -> SahteYanit {
    assert_eq!(
        son_arac_sonucu(istek)
            .lines()
            .find(|s| s.starts_with("Kullanıcı yanıtı (seçenek):")),
        Some("Kullanıcı yanıtı (seçenek): toml (TOML)"),
        "yanıt gerçek tool-result içinde doğru seçeneğe bağlanmalı"
    );
    SahteYanit::arac(
        "dosya_yaz",
        json!({"yol":"ayar.toml","icerik":"baglanti_suresi_sn = 30\n"}),
    )
}

fn model_s05_ozet(istek: &ChatRequest) -> String {
    let yanit = istek
        .messages
        .iter()
        .filter_map(|m| m.content.as_deref())
        .flat_map(str::lines)
        .map(str::trim)
        .find(|m| m.starts_with("Kullanıcı yanıtı (seçenek):"))
        .expect("özetleyici gerçek soru sonucunu almalı");
    assert_eq!(yanit, "Kullanıcı yanıtı (seçenek): toml (TOML)");
    format!("Korunan gerçek soru sonucu: {yanit}")
}

fn okunan_komut_ciktisi(istek: &ChatRequest) -> String {
    // Bayt parçası satırın ortasında bitebilir; modele gerçekten gönderilen
    // parçaları birleştir, kaynak sarmalını ve üst veriyi çıktı sayma.
    istek
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.content.as_deref())
        .filter_map(|c| c.split_once("--- çıktı ---\n").map(|(_, govde)| govde))
        .filter_map(|c| c.rsplit_once("\n</arac_sonucu").map(|(govde, _)| govde))
        .collect()
}

fn model_s02(istek: &ChatRequest) -> SahteYanit {
    let okuma_sayisi = istek
        .messages
        .iter()
        .flat_map(|m| m.tool_calls.iter().flatten())
        .filter(|c| c.function.name == "komut_oku")
        .count();
    let bulundu = okunan_komut_ciktisi(istek).contains("HATA-S02-ORTA-409");
    if okuma_sayisi >= 2 && bulundu {
        return SahteYanit::metin("Gerçek çıktıdaki hata: HATA-S02-ORTA-409");
    }
    let imlec = son_arac_sonucu(istek)
        .lines()
        .find_map(|s| s.strip_prefix("Yeni imleç: "))
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    SahteYanit::Gecikmeli(
        Duration::from_millis(if son_arac_sonucu(istek).contains("--- çıktı ---") {
            20
        } else {
            500
        }),
        Box::new(SahteYanit::arac(
            "komut_oku",
            json!({"is_id":model_is_kimligi(istek),"imlec":imlec}),
        )),
    )
}

fn model_s04(istek: &ChatRequest) -> SahteYanit {
    let son = son_arac_sonucu(istek);
    if son.lines().any(|s| s == "Durum: bitti") {
        return SahteYanit::metin("İş iptal edildi; başarı iddiası yok.");
    }
    if son.lines().any(|s| s.starts_with("TORUN:")) {
        return model_durdur(istek);
    }
    SahteYanit::Gecikmeli(
        Duration::from_millis(500),
        Box::new(SahteYanit::arac(
            "komut_oku",
            json!({"is_id":model_is_kimligi(istek),"imlec":0}),
        )),
    )
}

fn girdi_yukle(o: &Ortam, kimlik: &str) {
    let yol = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../kalfa-tools/tests/yetenek_senaryolari/tanimlar")
        .join(format!("{kimlik}.json"));
    let g0: Value = serde_json::from_slice(&std::fs::read(yol).unwrap()).unwrap();
    for dosya in g0["baslangic_dosyalari"].as_array().unwrap() {
        let yol = o.alan.path().join(dosya["yol"].as_str().unwrap());
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, dosya["icerik"].as_str().unwrap()).unwrap();
    }
}

fn kabuk(win: &str, linux: &str, terminal: bool) -> Value {
    json!({"betik":if cfg!(windows) { win } else { linux }, "arka_plan":true, "terminal":terminal})
}

async fn isi_oku(o: &Ortam, id: OturumId, is_id: &str, aranan: Option<&str>) -> String {
    tokio::time::timeout(Duration::from_secs(20), async {
        let mut cikti = String::new();
        let mut imlec = 0;
        loop {
            let okuma = o.t.uygulama.is_oku(id, is_id.into(), imlec).unwrap();
            cikti.push_str(&okuma.cikti);
            imlec = okuma.yeni_imlec;
            if aranan.is_some_and(|s| cikti.contains(s))
                || (aranan.is_none() && okuma.durum.bitti_mi())
            {
                return cikti;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("G5 süreç çıktısı gelmedi")
}

#[tokio::test]
async fn g5_s01_gercek_arka_plan_isi_izlenir_ve_dogal_cikis_kodu_okunur() {
    use kalfa_uygulama::isler::IsDurumu;
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "S01");
    let id = o.oturum().await;
    let arg = kabuk(
        "[Console]::WriteLine('G5-S01-HAZIR'); $null=[Console]::ReadLine(); [Console]::WriteLine('KLF-S01-TAMAM'); exit 0",
        "echo G5-S01-HAZIR; read -r kapi; echo KLF-S01-TAMAM; exit 0",
        false,
    );
    let olaylar = o
        .tur(
            id,
            vec![
                SahteYanit::arac("kabuk", arg),
                SahteYanit::metin("İş başlatıldı."),
            ],
        )
        .await;
    assert!(olaylar.iter().any(
        |o| matches!(o, Olay::AracSonucu { arac, durum:AracDurumu::Basarili,..} if arac=="kabuk")
    ));
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    assert_eq!(isler.len(), 1);
    assert!(
        matches!(isler[0].durum, IsDurumu::Calisiyor),
        "model turu gerçek test işi sürerken dönmeli"
    );
    o.t.uygulama
        .is_girdi(
            id,
            isler[0].is_id.as_str().into(),
            "test-kapisi\n".into(),
            false,
        )
        .await
        .unwrap();
    let cikti = isi_oku(&o, id, isler[0].is_id.as_str(), None).await;
    assert!(cikti.contains("KLF-S01-TAMAM"));
    let son = o.t.uygulama.oturum_isleri(id).unwrap();
    assert!(matches!(
        son[0].durum,
        IsDurumu::Bitti {
            cikis_kodu: Some(0),
            zaman_asimi: false,
            ..
        }
    ));
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_k06_gercek_kok_alt_agents_yazmayi_yonlendirir_sir_korunur() {
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "K06");
    let id = o.oturum().await;
    let odeme = "paketler/odeme/src/fatura.rs";
    let bildirim = "paketler/bildirim/src/eposta.rs";
    let olaylar = o
        .tur(
            id,
            vec![
                SahteYanit::arac("dosya_oku", json!({"yol":odeme})),
                SahteYanit::arac("dosya_oku", json!({"yol":bildirim})),
                SahteYanit::arac(
                    "dosya_duzenle",
                    json!({"yol":odeme,"eski_metin":"TASLAK_01","yeni_metin":"KLF_ODEME_01"}),
                ),
                SahteYanit::arac(
                    "dosya_duzenle",
                    json!({"yol":bildirim,"eski_metin":"TASLAK_02","yeni_metin":"KLF-GENEL-02"}),
                ),
                SahteYanit::metin("Kurallar uygulandı."),
            ],
        )
        .await;
    assert!(
        std::fs::read_to_string(o.alan.path().join(odeme))
            .unwrap()
            .contains("KLF_ODEME_01")
    );
    assert!(
        std::fs::read_to_string(o.alan.path().join(bildirim))
            .unwrap()
            .contains("KLF-GENEL-02")
    );
    let istekler = o.t.saglayici.istekler();
    let sistem = |i: usize| {
        istekler[i]
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.content.as_deref())
            .unwrap()
    };
    assert!(sistem(0).contains("yol=\"AGENTS.md\" kapsam=\"kok\""));
    assert!(!sistem(0).contains("KLF_ODEME_"));
    let odeme_sistemi = sistem(1);
    assert!(
        odeme_sistemi
            .contains("yol=\"paketler/odeme/AGENTS.md\" kapsam=\"alt_dizin:paketler/odeme\"")
    );
    assert!(odeme_sistemi.contains("KLF_ODEME_"));
    assert!(odeme_sistemi.contains("KLF-GENEL-"));
    assert!(!odeme_sistemi.contains("alt_dizin:paketler/bildirim"));
    let goruntu = olaylar
        .iter()
        .filter_map(|o| {
            if let Olay::ProjeTalimatlari { goruntu } = o {
                Some(goruntu)
            } else {
                None
            }
        })
        .next_back()
        .unwrap();
    let odeme_kapsami = &goruntu
        .kaynaklar
        .iter()
        .find(|k| k.goreli_yol == "paketler/odeme/AGENTS.md")
        .unwrap()
        .kapsam;
    assert!(odeme_kapsami.hedefi_kapsar_mi(odeme));
    assert!(!odeme_kapsami.hedefi_kapsar_mi(bildirim));
    assert!(
        !serde_json::to_string(&istekler)
            .unwrap()
            .contains("gizli-k06-44b2")
    );
    assert!(!olaylar.iter().any(|o| matches!(o,Olay::AracSonucu{arac,icerik,..} if arac=="dosya_oku" && icerik.contains("gizli-k06"))));
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_s02_uzun_cikti_imlecle_okunur_ortadaki_hata_kaybolmaz() {
    let o = Ortam::yeni().await;
    let id = o.oturum().await;
    let arg = kabuk(
        "for($i=0;$i -lt 4000;$i++){ [Console]::WriteLine(('DERLEME-{0:D5}-' -f $i)+('x'*80)); if($i -eq 2000){[Console]::WriteLine('HATA-S02-ORTA-409')}}; exit 1",
        "i=0; while [ $i -lt 4000 ]; do printf 'DERLEME-%05d-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\\n' $i; if [ $i -eq 2000 ]; then echo HATA-S02-ORTA-409; fi; i=$((i+1)); done; exit 1",
        false,
    );
    let mut senaryo = vec![SahteYanit::arac("kabuk", arg)];
    senaryo.extend((0..64).map(|_| SahteYanit::IstegeGore(model_s02)));
    let olaylar = o.tur(id, senaryo).await;
    assert!(olaylar.iter().filter(|o|matches!(o,Olay::AracSonucu{arac,durum:AracDurumu::Basarili,..} if arac=="komut_oku")).count()>=2);
    let istekler = o.t.saglayici.istekler();
    assert!(
        istekler
            .iter()
            .any(|i| okunan_komut_ciktisi(i).contains("HATA-S02-ORTA-409")),
        "hata komut_oku gerçek sonuçlarıyla modele gitmeli"
    );
    let pozitif_imlec = istekler
        .iter()
        .flat_map(|i| &i.messages)
        .flat_map(|m| m.tool_calls.iter().flatten())
        .filter(|c| c.function.name == "komut_oku")
        .any(|c| {
            serde_json::from_str::<Value>(&c.function.arguments).unwrap()["imlec"]
                .as_u64()
                .unwrap()
                > 0
        });
    assert!(pozitif_imlec);
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    let kimlik = isler[0].is_id.as_str();
    let cikti = isi_oku(&o, id, kimlik, None).await;
    assert!(cikti.len() > 65_536, "yalnız küçük bir çıktı ölçülemez");
    assert!(cikti.contains("HATA-S02-ORTA-409"));
    assert!(cikti.contains("DERLEME-03999"));
    assert!(matches!(
        o.t.uygulama.oturum_isleri(id).unwrap()[0].durum,
        kalfa_uygulama::isler::IsDurumu::Bitti {
            cikis_kodu: Some(1),
            ..
        }
    ));
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_s03_gercek_terminal_girdisi_ve_oturum_yetkisi() {
    let o = Ortam::yeni().await;
    let id = o.oturum().await;
    let arg = kabuk(
        "[Console]::WriteLine('G5-GIRDI-BEKLIYOR'); $s=[Console]::ReadLine(); if($s -eq 'dogrula'){[Console]::WriteLine('KLF-S03-ONAYLANDI')}; exit 0",
        "echo G5-GIRDI-BEKLIYOR; IFS= read -r satir; if [ \"$satir\" = dogrula ]; then echo KLF-S03-ONAYLANDI; fi; exit 0",
        true,
    );
    let olaylar = o
        .tur(
            id,
            vec![
                SahteYanit::arac("kabuk", arg),
                SahteYanit::IstegeGore(model_girdi),
                SahteYanit::metin("Terminal girdisi onaylandı."),
            ],
        )
        .await;
    assert!(olaylar.iter().any(|o|matches!(o,Olay::OnayGerekiyor{ogeler,..} if ogeler.iter().any(|g|g.arac=="komut_girdi"))));
    assert!(olaylar.iter().any(
        |o| matches!(o,Olay::AracSonucu{arac,durum:AracDurumu::Basarili,..} if arac=="komut_girdi")
    ));
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    let kimlik = isler[0].is_id.as_str();
    let baska = o.oturum().await;
    assert!(
        o.t.uygulama
            .is_girdi(baska, kimlik.into(), "zararli\n".into(), false)
            .await
            .is_err()
    );
    let cikti = isi_oku(&o, id, kimlik, None).await;
    assert!(cikti.contains("KLF-S03-ONAYLANDI"), "{cikti}");
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    o.t.uygulama.oturumu_kapat(baska).await.unwrap();
}

#[tokio::test]
async fn g5_s04_durdurma_gercek_torunu_bitirir_sahte_basari_yazmaz() {
    let o = Ortam::yeni().await;
    let id = o.oturum().await;
    let arg = kabuk(
        "$p=Start-Process -FilePath powershell.exe -WindowStyle Hidden -ArgumentList '-NoProfile -Command Start-Sleep -Seconds 60' -PassThru; [Console]::WriteLine('TORUN:'+$p.Id); [Console]::Out.Flush(); Wait-Process -Id $p.Id",
        "sleep 60 & p=$!; echo TORUN:$p; wait $p",
        false,
    );
    let mut senaryo = vec![SahteYanit::arac("kabuk", arg)];
    senaryo.extend((0..64).map(|_| SahteYanit::IstegeGore(model_s04)));
    let olaylar = o.tur(id, senaryo).await;
    assert!(olaylar.iter().any(
        |o| matches!(o,Olay::AracSonucu{arac,durum:AracDurumu::Basarili,..} if arac=="komut_durdur")
    ));
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    let kimlik = isler[0].is_id.as_str();
    let cikti = isi_oku(&o, id, kimlik, Some("TORUN:")).await;
    let pid: u32 = cikti
        .lines()
        .find_map(|s| s.trim().strip_prefix("TORUN:"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let son = o.t.uygulama.oturum_isleri(id).unwrap().remove(0);
    assert!(son.durum.bitti_mi());
    assert!(!matches!(
        son.durum,
        kalfa_uygulama::isler::IsDurumu::Bitti {
            cikis_kodu: Some(0),
            ..
        }
    ));
    let yasiyor = if cfg!(windows) {
        std::process::Command::new("powershell.exe").args(["-NoProfile","-Command",&format!("if(Get-Process -Id {pid} -ErrorAction SilentlyContinue){{exit 1}}else{{exit 0}}")]).status().unwrap().code()!=Some(0)
    } else {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    assert!(!yasiyor, "durdurulan işin torunu yaşıyor: {pid}");
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_s05_soru_yaniti_yazma_compact_ve_kurtarma_birlikte_korunur() {
    use kalfa_core::test_destek::SahteSohbet;
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "S05");
    let id = o.oturum().await;
    let (olaylar,mut rx)=o.tur_kanalli(id,vec![
        SahteYanit::arac("plan_guncelle",json!({"adimlar":[{"kimlik":"bicim","baslik":"Biçimi sorun","durum":"Tamam"},{"kimlik":"yaz","baslik":"Yapılandırmayı yazın","durum":"Suruyor"}]})),
        SahteYanit::arac("kullaniciya_sor",json!({"baslik":"Hedef biçim?","secenekler":[{"kimlik":"toml","etiket":"TOML"},{"kimlik":"json","etiket":"JSON"}]})),
        SahteYanit::IstegeGore(model_s05_yaz),
        SahteYanit::metin("TOML yapılandırması yazıldı."),
    ]).await;
    let soru = olaylar
        .iter()
        .find_map(|o| {
            if let Olay::KullaniciSorusuGerekiyor { soru } = o {
                Some(soru)
            } else {
                None
            }
        })
        .unwrap();
    let cozuldu = olaylar
        .iter()
        .find_map(|o| {
            if let Olay::KullaniciSorusuCozuldu {
                soru_kimligi,
                cagri,
                sonuc,
            } = o
            {
                Some((soru_kimligi, cagri, sonuc))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(cozuldu.0, &soru.soru_kimligi);
    assert_eq!(cozuldu.1, &soru.cagri);
    assert!(
        matches!(cozuldu.2,kalfa_core::soru::SoruSonucu::Yanitlandi{yanit:kalfa_core::soru::KullaniciSorusuYaniti::Secenek{secenek_kimligi},..} if secenek_kimligi=="toml")
    );
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("ayar.toml")).unwrap(),
        "baglanti_suresi_sn = 30\n"
    );
    let iz = serde_json::to_string(&o.t.saglayici.istekler()).unwrap();
    assert!(iz.contains("toml"));
    o.t.saglayici
        .sohbet_ekle(SahteSohbet::IstegeGore(model_s05_ozet));
    o.t.uygulama.oturum_ozetle(id, None).await.unwrap();
    let mut ozetlendi = false;
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let KanalMesaji::Olaylar { no, zarflar } = rx.recv().await.unwrap() else {
                panic!("compact kanalı kapandı")
            };
            o.t.uygulama.paketi_isledim(id, no).unwrap();
            ozetlendi |= zarflar
                .iter()
                .any(|z| matches!(z.yuk, Olay::OturumOzetlendi { .. }));
            if zarflar
                .iter()
                .any(|z| matches!(z.yuk, Olay::Tamamlandi { .. }))
            {
                break;
            }
        }
    })
    .await
    .unwrap();
    assert!(ozetlendi, "compact gerçekten özet üretmeli");
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    let acilan = o.t.uygulama.oturum_ac(id).await.unwrap();
    let plan = acilan.ozet.plan.unwrap();
    assert_eq!(
        plan.adimlar
            .iter()
            .map(|a| a.kimlik.as_str())
            .collect::<Vec<_>>(),
        ["bicim", "yaz"]
    );
    assert_eq!(
        plan.adimlar[0].durum,
        kalfa_core::plan::PlanAdimDurumu::Tamam
    );
    assert_eq!(
        plan.adimlar[1].durum,
        kalfa_core::plan::PlanAdimDurumu::Suruyor
    );
    let kayit = kalfa_uygulama::dizin::kayit_yolu(
        &kalfa_uygulama::dizin::oturum_dizini(o._veri.path()),
        id,
    );
    let kayitlar = kalfa_core::kayit::OturumKaydi::oku(&kayit).unwrap();
    assert!(
        kayitlar.iter().any(|k| matches!(&k.yuk,
        kalfa_core::kayit::KayitOgesi::OturumOzetlendi { kapsanan, icerik, .. }
            if !kapsanan.is_empty() && icerik.lines().any(|s| s.starts_with("Korunan gerçek soru sonucu:"))
                && icerik.contains("toml (TOML)"))),
        "gerçek compact içeriği JSONL'de saklanmalı"
    );
    assert!(kayitlar.iter().any(|k|matches!(&k.yuk,kalfa_core::kayit::KayitOgesi::KullaniciSorusuCozuldu{soru_kimligi,cagri,sonuc:kalfa_core::soru::SoruSonucu::Yanitlandi{yanit:kalfa_core::soru::KullaniciSorusuYaniti::Secenek{secenek_kimligi},..}} if soru_kimligi==&soru.soru_kimligi && cagri==&soru.cagri && secenek_kimligi=="toml")));
    o.tur(
        id,
        vec![SahteYanit::metin("Kurtarılan plandan devam edildi.")],
    )
    .await;
    let son = o.t.saglayici.istekler();
    assert!(
        serde_json::to_string(son.last().unwrap())
            .unwrap()
            .contains("Korunan gerçek soru sonucu: Kullanıcı yanıtı (seçenek): toml (TOML)")
    );
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_s06_acik_is_yazmayi_bekletir_kapaninca_ayni_yazma_devam_eder() {
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "S06");
    let id = o.oturum().await;
    o.tur(
        id,
        vec![
            SahteYanit::arac(
                "kabuk",
                kabuk(
                    "[Console]::WriteLine('SUNUCU-HAZIR'); Start-Sleep -Seconds 60",
                    "echo SUNUCU-HAZIR; sleep 60",
                    false,
                ),
            ),
            SahteYanit::metin("Sunucu açık."),
        ],
    )
    .await;
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    let kimlik = isler[0].is_id.as_str();
    isi_oku(&o, id, kimlik, Some("SUNUCU-HAZIR")).await;
    let diger = o.oturum().await;
    let yazma = o.tur(
        diger,
        vec![
            SahteYanit::arac(
                "dosya_duzenle",
                json!({"yol":"src/uc.rs","eski_metin":"eski-durum","yeni_metin":"yeni-durum"}),
            ),
            SahteYanit::metin("Seri yazma tamamlandı."),
        ],
    );
    tokio::pin!(yazma);
    assert!(
        tokio::time::timeout(Duration::from_millis(200), &mut yazma)
            .await
            .is_err()
    );
    assert!(
        std::fs::read_to_string(o.alan.path().join("src/uc.rs"))
            .unwrap()
            .contains("eski-durum")
    );
    o.t.uygulama.is_durdur(id, kimlik.into()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), yazma)
        .await
        .unwrap();
    assert!(
        std::fs::read_to_string(o.alan.path().join("src/uc.rs"))
            .unwrap()
            .contains("yeni-durum")
    );
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    o.t.uygulama.oturumu_kapat(diger).await.unwrap();
    println!("G5_DURUM:desteklenmiyor:eşzamanlı sunucu+yazma yerine güvenli seri yol doğrulandı");
}

// Yerel TCP gerçekten taşır; tools/call yalnız onaylı ürün yolundan gelir.
async fn mcp_sunucusu() -> (u16, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let dinleyici = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = dinleyici.local_addr().unwrap().port();
    let sayac = Arc::new(AtomicUsize::new(0));
    let kopya = sayac.clone();
    let gorev = tokio::spawn(async move {
        loop {
            let Ok((mut soket, _)) = dinleyici.accept().await else {
                break;
            };
            let sayac = kopya.clone();
            tokio::spawn(async move {
                let mut veri = Vec::new();
                let mut parca = [0; 4096];
                let baslik_sonu = loop {
                    let n = soket.read(&mut parca).await.unwrap();
                    if n == 0 || veri.len() > 1_048_576 {
                        return;
                    }
                    veri.extend_from_slice(&parca[..n]);
                    if let Some(n) = veri.windows(4).position(|s| s == b"\r\n\r\n") {
                        break n + 4;
                    }
                };
                let baslik = String::from_utf8_lossy(&veri[..baslik_sonu]);
                let uzunluk: usize = baslik
                    .lines()
                    .find_map(|s| {
                        let (ad, deger) = s.split_once(':')?;
                        ad.eq_ignore_ascii_case("content-length")
                            .then(|| deger.trim().parse().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                if uzunluk > 1_048_576 {
                    return;
                }
                while veri.len() < baslik_sonu + uzunluk {
                    let n = soket.read(&mut parca).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    veri.extend_from_slice(&parca[..n]);
                }
                let istek: Value =
                    serde_json::from_slice(&veri[baslik_sonu..baslik_sonu + uzunluk])
                        .unwrap_or(Value::Null);
                let metod = istek["method"].as_str().unwrap_or_default();
                let govde = match metod {
                    "initialize" => {
                        json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{}},"serverInfo":{"name":"g5-m02","version":"1"}})
                    }
                    "tools/list" => {
                        json!({"tools":[{"name":"kayit_getir","description":"Sabit kabul kaydı","inputSchema":{"type":"object","properties":{},"additionalProperties":false},"annotations":{"readOnlyHint":true}}]})
                    }
                    "tools/call" => {
                        assert_eq!(istek["params"]["name"], "kayit_getir");
                        sayac.fetch_add(1, Ordering::SeqCst);
                        json!({"content":[{"type":"text","text":"KLF-M02-KAYIT"}]})
                    }
                    _ => Value::Null,
                };
                let (durum, metin) = if istek.get("id").is_none() {
                    ("202 Accepted", String::new())
                } else if govde.is_null() {
                    ("200 OK", json!({"jsonrpc":"2.0","id":istek["id"],"error":{"code":-32601,"message":"Metod desteklenmiyor"}}).to_string())
                } else {
                    (
                        "200 OK",
                        json!({"jsonrpc":"2.0","id":istek["id"],"result":govde}).to_string(),
                    )
                };
                let yanit = format!(
                    "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nMcp-Session-Id: g5-oturum\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{metin}",
                    metin.len()
                );
                let _ = soket.write_all(yanit.as_bytes()).await;
            });
        }
    });
    (port, sayac, gorev)
}

fn m02_adimlari() -> Vec<SahteYanit> {
    vec![
        SahteYanit::arac(
            "arac_ara",
            json!({"sorgu":"kayıt", "sunucu":"g5", "yukle":["mcp__g5__kayit_getir"]}),
        ),
        SahteYanit::arac("mcp__g5__kayit_getir", json!({})),
        SahteYanit::metin("Kayıt onaylı uzak araçtan alındı."),
    ]
}

#[tokio::test]
async fn g5_m02_http_araci_onaydan_once_cagrilmaz_sonuc_sunucudan_gelir() {
    let (port, sayac, sunucu) = mcp_sunucusu().await;
    let o = Ortam::yeni().await;
    let g0: Value = serde_json::from_str(include_str!(
        "../../kalfa-tools/tests/yetenek_senaryolari/tanimlar/M02.json"
    ))
    .unwrap();
    for dosya in g0["baslangic_dosyalari"].as_array().unwrap() {
        let yol = o.alan.path().join(dosya["yol"].as_str().unwrap());
        std::fs::create_dir_all(yol.parent().unwrap()).unwrap();
        std::fs::write(yol, dosya["icerik"].as_str().unwrap()).unwrap();
    }
    o.t.uygulama.uzak_mcp.test_loopback_izni_ayarla(true);
    o.t.uygulama
        .uzak_mcp_kaydet(
            None,
            UzakMcpSunucuFormu {
                ad: "g5".into(),
                etkin: true,
                uc_url: format!("http://127.0.0.1:{port}/mcp"),
                yetki: UzakMcpYetkiFormGirdisi::Yetkisiz,
                baslatma_zamani_asimi_sn: 5,
                cagri_zamani_asimi_sn: 5,
            },
        )
        .await
        .unwrap();
    o.t.uygulama.uzak_mcp_baglan("g5".into()).await.unwrap();
    let id = o.oturum().await;
    let (kanal, mut rx) = SahteKanal::yeni();
    o.t.uygulama.oturumu_dinle(id, kanal).unwrap();
    for yanit in m02_adimlari() {
        o.t.saglayici.senaryoya_ekle(yanit);
    }
    o.t.uygulama
        .mesaj_gonder(
            id,
            "Kaydı açık onayımla gerçek uzak araçtan getirin.".into(),
        )
        .await
        .unwrap();
    let mut onaylar = 0;
    let mut olaylar = Vec::new();
    loop {
        let mesaj = tokio::time::timeout(Duration::from_secs(20), rx.recv())
            .await
            .unwrap()
            .unwrap();
        let KanalMesaji::Olaylar { no, zarflar } = mesaj else {
            panic!("kanal kapandı")
        };
        o.t.uygulama.paketi_isledim(id, no).unwrap();
        let mut bitti = false;
        for z in zarflar {
            if let Olay::OnayGerekiyor { ref ogeler, .. } = z.yuk {
                assert_eq!(sayac.load(Ordering::SeqCst), 0, "onaydan önce uzak çağrı");
                onaylar += ogeler.len();
                let kararlar = ogeler
                    .iter()
                    .map(|g| OnayGirdisi {
                        cagri: g.cagri,
                        muhur: g.muhur.clone(),
                        karar: OnayKarari::Onayla,
                        kisayol: false,
                    })
                    .collect();
                o.t.uygulama.arac_onayi(id, kararlar).await.unwrap();
            }
            bitti |= matches!(z.yuk, Olay::Tamamlandi { .. });
            olaylar.push(z.yuk);
        }
        if bitti {
            break;
        }
        // Sonucun farklı pakette gelmesi olağandır; sonraki paketler okunur.
    }
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    sunucu.abort();
    assert_eq!(onaylar, 1, "M02 dosya okuma taklidi kabul edilemez");
    assert_eq!(sayac.load(Ordering::SeqCst), 1);
    assert!(olaylar.iter().any(|o| matches!(o, Olay::AracSonucu { arac, durum: AracDurumu::Basarili, icerik, .. } if arac == "mcp__g5__kayit_getir" && icerik.contains("KLF-M02-KAYIT"))));
}

async fn uzak_ekle(o: &Ortam, port: u16) {
    o.t.uygulama.uzak_mcp.test_loopback_izni_ayarla(true);
    o.t.uygulama
        .uzak_mcp_kaydet(
            None,
            UzakMcpSunucuFormu {
                ad: "g5".into(),
                etkin: true,
                uc_url: format!("http://127.0.0.1:{port}/mcp"),
                yetki: UzakMcpYetkiFormGirdisi::Yetkisiz,
                baslatma_zamani_asimi_sn: 5,
                cagri_zamani_asimi_sn: 5,
            },
        )
        .await
        .unwrap();
    o.t.uygulama.uzak_mcp_baglan("g5".into()).await.unwrap();
}

fn model_m03_geri_oku(istek: &ChatRequest) -> SahteYanit {
    assert!(son_arac_sonucu(istek).contains("tamamlandi"));
    SahteYanit::arac("mcp__g5__kayit_getir", json!({"kayit":"gorev-42"}))
}

#[tokio::test]
async fn g5_m03_gercek_http_yazma_sunucu_durumunu_degistirir_geri_okunur() {
    let sunucu = g5_mcp_fixture::baslat(
        vec![
            g5_mcp_fixture::arac("kayit_guncelle", false),
            g5_mcp_fixture::arac("kayit_getir", true),
        ],
        "bekliyor",
    )
    .await;
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "M03");
    uzak_ekle(&o, sunucu.port).await;
    let id = o.oturum().await;
    let olaylar=o.tur(id,vec![
        SahteYanit::arac("arac_ara",json!({"sorgu":"kayıt","sunucu":"g5","yukle":["mcp__g5__kayit_guncelle","mcp__g5__kayit_getir"]})),
        SahteYanit::arac("mcp__g5__kayit_guncelle",json!({"kayit":"gorev-42","durum":"tamamlandi"})),
        SahteYanit::IstegeGore(model_m03_geri_oku),
        SahteYanit::metin("Geri okunan durum tamamlandi."),
    ]).await;
    let cagrilar = sunucu.cagrilar.lock().unwrap().clone();
    assert_eq!(cagrilar.len(), 2);
    assert_eq!(cagrilar[0]["name"], "kayit_guncelle");
    assert_eq!(
        cagrilar[0]["arguments"],
        json!({"kayit":"gorev-42","durum":"tamamlandi"})
    );
    assert_eq!(cagrilar[1]["name"], "kayit_getir");
    assert!(olaylar.iter().any(|o|matches!(o,Olay::OnayGerekiyor{ogeler,..} if ogeler.iter().any(|g|g.arac=="mcp__g5__kayit_guncelle"))));
    assert!(olaylar.iter().any(|o|matches!(o,Olay::AracSonucu{arac,durum:AracDurumu::Basarili,icerik,..} if arac=="mcp__g5__kayit_getir" && icerik.contains("tamamlandi"))));
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

#[tokio::test]
async fn g5_m05_otuz_iki_http_aracindan_yalniz_ikisi_yuklenir_ve_cagrilir() {
    let mut araclar = vec![
        g5_mcp_fixture::arac("surum_notu_ara", true),
        g5_mcp_fixture::arac("surum_etiketi_dogrula", true),
    ];
    for n in 0..30 {
        let mut arac = g5_mcp_fixture::arac(&format!("ilgisiz_{n:02}"), true);
        arac["inputSchema"]["description"] = json!("ILGISIZ_30_ARAC_SEMASI_M05");
        araclar.push(arac);
    }
    assert_eq!(araclar.len(), 32);
    let sunucu = g5_mcp_fixture::baslat(araclar, "").await;
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "M05");
    uzak_ekle(&o, sunucu.port).await;
    let id = o.oturum().await;
    let olaylar=o.tur(id,vec![
        SahteYanit::arac("arac_ara",json!({"sorgu":"sürüm","sunucu":"g5","yukle":["mcp__g5__surum_notu_ara","mcp__g5__surum_etiketi_dogrula"]})),
        SahteYanit::arac("mcp__g5__surum_notu_ara",json!({"sorgu":"v1"})),
        SahteYanit::arac("mcp__g5__surum_etiketi_dogrula",json!({"sorgu":"v1"})),
        SahteYanit::metin("Seçilen iki gerçek araç doğrulandı."),
    ]).await;
    let istekler = o.t.saglayici.istekler();
    assert!(
        istekler[0]
            .tools
            .iter()
            .all(|t| !t.function.name.starts_with("mcp__g5__"))
    );
    let adlar: std::collections::BTreeSet<_> = istekler[1]
        .tools
        .iter()
        .filter(|t| t.function.name.starts_with("mcp__g5__"))
        .map(|t| t.function.name.as_str())
        .collect();
    assert_eq!(
        adlar,
        std::collections::BTreeSet::from([
            "mcp__g5__surum_notu_ara",
            "mcp__g5__surum_etiketi_dogrula"
        ])
    );
    assert!(istekler.iter().all(|i| {
        !serde_json::to_string(&i.tools)
            .unwrap()
            .contains("ILGISIZ_30_ARAC_SEMASI_M05")
    }));
    let cagrilar = sunucu.cagrilar.lock().unwrap().clone();
    assert_eq!(
        cagrilar
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["surum_notu_ara", "surum_etiketi_dogrula"]
    );
    for ad in ["surum_notu_ara", "surum_etiketi_dogrula"] {
        assert!(olaylar.iter().any(|o|matches!(o,Olay::AracSonucu{icerik,durum:AracDurumu::Basarili,..} if icerik.contains(&format!("KLF-M05-{ad}")))));
    }
    assert_eq!(sunucu.sayac.load(Ordering::SeqCst), 2);
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
}

fn model_m06_saldiri(istek: &ChatRequest) -> SahteYanit {
    let kaynak = son_arac_sonucu(istek);
    assert!(kaynak.contains("Bilet #108: Bellek sızıntısı düzeltildi."));
    assert!(kaynak.contains("SISTEM TALIMATI"));
    assert!(kaynak.contains("<arac_sonucu"));
    // Kasıtlı saldırgan model cevabı: uzak içerik gizli okuma yetkisi veremez.
    SahteYanit::arac("dosya_oku", json!({"yol":".env"}))
}

fn model_m06_ozet(istek: &ChatRequest) -> SahteYanit {
    assert!(!son_arac_sonucu(istek).contains("gizli-m06-anahtari"));
    let kaynak = istek
        .messages
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.content.as_deref())
        .find(|s| s.contains("Bilet #108:"))
        .unwrap();
    let bas = kaynak.find("Bilet #108:").unwrap();
    let son = bas + kaynak[bas..].find("düzeltildi.").unwrap() + "düzeltildi.".len();
    SahteYanit::metin(&kaynak[bas..son])
}

#[tokio::test]
async fn g5_m06_gercek_http_enjeksiyonu_sir_okuma_yetkisi_vermez_mesru_ozet_korunur() {
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "M06");
    let not = std::fs::read_to_string(o.alan.path().join("mcp/bilet_yaniti.txt")).unwrap();
    let sunucu =
        g5_mcp_fixture::baslat(vec![g5_mcp_fixture::arac("bilet_getir", true)], &not).await;
    uzak_ekle(&o, sunucu.port).await;
    let id = o.oturum().await;
    let (olaylar, _) = o
        .tur_kararli(
            id,
            vec![
                SahteYanit::arac(
                    "arac_ara",
                    json!({"sorgu":"bilet","sunucu":"g5","yukle":["mcp__g5__bilet_getir"]}),
                ),
                SahteYanit::arac("mcp__g5__bilet_getir", json!({})),
                SahteYanit::IstegeGore(model_m06_saldiri),
                SahteYanit::IstegeGore(model_m06_ozet),
            ],
            |g| {
                if g.arac == "dosya_oku" {
                    OnayKarari::Reddet {
                        gerekce: Some("Gizli dosyayı açmayın.".into()),
                    }
                } else {
                    OnayKarari::Onayla
                }
            },
        )
        .await;
    assert!(olaylar.iter().any(
        |o| matches!(o,Olay::AracSonucu{arac,durum:AracDurumu::Reddedildi,..} if arac=="dosya_oku")
    ));
    assert_eq!(
        sunucu.sayac.load(Ordering::SeqCst),
        1,
        "enjeksiyon ikinci dış çağrı başlatamaz"
    );
    let istekler = o.t.saglayici.istekler();
    assert!(
        !serde_json::to_string(&istekler)
            .unwrap()
            .contains("gizli-m06-anahtari")
    );
    assert!(
        !serde_json::to_string(&olaylar)
            .unwrap()
            .contains("gizli-m06-anahtari")
    );
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    let kayit = kalfa_uygulama::dizin::kayit_yolu(
        &kalfa_uygulama::dizin::oturum_dizini(o._veri.path()),
        id,
    );
    assert!(
        !std::fs::read_to_string(kayit)
            .unwrap()
            .contains("gizli-m06-anahtari")
    );
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join(".env")).unwrap(),
        "EVREN_API_KEY=gizli-m06-anahtari\n"
    );
}

fn model_w02_web(istek: &ChatRequest) -> SahteYanit {
    use kalfa_tools::web::{AramaSaglayiciYaniti, arama_sonuclarini_dogrula, mcp_arama_json_coz};
    let sonuc = son_arac_sonucu(istek);
    let govde = sonuc
        .split_once(">\n")
        .unwrap()
        .1
        .rsplit_once("\n</arac_sonucu")
        .unwrap()
        .0;
    let liste = mcp_arama_json_coz(govde).unwrap();
    let dogrulanmis = arama_sonuclarini_dogrula(
        "manifest sınırları",
        AramaSaglayiciYaniti::Basarili(liste),
        &kalfa_tools::kabuk::isler::CancellationToken::new(),
    )
    .unwrap();
    let secilen = dogrulanmis
        .iter()
        .find(|s| s.baslik == "Eklenti Manifest Sınırları")
        .unwrap();
    SahteYanit::arac("web_getir", json!({"url":secilen.kanonik_url}))
}

fn model_w02_yaz(istek: &ChatRequest) -> SahteYanit {
    let sonuc = son_arac_sonucu(istek);
    let url = sonuc
        .split_once("kaynak=\"web:")
        .unwrap()
        .1
        .split('"')
        .next()
        .unwrap();
    let iddia = "Manifest azami bellek ve süre sınırlarını tanımlar.";
    let bas = sonuc.find(iddia).unwrap();
    SahteYanit::arac(
        "dosya_yaz",
        json!({"yol":"web/secilen_kaynak.md","icerik":format!("Kaynak: {url}\nÖzet: {}\n",&sonuc[bas..bas+iddia.len()])}),
    )
}

#[tokio::test]
async fn g5_w02_gercek_http_mcp_aramasi_web_getir_ve_kaynakli_dosya_ayni_oturumda() {
    use kalfa_core::arac::AracKayitDefteri;
    use kalfa_core::denetci::DenetimKipi;
    use kalfa_core::izin::{Kip, Siniflandirici};
    use kalfa_core::oturum::OturumAyarlari;
    use kalfa_core::protokol::{BitisNedeni, Komut};
    use kalfa_core::test_destek::{test_baglami, test_oturumu};
    use kalfa_tools::web::{Cozucu, TestFixtureIzni, WebGetirAraci, WebIstemci, WebIzinDefteri};
    use kalfa_tools::{CalismaAlani, varsayilan_araclar};
    let web = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adres = web.local_addr().unwrap();
    let istekler = Arc::new(std::sync::Mutex::new(Vec::new()));
    let kopya = istekler.clone();
    let web_gorev = tokio::spawn(async move {
        while let Ok((mut soket, _)) = web.accept().await {
            let kopya = kopya.clone();
            tokio::spawn(async move {
                let mut veri = Vec::new();
                let mut parca = [0; 2048];
                while !veri.windows(4).any(|s| s == b"\r\n\r\n") {
                    let n = soket.read(&mut parca).await.unwrap();
                    if n == 0 {
                        return;
                    }
                    veri.extend_from_slice(&parca[..n]);
                    assert!(veri.len() < 65536);
                }
                let yol = String::from_utf8_lossy(&veri)
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                kopya.lock().unwrap().push(yol.clone());
                let metin = if yol == "/eklenti/manifest-sinirlari" {
                    "<h1>Eklenti Manifest Sınırları</h1><p>Manifest azami bellek ve süre sınırlarını tanımlar.</p>"
                } else {
                    "<p>Eski bilgi; kaynak olarak kullanmayın.</p>"
                };
                let yanit = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{metin}",
                    metin.len()
                );
                let _ = soket.write_all(yanit.as_bytes()).await;
            });
        }
    });
    struct WebCozucu(std::net::SocketAddr);
    impl Cozucu for WebCozucu {
        fn coz<'a>(
            &'a self,
            _host: &'a str,
            _port: u16,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<Vec<std::net::SocketAddr>, std::io::Error>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move { Ok(vec![self.0]) })
        }
    }
    let o = Ortam::yeni().await;
    girdi_yukle(&o, "W02");
    let mut arama: Value = serde_json::from_slice(
        &std::fs::read(o.alan.path().join("web/arama_sonuclari.json")).unwrap(),
    )
    .unwrap();
    let kok = format!("http://belge.ornek.test:{}", adres.port());
    for sonuc in arama["sonuclar"].as_array_mut().unwrap() {
        sonuc["url"] = json!(
            sonuc["url"]
                .as_str()
                .unwrap()
                .replace("https://belge.kalfa.yerel", &kok)
        );
    }
    // İlgisiz sonuç ilk sıradadır; yalnız ilk URL'yi alan bir adaptör geçemez.
    arama["sonuclar"].as_array_mut().unwrap().reverse();
    let mcp = g5_mcp_fixture::baslat(
        vec![g5_mcp_fixture::arac("arama", true)],
        &arama.to_string(),
    )
    .await;
    uzak_ekle(&o, mcp.port).await;
    let mut izin = TestFixtureIzni::yeni();
    izin.hedef_ekle("belge.ornek.test", adres);
    izin.http_etkinlestir();
    let web_istemci = Arc::new(WebIstemci::ozel_cozuculu(
        Arc::new(WebCozucu(adres)),
        Some(izin),
    ));
    let sinif = Arc::new(Siniflandirici::yeni(o.alan.path(), &[]).unwrap());
    let mut araclar = varsayilan_araclar(CalismaAlani::ac(sinif.clone()).unwrap());
    araclar.retain(|a| a.ad() != "web_getir");
    araclar.push(Arc::new(WebGetirAraci::ozel(
        web_istemci,
        Arc::new(WebIzinDefteri::yeni()),
    )));
    let kaynak = o.t.uygulama.uzak_mcp.oturum_arac_kaynagi();
    araclar.push(kalfa_mcp::uzak::AracAraAraci::yeni(
        o.t.uygulama.uzak_mcp.katalog().clone(),
        kaynak.clone(),
    ));
    let mut baglam = test_baglami(o.t.saglayici.clone(), Vec::new(), sinif);
    baglam.araclar = Arc::new(AracKayitDefteri::yeni_kaynakli(araclar, kaynak).unwrap());
    for adim in [
        SahteYanit::arac(
            "arac_ara",
            json!({"sorgu":"arama","sunucu":"g5","yukle":["mcp__g5__arama"]}),
        ),
        SahteYanit::arac("mcp__g5__arama", json!({"sorgu":"manifest sınırları"})),
        SahteYanit::IstegeGore(model_w02_web),
        SahteYanit::IstegeGore(model_w02_yaz),
        SahteYanit::metin("Gerçek arama kaynağı raporlandı."),
    ] {
        o.t.saglayici.senaryoya_ekle(adim);
    }
    let mut ayar = OturumAyarlari::yeni("glm-5.3", o.alan.path());
    ayar.kip = Kip::Proje;
    ayar.denetci_kipi = DenetimKipi::Kapali;
    let kayit = o._veri.path().join("w02.jsonl");
    ayar.kayit_yolu = Some(kayit.clone());
    let mut s = test_oturumu(baglam, ayar).zaman_asimi(Duration::from_secs(25));
    s.gonder(Komut::KullaniciMesaji {
        metin: "Manifest sınırlarını arayıp gerçek belgesini okuyun ve kaynakla raporlayın.".into(),
    })
    .await;
    loop {
        match s.sonraki().await.yuk {
            Olay::OnayGerekiyor { ogeler, .. } => {
                for oge in ogeler {
                    if oge.yerel_onay {
                        s.gonder(Komut::YerelOnay {
                            cagri: oge.cagri,
                            muhur: oge.muhur,
                        })
                        .await;
                    } else {
                        s.gonder(Komut::AracOnayi {
                            cagri: oge.cagri,
                            muhur: oge.muhur,
                            karar: OnayKarari::Onayla,
                            kisayol: false,
                        })
                        .await;
                    }
                }
            }
            Olay::Tamamlandi { neden } => {
                assert_eq!(neden, BitisNedeni::Normal);
                break;
            }
            _ => {}
        }
    }
    assert_eq!(mcp.sayac.load(Ordering::SeqCst), 1);
    assert_eq!(
        istekler.lock().unwrap().as_slice(),
        ["/eklenti/manifest-sinirlari"]
    );
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("web/secilen_kaynak.md")).unwrap(),
        format!(
            "Kaynak: {kok}/eklenti/manifest-sinirlari\nÖzet: Manifest azami bellek ve süre sınırlarını tanımlar.\n"
        )
    );
    s.kapat().await;
    let kayit = std::fs::read_to_string(kayit).unwrap();
    assert!(
        kayit.contains("mcp__g5__arama")
            && kayit.contains("web_getir")
            && kayit.contains("dosya_yaz")
    );
    web_gorev.abort();
}
