# Yetenek 05 — Web Getirme ve Kaynaklı Arama: Bulgular ve Kararlar

- **Tarih:** 2026-10-20
- **Hat / Ajan:** Yetenek 05 (`Antigravity-Hat-05`)
- **Dal / PR:** `codex/plan-yetenek-web` ([PR #41](https://github.com/heimdilon/kalfa/pull/41))
- **Kapsam:** `G4a` (Ağ/izin sözleşmesi, yerel HTTP/HTTPS test sunucuları, sınırlı `web_getir` aracı, kaynak/güven zarflaması, sızıntı önleme ve arayüz kaynak kartı) + `G4b`'nin yerel/fixture doğrulama katmanı (`W01`–`W06`).

---

## 1. Görev 1 — Ağ Sınırı ve İzin Deneyi (`G4a`)

### 1.1 Ana Süreçteki HTTP İstekleri ve Kabuk Yalıtımı Ayrımı

Kalfa'nın kabuk yalıtımı (`kalfa-sandbox`: Linux'ta `bubblewrap` + `seccomp` + `kalfa-kopru`; Windows'ta AppContainer + Job Object) yalnızca **alt süreç** olarak başlatılan kabuk komutlarını sarar. Oysa `web_getir` aracı doğrudan Kalfa'nın ana Rust sürecinde (`kalfa-tools`) çalışır. Bu nedenle:

- Ana süreçten yapılan bir `reqwest` çağrısı işletim sistemi kabuk yalıtımına girmez.
- Ana sürecin kendi soket doğrulaması, yönlendirme politikası ve vekil (`proxy`) kısıtlaması yoksa yerel ağa (`127.0.0.1`, `::1`, RFC1918), link-local adreslere (`169.254.0.0/16`, `fe80::/10`) ve bulut metadata servislerine (`169.254.169.254`, `fd00:ec2::254`) SSRF erişimi açılır.

### 1.2 Naif URL Ön Kontrolünün Yetersizliği (Deneysel Kanıt)

`crates/kalfa-tools/src/web/ag.rs` içindeki `naif_url_on_kontrolu` fonksiyonu ve `crates/kalfa-tools/tests/web_getir.rs` içindeki `gorev1_naif_on_kontrol_dns_rebinding_ve_coklu_kayitta_yanilir_web_istemci_engeller` testi iki temel açığı kanıtlar:

1. **Çoklu DNS Kaydı (Multi-A / Multi-AAAA):** Bir alan adı hem genel bir IP (`93.184.216.34`) hem de yerel bir IP (`127.0.0.1`) döndürdüğünde yalnızca ilk kaydı kontrol eden naif ön kontrol isteğe izin verir; HTTP istemcisi ise bağlantı kurarken ikinci kayda (`127.0.0.1`) düşebilir.
2. **DNS Rebinding (TOCTOU):** Ön kontrol sırasındaki DNS sorgusu genel IP döndürürken, milisaniyeler sonraki gerçek soket bağlantısında DNS çözücüsü `127.0.0.1` döndürebilir.
3. **Sistem/Ortam Vekili (`HTTP_PROXY` / `HTTPS_PROXY`):** `reqwest` varsayılan olarak ortam değişkenlerindeki vekil sunucuları okur; vekil kullanıldığında DNS çözümlemesi vekile devredilir ve yerel IP denetimi atlanır.

### 1.3 Uygulanan Güvenlik Sınırları (`crates/kalfa-tools/src/web/ag.rs` ve `izin.rs`)

- **Vekil Kapatma:** `reqwest::ClientBuilder::no_proxy()` ile ortam ve sistem vekilleri koşulsuz devre dışı bırakıldı.
- **Tüm DNS Kayıtlarının Denetimi ve IP Sabitleme:** Her adımda özel `Cozucu` (`kalfa_sandbox::vekil::Cozucu`) ile alan adının döndürdüğü **tüm** `SocketAddr` kayıtları `adres_denetle` süzgecinden geçirilir. Kayıtlardan biri bile yasaklıysa bağlantı başlamadan reddedilir (`WebGetirHatasi::YasakliAdres`). Doğrulanan soket listesi `resolve_to_addrs(host, &dogrulanmis_soketler)` ile o adımın istemcisine sabitlenir (DNS rebinding TOCTOU engeli).
- **Gerçek Bağlanılan Soketin Doğrulanması:** Bağlantı kurulduktan sonra `yanit.remote_addr()` tekrar `soket_denetle` ve `dogrulanmis_soketler.contains(&soket)` ile doğrulanır.
- **Gömülü IPv6 Denetimi:** `adres_denetle`, `kalfa_sandbox::vekil::adres_izinli_mi(ip, false)` kontrolüne ek olarak IPv4-mapped (`::ffff:0:0/96`), IPv4-compatible, NAT64 (`64:ff9b::/96`, `64:ff9b:1::/48`), 6to4 (`2002::/16`) ve Teredo (`2001:0000::/32`) adreslerinin içindeki gömülü IPv4 adreslerini ve AWS IPv6 IMDS (`fd00:ec2::254`) adresini de denetler.
- **Yönlendirme Politikası:** Otomatik yönlendirme kapalıdır (`redirect::Policy::none()`). En çok `5` yönlendirme (`AZAMI_YONLENDIRME`) elle izlenir. Her adımda:
  - Şema (`http`/`https`) ve URL içi kimlik bilgisi (`kullanici:parola@`) denetlenir.
  - `https` kaynağından `http` hedefine düşüş (`HttpsHttpDususu`) koşulsuz reddedilir.
  - Varsayılan `YonlendirmeIzni::AyniHost` politikasıyla farklı host'a yönlendirme (`FarkliHostYonlendirmesi`) reddedilir.
- **İzin ve Mühür Sözleşmesi (`crates/kalfa-tools/src/web/izin.rs`):**
  - `Kip::Inceleme` kipinde `web_getir` yasaktır ve araç listesinde modele sunulmaz (`yazar_mi() == true`).
  - `Kip::Siki`, `Kip::Proje` ve `Kip::Serbest` kiplerinde daha önce onaylanmamış her `(host, port)` hedefi `Karar::YerelOnayli` döndürür (`Serbest` kip ilk dış host erişimini otomatik onaylamaz).
  - `web_muhur_ozeti(&WebTarifi)` (`SHA-256`), kanonik URL, host, port ve yönlendirme iznini mühürler. `Hazirlik.eklenti` üzerinden `kalfa_core::onay::muhur_hesapla` içine bağlanır; onaydan sonra URL değiştirilirse `AracHatasi::OnayEskidi` döner.

---

## 2. Görev 2 — Sınırlı HTTP, Akışlı Sıkıştırma ve HTML/Metin Çıkarımı

### 2.1 Bağımlılık Kararı: Sıfır Yeni Crate, Durum Makineli Ayrıştırıcı

Planda HTML ayrıştırması için regex kullanılmaması ve yeni bağımlılık (`dom_query` / `html5ever`) gerekiyorsa önce gerekçelendirilmesi istenmiştir. Değerlendirme sonucunda:

- `html5ever` / `dom_query` ağacı çalışma alanına onlarca geçişli crate eklemekte ve `cargo audit` yüzeyini genişletmektedir.
- Bunun yerine `crates/kalfa-tools/src/web/html.rs` içinde **sıfır yeni bağımlılıkla**, regex kullanmayan, karakter/durum makineli (state-machine) bir HTML ayrıştırıcı ve `crates/kalfa-tools/src/web/sikistirma.rs` içinde RFC 1951 (DEFLATE) / RFC 1952 (GZIP) / RFC 1950 (ZLIB) akış açıcısı yazılmıştır.
- `html.rs`:
  - `<script>`, `<style>`, `<noscript>`, `<template>`, `<svg>`, `<iframe/object/embed>` ve `<!-- ... -->` yorumlarını ham veri durumlarıyla (tırnak içi `>` karakterlerini de doğru izleyerek) tamamen atar.
  - `<title>`, `<meta charset>` / `<meta http-equiv="content-type">`, `<h1>`–`<h6>` başlıklarını (`id` öznitelikleriyle), `<a href>`, `<pre>/<code>`, `<li>`, `<table>/<tr>/<td>` yapılarını ve HTML karakter varlıklarını (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#...;`, `&#x...;`) okunabilir düz metne dönüştürür.
  - `bolum` parametresi verildiğinde başlık metni veya `id` eşleşmesiyle yalnızca ilgili bölümü (alt başlıklarıyla birlikte) döndürür.

### 2.2 Boyut, Süre ve İmleç Sınırları (`crates/kalfa-tools/src/web/metin.rs`)

- **Toplam Zaman Aşımı:** 30 saniye (`TOPLAM_ZAMAN_ASIMI`), bağlantı zaman aşımı 10 saniye (`BAGLANTI_ZAMAN_ASIMI`). Yavaş/sonsuz akışlar `WebGetirHatasi::ZamanAsimi` ile kesilir (`W05`).
- **Açılmış Gövde Sınırı:** 4 MiB (`AZAMI_ACILMIS_GOVDE_BAYT = 4 * 1024 * 1024`). Gzip/deflate akışı açılırken sınır aşılırsa tüm gövde belleğe alınmadan `SikistirmaHatasi::AcilmisBoyutAsildi` ile durdurulur (`W05`).
- **Çıktı Sınırı ve Anlık Görüntü İmleci:** Modele verilen metin varsayılan olarak en çok 64 KiB (`AZAMI_CIKTI_BAYT = 65_536`) yer kaplar ve UTF-8 karakter/satır sınırında kırpılır. Kırpıldığında `web1:<sha256_16>:<satir_indisi>` biçiminde devam imleci üretilir. Sayfa içeriği sonraki çağrıda değişmişse özet uyuşmazlığı yakalanır ve `MetinCikarimHatasi::ImlecSayfaDegisti` döner (`W02`).
- **İkili İçerik Reddi:** Yalnızca metin tabanlı `Content-Type` değerleri kabul edilir; ayrıca PDF (`%PDF-`), PNG, JPEG, GIF, ZIP, WASM, ELF, PE (`MZ`) sihirli baytları ve NUL bayt / yüksek kontrol karakteri oranı `MetinCikarimHatasi::IkiliIcerikSaptandi` ile reddedilir (`W04`).

---

## 3. Görev 3 — Araç, Kaynak Zarflaması, Sızıntı Önleme ve Arayüz

- **Dış İçerik Kaynağı (`crates/kalfa-tools/src/web/arac.rs`):** `WebGetirAraci::calistir` çıktısı her zaman `kalfa_uretimi: false` ve `kaynak: "web:<maskeli_son_url>"` taşır. Bu sayede `kalfa_core::arac::denetim_durumu` çıktıyı otomatik olarak `DenetimGereksinimi::Denetlenecek` sınıfına sokar ve gönderim önizlemesinde dış kaynak olarak işaretler.
- **Hassas Parametre Maskeleme ve Sızıntı Önleme (`W06`):**
  - `url_maskele`, sorgu parametrelerindeki `api_key`, `token`, `secret`, `password`, `sig`, `auth` vb. anahtarları ve yüksek entropili jeton görünümlü değerleri `[MASKELENDI]` ile maskeler.
  - `gizli_veri_sizintisi_kontrolu`, dış sayfadan gelen prompt injection'ın yerel `.env` veya `EVREN_API_KEY` (`evr-...`, `sk-...`, `ghp_...`, `AKIA...`, `eyJ...`) değerlerini URL üzerinden dışarı sızdırma girişimini daha `hazirla()` aşamasında `AracHatasi::GecersizArguman` ile durdurur; hedef sunucuya sıfır istek gider (`W06`).
- **Arayüz (`ui/src/bilesenler/WebKaynakKarti.svelte`):**
  - Kaynak URL, yönlendirme sonrası son URL, sayfa başlığı, çıktı/kaynak bayt boyutu, kırpılma durumu ve devam imlecini gösterir.
  - Ham HTML (`{@html}`) ve sabit `style="…"` özniteliği kullanmaz (CSP uyumu).
  - Harici bağlantıyı otomatik açmaz; yalnızca `http:` / `https:` şemalı ve kimlik bilgisiz URL'lerde iki adımlı kullanıcı onayı (`Harici bağlantıyı aç` → `Tarayıcıda aç` / `Vazgeç`) sonrasında geri çağırımı tetikler.
- **Ortak Çekirdek / IPC Birleşme Sırası Notu:**
  - Koordinasyon planındaki `#38 → #39 → #40` birleşme sırasına uyularak `kalfa-core`, `kalfa-uygulama`, `crates/kalfa`, `ui/src/ipc/tipler.ts` ve `ui/src/dil/tr.ts` dosyaları bu PR'da değiştirilmemiştir. `#40` birleştikten sonra `WebGetirAraci` tek satırla `varsayilan_araclar` listesine eklenecektir.

---

## 4. Görev 4 — Arama Sağlayıcısı Seçenekleri ve Kullanıcı Kararı (`G4b`)

Plan kuralları gereği kullanıcı onayı olmadan dış serviste hesap açılmamış, satın alma yapılmamış ve MCP arama aracıyla çakışacak ikinci bir yerleşik arama aracı tanımlanmamıştır. Uzak MCP üzerinden web araması için resmi belgeleri incelenen iki aday aşağıdadır:

| Ölçüt | Aday 1 (Önerilen): **Brave Search MCP** | Aday 2: **Exa MCP** |
|---|---|---|
| **Resmi Belgeler** | [Brave Search API Docs](https://api-dashboard.search.brave.com/app/documentation/web-search/get-started), [MCP `brave-search` sunucusu](https://github.com/modelcontextprotocol/servers/tree/main/src/brave-search) | [Exa MCP Docs](https://docs.exa.ai/reference/exa-mcp), [`exa-mcp-server`](https://github.com/exa-labs/exa-mcp-server) |
| **Taşıma / Kurulum** | Yerel stdio MCP (`BRAVE_API_KEY` ile) veya `#40` G3b uzak HTTP MCP sarmalayıcısı | Doğrudan uzak Streamable HTTP MCP (`https://mcp.exa.ai/mcp`) veya yerel stdio (`EXA_API_KEY`) |
| **Hesap / Ücretsiz Kota / Fiyat** | Ücretsiz plan: 2.000 sorgu/ay (1 sorgu/sn); ücretli Base plan: \$3–\$5 / 1.000 sorgu. Kayıtta kart doğrulaması isteyebilir. | Kayıtta tek seferlik başlangıç deneme kredisi; sonrasında kullanım başına ücretli. |
| **Veri Saklama Politikası** | Sorgular reklam profillemesi için kullanılmaz; bağımsız web indeksine sahiptir. | İstekler faturalama ve güvenlik için günlüklenir; kurumsal ZDR ayrı sözleşme ister. |
| **Kalfa Mimarisiyle Uyum** | Yalnızca başlık, URL ve kısa özet (`title`, `url`, `description`) döndürür; tam sayfa okumayı Kalfa'nın güvenli `web_getir` aracına bırakır. | Tam metin kazıma özelliği vardır ancak Kalfa'da güvenlik sınırları için yalnızca başlık/URL/özet modu kullanılmalıdır. |

### Öneri ve Beklenen Karar

1. **Öneri:** Bağımsız arama indeksi, aylık yinelenen ücretsiz kotası (2.000 sorgu/ay) ve yalnızca başlık/URL/özet döndürüp sayfa okumayı Kalfa'nın `web_getir` güvenlik katmanına bırakması nedeniyle **Brave Search MCP** (veya doğrudan uzak HTTP MCP uç noktası tercih edilirse **Exa MCP**) önerilir.
2. **Açık Bağımlılıklar:**
   - **Hat 04 (`#40`, G3b):** Uzak HTTP MCP istemci ve kimlik doğrulama desteğinin birleşmesi.
   - **Kullanıcı Kararı:** Brave Search MCP mi yoksa Exa MCP mi kullanılacağının ve API anahtarının kullanıcı tarafından sağlanmasının onayı.
3. **Hazır Katman (`crates/kalfa-tools/src/web/arama.rs`):**
   - MCP arama aracından dönen JSON yanıtlarını (`results` / `web.results`) ayrıştıran `mcp_arama_json_coz`,
   - Hata durumlarını (`IptalEdildi`, `HizSiniri`, `YetersizBakiyeVeyaKimlik`, `BosSonuc`, `GecersizSonucUrl`) sınıflandıran `arama_sonuclarini_dogrula`,
   - Seçilen sonuçları `WebIstemci` + `yaniti_metne_donustur` ile okuyan `arama_kaynaklarini_oku`,
   - Yanıttaki iddiaların gerçekten okunan sayfalardaki alıntılara dayandığını ve karşılaştırma sorularında en az 2 farklı kaynağa (`W03`) atıf yapıldığını doğrulayan `kaynakli_yanit_dogrula` fonksiyonları tamamlanmış ve yerel HTTPS fixture'larıyla doğrulanmıştır.
