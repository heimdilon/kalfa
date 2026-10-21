//! G5 v2: gerçek uygulama/oturum yolu; yalnız dış model ve sunucu verisi sabittir.
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use kalfa_core::aktarim::KanalMesaji;
use kalfa_core::kimlik::OturumId;
use kalfa_core::protokol::{AracDurumu, Olay, OnayKarari};
use kalfa_core::saglayici::Saglayici;
use kalfa_core::test_destek::{SahteSaglayici, SahteYanit};
use kalfa_uygulama::mcp::komutlar::{UzakMcpSunucuFormu, UzakMcpYetkiFormGirdisi};
use kalfa_uygulama::sozlesme::OnayGirdisi;
use kalfa_uygulama::test_destek::{SahteKanal, TestUygulamasi, test_uygulamasi};
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedReceiver;

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
                            karar: OnayKarari::Onayla,
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
                let son = matches!(zarf.yuk, Olay::Tamamlandi { .. });
                olaylar.push(zarf.yuk);
                if son {
                    return (olaylar, rx);
                }
            }
        }
    }
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
        "[Console]::WriteLine('G5-S01-HAZIR'); Start-Sleep -Milliseconds 500; [Console]::WriteLine('KLF-S01-TAMAM'); exit 0",
        "echo G5-S01-HAZIR; sleep .5; echo KLF-S01-TAMAM; exit 0",
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
    let istekler = serde_json::to_string(&o.t.saglayici.istekler()).unwrap();
    assert!(istekler.contains("KLF_ODEME_"));
    assert!(istekler.contains("KLF-GENEL-"));
    assert!(!istekler.contains("gizli-k06-44b2"));
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
    o.tur(
        id,
        vec![
            SahteYanit::arac("kabuk", arg),
            SahteYanit::metin("İş başlatıldı."),
        ],
    )
    .await;
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
async fn g5_s03_gercek_terminal_girdi_boyut_ve_oturum_yetkisi() {
    let o = Ortam::yeni().await;
    let id = o.oturum().await;
    let arg = kabuk(
        "[Console]::WriteLine('G5-GIRDI-BEKLIYOR'); $s=[Console]::ReadLine(); if($s -eq 'dogrula'){[Console]::WriteLine('KLF-S03-ONAYLANDI')}; exit 0",
        "echo G5-GIRDI-BEKLIYOR; IFS= read -r satir; if [ \"$satir\" = dogrula ]; then echo KLF-S03-ONAYLANDI; fi; exit 0",
        true,
    );
    o.tur(
        id,
        vec![
            SahteYanit::arac("kabuk", arg),
            SahteYanit::metin("Terminal hazır."),
        ],
    )
    .await;
    let isler = o.t.uygulama.oturum_isleri(id).unwrap();
    let kimlik = isler[0].is_id.as_str();
    isi_oku(&o, id, kimlik, Some("G5-GIRDI-BEKLIYOR")).await;
    let baska = o.oturum().await;
    assert!(
        o.t.uygulama
            .is_girdi(baska, kimlik.into(), "zararli\n".into(), false)
            .await
            .is_err()
    );
    assert!(
        o.t.uygulama
            .is_boyutlandir(id, kimlik.into(), 30, 100)
            .is_ok()
    );
    // Gerçek terminalde Enter CR'dır; borulu CLI girdi testi LF kullanır.
    o.t.uygulama
        .is_girdi(id, kimlik.into(), "dogrula\r".into(), false)
        .await
        .unwrap();
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
    o.tur(
        id,
        vec![
            SahteYanit::arac("kabuk", arg),
            SahteYanit::metin("İş sürüyor."),
        ],
    )
    .await;
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
    let son = o.t.uygulama.is_durdur(id, kimlik.into()).await.unwrap();
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
        SahteYanit::arac("dosya_yaz",json!({"yol":"ayar.toml","icerik":"baglanti_suresi_sn = 30\n"})),
        SahteYanit::metin("TOML yapılandırması yazıldı."),
    ]).await;
    assert!(
        olaylar
            .iter()
            .any(|o| matches!(o, Olay::KullaniciSorusuGerekiyor { .. }))
    );
    assert_eq!(
        std::fs::read_to_string(o.alan.path().join("ayar.toml")).unwrap(),
        "baglanti_suresi_sn = 30\n"
    );
    let iz = serde_json::to_string(&o.t.saglayici.istekler()).unwrap();
    assert!(iz.contains("toml"));
    o.t.saglayici.sohbet_ekle(SahteSohbet::Metin(
        "Kullanıcı TOML seçti; ayar.toml yazıldı, iki adımlı plan sürüyor.".into(),
    ));
    o.t.uygulama.oturum_ozetle(id, None).await.unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let KanalMesaji::Olaylar { no, zarflar } = rx.recv().await.unwrap() else {
                panic!("compact kanalı kapandı")
            };
            o.t.uygulama.paketi_isledim(id, no).unwrap();
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
    o.t.uygulama.oturumu_kapat(id).await.unwrap();
    let acilan = o.t.uygulama.oturum_ac(id).await.unwrap();
    assert_eq!(acilan.ozet.plan.unwrap().adimlar.len(), 2);
    let kayit = kalfa_uygulama::dizin::kayit_yolu(
        &kalfa_uygulama::dizin::oturum_dizini(o._veri.path()),
        id,
    );
    let metin = std::fs::read_to_string(kayit).unwrap();
    assert!(metin.contains("KullaniciSorusuCozuldu"));
    assert!(metin.contains("toml"));
    assert!(metin.contains("Ozet"));
    o.tur(
        id,
        vec![SahteYanit::metin("Kurtarılan plandan devam edildi.")],
    )
    .await;
    let son = o.t.saglayici.istekler();
    assert!(
        serde_json::to_string(son.last().unwrap())
            .unwrap()
            .contains("TOML")
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
