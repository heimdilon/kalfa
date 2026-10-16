//! Özel izleme iş parçacığında çalışan, sınırlı MCP istemcisi.
use crate::tasima::{KUYRUK, SinirliTasima, TasimaHatasi, TasimaKontrol, YenilemeDurumu};
use rmcp::model::{
    CallToolRequest, CallToolRequestParams, CallToolResult, ClientRequest, PaginatedRequestParams,
    ProtocolVersion, ServerResult, SubscriptionFilter, Tool,
};
use rmcp::service::{
    ClientInitializeError, ClientLifecycleMode, PeerRequestOptions, ServiceError, Subscription,
};
use rmcp::{ClientServiceExt, RoleClient};
use std::collections::HashSet;
use std::fmt;
use std::future::Future;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};

const AZAMI_SAYFA: usize = 10;
const AZAMI_ARAC: usize = 128;
const AZAMI_TANIM: usize = 256 * 1024;

#[derive(Clone)]
pub enum McpHatasi {
    UzakRpc(RpcHatasi),
    Tasima(TasimaHatasi),
    Baglanti,
    Kapali,
    Iptal,
    AracListesiUzun,
    AracSayisi,
    TanimBoyutu,
    DesteklenmeyenCevap,
}

/// Ham uzak hatayı yalnız açıkça isteyen denetçiye veren kapalı taşıyıcı.
#[derive(Clone)]
pub struct RpcHatasi {
    kod: i32,
    metin: String,
    veri: Option<serde_json::Value>,
}

impl McpHatasi {
    /// Uzak RPC metnini denetçiye verir; Debug ve Display bu alanı yazmaz.
    pub fn uzak_rpc(&self) -> Option<(i32, &str, Option<&serde_json::Value>)> {
        match self {
            Self::UzakRpc(hata) => Some((hata.kod, &hata.metin, hata.veri.as_ref())),
            _ => None,
        }
    }
}

impl fmt::Debug for McpHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UzakRpc(hata) => f.debug_struct("UzakRpc").field("kod", &hata.kod).finish(),
            Self::Tasima(hata) => f.debug_tuple("Tasima").field(hata).finish(),
            Self::Baglanti => f.write_str("Baglanti"),
            Self::Kapali => f.write_str("Kapali"),
            Self::Iptal => f.write_str("Iptal"),
            Self::AracListesiUzun => f.write_str("AracListesiUzun"),
            Self::AracSayisi => f.write_str("AracSayisi"),
            Self::TanimBoyutu => f.write_str("TanimBoyutu"),
            Self::DesteklenmeyenCevap => f.write_str("DesteklenmeyenCevap"),
        }
    }
}

impl fmt::Display for McpHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UzakRpc(hata) => write!(f, "MCP uzak RPC hatası (kod {})", hata.kod),
            Self::Tasima(hata) => write!(f, "MCP taşıma hatası: {hata}"),
            Self::Baglanti => f.write_str("MCP sunucusuyla güvenli bağlantı kurulamadı"),
            Self::Kapali => f.write_str("MCP istemcisi kapalı"),
            Self::Iptal => f.write_str("MCP işlemi kapatılırken iptal edildi"),
            Self::AracListesiUzun => f.write_str("MCP araç listesi sayfa sınırını aştı"),
            Self::AracSayisi => f.write_str("MCP araç sayısı sınırı aşıldı"),
            Self::TanimBoyutu => f.write_str("MCP araç tanımı bayt sınırını aştı"),
            Self::DesteklenmeyenCevap => {
                f.write_str("MCP sunucusu desteklenmeyen yanıt biçimi döndürdü")
            }
        }
    }
}

impl std::error::Error for McpHatasi {}

enum Komut {
    AraclariListele(oneshot::Sender<Result<Vec<Tool>, McpHatasi>>),
    AracCagir {
        ad: String,
        argumanlar: rmcp::model::JsonObject,
        yanit: oneshot::Sender<Result<CallToolResult, McpHatasi>>,
    },
}

pub struct McpIstemcisi {
    komutlar: mpsc::Sender<Komut>,
    kapat: watch::Sender<bool>,
    is_parcacigi: Option<thread::JoinHandle<()>>,
    birlestirme: Option<tokio::task::JoinHandle<thread::Result<()>>>,
    tasima_kontrol: TasimaKontrol,
    yenileme: Arc<YenilemeDurumu>,
}

impl McpIstemcisi {
    /// Taşıma görevleri çağıranın Tokio çalışma zamanında kalır; rmcp yalnızca
    /// özel current-thread çalışma zamanında çalışır.
    pub async fn yeni(
        tasima: SinirliTasima,
        hata: watch::Receiver<Option<TasimaHatasi>>,
    ) -> Result<Self, McpHatasi> {
        let yenileme = tasima.yenileme.clone();
        let tasima_kontrol = tasima.kontrol();
        let is_parcacigi_yenileme = yenileme.clone();
        let (komut_gonder, komut_al) = mpsc::channel(KUYRUK);
        let (kapat_gonder, kapat_al) = watch::channel(false);
        let (hazir_gonder, hazir_al) = oneshot::channel();
        let is_parcacigi = thread::Builder::new()
            .name("kalfa-mcp-sdk".to_owned())
            .spawn(move || {
                tracing::subscriber::with_default(
                    tracing::subscriber::NoSubscriber::default(),
                    || {
                        let calisma_zamani = match tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(calisma_zamani) => calisma_zamani,
                            Err(_) => {
                                let _ = hazir_gonder.send(Err(McpHatasi::Baglanti));
                                return;
                            }
                        };
                        calisma_zamani.block_on(istemci_is_parcacigi(
                            tasima,
                            hata,
                            komut_al,
                            kapat_al,
                            hazir_gonder,
                            is_parcacigi_yenileme,
                        ));
                        drop(calisma_zamani);
                    },
                );
            })
            .map_err(|_| McpHatasi::Baglanti)?;
        // Hazırlık beklenirken gelecek bırakılırsa Drop her iki kapatma yolunu işletir.
        let mut istemci = Self {
            komutlar: komut_gonder,
            kapat: kapat_gonder,
            is_parcacigi: Some(is_parcacigi),
            birlestirme: None,
            tasima_kontrol,
            yenileme,
        };
        // Başlatma süresini sunucu tanımı belirler; çağıran bu geleceği
        // o süreyle sınırlar. İkinci bir sabit tavan geçerli tanımı daraltmaz.
        let hazir = match hazir_al.await {
            Ok(hazir) => hazir,
            Err(_) => Err(McpHatasi::Baglanti),
        };
        match hazir {
            Ok(()) => Ok(istemci),
            Err(hata) => {
                istemci.kapat().await;
                Err(hata)
            }
        }
    }

    pub fn araclari_listele(
        &self,
    ) -> impl Future<Output = Result<Vec<Tool>, McpHatasi>> + Send + 'static {
        let komutlar = self.komutlar.clone();
        async move {
            let (yanit_gonder, yanit_al) = oneshot::channel();
            komutlar
                .send(Komut::AraclariListele(yanit_gonder))
                .await
                .map_err(|_| McpHatasi::Kapali)?;
            yanit_al.await.map_err(|_| McpHatasi::Kapali)?
        }
    }

    pub fn arac_cagir(
        &self,
        ad: impl Into<String>,
        argumanlar: rmcp::model::JsonObject,
    ) -> impl Future<Output = Result<CallToolResult, McpHatasi>> + Send + 'static {
        let komutlar = self.komutlar.clone();
        let ad = ad.into();
        async move {
            let (yanit_gonder, yanit_al) = oneshot::channel();
            komutlar
                .send(Komut::AracCagir {
                    ad,
                    argumanlar,
                    yanit: yanit_gonder,
                })
                .await
                .map_err(|_| McpHatasi::Kapali)?;
            yanit_al.await.map_err(|_| McpHatasi::Kapali)?
        }
    }

    pub fn yenileme_durumu(&self) -> Arc<YenilemeDurumu> {
        self.yenileme.clone()
    }

    pub async fn kapat(&mut self) {
        self.kapat.send_replace(true);
        // Nazik iptal dolu kuyrukta beklese de zorunlu kapanış ilerleyebilmeli.
        self.tasima_kontrol.kapat();
        if let Some(is_parcacigi) = self.is_parcacigi.take() {
            self.birlestirme = Some(tokio::task::spawn_blocking(move || is_parcacigi.join()));
        }
        // Tutamak await boyunca nesnede kalır; yarım kalan kapanış yeniden beklenebilir.
        if let Some(birlestirme) = self.birlestirme.as_mut() {
            let _ = birlestirme.await;
        }
        self.birlestirme = None;
    }
}

impl Drop for McpIstemcisi {
    fn drop(&mut self) {
        self.kapat.send_replace(true);
        self.tasima_kontrol.kapat();
    }
}

async fn istemci_is_parcacigi(
    tasima: SinirliTasima,
    mut hata: watch::Receiver<Option<TasimaHatasi>>,
    mut komutlar: mpsc::Receiver<Komut>,
    mut kapat: watch::Receiver<bool>,
    hazir: oneshot::Sender<Result<(), McpHatasi>>,
    yenileme: Arc<YenilemeDurumu>,
) {
    let mut hizmet = tokio::select! {
        biased;
        _ = kapat_bekle(&mut kapat) => {
            let _ = hazir.send(Err(McpHatasi::Iptal));
            return;
        }
        degisti = hata.changed() => {
            let hata = hata.borrow().as_ref().copied().map(McpHatasi::Tasima)
                .unwrap_or(McpHatasi::Baglanti);
            let _ = (degisti, hazir.send(Err(hata)));
            return;
        }
        sonuc = ().serve_with_lifecycle(
            tasima,
            ClientLifecycleMode::Auto {
                preferred_versions: vec![ProtocolVersion::V_2026_07_28],
                legacy_version: Some(ProtocolVersion::V_2025_11_25),
            },
        ) => match sonuc {
            Ok(hizmet) => hizmet,
            Err(uzak) => {
                let hata = hata.borrow().as_ref().copied()
                    .map(McpHatasi::Tasima)
                    .unwrap_or_else(|| baslangic_hatasi(uzak));
                let _ = hazir.send(Err(hata));
                return;
            }
        }
    };

    hizmet
        .peer()
        .set_response_cache_config(rmcp::ClientCacheConfig::disabled())
        .await;

    let protokol = hizmet
        .peer()
        .peer_info()
        .map(|bilgi| bilgi.protocol_version.clone());
    let mut abonelik = None;
    if protokol.is_some_and(|surum| surum >= ProtocolVersion::V_2026_07_28)
        && let Some(kanal_kapasitesi) = NonZeroUsize::new(KUYRUK)
    {
        let abonelik_istegi = tokio::time::timeout(
            Duration::from_secs(5),
            kesilebilir_istek(
                hizmet.peer().listen_with_capacity(
                    SubscriptionFilter::builder().tools_list_changed().build(),
                    kanal_kapasitesi,
                ),
                &mut kapat,
                &mut hata,
            ),
        );
        match abonelik_istegi.await {
            Ok(Ok(mut akim)) => {
                if akim.acknowledged().tools_list_changed == Some(true) {
                    abonelik = Some(akim);
                } else {
                    let _ = akim.cancel().await;
                    yenileme.abonelik_sonlandi();
                    let anlik_hata = { hata.borrow().as_ref().copied() };
                    if let Some(hata) = anlik_hata {
                        let _ = hazir.send(Err(McpHatasi::Tasima(hata)));
                        let _ = hizmet.close().await;
                        return;
                    }
                }
            }
            Ok(Err(_)) | Err(_) => {
                let baslatma_hatasi = hata
                    .borrow()
                    .as_ref()
                    .copied()
                    .map(McpHatasi::Tasima)
                    .or_else(|| (*kapat.borrow()).then_some(McpHatasi::Iptal));
                if let Some(baslatma_hatasi) = baslatma_hatasi {
                    let _ = hazir.send(Err(baslatma_hatasi));
                    let _ = hizmet.close().await;
                    return;
                }
            }
        }
    }
    if abonelik.is_none() {
        yenileme.abonelik_sonlandi();
    }
    let _ = hazir.send(Ok(()));

    let mut abonelik_gorevi = abonelik.map(|akim: Subscription| {
        let yenileme = yenileme.clone();
        tokio::spawn(async move {
            let mut akim = akim;
            loop {
                match akim.next().await {
                    Ok(Some(_)) => yenileme.degisim_bildirimi(),
                    Ok(None) | Err(_) => {
                        yenileme.abonelik_sonlandi();
                        break;
                    }
                }
            }
        })
    });

    loop {
        if let Some(uzak_hata) = hata.borrow().as_ref().copied() {
            let _ = uzak_hata;
            break;
        }
        tokio::select! {
            biased;
            _ = kapat_bekle(&mut kapat) => break,
            degisti = hata.changed() => {
                if degisti.is_err() || hata.borrow().is_some() {
                    break;
                }
            }
            komut = komutlar.recv() => match komut {
                Some(Komut::AraclariListele(mut yanit)) => {
                    let yenileme_surumu = yenileme.listeleme_baslangici();
                    tokio::select! {
                        biased;
                        _ = yanit.closed() => {}
                        sonuc = araclari_listele(&hizmet, &mut kapat, &mut hata) => {
                            if sonuc.is_ok() {
                                yenileme.listeleme_basarili(yenileme_surumu, std::time::Instant::now());
                            }
                            let _ = yanit.send(sonuc);
                        }
                    }
                }
                Some(Komut::AracCagir { ad, argumanlar, mut yanit }) => {
                    let sonuc = arac_cagir(
                        &hizmet,
                        ad,
                        argumanlar,
                        &mut kapat,
                        &mut hata,
                        &mut yanit,
                    )
                    .await;
                    let _ = yanit.send(sonuc);
                }
                None => break,
            }
        }
    }
    if let Some(gorev) = abonelik_gorevi.take() {
        gorev.abort();
        let _ = gorev.await;
    }
    let _ = hizmet.close().await;
}

async fn araclari_listele(
    hizmet: &rmcp::service::RunningService<RoleClient, ()>,
    kapat: &mut watch::Receiver<bool>,
    hata: &mut watch::Receiver<Option<TasimaHatasi>>,
) -> Result<Vec<Tool>, McpHatasi> {
    let mut cursor = None;
    let mut onceki_cursor = HashSet::new();
    let mut araclar = Vec::new();
    let mut toplam_bayt = 0usize;

    for sayfa in 0..AZAMI_SAYFA {
        let params = Some(PaginatedRequestParams::default().with_cursor(cursor.clone()));
        let sonuc = kesilebilir_istek(hizmet.peer().list_tools(params), kapat, hata).await?;
        araclari_ekle(&mut araclar, sonuc.tools, &mut toplam_bayt)?;
        let Some(yeni_cursor) = sonuc.next_cursor else {
            return Ok(araclar);
        };
        if sayfa + 1 >= AZAMI_SAYFA || !onceki_cursor.insert(yeni_cursor.clone()) {
            return Err(McpHatasi::AracListesiUzun);
        }
        cursor = Some(yeni_cursor);
    }
    Err(McpHatasi::AracListesiUzun)
}

async fn arac_cagir(
    hizmet: &rmcp::service::RunningService<RoleClient, ()>,
    ad: String,
    argumanlar: rmcp::model::JsonObject,
    kapat: &mut watch::Receiver<bool>,
    hata: &mut watch::Receiver<Option<TasimaHatasi>>,
    yanit: &mut oneshot::Sender<Result<CallToolResult, McpHatasi>>,
) -> Result<CallToolResult, McpHatasi> {
    if yanit.is_closed() {
        return Err(McpHatasi::Iptal);
    }
    let params = CallToolRequestParams::new(ad).with_arguments(argumanlar);
    let request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
    let gonderim = hizmet
        .peer()
        .send_request_with_option(request, PeerRequestOptions::no_options());
    let mut istek = tokio::select! {
        biased;
        _ = kapat_bekle(kapat) => return Err(McpHatasi::Iptal),
        degisti = hata.changed() => {
            return Err(hata.borrow().as_ref().copied().map(McpHatasi::Tasima)
                .unwrap_or_else(|| { let _ = degisti; McpHatasi::Baglanti }));
        }
        _ = yanit.closed() => return Err(McpHatasi::Iptal),
        sonuc = gonderim => match sonuc {
            Ok(istek) => istek,
            Err(service_hatasi) => return Err(
                hata.borrow().as_ref().copied().map(McpHatasi::Tasima)
                    .unwrap_or_else(|| uzak_hata(service_hatasi))
            ),
        }
    };
    if yanit.is_closed() {
        let _ = istek
            .cancel(Some("İstek sahibi yanıtı beklemeyi bıraktı".to_owned()))
            .await;
        return Err(McpHatasi::Iptal);
    }
    tokio::select! {
        biased;
        _ = kapat_bekle(kapat) => {
            let _ = istek.cancel(Some("Kalfa MCP isteğini iptal etti".to_owned())).await;
            Err(McpHatasi::Iptal)
        }
        degisti = hata.changed() => {
            let sonuc = hata.borrow().as_ref().copied().map(McpHatasi::Tasima)
                .unwrap_or(McpHatasi::Baglanti);
            let _ = (degisti, istek.cancel(Some("MCP taşıması kapandı".to_owned())).await);
            Err(sonuc)
        }
        _ = yanit.closed() => {
            let _ = istek.cancel(Some("İstek sahibi yanıtı beklemeyi bıraktı".to_owned())).await;
            Err(McpHatasi::Iptal)
        }
        sonuc = &mut istek.rx => match sonuc {
            Ok(Ok(ServerResult::CallToolResult(sonuc))) => Ok(sonuc),
            Ok(Ok(_)) => Err(McpHatasi::DesteklenmeyenCevap),
            Ok(Err(hata)) => Err(uzak_hata(hata)),
            Err(_) => Err(McpHatasi::Baglanti),
        }
    }
}

fn arac_tanim_bayti(arac: &Tool) -> Result<usize, McpHatasi> {
    let mut bayt = arac.name.len();
    bayt = bayt
        .checked_add(arac.description.as_deref().map(str::len).unwrap_or(0))
        .ok_or(McpHatasi::TanimBoyutu)?;
    bayt = bayt
        .checked_add(
            serde_json::to_vec(arac.input_schema.as_ref())
                .map_err(|_| McpHatasi::TanimBoyutu)?
                .len(),
        )
        .ok_or(McpHatasi::TanimBoyutu)?;
    if let Some(schema) = &arac.output_schema {
        bayt = bayt
            .checked_add(
                serde_json::to_vec(schema.as_ref())
                    .map_err(|_| McpHatasi::TanimBoyutu)?
                    .len(),
            )
            .ok_or(McpHatasi::TanimBoyutu)?;
    }
    Ok(bayt)
}

fn araclari_ekle(
    mevcut: &mut Vec<Tool>,
    yeniler: Vec<Tool>,
    toplam_bayt: &mut usize,
) -> Result<(), McpHatasi> {
    let toplam_arac = mevcut
        .len()
        .checked_add(yeniler.len())
        .ok_or(McpHatasi::AracSayisi)?;
    if toplam_arac > AZAMI_ARAC {
        return Err(McpHatasi::AracSayisi);
    }
    for arac in yeniler {
        let sonraki_bayt = toplam_bayt
            .checked_add(arac_tanim_bayti(&arac)?)
            .ok_or(McpHatasi::TanimBoyutu)?;
        if sonraki_bayt > AZAMI_TANIM {
            return Err(McpHatasi::TanimBoyutu);
        }
        *toplam_bayt = sonraki_bayt;
        mevcut.push(arac);
    }
    Ok(())
}

async fn kesilebilir_istek<T>(
    gelecek: impl Future<Output = Result<T, ServiceError>>,
    kapat: &mut watch::Receiver<bool>,
    hata: &mut watch::Receiver<Option<TasimaHatasi>>,
) -> Result<T, McpHatasi> {
    if let Some(uzak_hata) = hata.borrow().as_ref().copied() {
        return Err(McpHatasi::Tasima(uzak_hata));
    }
    let sonuc = tokio::select! {
        biased;
        _ = kapat_bekle(kapat) => Err(McpHatasi::Iptal),
        degisti = hata.changed() => {
            let hata = hata.borrow().as_ref().copied().map(McpHatasi::Tasima)
                .unwrap_or(McpHatasi::Baglanti);
            let _ = degisti;
            Err(hata)
        }
        sonuc = gelecek => sonuc.map_err(uzak_hata)
    };
    match sonuc {
        Err(_) if hata.borrow().is_some() => Err(McpHatasi::Tasima(
            hata.borrow()
                .as_ref()
                .copied()
                .unwrap_or(TasimaHatasi::GirdiCikti),
        )),
        diger => diger,
    }
}

async fn kapat_bekle(kapat: &mut watch::Receiver<bool>) {
    if *kapat.borrow() {
        return;
    }
    let _ = kapat.changed().await;
}

fn uzak_hata(hata: ServiceError) -> McpHatasi {
    match hata {
        ServiceError::McpError(hata) => McpHatasi::UzakRpc(RpcHatasi {
            kod: hata.code.0,
            metin: hata.message.into_owned(),
            veri: hata.data,
        }),
        ServiceError::TransportClosed | ServiceError::TransportSend(_) => McpHatasi::Baglanti,
        _ => McpHatasi::Baglanti,
    }
}

fn baslangic_hatasi(hata: ClientInitializeError) -> McpHatasi {
    match hata {
        ClientInitializeError::JsonRpcError(hata) => McpHatasi::UzakRpc(RpcHatasi {
            kod: hata.code.0,
            metin: hata.message.into_owned(),
            veri: hata.data,
        }),
        ClientInitializeError::LegacyFallbackFailed { fallback, .. } => baslangic_hatasi(*fallback),
        _ => McpHatasi::Baglanti,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arac(ad: String) -> Tool {
        let nesne =
            serde_json::from_value::<rmcp::model::JsonObject>(serde_json::json!({})).unwrap();
        Tool::new(ad, "", nesne)
    }

    #[test]
    fn arac_sayisi_yuz_yirmi_sekiz_sinirinda_kalir() {
        let tools = (0..AZAMI_ARAC).map(|i| arac(i.to_string())).collect();
        let mut sonuc = Vec::new();
        let mut toplam = 0;
        assert!(araclari_ekle(&mut sonuc, tools, &mut toplam).is_ok());
        assert_eq!(sonuc.len(), AZAMI_ARAC);
        assert!(matches!(
            araclari_ekle(&mut sonuc, vec![arac("fazla".to_owned())], &mut toplam),
            Err(McpHatasi::AracSayisi)
        ));
    }

    #[test]
    fn toplam_tanim_bayti_256_kib_esitliginde_kabul_edilir() {
        let mut tool = arac("a".to_owned());
        let taban = arac_tanim_bayti(&tool).unwrap();
        let aciklama = "x".repeat(AZAMI_TANIM - taban);
        tool.description = Some(aciklama.clone().into());
        assert_eq!(arac_tanim_bayti(&tool).unwrap(), AZAMI_TANIM);
        let mut araclar = Vec::new();
        let mut toplam = 0;
        assert!(araclari_ekle(&mut araclar, vec![tool.clone()], &mut toplam).is_ok());
        tool.description = Some(format!("{aciklama}x").into());
        assert!(matches!(
            araclari_ekle(&mut Vec::new(), vec![tool], &mut 0),
            Err(McpHatasi::TanimBoyutu)
        ));
    }

    #[test]
    fn uzak_rpc_ham_metni_yalniz_denetcinin_yonteminden_gorunur() {
        let hata = McpHatasi::UzakRpc(RpcHatasi {
            kod: -32000,
            metin: "gizli-yansi".to_owned(),
            veri: Some(serde_json::json!({"sır": "gizli-yansi"})),
        });
        let gorunen = format!("{hata:?} {hata}");
        assert!(!gorunen.contains("gizli-yansi"));
        assert_eq!(
            hata.uzak_rpc().map(|(_, metin, _)| metin),
            Some("gizli-yansi")
        );
    }

    #[tokio::test]
    async fn yarim_kalan_kapatma_tekrarlandiginda_is_parcacigini_beklemeyi_surdurur() {
        let (birak, bekle) = std::sync::mpsc::channel::<()>();
        let is_parcacigi = thread::spawn(move || {
            let _ = bekle.recv();
        });
        let (sunucu, boru) = tokio::io::duplex(64);
        let (okuma, yazma) = tokio::io::split(boru);
        let (tasima, _) = SinirliTasima::yeni(okuma, yazma);
        let (komutlar, _komut_al) = mpsc::channel(KUYRUK);
        let (kapat, _kapat_al) = watch::channel(false);
        let mut istemci = McpIstemcisi {
            komutlar,
            kapat,
            birlestirme: None,
            tasima_kontrol: tasima.kontrol(),
            is_parcacigi: Some(is_parcacigi),
            yenileme: tasima.yenileme.clone(),
        };
        let mut bekledi = Vec::new();
        for _ in 0..3 {
            bekledi.push(
                tokio::time::timeout(Duration::from_millis(10), istemci.kapat())
                    .await
                    .is_err(),
            );
        }
        birak.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(3), istemci.kapat())
            .await
            .unwrap();
        drop((tasima, sunucu));
        assert!(
            bekledi.into_iter().all(|bekledi| bekledi),
            "tekrar kapatma, yaşayan iş parçacığı bitmeden dönmemeli"
        );
    }
}
