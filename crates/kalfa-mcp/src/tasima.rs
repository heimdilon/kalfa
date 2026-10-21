//! MCP stdio için sınırı ve hata sahipliğini taşıyan taşıma katmanı.
use rmcp::RoleClient;
use rmcp::model::{
    ClientJsonRpcMessage, ClientNotification, ClientResult, ErrorCode, ErrorData, GetMeta,
    RequestId, ServerJsonRpcMessage, ServerNotification, ServerRequest,
};
use rmcp::transport::Transport;
use serde_json::Value;
use std::collections::{HashSet, VecDeque};
use std::io;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;

pub const CERCEVE: usize = 4 * 1024 * 1024;
pub const KUYRUK: usize = 64;
pub const EN_AZ_YENILEME: Duration = Duration::from_secs(5);
pub const YOKLAMA_ARALIGI: Duration = Duration::from_secs(30);
const GECMIS_ABONELIK_SINIRI: usize = 64;
pub(crate) const IC_ISARET_CONTENT_ALANI_VAR: &str = "__kalfa_internal_v1_content_alani_var";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TasimaHatasi {
    Cerceve,
    YarimSatir,
    BozukJson,
    IletiSeli,
    GidenIletiSeli,
    YinelenenIstekKimligi,
    GirdiCikti,
    UzakKapandi,
    BeklenmeyenAbonelikAck,
    YinelenenAbonelikAck,
    AgPolitikasiReddi,
    YetkiGerekli,
    BelirsizCagriSonucu,
}

impl std::fmt::Display for TasimaHatasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let metin = match self {
            Self::Cerceve => "MCP ileti çerçevesi sınırı aşıldı",
            Self::YarimSatir => "MCP iletisi tamamlanmadan bağlantı kapandı",
            Self::BozukJson => "MCP iletisi geçerli JSON değil",
            Self::IletiSeli => "MCP sunucusundan ileti seli alındı",
            Self::GidenIletiSeli => "MCP giden ileti kuyruğu sınırı aşıldı",
            Self::YinelenenIstekKimligi => "MCP sunucusunda yinelenen istek kimliği oluştu",
            Self::GirdiCikti => "MCP stdio taşımasında girdi/çıktı hatası",
            Self::UzakKapandi => "MCP sunucusu stdout akışını kapattı",
            Self::BeklenmeyenAbonelikAck => "MCP abonelik onayı beklenen kimlikle eşleşmiyor",
            Self::YinelenenAbonelikAck => "MCP abonelik onayı yinelendi",
            Self::AgPolitikasiReddi => "Uzak MCP hedefi ağ politikası tarafından reddedildi",
            Self::YetkiGerekli => "Uzak MCP sunucusu yetkilendirme gerektiriyor",
            Self::BelirsizCagriSonucu => {
                "Uzak MCP araç çağrısı yanıt alınmadan kesildi; sonuç bilinmiyor"
            }
        };
        f.write_str(metin)
    }
}

impl std::error::Error for TasimaHatasi {}

pub(crate) struct AbonelikKimlikleri {
    giden: HashSet<RequestId>,
    arac_cagrilari: HashSet<RequestId>,
    beklenen_onaylar: HashSet<RequestId>,
    onaylananlar: VecDeque<RequestId>,
    aktif_abonelikler: HashSet<RequestId>,
}

impl AbonelikKimlikleri {
    pub(crate) fn yeni() -> Self {
        Self {
            giden: HashSet::new(),
            arac_cagrilari: HashSet::new(),
            beklenen_onaylar: HashSet::new(),
            onaylananlar: VecDeque::new(),
            aktif_abonelikler: HashSet::new(),
        }
    }

    pub(crate) fn giden_istegi_ekle(
        &mut self,
        item: &ClientJsonRpcMessage,
    ) -> Result<(), TasimaHatasi> {
        if let ClientJsonRpcMessage::Request(istek) = item {
            if self.giden.contains(&istek.id) {
                return Err(TasimaHatasi::YinelenenIstekKimligi);
            }
            if self.giden.len() >= KUYRUK {
                return Err(TasimaHatasi::GidenIletiSeli);
            }
            self.giden.insert(istek.id.clone());
            if istek.request.method() == "subscriptions/listen" {
                self.beklenen_onaylar.insert(istek.id.clone());
            }
            if istek.request.method() == "tools/call" {
                self.arac_cagrilari.insert(istek.id.clone());
            }
        } else if let ClientJsonRpcMessage::Notification(bildirim) = item
            && let ClientNotification::CancelledNotification(iptal) = &bildirim.notification
        {
            self.giden_iptali(iptal.params.request_id.as_ref());
        }
        Ok(())
    }

    pub(crate) fn yanit_alindi(&mut self, id: &RequestId) -> bool {
        let bulundu = self.giden.remove(id);
        self.arac_cagrilari.remove(id);
        self.beklenen_onaylar.remove(id);
        self.aktif_abonelikler.remove(id);
        bulundu
    }

    pub(crate) fn giden_iptali(&mut self, id: Option<&RequestId>) {
        if let Some(id) = id {
            self.giden.remove(id);
            self.arac_cagrilari.remove(id);
            self.beklenen_onaylar.remove(id);
            self.aktif_abonelikler.remove(id);
        }
    }

    pub(crate) fn abonelik_iptalini_dogrula(&mut self, id: Option<&RequestId>) -> bool {
        id.is_some_and(|id| self.aktif_abonelikler.remove(id))
    }

    pub(crate) fn abonelik_onayini_dogrula(
        &mut self,
        id: Option<RequestId>,
    ) -> Result<(), TasimaHatasi> {
        let Some(id) = id else {
            return Err(TasimaHatasi::BeklenmeyenAbonelikAck);
        };
        if self.giden.contains(&id) && self.beklenen_onaylar.remove(&id) {
            self.aktif_abonelikler.insert(id.clone());
            self.onaylananlar.push_back(id);
            while self.onaylananlar.len() > GECMIS_ABONELIK_SINIRI {
                self.onaylananlar.pop_front();
            }
            return Ok(());
        }
        if self.onaylananlar.contains(&id) {
            Err(TasimaHatasi::YinelenenAbonelikAck)
        } else {
            Err(TasimaHatasi::BeklenmeyenAbonelikAck)
        }
    }

    pub(crate) fn arac_cagrisi_mi(&self, id: &RequestId) -> bool {
        self.arac_cagrilari.contains(id)
    }
}

pub(crate) fn arac_cagrisi_yanitina_isaret_ekle(
    ileti: &mut Value,
    kimlikler: &AbonelikKimlikleri,
) -> bool {
    let Some(rpc) = ileti.as_object_mut() else {
        return false;
    };
    if rpc.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || rpc.contains_key("method")
        || rpc.contains_key("error")
    {
        return false;
    }
    let Some(id) = rpc
        .get("id")
        .and_then(|id| serde_json::from_value::<RequestId>(id.clone()).ok())
    else {
        return false;
    };
    if !kimlikler.arac_cagrisi_mi(&id) {
        return false;
    }
    let Some(sonuc) = rpc.get_mut("result").and_then(Value::as_object_mut) else {
        return false;
    };
    if sonuc
        .get("resultType")
        .is_some_and(|tur| tur.as_str() != Some("complete"))
    {
        return false;
    }

    let content_alani_var = sonuc.contains_key("content");
    let meta_gecerli = sonuc.get("_meta").is_some_and(Value::is_object);
    let call_tool_result = content_alani_var
        || sonuc.contains_key("structuredContent")
        || sonuc.contains_key("isError")
        || meta_gecerli;
    if !call_tool_result {
        return false;
    }

    let meta = sonuc
        .entry("_meta".to_owned())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if meta.is_null() {
        *meta = Value::Object(serde_json::Map::new());
    }
    let Some(meta) = meta.as_object_mut() else {
        return false;
    };
    // Bu değer taşıma tarafından ham yanıttan hesaplanır. Sunucunun aynı
    // anahtardaki _meta değeri güven kaynağı değildir ve daima ezilir.
    meta.insert(
        IC_ISARET_CONTENT_ALANI_VAR.to_owned(),
        Value::Bool(content_alani_var),
    );
    true
}

/// Yenileme bayrağı ile listenin son başarılı okuma zamanını birlikte tutar.
pub struct YenilemeDurumu {
    degisim: AtomicU64,
    gorulen_degisim: AtomicU64,
    abonelik_sonlandi: AtomicBool,
    son_basarili_liste: Mutex<Instant>,
}

impl YenilemeDurumu {
    pub(crate) fn yeni() -> Self {
        Self {
            degisim: AtomicU64::new(0),
            gorulen_degisim: AtomicU64::new(0),
            abonelik_sonlandi: AtomicBool::new(false),
            son_basarili_liste: Mutex::new(Instant::now()),
        }
    }

    pub fn degisim_bildirimi(&self) {
        self.degisim.fetch_add(1, Ordering::AcqRel);
    }

    pub fn abonelik_sonlandi(&self) {
        self.abonelik_sonlandi.store(true, Ordering::Release);
    }

    pub fn yenileme_gerekli(&self, simdi: Instant) -> bool {
        let degisti =
            self.degisim.load(Ordering::Acquire) != self.gorulen_degisim.load(Ordering::Acquire);
        let yoklaniyor = self.abonelik_sonlandi.load(Ordering::Acquire);
        if !degisti && !yoklaniyor {
            return false;
        }
        let en_az = if degisti {
            EN_AZ_YENILEME
        } else {
            YOKLAMA_ARALIGI
        };
        self.son_basarili_liste
            .lock()
            .map(|son| simdi.saturating_duration_since(*son) >= en_az)
            .unwrap_or(false)
    }

    /// Listeleme başlamadan alınır; listeleme sırasında yeni değişiklik gelirse korunur.
    pub fn listeleme_baslangici(&self) -> u64 {
        self.degisim.load(Ordering::Acquire)
    }

    pub fn listeleme_basarili(&self, degisim_surumu: u64, simdi: Instant) {
        self.gorulen_degisim
            .store(degisim_surumu, Ordering::Release);
        if let Ok(mut son) = self.son_basarili_liste.lock() {
            *son = simdi;
        }
    }
}

struct TasimaKapatmaDurumu {
    kapat: watch::Sender<bool>,
}

// Görevin beklenmeyen bitişi de sahibine bildirilir; ham hata saklanmaz.
struct GorevKorumasi {
    hata: watch::Sender<Option<TasimaHatasi>>,
    kapat: watch::Receiver<bool>,
}

impl Drop for GorevKorumasi {
    fn drop(&mut self) {
        if !*self.kapat.borrow() {
            ilk_hatayi_yaz(&self.hata, TasimaHatasi::GirdiCikti);
        }
    }
}

/// Taşıma görevlerine kapatma isteği iletir; bitişi `Transport::close` bekler.
#[derive(Clone)]
pub struct TasimaKontrol(Arc<TasimaKapatmaDurumu>);

impl TasimaKontrol {
    pub(crate) fn yeni(kapat: watch::Sender<bool>) -> Self {
        Self(Arc::new(TasimaKapatmaDurumu { kapat }))
    }

    pub fn kapat(&self) {
        self.0.kapat.send_replace(true);
    }
}

// İptal bildiriminin kuyruğa konması yeterli değildir: stdin kapanmadan
// önce yazma/flush makbuzu beklenir. Diğer iletiler aynı sınırlı kuyruktadır.
struct GidenIleti {
    ileti: ClientJsonRpcMessage,
    yazildi: Option<oneshot::Sender<()>>,
}

impl From<ClientJsonRpcMessage> for GidenIleti {
    fn from(ileti: ClientJsonRpcMessage) -> Self {
        Self {
            ileti,
            yazildi: None,
        }
    }
}
/// Ham stdout doğrudan SDK'ye verilmez; çerçeveleme ve ayrıştırma burada yapılır.
pub struct SinirliTasima {
    pub yenileme: Arc<YenilemeDurumu>,
    gelen: mpsc::Receiver<ServerJsonRpcMessage>,
    giden: mpsc::Sender<GidenIleti>,
    hata: watch::Receiver<Option<TasimaHatasi>>,
    hata_gonder: watch::Sender<Option<TasimaHatasi>>,
    kimlikler: Arc<Mutex<AbonelikKimlikleri>>,
    kontrol: TasimaKontrol,
    gorevler: Vec<JoinHandle<()>>,
}

impl SinirliTasima {
    pub fn yeni<R, W>(r: R, mut w: W) -> (Self, watch::Receiver<Option<TasimaHatasi>>)
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (gelen_gonder, gelen) = mpsc::channel(KUYRUK);
        let (giden, mut giden_al) = mpsc::channel::<GidenIleti>(KUYRUK);
        let (hata_gonder, hata) = watch::channel(None);
        let (kapat_gonder, _kapat_al) = watch::channel(false);
        let kimlikler = Arc::new(Mutex::new(AbonelikKimlikleri::yeni()));
        let yenileme = Arc::new(YenilemeDurumu::yeni());

        let okuma_hata = hata_gonder.clone();
        let okuma_giden = giden.clone();
        let okuma_kapat = kapat_gonder.subscribe();
        let okuma_kimlik = kimlikler.clone();
        let okuma_yenileme = yenileme.clone();
        let okuma_korumasi = GorevKorumasi {
            hata: hata_gonder.clone(),
            kapat: kapat_gonder.subscribe(),
        };
        let okuma = tokio::spawn(async move {
            let _koruma = okuma_korumasi;
            let mut r = BufReader::new(r);
            loop {
                let ham = tokio::select! {
                    biased;
                    _ = kapat_bekle(okuma_kapat.clone()) => break,
                    satir = satir(&mut r, CERCEVE) => match satir {
                        Ok(Some(ham)) => ham,
                        Ok(None) => {
                            ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::UzakKapandi);
                            break;
                        }
                        Err(hata) => {
                            ilk_hatayi_yaz(&okuma_hata, hata);
                            break;
                        }
                    }
                };
                let mut ham_json: Value = match serde_json::from_slice(&ham) {
                    Ok(ileti) => ileti,
                    Err(_) => {
                        ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::BozukJson);
                        break;
                    }
                };
                let isaretleme = okuma_kimlik
                    .lock()
                    .map(|kimlikler| arac_cagrisi_yanitina_isaret_ekle(&mut ham_json, &kimlikler));
                match isaretleme {
                    Ok(_) => {}
                    Err(_) => {
                        ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::GirdiCikti);
                        break;
                    }
                }
                let ileti: ServerJsonRpcMessage = match serde_json::from_value(ham_json) {
                    Ok(ileti) => ileti,
                    Err(_) => {
                        ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::BozukJson);
                        break;
                    }
                };
                match ileti {
                    ServerJsonRpcMessage::Notification(mut bildirim) => {
                        match &mut bildirim.notification {
                            ServerNotification::ToolListChangedNotification(_) => {
                                okuma_yenileme.degisim_bildirimi();
                            }
                            ServerNotification::SubscriptionsAcknowledgedNotification(_) => {
                                let kimlik = bildirim.notification.get_meta().subscription_id();
                                let sonuc = okuma_kimlik
                                    .lock()
                                    .map_err(|_| TasimaHatasi::GirdiCikti)
                                    .and_then(|mut durum| durum.abonelik_onayini_dogrula(kimlik));
                                if let Err(hata) = sonuc {
                                    ilk_hatayi_yaz(&okuma_hata, hata);
                                    break;
                                }
                                if gelen_gonder
                                    .try_send(ServerJsonRpcMessage::Notification(bildirim))
                                    .is_err()
                                {
                                    ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::IletiSeli);
                                    break;
                                }
                            }
                            ServerNotification::CancelledNotification(iptal) => {
                                // Sunucu açıklaması sır veya başka dış metin taşıyabilir.
                                iptal.params.reason = None;
                                let aktar = okuma_kimlik
                                    .lock()
                                    .map(|mut durum| {
                                        durum.abonelik_iptalini_dogrula(
                                            iptal.params.request_id.as_ref(),
                                        )
                                    })
                                    .unwrap_or(false);
                                if !aktar {
                                    continue;
                                }
                                okuma_yenileme.abonelik_sonlandi();
                                if gelen_gonder
                                    .try_send(ServerJsonRpcMessage::Notification(bildirim))
                                    .is_err()
                                {
                                    ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::IletiSeli);
                                    break;
                                }
                            }
                            _ => {
                                // Diğer bildirimler SDK'ye aktarılmaz ve görev kuyruğu oluşturmaz.
                            }
                        }
                    }
                    ServerJsonRpcMessage::Request(istek) => {
                        let yanit = if matches!(istek.request, ServerRequest::PingRequest(_)) {
                            ClientJsonRpcMessage::response(ClientResult::empty(()), istek.id)
                        } else {
                            ClientJsonRpcMessage::error(
                                ErrorData::new(
                                    ErrorCode::METHOD_NOT_FOUND,
                                    "Yöntem desteklenmiyor",
                                    None,
                                ),
                                Some(istek.id),
                            )
                        };
                        if okuma_giden.try_send(yanit.into()).is_err() {
                            ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::IletiSeli);
                            break;
                        }
                    }
                    ServerJsonRpcMessage::Response(yanit) => {
                        let eslesti = okuma_kimlik
                            .lock()
                            .map(|mut durum| durum.yanit_alindi(&yanit.id))
                            .unwrap_or(false);
                        if eslesti
                            && gelen_gonder
                                .try_send(ServerJsonRpcMessage::Response(yanit))
                                .is_err()
                        {
                            ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::IletiSeli);
                            break;
                        }
                    }
                    ServerJsonRpcMessage::Error(hata) => {
                        let kimlik = hata.id.clone();
                        let eslesti = kimlik.as_ref().is_some_and(|id| {
                            okuma_kimlik
                                .lock()
                                .map(|mut durum| durum.yanit_alindi(id))
                                .unwrap_or(false)
                        });
                        if eslesti
                            && gelen_gonder
                                .try_send(ServerJsonRpcMessage::Error(hata))
                                .is_err()
                        {
                            ilk_hatayi_yaz(&okuma_hata, TasimaHatasi::IletiSeli);
                            break;
                        }
                    }
                }
            }
        });

        let yazma_hata = hata_gonder.clone();
        let yazma_hata_bekle = hata_gonder.subscribe();
        let yazma_kapat = kapat_gonder.subscribe();
        let yazma_korumasi = GorevKorumasi {
            hata: hata_gonder.clone(),
            kapat: kapat_gonder.subscribe(),
        };
        let yazma = tokio::spawn(async move {
            let _koruma = yazma_korumasi;
            loop {
                let item = tokio::select! {
                    biased;
                    _ = kapat_bekle(yazma_kapat.clone()) => break,
                    _ = hata_bekle(yazma_hata_bekle.clone()) => break,
                    item = giden_al.recv() => match item {
                        Some(item) => item,
                        None => break,
                    }
                };
                let mut ham = match serde_json::to_vec(&item.ileti) {
                    Ok(ham) if ham.len() <= CERCEVE => ham,
                    Ok(_) => {
                        ilk_hatayi_yaz(&yazma_hata, TasimaHatasi::Cerceve);
                        break;
                    }
                    Err(_) => {
                        ilk_hatayi_yaz(&yazma_hata, TasimaHatasi::BozukJson);
                        break;
                    }
                };
                ham.push(b'\n');
                let yazim = tokio::select! {
                    biased;
                    _ = kapat_bekle(yazma_kapat.clone()) => break,
                    _ = hata_bekle(yazma_hata_bekle.clone()) => break,
                    sonuc = async {
                        w.write_all(&ham).await?;
                        w.flush().await
                    } => sonuc
                };
                if yazim.is_err() {
                    ilk_hatayi_yaz(&yazma_hata, TasimaHatasi::GirdiCikti);
                    break;
                }
                if let Some(yazildi) = item.yazildi {
                    let _ = yazildi.send(());
                }
            }
        });

        let kontrol = TasimaKontrol(Arc::new(TasimaKapatmaDurumu {
            kapat: kapat_gonder,
        }));
        let tasima = Self {
            yenileme,
            gelen,
            giden,
            hata: hata.clone(),
            hata_gonder,
            kimlikler,
            kontrol,
            gorevler: vec![okuma, yazma],
        };
        (tasima, hata)
    }

    pub fn kontrol(&self) -> TasimaKontrol {
        self.kontrol.clone()
    }
}

async fn kapat_bekle(mut al: watch::Receiver<bool>) {
    if *al.borrow() {
        return;
    }
    let _ = al.changed().await;
}

async fn hata_bekle(mut al: watch::Receiver<Option<TasimaHatasi>>) {
    if al.borrow().is_some() {
        return;
    }
    let _ = al.changed().await;
}

fn ilk_hatayi_yaz(gonder: &watch::Sender<Option<TasimaHatasi>>, hata: TasimaHatasi) {
    gonder.send_if_modified(|mevcut| {
        if mevcut.is_none() {
            *mevcut = Some(hata);
            true
        } else {
            false
        }
    });
}

/// Ayrıştırmadan önce en çok sınır kadar veri biriktirir. LF bütçeye girmez; CR girer.
pub async fn satir<R: AsyncBufRead + Unpin>(
    r: &mut R,
    sinir: usize,
) -> Result<Option<Vec<u8>>, TasimaHatasi> {
    let mut veri = Vec::new();
    loop {
        let gelen = r.fill_buf().await.map_err(|_| TasimaHatasi::GirdiCikti)?;
        if gelen.is_empty() {
            return if veri.is_empty() {
                Ok(None)
            } else {
                Err(TasimaHatasi::YarimSatir)
            };
        }
        let son = gelen.iter().position(|b| *b == b'\n');
        let n = son.unwrap_or(gelen.len());
        if n > sinir.saturating_sub(veri.len()) {
            return Err(TasimaHatasi::Cerceve);
        }
        veri.extend_from_slice(&gelen[..n]);
        r.consume(n + usize::from(son.is_some()));
        if son.is_some() {
            return Ok(Some(veri));
        }
    }
}

impl Transport<RoleClient> for SinirliTasima {
    type Error = io::Error;

    fn send(
        &mut self,
        item: ClientJsonRpcMessage,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let giden = self.giden.clone();
        let kimlikler = self.kimlikler.clone();
        let hata = self.hata_gonder.clone();
        let hata_dinle = self.hata.clone();
        let kapat = self.kontrol.0.kapat.subscribe();
        async move {
            if hata_dinle.borrow().is_some() || *kapat.borrow() {
                return Err(io::Error::other(
                    "MCP taşıması ölümcül hata nedeniyle kapalı",
                ));
            }
            let permit = tokio::select! {
                biased;
                _ = hata_bekle(hata_dinle.clone()) => return Err(io::Error::other("MCP taşıması kapalı")),
                _ = kapat_bekle(kapat.clone()) => return Err(io::Error::other("MCP taşıması kapalı")),
                sonuc = giden.reserve_owned() => sonuc.map_err(|_| io::Error::other("MCP taşıması kapalı"))?,
            };
            if hata_dinle.borrow().is_some() || *kapat.borrow() {
                return Err(io::Error::other("MCP taşıması kapalı"));
            }
            let kayit = kimlikler
                .lock()
                .map_err(|_| io::Error::other("MCP taşıma durumu kullanılamıyor"))?
                .giden_istegi_ekle(&item);
            if let Err(kesinti) = kayit {
                ilk_hatayi_yaz(&hata, kesinti);
                return Err(io::Error::other("MCP taşıma sınırı aşıldı"));
            }
            let (yazildi, makbuz) = if matches!(
                &item,
                ClientJsonRpcMessage::Notification(bildirim)
                    if matches!(bildirim.notification, ClientNotification::CancelledNotification(_))
            ) {
                let (gonder, al) = oneshot::channel();
                (Some(gonder), Some(al))
            } else {
                (None, None)
            };
            permit.send(GidenIleti {
                ileti: item,
                yazildi,
            });
            if let Some(makbuz) = makbuz {
                tokio::select! {
                    biased;
                    _ = hata_bekle(hata_dinle) => return Err(io::Error::other("MCP iptali yazılamadı")),
                    _ = kapat_bekle(kapat) => return Err(io::Error::other("MCP iptali yazılamadı")),
                    sonuc = makbuz => sonuc.map_err(|_| io::Error::other("MCP iptali yazılamadı"))?,
                }
            }
            Ok(())
        }
    }

    async fn receive(&mut self) -> Option<ServerJsonRpcMessage> {
        if self.hata.borrow().is_some() {
            return None;
        }
        tokio::select! {
            biased;
            degisti = self.hata.changed() => {
                if degisti.is_err() || self.hata.borrow().is_some() {
                    None
                } else {
                    self.gelen.recv().await
                }
            }
            ileti = self.gelen.recv() => ileti
        }
    }

    async fn close(&mut self) -> io::Result<()> {
        self.kontrol.kapat();
        for gorev in &mut self.gorevler {
            gorev.abort();
            let _ = gorev.await;
        }
        Ok(())
    }
}

impl Drop for SinirliTasima {
    fn drop(&mut self) {
        self.kontrol.0.kapat.send_replace(true);
        for gorev in &self.gorevler {
            gorev.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn liste_degisimi_yalniz_bes_saniyede_yeniler() {
        let durum = YenilemeDurumu::yeni();
        let simdi = Instant::now();
        durum.degisim_bildirimi();
        assert!(!durum.yenileme_gerekli(simdi + Duration::from_millis(4_999)));
        assert!(durum.yenileme_gerekli(simdi + Duration::from_secs(5)));
    }

    #[test]
    fn abonelik_bittiginde_otuz_saniyelik_yoklama_baslar() {
        let durum = YenilemeDurumu::yeni();
        let simdi = Instant::now();
        durum.abonelik_sonlandi();
        assert!(!durum.yenileme_gerekli(simdi + Duration::from_secs(29)));
        assert!(durum.yenileme_gerekli(simdi + Duration::from_secs(30)));
    }

    #[test]
    fn listeleme_sirasinda_yeni_degisim_bayragi_kaybolmaz() {
        let durum = YenilemeDurumu::yeni();
        let simdi = Instant::now();
        durum.degisim_bildirimi();
        let surum = durum.listeleme_baslangici();
        durum.degisim_bildirimi();
        durum.listeleme_basarili(surum, simdi);
        assert!(durum.yenileme_gerekli(simdi + Duration::from_secs(5)));
    }

    #[test]
    fn abonelik_yaniti_gec_ack_kaydini_gecersiz_kilar() {
        let mut kimlikler = AbonelikKimlikleri::yeni();
        let istek: ClientJsonRpcMessage = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":77,"method":"subscriptions/listen","params":{"notifications":{"toolsListChanged":true}}}"#,
        )
        .unwrap();
        let id = match &istek {
            ClientJsonRpcMessage::Request(istek) => istek.id.clone(),
            _ => panic!("test girdisi istek olmalı"),
        };
        kimlikler.giden_istegi_ekle(&istek).unwrap();
        assert!(kimlikler.yanit_alindi(&id));
        assert_eq!(
            kimlikler.abonelik_onayini_dogrula(Some(id)),
            Err(TasimaHatasi::BeklenmeyenAbonelikAck)
        );
    }

    #[test]
    fn yinelenen_istek_kimligi_aktif_kuyrugu_buyutmez() {
        let mut kimlikler = AbonelikKimlikleri::yeni();
        let istek: ClientJsonRpcMessage =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#)
                .unwrap();
        kimlikler.giden_istegi_ekle(&istek).unwrap();
        assert_eq!(
            kimlikler.giden_istegi_ekle(&istek),
            Err(TasimaHatasi::YinelenenIstekKimligi)
        );
    }

    fn arac_cagrisi_kimlikleri() -> (AbonelikKimlikleri, RequestId) {
        let mut kimlikler = AbonelikKimlikleri::yeni();
        let istek: ClientJsonRpcMessage = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":17,"method":"tools/call","params":{"name":"ornek"}}"#,
        )
        .unwrap();
        let id = match &istek {
            ClientJsonRpcMessage::Request(istek) => istek.id.clone(),
            _ => panic!("test girdisi araç çağrısı olmalı"),
        };
        kimlikler.giden_istegi_ekle(&istek).unwrap();
        (kimlikler, id)
    }

    fn tasima_cevabini_tiple(sonuc: Value) -> crate::istemci::McpAracCevabi {
        let (kimlikler, _) = arac_cagrisi_kimlikleri();
        let mut ileti = serde_json::json!({"jsonrpc":"2.0", "id":17, "result":sonuc});
        assert!(arac_cagrisi_yanitina_isaret_ekle(&mut ileti, &kimlikler));
        let ileti: ServerJsonRpcMessage = serde_json::from_value(ileti).unwrap();
        let rmcp::model::ServerJsonRpcMessage::Response(yanit) = ileti else {
            panic!("yanıt olmalı");
        };
        let rmcp::model::ServerResult::CallToolResult(sonuc) = yanit.result else {
            panic!("araç sonucu olmalı");
        };
        crate::istemci::ic_isareti_tuket(sonuc).unwrap()
    }

    #[test]
    fn content_alani_yokken_tasima_false_isaretini_tasiyip_tuketir() {
        let cevap = tasima_cevabini_tiple(serde_json::json!({
            "structuredContent": {"durum":"tamam"}
        }));
        assert!(!cevap.content_alani_var);
        assert!(cevap.sonuc.content.is_empty());
        assert_eq!(
            cevap.sonuc.structured_content,
            Some(serde_json::json!({"durum":"tamam"}))
        );
        assert!(cevap.sonuc.meta.is_none());
    }

    #[test]
    fn acik_bos_content_alani_varligini_korur() {
        let cevap = tasima_cevabini_tiple(serde_json::json!({
            "content": [],
            "structuredContent": {"durum":"kullanilmaz"}
        }));
        assert!(cevap.content_alani_var);
        assert!(cevap.sonuc.content.is_empty());
        assert_eq!(
            cevap.sonuc.structured_content,
            Some(serde_json::json!({"durum":"kullanilmaz"}))
        );
        assert!(cevap.sonuc.meta.is_none());
    }

    #[test]
    fn sahte_meta_isareti_ham_content_varligina_gore_ezilir() {
        let absent = tasima_cevabini_tiple(serde_json::json!({
            "structuredContent": {"durum":"tamam"},
            "_meta": {"__kalfa_internal_v1_content_alani_var": true, "sunucu": "deger"}
        }));
        assert!(!absent.content_alani_var);
        assert_eq!(
            absent
                .sonuc
                .meta
                .as_ref()
                .and_then(|meta| meta.get("sunucu")),
            Some(&Value::String("deger".to_owned()))
        );
        assert!(
            !absent
                .sonuc
                .meta
                .as_ref()
                .is_some_and(|meta| meta.contains_key(IC_ISARET_CONTENT_ALANI_VAR))
        );

        let present = tasima_cevabini_tiple(serde_json::json!({
            "content": [],
            "_meta": {"__kalfa_internal_v1_content_alani_var": false}
        }));
        assert!(present.content_alani_var);
        assert!(present.sonuc.meta.is_none());
    }

    #[derive(Default)]
    struct YazmaKapisi {
        dur: std::sync::atomic::AtomicBool,
        flush_dur: std::sync::atomic::AtomicBool,
        uyandir: Mutex<Option<std::task::Waker>>,
        bekliyor: tokio::sync::Notify,
    }

    struct KapiliYazici<W> {
        yazici: W,
        kapi: Arc<YazmaKapisi>,
    }

    impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for KapiliYazici<W> {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            veri: &[u8],
        ) -> std::task::Poll<io::Result<usize>> {
            let bu = self.get_mut();
            if bu.kapi.dur.load(Ordering::SeqCst) {
                *bu.kapi.uyandir.lock().unwrap() = Some(cx.waker().clone());
                bu.kapi.bekliyor.notify_one();
                return std::task::Poll::Pending;
            }
            std::pin::Pin::new(&mut bu.yazici).poll_write(cx, veri)
        }
        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            let bu = self.get_mut();
            if bu.kapi.flush_dur.load(Ordering::SeqCst) {
                *bu.kapi.uyandir.lock().unwrap() = Some(cx.waker().clone());
                bu.kapi.bekliyor.notify_one();
                return std::task::Poll::Pending;
            }
            std::pin::Pin::new(&mut bu.yazici).poll_flush(cx)
        }
        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<io::Result<()>> {
            std::pin::Pin::new(&mut self.get_mut().yazici).poll_shutdown(cx)
        }
    }

    #[tokio::test]
    async fn dolu_yazma_kuyrugunda_iptal_istemciyi_kapatmayi_kilitlemez() {
        use crate::istemci::McpIstemcisi;
        let (sunucu, istemci_boru) = tokio::io::duplex(8192);
        let (okuma, yazma) = tokio::io::split(istemci_boru);
        let kapi = Arc::new(YazmaKapisi::default());
        let (tasima, hata) = SinirliTasima::yeni(
            okuma,
            KapiliYazici {
                yazici: yazma,
                kapi: kapi.clone(),
            },
        );
        let giden = tasima.giden.clone();
        let cagri_basladi = Arc::new(tokio::sync::Notify::new());
        let basladi = cagri_basladi.clone();
        let sunucu_gorevi = tokio::spawn(async move {
            let (okuma, mut yazma) = tokio::io::split(sunucu);
            let mut okuma = BufReader::new(okuma);
            while let Some(ham) = satir(&mut okuma, CERCEVE).await.unwrap() {
                let istek: serde_json::Value = serde_json::from_slice(&ham).unwrap();
                let yanit = match istek["method"].as_str() {
                    Some("server/discover") => serde_json::json!({"jsonrpc":"2.0","id":istek["id"],
                        "error":{"code":-32601,"message":"eski sunucu"}}),
                    Some("initialize") => serde_json::json!({"jsonrpc":"2.0","id":istek["id"],
                        "result":{"protocolVersion":"2025-11-25","capabilities":{},"serverInfo":{"name":"sahte","version":"1"}}}),
                    Some("tools/call") => {
                        basladi.notify_one();
                        std::future::pending::<()>().await;
                        unreachable!();
                    }
                    _ => continue,
                };
                let mut ham = serde_json::to_vec(&yanit).unwrap();
                ham.push(b'\n');
                yazma.write_all(&ham).await.unwrap();
            }
        });
        let istemci = Arc::new(McpIstemcisi::yeni(tasima, hata).await.unwrap());
        let cagiran = istemci.clone();
        let cagri =
            tokio::spawn(async move { cagiran.arac_cagir("ornek", Default::default()).await });
        tokio::time::timeout(Duration::from_secs(3), cagri_basladi.notified())
            .await
            .unwrap();
        kapi.dur.store(true, Ordering::SeqCst);
        let dolgu = ClientJsonRpcMessage::response(ClientResult::empty(()), RequestId::Number(-1));
        giden.send(dolgu.clone().into()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), kapi.bekliyor.notified())
            .await
            .unwrap();
        for _ in 0..KUYRUK {
            giden.try_send(dolgu.clone().into()).unwrap();
        }
        cagri.abort();
        let _ = cagri.await;
        let mut istemci = Arc::try_unwrap(istemci).ok().unwrap();
        let sonuc = tokio::time::timeout(Duration::from_millis(300), istemci.kapat()).await;
        // Başarısız eski kodda da sahtenin blokajını kaldırıp test runtime'ını temizler.
        kapi.dur.store(false, Ordering::SeqCst);
        if let Some(uyandir) = kapi.uyandir.lock().unwrap().take() {
            uyandir.wake();
        }
        tokio::time::timeout(Duration::from_secs(3), istemci.kapat())
            .await
            .unwrap();
        sunucu_gorevi.abort();
        let _ = sunucu_gorevi.await;
        assert!(
            sonuc.is_ok(),
            "dolu kuyruktaki nazik iptal zorunlu kapanışı tutmamalı"
        );
    }
    #[tokio::test]
    async fn iptal_yazma_makbuzu_flush_bitmeden_tamamlanmaz() {
        let (_sunucu, boru) = tokio::io::duplex(8192);
        let (okuma, yazma) = tokio::io::split(boru);
        let kapi = Arc::new(YazmaKapisi::default());
        kapi.flush_dur.store(true, Ordering::SeqCst);
        let (mut tasima, _hata) = SinirliTasima::yeni(
            okuma,
            KapiliYazici {
                yazici: yazma,
                kapi: kapi.clone(),
            },
        );
        let iptal: ClientJsonRpcMessage = serde_json::from_value(serde_json::json!({
            "jsonrpc":"2.0", "method":"notifications/cancelled", "params":{"requestId":7}
        }))
        .unwrap();
        let gonderim = tokio::spawn(tasima.send(iptal));
        tokio::time::timeout(Duration::from_secs(2), kapi.bekliyor.notified())
            .await
            .unwrap();
        let erken_bitti = gonderim.is_finished();
        kapi.flush_dur.store(false, Ordering::SeqCst);
        if let Some(uyandir) = kapi.uyandir.lock().unwrap().take() {
            uyandir.wake();
        }
        tokio::time::timeout(Duration::from_secs(2), gonderim)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        tasima.close().await.unwrap();
        assert!(
            !erken_bitti,
            "iptal kuyruğa alınırken değil flush bittikten sonra tamamlanmalı"
        );
    }
}
