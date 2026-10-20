# Yetenek Programı Hat 04 (G3a) — `rmcp 3.5.1` Streamable HTTP Ön Doğrulama Raporu

Tarih: 20 Ekim 2026  
Ajan: `Antigravity-Hat-04`  
Dal: `codex/plan-yetenek-uzak-mcp` (PR #40)  
Prototip paketi: `prototip/yetenek-mcp-http/` (ana workspace üyesi değildir; `rmcp =3.5.1` sabit sürümüyle çevrimdışı doğrulanmıştır)

## 1. Amaç ve Kapsam

Bu ön doğrulama (G3a), ürün koduna (`crates/kalfa-mcp/src/uzak/`) geçmeden önce sabit `rmcp =3.5.1` sürümünün uzak MCP Streamable HTTP akışında nasıl davrandığını, sürüm müzakeresini, başlık kurallarını, `application/json` ve `text/event-stream` yanıtlarını, hata/iptal/zaman aşımı semantiğini ve yan etkili `tools/call` sırasında bağlantı kopması davranışını izole bir prototiple iki platformda (Windows ve WSL `Ubuntu-24.04`) kanıtlar.

Kısıtlar:
- `rmcp =3.5.1` yükseltilmemiştir; yerel `stdio` (`SinirliTasima`) davranışı değiştirilmemiştir.
- Yeni dış crate indirilmemiş; ana `Cargo.lock` içindeki sabit sürümler (`rmcp 3.5.1`, `reqwest 0.13.5`, `rustls 0.23.46`, `tokio 1.53.1`, `url 2.5.8`, `bytes 1.12.1`) ile `--locked --offline` kipinde koşulmuştur.

## 2. `rmcp 3.5.1` Protokol Sürümü ve Başlık Uyumluluk Matrisi

`rmcp 3.5.1` (`ProtocolVersion::KNOWN_VERSIONS`) aşağıdaki protokol sürümlerini tanır ve `ClientLifecycleMode::Auto { preferred_versions: vec![ProtocolVersion::V_2026_07_28], legacy_version: Some(ProtocolVersion::V_2025_11_25) }` ile iki aşamalı başlatma uygular:

| Özellik / Davranış | `2026-07-28` (Modern / SEP-2567, SEP-2575, SEP-2243) | `2025-11-25` (Eski Dönem Tavanı) | `2025-06-18` (Eski Dönem) | `2025-03-26` (Eski Dönem) |
|---|---|---|---|---|
| **Başlatma el sıkışması** | `server/discover` (`DiscoverRequest` -> `DiscoverResult`); `initialize` ve `notifications/initialized` **gönderilmez** | `server/discover` reddi sonrası otomatik `initialize` + `notifications/initialized` | `server/discover` reddi sonrası `initialize` (sunucu `2025-06-18` döner) + `notifications/initialized` | `server/discover` reddi sonrası `initialize` (sunucu `2025-03-26` döner) + `notifications/initialized` |
| **İstek gövdesi `_meta` alanları** | Her istekte `params._meta["io.modelcontextprotocol/protocolVersion"] = "2026-07-28"`, `params._meta["io.modelcontextprotocol/clientCapabilities"] = {}` ve `params._meta["io.modelcontextprotocol/clientInfo"]` taşınır | Başlatma bilgisi `initialize` içinde gönderilir; sonraki isteklerde SEP-2575 `_meta` zorunlu değildir | Başlatma bilgisi `initialize` içinde gönderilir | Başlatma bilgisi `initialize` içinde gönderilir |
| **`MCP-Protocol-Version` HTTP başlığı** | `server/discover` dahil tüm isteklerde `MCP-Protocol-Version: 2026-07-28` | `initialize` sonrasında (`notifications/initialized`, `tools/list`, `tools/call`) `MCP-Protocol-Version: 2025-11-25` | `initialize` sonrasında `MCP-Protocol-Version: 2025-06-18` | `initialize` sonrasında `MCP-Protocol-Version: 2025-03-26` |
| **`Mcp-Session-Id` oturum başlığı** | Durumsuz (stateless) modern HTTP; `Mcp-Session-Id` kullanılmaz | Sunucu `initialize` yanıtında `Mcp-Session-Id` dönerse (ASCII `0x21..=0x7E`), izleyen tüm POST isteklerinde iletilir | Sunucu `initialize` yanıtında `Mcp-Session-Id` dönerse izleyen tüm POST isteklerinde iletilir | Sunucu `initialize` yanıtında `Mcp-Session-Id` dönerse izleyen tüm POST isteklerinde iletilir |
| **SEP-2243 yönlendirme başlıkları (`Mcp-Method`, `Mcp-Name`)** | `version >= ProtocolVersion::STANDARD_HEADERS` (`2026-07-28`) olduğunda `Mcp-Method` ve `tools/call` için `Mcp-Name` (ASCII dışı/boşluklu adlarda `=?base64?...?=`) gönderilir | Gönderilmez | Gönderilmez | Gönderilmez |
| **İptal (`notifications/cancelled`)** | Ayrı iptal POST'u gönderilmez; açık HTTP yanıt akışı düşürülür | `Mcp-Session-Id` ile `notifications/cancelled` POST edilir (`202 Accepted` / `200 OK`) | `Mcp-Session-Id` ile `notifications/cancelled` POST edilir | `Mcp-Session-Id` ile `notifications/cancelled` POST edilir |

## 3. Kritik Protokol ve Taşıma Bulguları (Ürün Koduna Bağlanacak Kurallar)

1. **SEP-2575 `_meta` Anahtar Adlandırması (`io.modelcontextprotocol/*`):**
   - `rmcp 3.5.1` içinde `RequestMetaObject::with_client_context`, protokol sürümünü `_meta["protocolVersion"]` olarak değil, `_meta["io.modelcontextprotocol/protocolVersion"]`, `_meta["io.modelcontextprotocol/clientCapabilities"]` ve `_meta["io.modelcontextprotocol/clientInfo"]` olarak serileştirir.
   - `request.request.get_meta().protocol_version()` yardımcı metodu bu ad alanlı anahtarı doğru çözer. Özel HTTP taşıması sürüm başlığını doğrudan `get_meta().protocol_version()` üzerinden okumalıdır.
2. **Eski Sunucularda `server/discover` HTTP 4xx Geri Dönüşü:**
   - `rmcp 3.5.1`'in `discover_startup` işlevi yalnızca korele edilmiş bir `ServerJsonRpcMessage::Error` (modern red kodları `MISSING_REQUIRED_CLIENT_CAPABILITY` ve `HEADER_MISMATCH` hariç) aldığında `DiscoverOutcome::Legacy` döndürür ve aynı taşıma üzerinde `initialize` el sıkışmasına geçer.
   - Oturumsuz `server/discover` POST'una HTTP `400 Bad Request` / `404 Not Found` / `422` döndüren eski (`2025-*`) ara katmanlarda, taşıma (`401`/`403` yetki hataları hariç) bu yanıtı korele `ServerJsonRpcMessage::error(ErrorCode::METHOD_NOT_FOUND, ..., Some(discover_id))` iletisine dönüştürdüğünde `ClientLifecycleMode::Auto` kesintisiz biçimde `initialize` + `notifications/initialized` akışına düşer.
   - `401 Unauthorized` ve `403 Forbidden` yanıtları ise asla eski sürüm geri dönüşü sayılmaz; `WWW-Authenticate` başlığıyla birlikte `YetkiGerekli` olarak ayrıştırılır.
3. **`tools/list` Önbelleği ve `_meta` İç İşareti:**
   - `rmcp 3.5.1` varsayılan olarak `ClientCacheConfig::default()` ile pozitif `ttlMs` taşıyan `tools/list` yanıtlarını SDK içinde önbelleğe alır. Denetlenmemiş uzak araç şemalarının SDK önbelleğine girmemesi için `peer().set_response_cache_config(ClientCacheConfig::disabled())` zorunludur.
   - `kalfa-mcp` içindeki `sonuc.rs` (`ic_isareti_tuket`), `CallToolResult` içindeki `content: []` ile eksik `content` alanını ayırt etmek için ham JSON aşamasında `_meta.__kalfa_internal_v1_content_alani_var = true` işaretini bekler. Özel sınırlı HTTP taşıması hem `application/json` hem `text/event-stream` (`data:` satırları) üzerinden gelen `ServerJsonRpcMessage::Response` iletilerinde bu işareti enjekte eder.
4. **Sınırlı SSE Çerçeveleme, Bildirim Seli ve Desteklenmeyen Sunucu İstekleri:**
   - `text/event-stream` yanıtları bayt düzeyinde satır sonuna (`\r\n`, `\n`, `\r`) göre bölündükten sonra UTF-8'e çevrilir; böylece TCP parça sınırında bölünen çok baytlı Türkçe UTF-8 karakterler (`İğüşöç`) bozulmaz.
   - Tek satır ve tek SSE olayı/JSON gövdesi `CERCEVE_SINIRI = 4 MiB` (`4_194_304` bayt) ile sınırlıdır; sınır aşıldığında bellek büyütülmeden `HttpKesinti::CerceveCokBuyuk` ile reddedilir.
   - Beklenmeyen `Content-Type` (ör. `text/html`) `HttpKesinti::DesteklenmeyenIcerikTipi` ile kapalı (fail-closed) reddedilir.
   - `notifications/tools/list_changed` bildirim seli SDK görev kuyruğuna sokulmadan tek bir `AtomicBool` (`yenileme`) bayrağına indirgenir; sunucudan gelen `ping` isteğine boş sonuç dönülürken `sampling/createMessage`, `roots/list`, `elicitation/create` gibi kapsam dışı istekler taşıma sınırında `-32601 METHOD_NOT_FOUND` ile reddedilir.
5. **Yan Etkili `tools/call` Sırasında Bağlantı Kopması ve Zaman Aşımı (M05):**
   - `tools/call` POST isteği gönderildikten sonra HTTP akışı son JSON-RPC yanıtı gelmeden kapanırsa veya zaman aşımına uğrarsa, taşıma durumu `HttpKesinti::BelirsizCagriSonucu { istek_id, arac_adi }` olarak kaydeder ve **otomatik yeniden deneme yapmaz** (`cagri_sayaci == 1`).

## 4. İki Platform Doğrulama Kanıtları

Doğrulanan `prototip/yetenek-mcp-http/src/lib.rs` SHA-256 özeti:
`744729f24e9fe931bccd97a17e02b3870110cd8dd04ece7901652d31858933c4`

Doğrulanan `prototip/yetenek-mcp-http/src/lib.rs` testleri:
1. `protokol_2026_07_28_discover_ve_standart_basliklar`
2. `eski_donem_surum_matrisi_2025_11_25_2025_06_18_2025_03_26`
3. `sse_cok_parcali_turkce_utf8_bildirim_seli_ve_sunucu_istek_reddi`
4. `bozuk_json_asiri_buyuk_govde_ve_beklenmeyen_icerik_tipi_reddedilir`
5. `akis_ortasinda_baglanti_kopmasi_belirsiz_sonuc_verir_ve_otomatik_tekrar_yapmaz`
6. `zaman_asimi_ve_401_yetki_hatasi_ayristirilir`

| Kontrol | Windows (`1.98.1`, `--locked --offline`) | WSL `Ubuntu-24.04` (`kalfatest`, `--locked --offline`) |
|---|---|---|
| `cargo fmt -- --check` | Geçti | Geçti |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | Geçti (0 uyarı) | Geçti (0 uyarı) |
| `cargo test --locked --offline` | Geçti (6/6 test) | Geçti (6/6 test) |


## 5. Ürün Uygulaması ve Kabul Doğrulaması (Görev 2–6)

G3a prototipinde doğrulanan `rmcp =3.5.1` Streamable HTTP kuralları aşağıdaki commit dizisiyle ürün modüllerine taşınmış ve iki platformda doğrulanmıştır:

- **Görev 2 (`5bc009d73f31400f06871a67a90a3fe805db3c08`):** Yerel `McpTanim` ve uzak `UzakMcpTanim` / `McpSunucuKaynagi` ayrımı, `uzak_sunucu_tanim_ozeti`, `uzak_arac_tanim_ozeti`, `uzak_calisma_tarifi`, `UzakMcpSunucu` ayarları ve kasa entegrasyonu.
- **Görev 3 / G3b (`bc40673683f71337d548e979bde74bb611b1a5b3`):** `crates/kalfa-mcp/src/uzak/{http,sse,oturum}.rs` içinde `SinirliHttpTasima`, DNS/IP/yönlendirme ağ politikası, 4 MiB çerçeve sınırı, `McpIstemcisi::yeni_uzak` ve `M02`/`M05`/`M06` (`crates/kalfa-mcp/tests/uzak_http.rs`) testleri.
- **Görev 4 (`e9635b1f3ed3e67c8f1e312272dcc2f0e45285ee`):** `crates/kalfa-mcp/src/uzak/oauth.rs` ve `crates/kalfa-uygulama/src/mcp/uzak_oauth.rs` içinde PKCE (`S256`), `state`/oturum bağlama, `issuer`/`resource` doğrulaması, tek uçuşlu yenileme, yerel onaylı tarayıcı açma, `GizliKasa` saklaması ve `M03` testleri (`crates/kalfa-mcp/tests/uzak_oauth.rs` ve `uzak_oauth::testler`).
- **Görev 5 (`a8769087b7ac7db6eadf2f3e63e21e9a0af08682`):** `crates/kalfa-mcp/src/uzak/kesif.rs` içinde `UzakAracKatalogu`, `AracAraAraci` (`arac_ara`), `UzakAracKaynagi` (en çok 8 aday, 16 etkin araç, 64 KiB şema bütçesi, sonraki tur anlık görüntü garantisi) ve `M04` testleri (`crates/kalfa-mcp/tests/uzak_kesif.rs`).
- **Görev 6 (`63ae19ca367c854bae81ee84b3378c6ada2cd09a`):** `crates/kalfa-uygulama/src/mcp/uzak_yonetici.rs` (`UzakMcpYoneticisi`, `UzakMcpRozeti::UzakStreamableHttp`, `UzakMcpDurumEtiketi`), `ui/src/bilesenler/UzakMcpFormu.svelte`, `ui/src/bilesenler/UzakMcpSunuculari.svelte`, `ui/src/bilesenler/uzakMcp.test.ts`, `ui/src/ipc/tipler.ts` ve `ui/src/dil/tr.ts`.

| Ürün Doğrulama Kontrolü | Windows (`1.98.1`) | WSL `Ubuntu-24.04` (`kalfatest`, `1.98.1`) |
|---|---|---|
| `cargo check -p kalfa-mcp --lib --bins --locked --offline` | Geçti | Geçti |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | Geçti (0 uyarı) | Geçti (0 uyarı) |
| `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked --offline -- -D warnings` | Geçti (0 uyarı) | Geçti (0 uyarı) |
| `cargo test -p kalfa-mcp --locked --offline` (birim + `protokol` + `sinirlar` + `uzak_http` + `uzak_oauth` + `uzak_kesif`) | Geçti (79 + 17 + 12 + 3 + 1 + 3 test) | Geçti (79 + 17 + 12 + 3 + 1 + 3 test) |
| `cargo test -p kalfa-uygulama mcp:: --locked --offline` + `--test mcp_komutlari` | Geçti (82 + 6 test) | Geçti (82 + 6 test) |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked --offline` | Geçti | Geçti |
| `ui/`: `npm run check && npm test && npm run build` | Geçti (`svelte-check` 0 hata/0 uyarı, `vitest` 22 dosya geçti, `Paket denetimi temiz.`) | — |

## 6. İnceleme Düzeltme Turu (SP-06, SP-04, SP-08) ve Ortak Çekirdek/IPC Bütünleştirmesi

20 Ekim 2026 inceleme raporundaki (`INCELEME.md`) uzak MCP hattına ait üç bulgu önce davranışı ölçen düşen regresyonlarla (RED), ardından düzeltmelerle (GREEN) kapatılmış ve `02 -> 03 -> 04` sırasıyla ortak çekirdek/IPC birleştirmesi tamamlanmıştır:

1. **SP-06 — Üretim TLS Güven Kökleri ve Yerel HTTPS/TLS Doğrulama Testleri (`85fe910a932c20a3d10d7f04dbcf7cf8715808fd`):**
   - **Sorun:** `crates/kalfa-mcp/src/uzak/http.rs` içindeki `kok_sertifikalar()` yalnız `RootCertStore::empty()` döndürdüğü için gerçek `https://` bağlantıları TLS el sıkışmasında düşüyordu.
   - **RED Kanıtı:** `m02b_https_tls_kok_zinciri_ve_host_adi_dogrulamasi_calisir` (`crates/kalfa-mcp/tests/uzak_http.rs`) ve `m03b_oauth_https_tls_kok_ve_host_dogrulamasi_calisir` (`crates/kalfa-mcp/tests/uzak_oauth.rs`) eklendiğinde `RootCertStore::empty()` ile güvenilen zincir vakası `HttpHatasi::IstekBasarisiz` ile düştü.
   - **Düzeltme ve GREEN Kanıtı:** Sertifika doğrulaması kapatılmadan, varsayılan durumda `rustls_platform_verifier::BuilderVerifierExt::with_platform_verifier` (işletim sistemi güven kökleri), özel kök verildiğinde ise `RootCertStore` + `WebPkiServerVerifier` kullanılacak şekilde hem MCP HTTP (`SinirliHttpTasima`) hem OAuth metadata/token istemcileri güncellendi. Güvenilen zincir + doğru host -> geçti; güvenilmeyen zincir -> reddedildi; yanlış host adı -> reddedildi; özel CA'ya karşı varsayılan işletim sistemi kök deposu -> reddedildi.
2. **SP-04 — Uzak MCP Erişim Sözleşmesi, Mühür Bağlamı ve Onay Kartı Ayrımı (`d1cd78cfbe4e73df38a02cb1ec2736d3cbbf07aa`):**
   - **Sorun:** `UzakMcpArac::hazirla` içinde `McpErisim::Yalitimli` kullanıldığı için uzak MCP çağrıları yerel AppContainer/bwrap yalıtımı varmış gibi sınıflandırılıyor, onay kartında uzak origin/kimlik kapsamı ve veri çıkışı uyarısı gösterilmiyordu.
   - **RED Kanıtı:** `m05_uzak_mcp_arac_hazirligi_yerel_yalitimli_degildir_ve_uzak_baglam_tasir` (`uzak_kesif.rs`), `uzak_mcp_her_kipte_onay_ister_ve_yerel_yalitim_iddia_etmez` (`crates/kalfa-core/tests/izin.rs`) ve `bekleyenOnay.test.ts` içindeki SP-04 testi önceki sözleşmeyle düştü.
   - **Düzeltme ve GREEN Kanıtı:** `McpErisim::UzakStreamableHttp`, `McpAgKarti::UzakHedef { host, origin, kimlik_kapsami }`, `VeriCikisi::UzakMcpCagrisi`, `MCP_UZAK_VERI_CIKISI_UYARISI` ve `McpOnayKarti { uzak_origin, kimlik_kapsami }` eklendi. `MCP_UZAK_VERI_CIKISI_UYARISI` bilerek `HAKEME_UYGUN_UYARILAR` dışında bırakıldı (`hakeme_uygun == false`); uzak MCP çağrısı `Kip::Otomatik` dahil her kipte kullanıcı onayı ister. `muhur_hesapla` uzak origin ve kimlik kapsamına bağlandı; `BekleyenOnay.svelte` ve `OnaySayfasi.svelte` yerel yalıtım rozeti yerine `Uzak MCP (HTTPS)` rozetini, hedef origin'i, kimlik kapsamını ve uzak veri çıkışı uyarısını gösterir.
3. **SP-08 — `UzakMcpYoneticisi`, Ayarlar Ekranı, Tauri IPC ve Oturum/OAuth Bağlantısı (`c024ea5ef707be02d4bb9fbca2da76a4b908843f` + `887a5177c2586c51f5ef03d145c36932de9cf15d`):**
   - **Sorun:** `UzakMcpYoneticisi`, `AracAraAraci`, Tauri IPC komutları ve `Ayarlar.svelte` içindeki `UzakMcpSunuculari` bileşeni ana uygulamaya bağlanmamıştı.
   - **RED Kanıtı:** `npx vitest run src/ana/ayarlar.test.ts -t "uzak MCP"` (`Unable to find an element with the text: Uzak MCP sunucuları`) ve `cargo test -p kalfa-uygulama --test uzak_mcp_entegrasyon` (`no field uzak_mcp on type &Uygulama`) düştü.
   - **Düzeltme ve GREEN Kanıtı:** `Uygulama::uzak_mcp`, `BirlesikMcpAracKaynagi` (yerel + uzak MCP araç yayımı), `AracAraAraci` kaydı, 8 yeni Tauri IPC komutu (`uzak_mcp_sunuculari_listele`, `uzak_mcp_sunucu_ayrintisi`, `uzak_mcp_sunucu_ekle`, `uzak_mcp_sunucu_baglan`, `uzak_mcp_sunucu_durdur`, `uzak_mcp_sunucu_sil`, `uzak_mcp_oauth_bagla`, `uzak_mcp_kimlik_kaldir`), yerel loopback geri dönüş dinleyicili `kullanici_oauth_bagla_ve_tamamla` ve `Ayarlar.svelte` entegrasyonu tamamlandı. `uygulama_uzak_mcp_ekle_oauth_bagla_kesfet_oturumda_cagir_ve_kimlik_kaldir` testi ekle -> OAuth bağla -> keşfet (`arac_ara` + `yukle`) -> oturumda onaylı çağır (`McpErisim::UzakStreamableHttp`) -> kimliği kaldır -> eski hazırlığın `OnayEskidi` ile düşmesi akışını uçtan uca doğruladı.
   - **Ortak Çekirdek/IPC (`02 -> 03 -> 04`):** `main` (`#35`), Hat 02 (`d627f6d`, `bca07f6` -> dalımızda `d0ba737`, `001362b`) ve Hat 03 (`53c2ed0`) sırasıyla `887a5177c2586c51f5ef03d145c36932de9cf15d` commit'inde birleştirildi ve `8a2dcd96b8585307b78549cf54becd7c349ceb5b` ile başsız Linux test kasası yalıtımı tamamlandı; `ANA_PENCERE_KOMUTLARI` (`98` komut) ve `capabilities/ana.json` izin testleriyle (`crates/kalfa/tests/izinler.rs` 9/9) doğrulandı.
   - **Taban Dal Notu (Wasmtime Audit):** `cargo audit` kontrolündeki `wasmtime` (`kalfa-plugin`) duyurusu taban daldan (`main`) gelmektedir; plan kısıtları gereği bu hatta rastgele bağımlılık yükseltmesi yapılmamıştır.
