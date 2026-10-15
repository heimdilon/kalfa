# MCP 1.1a-2 — G2 taşıma ve protokol ön doğrulaması

Tarih: 15 Ekim 2026. Ajan: Codex; bağımsız kaynak/plan incelemesi GPT-6 Luna,
max. Taban main `cf4e5f3`. Yeni ürün kodu yazılmadı; `prototip/mcp-tasima/`
bağımsız Cargo workspace'idir. Ana Cargo.toml/Cargo.lock ve crates/ui/e2e
kaynakları bu PR'da değişmez.

## İzin ve kaynak

Kullanıcı bu oturumdaki dar indirme sorusuna doğrudan “İndir ve iki
platformda ön doğrula” dedi. rmcp 3.5.1 ve gerekli paketler Windows/WSL
Cargo önbelleğine indirildi. Sistem kurulumu, yönetici işlemi ve canlı
EVREN deneyi yapılmadı. Çevrimdışı ilk çözümleme rmcp bulunamadığı için
hata verdi; indirme bu onaydan sonra yapıldı.

[Resmi 3.5.1 belgeleri](https://docs.rs/rmcp/3.5.1/rmcp/) ve
[SDK etiketi](https://github.com/modelcontextprotocol/rust-sdk/tree/rmcp-v3.5.1)
incelendi. Etiketin commit'i `79437f291b2c44053d00dcd5db969fd0cca7c887`.
Kurulan crate'in Transport, client lifecycle, cache ve service kaynakları
API ve veri akışı için esas alındı. Yeni standalone lock 75 paket kaydı
barındırır; ürün lock'u değişmez.

## Sonuçlar

Rust 1.98.1. Windows sıradan geliştirici hesabı; WSL Ubuntu-24.04,
`kalfatest`. Mevcut derleme önbellekleri kullanıldı; ayrı büyük target
oluşturulmadı. İndirme sonrasında test/clippy offline ve locked koştu.

| Kontrol | Windows | WSL |
|---|---|---|
| Yedi G2 testi | 7/7 | 7/7 |
| clippy all-targets, -D warnings | Geçti | Geçti |
| fmt check | Geçti | Geçti |

Doğrulanan `src/lib.rs` SHA-256:
`4c56739175f12d840ce3be2c190e6a712756654ce419c3f7a4dffa472e7b1059`.
PR kopyasının bu kaynakla eşitliği dosya özetiyle kontrol edilir.

1. LF hariç 4 MiB satır geçer; LF gelmeden sonraki bayt reddedilir.
2. Birer bayta bölünmüş Türkçe UTF-8 ve ardışık satırlar birebir alınır;
   yarım EOF hata olur.
3. Alıcı tüketmezken 65'inci küçük ileti 64 kapasiteli kuyruğu büyütmez;
   `IletiSeli` watch kanalından görünür.
4. Bin liste bildirimi tek bayrak olur, SDK'ye kuyruk/görev seli gitmez.
5. Bozuk JSON, ham satır yazılmadan tipli hata ve receive kapanışı üretir.
6. Eski sunucu Auto'da discover reddinden initialize'a döner; boş yetenek,
   initialized bildirimi, liste, ping ve desteklenmeyen istek reddi ölçülür.
7. 2026 sunucusu discover ile başlar; stdio listen/ACK ve liste bayrağı,
   liste, ping ve desteklenmeyen istek reddi ölçülür.

İki protokol testi pozitif `ttlMs=60000` döndürür. SDK cache'i kapalıyken
iki list_tools iki gerçek RPC'dir. Kapatma satırı geçici çıkarılınca
iki test de 1/2 RPC farkıyla düştü; geri konunca 7/7 geçti. Bu, ham
tanımların SDK cache'ine erken alınması hatasını gerçekten ayırt eder.

## Uygulamaya bağlanacak bulgular

- `ClientCacheConfig::default()` açık cache'tir; sıfır varsayılan TTL,
  sunucunun pozitif TTL veremeyeceği anlamına gelmez. İlk listeleme
  öncesinde disabled kullanılır. Denetlenmiş tanım cache'i Kalfa'dadır.
- SDK servis döngüsü normal istek/bildirimleri ayrı görevlerde işleyebilir.
  Sadece giriş kuyruğuna 64 sınırı koymak bütün etkin görevlerin üst
  sınırını kanıtlamaz. G2 listeyi tek bayrağa indirir, sıradan bildirimleri
  atar, ping/unsupported istekleri kendi sınırlı yazıcısında yanıtlar.
- SDK tracing olayları ham request/result/notification/hata yükleri
  taşıyabilir. Üründe bu hedefler log katmanlarından dışlanır; genel debug
  ayarının bunu aşamadığı canary testi gerekir. G2 log sızdırmazlığı
  kabulünün yerine geçmez.
- Dev `server` feature'ı transport-async-rw'yi açar; kullanılan istemci
  yolu özel Transport'tur. SDK'nin sınırsız read_until yoluna stdout verilmez.
- Mevcut core RAII kilidi hata dönüşünde düşer; manager'da sahipliği
  korunmadan kapat Err sözleşmesi sağlanamaz. Plan Görev 6 buna ayrıdır.
- Mevcut bütçe kapısı idle oturum talebini reddeder. Tanım keşfi idle'da
  sıraya alınır; denetim ilk aktif turda bütçe kanalı dinlenirken başlar.

## Deney sırasında düzeltmeler

İlk derlemede SubscriptionFilter builder'ın parametresiz olduğu ve
listen kapasitesinin NonZeroUsize istediği görüldü, kaynak buna uyarlandı.
Modern testin sentetik ACK'i önce yalnız ServerNotification gövdesini
serileştiriyordu. Dış `jsonrpc:2.0` zarfı eksikti; parse hatası listen'i
kapatıyordu. ServerJsonRpcMessage::notification ile düzelince test geçti.
Bu, SDK aboneliklerinin stdio'da çalışmadığının kanıtı değildir; G2'nin
hatalı sentetik sunucusuydu. Önceki hatalar yerel loglarda korunur.

## Kapsam ve temizlik

G2 gerçek OS süreci, Job Object, pidfd, stderr, UI veya EVREN kullanmaz;
1.1a-1'in yalıtım kanıtının yerine geçmez. Outbound bayt tavanı, bütün
uygulamanın toplam belleği, süreç kapatma/son denetim ve abonelik
iptalinden toparlanma bu yedi testle kanıtlanmış sayılmaz. Ürün planında
bunların sahibi ve kabul testleri ayrıca belirtilir.

İlk test kopyası `.yerel/mcp-11a2-on-dogrulama/`; WSL kopyası
`/home/kalfatest/kalfa-codex-mcp11a2-on`. Kaynak doğrulaması sonrasında
WSL kopyası kaldırıldı; paylaşılan eski target önbellekleri korundu.
Bu deney dosya/ağ/profil/ACE oluşturmadığından yeni yalıtım artığı yoktur.
Yerel kaynak, negatif ve son doğrulama logları
`.yerel/mcp-tasarim-inceleme/11A2-*.log` içindedir.

Ürün workspace testleri bu belge/prototip değişikliği için yerelde yeniden
koşturulmadı; ürün kaynak ağacı tabanla aynıdır. PR CI varsayılan workspace
komutlarına ek bağımsız G2 test/clippy/fmt ve G2 lock audit'ini çalıştırır.
Yeni arayüz ve gerçek yalıtım testleri ürün uygulamasının kapanış kapısıdır.