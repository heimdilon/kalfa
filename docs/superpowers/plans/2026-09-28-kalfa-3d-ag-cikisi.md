# Kalfa 3d — Ağ çıkışının kısıtlanması: Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ağlı komut yalnızca izin listesindeki hostlara çıkabilsin (güvenlik
analizi Bulgu 8). Bugün `ag: true` onaylanınca komut her hosta ulaşır. Bu
planın sonunda:
- **Tek çıkış yolu Kalfa'nın vekilidir.** Vekil hedefi sınırlar, içeriği
  değil: TLS açılmaz.
- **İzin listesi:** varsayılan liste (paket kayıtları ve GitHub indirmeleri),
  kullanıcının kalıcı listesi ve çağrıya özel `ag_hostlari`.
- **Linux'ta** ağlı profil de ağ ad alanını ayırır. Kabın içindeki köprü
  bağlantıları vekile verir.
- **Windows'ta** biçimi G6'nın ölçümü belirler: köprü ya da yedek yol (K6).
- **Model engeli görür:** listede olmayan host `403` alır ve sonuca "[ağ] …
  engellendi" satırı eklenir.

**Architecture:**
- **Vekil** (`kalfa-sandbox/src/vekil/`, tokio):
  - Kalfa sürecinde, komut başına bir tane çalışır.
  - `CONNECT` ve mutlak biçimli HTTP isteklerini okur. İzin listesini, adres
    denetimini ve SNI denetimini uygular, sonra baytları taşır.
  - Her bağlantıyı komutun ağ özetine yazar.
- **İzin listesi:** onaylanan tariften kurulur ve komut boyunca değişmez.
  Özeti yalıtım profilinin özetine, dolayısıyla mühre girer.
- **Linux:**
  - bwrap her profilde `--unshare-net` alır; ağlı profilde `resolv.conf`
    bağlanmaz.
  - Komut `kalfa-kopru` ile başlar. Köprü kabın `127.0.0.1`'inde dinler ve
    kabul ettiği bağlantının tanımlayıcısını `SCM_RIGHTS` ile vekile verir.
- **Çekirdek ve araç:**
  - `KabukTarifi.ag_hostlari` mühre girer.
  - Onay kartının ağ satırı listeyi gösterir.
  - Kabuk aracı `ag_hostlari`'yı doğrular ve sonuca `[ağ]` satırlarını ekler.

**Tech Stack:** Rust 1.98.1 (tokio `net`, libc), Svelte 5 + TypeScript,
Vitest, `tauri-driver` + WebdriverIO.

**Kaynaklar:**
- [Aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §2 (K4–K6),
  §4.1, §8, §10, §11.3, §13 (G5, G6, G7, G16, G17, G18).
- [G5 bulguları](../bulgular/2026-09-28-kalfa-3d-g5-linux-kopru.md) ve
  [prototip](../../../prototip/linux-kopru/README.md).
- [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §6.1
  (ortam), §7.1 (profil özeti), §8 (Linux), §9 (Windows), §12.3 (uyumluluk
  matrisi).
- [Üst belge](../specs/2026-09-21-kalfa-design.md) §7.5–§7.7 (r3.9).

**Doğrulama durumu:**
- **Hafif plan:** görevleri, arayüzleri ve önce yazılacak testleri verir.
  Kodu önceden yazılıp doğrulanmadı.
- **G5'in Linux kısmı ölçüldü** (Görev 1, bu PR). Tanımlayıcı taşıma
  çalışıyor, araçlar vekile uyuyor. bwrap'la kanal geçişi Görev 4'ün ilk
  adımıdır.
- **G6** (Windows) ölçülmedi; Görev 8'in ilk adımıdır.
- **Uygulayan:** Antigravity (kullanıcının kararı, 2026-10-05). Gerçek yalıtım
  testlerini ve iki platform doğrulamasını Claude yapar.

**Kapsam dışı:**
- Kullanıcının kalıcı listesinin arayüzü ve `ayarlar.json`'daki yeri (3e).
  3d listeyi bir parametre olarak alır; 3e onu ayardan okuyup arayüzle bağlar.
- Kurumsal vekil zinciri (G16). Desteklenmediği, ileti ile söylenir (Görev 6).
- TLS'i açıp içeriğe bakmak; yükleme uçlarını ayırt etmek.

## Kararlar

Kullanıcının kararları (tasarım §2.1):

1. **Ek host çağrıda istenir (K4):** `kabuk` aracının `ag_hostlari`
   parametresi. Host mühre ve karta girer.
2. **`github.com` varsayılan listede değildir (K5).**
3. **Windows'ta G6 olumsuzsa yedek yol (K6):** ağlı komut bugünkü gibi
   `internetClient` ile çalışır. Kart "Ağ: sınırsız (Windows'ta vekil yok)"
   der.
4. **Go'nun modül uçları varsayılan listede değildir (G18, 2026-10-05).**
   - `proxy.golang.org` önbelleğinde olmayan modülü, modülün kendi alan
     adından kendisi çeker. `proxy.golang.org/saldirgan.com/<veri>/@v/list`
     gibi bir istek veriyi saldırganın sunucusuna taşır. `sum.golang.org`
     de aynı yolla çalışır.
   - Gerekçe `github.com`'unkiyle (K5) aynıdır ve burada daha güçlüdür.
   - Go kullanan model bu uçları çağrıda `ag_hostlari` ile ister.

Bu plan yazılırken kullanıcıya kalan:

- **Uygulayan:** Antigravity (2026-10-05).

## Global Constraints

- **Araçlar:** Rust 1.98.1; `cargo` komutları `--locked` ile. Node
  `^22.22.2 || >=24.15.0`; kurulum `npm ci`.
- **Bağımlılık:**
  - Workspace'e yeni crate girmez. Köprü `kalfa-sandbox`'ın bir ikilisidir
    (`src/bin/kalfa-kopru.rs`, tasarım §4.1).
  - Workspace'teki tokio'ya `net` özelliği eklenir. Başka özellik ve paket
    eklenmez; `ui/`'ye paket eklenmez.
- **`unsafe`:** yalnızca şu dosyalarda, her blok `// SAFETY:` ile:
  - `kalfa-sandbox/src/vekil/kanal.rs`: `recvmsg`, `SCM_RIGHTS`, `fstat`,
    `getsockopt`, `getpeername`.
  - `kalfa-sandbox/src/bin/kalfa-kopru.rs`: `sendmsg`, `fcntl`.
  - `kalfa-sandbox/src/linux/mod.rs`: var olan `pre_exec`'e kanal eklenir;
    `socketpair`.
  - Windows'taki dosyalar [Windows alt planında](2026-10-05-kalfa-3d-windows-kopru.md)
    (Global Constraints).
- **Güvenlik sınırları:**
  - Liste bir izin listesidir. Host adı tam eşleşir; joker yoktur.
  - **Adres denetimi:**
    - Her zaman yasak: geri döngü, bağlantı yerel (bulut meta veri adresi
      dahil), çok noktaya yayın ve belirtilmemiş adresler.
    - Özel ağ (`10/8`, `172.16/12`, `192.168/16`, `100.64/10`, `fc00::/7`)
      yalnızca "özel ağ" işaretli kullanıcı hostuna açıktır.
    - IPv4'e eşlenmiş IPv6 adresi (`::ffff:a.b.c.d`) IPv4 olarak denetlenir.
    - Ad bir kez çözülür; bağlantı denetlenen adrese kurulur.
    - IP adresiyle `CONNECT` yalnızca listede IP olarak yazılmışsa geçer.
  - **SNI:** tünelin ilk kaydı ClientHello ise SNI `CONNECT` hostuyla aynı
    olmalıdır.
  - **Kanal:** kanaldan gelen tanımlayıcı, bağlı bir TCP soketi değilse
    kapatılır. Köprü güvenilir kod sayılmaz; sınır vekildir.
  - **Tarife bağlılık:** araç onaylanan tarife göre çalışır. Vekil onaylanan
    listeyle başlar; komut sürerken listeye host eklenmez. Liste ya da bir
    işaret değişirse bekleyen onay geçersizleşir.
  - **Yalıtım yoksa:** ağlı yalıtımlı komut köprü, vekil ya da `--unshare-net`
    olmadan başlamaz; yalıtımsız çalışmaya düşmez.
  - Test edilmeyen güvenlik sınırı yoktur; her görev kendi sınırının testini
    getirir.
  - Testler internete bağlı değildir: vekile test için bir çözümleyici verilir,
    izinli host yerel bir sahte sunucuya çözülür.
- **Dil:**
  - Kod, test ve açıklamalar Türkçedir.
  - Kullanıcıya görünen metinler `ui/src/dil/tr.ts`'te ya da Rust'ta sabittir
    ve "siz" diliyle yazılır. Arayüzde "yalıtım" denir.
  - `unwrap()` ve `expect()` yalnızca testlerde ve `test_destek.rs`'de
    kullanılır.
- **Arayüz:** CSP sıkıdır; sabit `style="…"` kullanılmaz, ham HTML basılmaz.
  Pencerelere Tauri'nin çekirdek izinleri verilmez.
- **Protokol:**
  - Yeni alanlar `#[serde(default)]` taşır. Eski kayıtların açılabildiğini bir
    test sınar.
  - `ui/src/ipc/tipler.ts` aynı commit'te güncellenir.
- **Her görevin sonunda:**
  - `cargo test --workspace --locked`
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`
  - `cargo fmt --all -- --check`
  - `kalfa-uygulama` ya da `kalfa`'ya dokunulduysa:
    `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked` ve aynı
    özellikle clippy.
  - `kalfa-sandbox` ya da `kalfa-tools`'a dokunulduysa, root olmayan
    kullanıcıyla: yalıtım clippy'si,
    `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim`
    ve `cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk`.
  - Arayüze dokunulduysa `ui/` içinde: `npm run check` (0 hata, 0 uyarı),
    `npm test`, `npm run build`.
  - Görev, Windows'ta ve WSL'de (Ubuntu-24.04) yeşil olmadan bitmiş sayılmaz.
    Görev 4 yalnızca Linux'tur; Görev 8 yalnızca Windows'tur.
- **Commit:** Türkçe ve önekli (`sandbox:`, `çekirdek:`, `araçlar:`,
  `uygulama:`, `ui:`, `e2e:`, `docs:`). Claude'un commit'leri
  `Co-Authored-By` satırı taşır; Antigravity'ninkiler taşımaz.
- **Dokunulmayanlar:** `design/`, `.env`, `Screenshot*.png`, `refs/shot-*.png`,
  kökteki `tasks.png`.

## Dosya Yapısı

```
crates/kalfa-sandbox/src/vekil/mod.rs       yeni: Vekil, VekilAyari, AgOzeti; bağlantı döngüsü
crates/kalfa-sandbox/src/vekil/liste.rs     yeni: VARSAYILAN_HOSTLAR, HostKurali, IzinListesi, ag_hostu_coz
crates/kalfa-sandbox/src/vekil/adres.rs     yeni: adres denetimi (yasak aralıklar, özel ağ)
crates/kalfa-sandbox/src/vekil/istek.rs     yeni: CONNECT ve mutlak biçimli istek ayrıştırıcısı
crates/kalfa-sandbox/src/vekil/sni.rs       yeni: ClientHello'dan SNI
crates/kalfa-sandbox/src/vekil/kanal.rs     yeni (Linux): SCM_RIGHTS ile alma ve tanımlayıcı denetimi
crates/kalfa-sandbox/src/bin/kalfa-kopru.rs yeni: kabın içindeki köprü
crates/kalfa-sandbox/src/linux/{mod,bwrap,seccomp}.rs   --unshare-net, köprü, kanal, pidfd_getfd
crates/kalfa-sandbox/src/{lib,profil}.rs    CalistirmaIstegi'ne liste; profil özetine listenin özeti
crates/kalfa-sandbox/src/windows/           Görev 8: köprü (Windows alt planı)
crates/kalfa-core/src/{kabuk,izin,protokol,tur_ozeti}.rs  ag_hostlari, kartın ağ satırı, komutun ağ özeti
crates/kalfa-tools/src/kabuk/mod.rs         ag_hostlari'nın doğrulanması, açıklama, [ağ] satırları
crates/kalfa-uygulama/src/                  kullanıcı listesinin parametresi (boş; 3e bağlar)
ui/src/ana/{OnaySayfasi,TurOzeti}.svelte, ui/src/{ipc/tipler.ts,dil/tr.ts}
e2e/testler/                                S13
docs/superpowers/specs/*, docs/superpowers/bulgular/*, docs/GELISTIRME_DURUMU.md
```

---

### Task 1: G5 prototipi ve ölçümü

**Durum:** yapıldı (bu PR).

- [x] `prototip/linux-kopru/`: köprü, vekilin sahtesi ve ölçüm düzeneği;
  `unshare` ile.
- [x] Ölçüm: tanımlayıcı taşıma, ad alanı, yaşam, soket kuralı, araçların
  uyumu, yapılandırma önceliği, ClientHello, maliyet.
- [x] Sonuçlar: [G5 bulguları](../bulgular/2026-09-28-kalfa-3d-g5-linux-kopru.md).
  Tasarım §8.2, §8.4, §8.5 ve §13 güncellendi.

### Task 2: İzin listesi ve adres denetimi

**Files:**
- `crates/kalfa-sandbox/src/vekil/{mod,liste,adres}.rs` (yeni),
  `crates/kalfa-sandbox/src/lib.rs`
- Testler: `crates/kalfa-sandbox/tests/vekil_liste.rs` (yeni)

**Interfaces (Produces):**
- **`VARSAYILAN_HOSTLAR: &[&str]`:** tasarım §8.3'teki sekiz host, port 443.
- **`HostKaynagi { Varsayilan, Kullanici, Cagri }`**,
  **`HostKurali { host: String, port: u16, ozel_ag: bool, kaynak: HostKaynagi }`**.
- **`ag_hostu_coz(s: &str) -> Result<(String, u16), ListeHatasi>`:**
  - `host` ya da `host:port`; port verilmezse 443.
  - Host küçük harfe çevrilir. Yalnızca ASCII kabul edilir: harf, rakam,
    `-` ve `.`. Punycode (`xn--`) geçer.
  - `*`, boş etiket, 253 karakterden uzun ad ve `0` port reddedilir.
  - IPv4 ve köşeli parantezli IPv6 kabul edilir.
- **`IzinListesi`:**
  - `yeni(kullanici: &[HostKurali], cagri: &[(String, u16)]) -> IzinListesi`.
    Varsayılan listeyi kendisi ekler. Çağrıya özel hostlarda `ozel_ag`
    her zaman `false`'tur.
  - `bul(&self, host: &str, port: u16) -> Option<&HostKurali>`: tam eşleşme.
  - `ozet(&self) -> String`: sıralı, kararlı. Host, port, `ozel_ag` ve kaynağı
    içerir; profil özetine girer.
- **`adres_izinli_mi(ip: IpAddr, ozel_ag: bool) -> Result<(), AdresReddi>`**,
  **`AdresReddi { Yerel, OzelAg }`.**

- [ ] **Step 1: Başarısız testleri yaz**
  - `varsayilan_liste_tasarimdaki_hostlar`: sekiz host; `github.com`,
    `api.github.com`, `crates.io` ve `upload.pypi.org` listede değil.
  - `host_tam_eslesir`: `evil-index.crates.io`, `index.crates.io.evil.com` ve
    `INDEX.CRATES.IO:443` (küçük harfe çevrildikten sonra eşleşir) durumları.
  - `port_varsayilani_443_ve_baska_port_eslesmez`.
  - `gecersiz_host_reddedilir`: `*.github.com`, `a..b`, 254 karakterlik ad,
    `host:0`, `host:99999`, ASCII olmayan ad, boş metin.
  - `yasak_adresler`: `127.0.0.1`, `::1`, `169.254.169.254`, `fe80::1`,
    `224.0.0.1`, `ff02::1`, `0.0.0.0`, `::`. `ozel_ag: true` olsa da yasak.
  - `ozel_ag_yalnizca_isaretle`: `10.1.2.3`, `172.16.0.1`, `192.168.1.1`,
    `100.64.0.1` ve `fd00::1` işaretsiz yasak, işaretli izinli.
  - `ipv4e_eslenmis_ipv6_ipv4_gibi_denetlenir`: `::ffff:127.0.0.1` ve
    `::ffff:10.0.0.1`.
  - `cagri_hostu_ozel_aga_isaretlenemez`.
  - `ozet_kararli_ve_liste_degisince_degisir`: sıra farkı özeti değiştirmez;
    host, port ya da işaret değişince özet değişir.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula.
- [ ] **Step 4:** Global Constraints'teki denetimler.
- [ ] **Step 5:** Commit `sandbox: ağ izin listesi ve adres denetimi`.

### Task 3: Vekil

**Files:**
- `crates/kalfa-sandbox/src/vekil/{mod,istek,sni}.rs`, `Cargo.toml` (tokio
  `net`)
- Testler: `crates/kalfa-sandbox/tests/vekil.rs` (yeni)

**Interfaces (Produces):**
- **`trait Cozucu: Send + Sync`:**
  `async fn coz(&self, host: &str, port: u16) -> io::Result<Vec<SocketAddr>>`.
  Gerçeği `tokio::net::lookup_host`'tur. Testlerde sahte çözümleyici hostu
  yerel bir sunucuya çözer.
- **`VekilAyari { liste: IzinListesi, cozucu: Arc<dyn Cozucu>, azami_baglanti: usize (64), bosta_kalma: Duration (5 dk) }`.**
- **`Vekil::baslat(ayar) -> Vekil`**,
  `baglanti_ver(&self, akis: impl AsyncRead + AsyncWrite + Unpin + Send + 'static)`.
  Linux'ta akış köprünün verdiği `TcpStream`'dir, Windows'ta Kalfa'nın boru
  ucu (Windows alt planı, W1). Vekil akışın türüne bakmaz.
  
  `bitir(self) -> AgOzeti`. `bitir` açık bağlantıları kapatır.
- **`AgOzeti { baglantilar: Vec<AgBaglantisi> }`**,
  **`AgBaglantisi { host, port, karar: AgKarari, giden: u64, gelen: u64 }`**.
  `AgKarari`:
  - `Izinli`
  - `ListedeYok`
  - `IpIleIstek`
  - `YasakAdres(AdresReddi)`
  - `SniUyusmuyor { sni: Option<String> }`
  - `Cozulemedi(String)`
  - `Baglanamadi(String)`
  - `Gecersiz(String)`
- **`istek.rs`:** `CONNECT host:port HTTP/1.0|1.1` ve `GET http://host[:port]/…`.
  Baş en çok 8 KiB, 10 saniye. Başka yöntem ya da biçim `400` alır.
- **`sni.rs`:** tünelin ilk kaydı bir TLS el sıkışmasıysa ClientHello'dan
  SNI çıkarılır (en çok 16 KiB). SNI yoksa ya da `CONNECT` hostundan farklıysa
  bağlantı kesilir. İlk kayıt TLS değilse akış olduğu gibi taşınır.

- [ ] **Step 1: Başarısız testleri yaz** (hepsi yerel sunucularla)
  - `izinli_host_aktarilir`: `CONNECT` → `200`; baytlar iki yönde;
    `AgOzeti`'nde giden ve gelen bayt.
  - `http_1_0_connect_kabul_edilir`.
  - `listede_olmayan_host_403_alir`: yanıt `403`, gövde hostu söyler; çözümleyici
    hiç çağrılmaz.
  - `ip_ile_connect_reddedilir` ve `listede_ip_olarak_yazilan_adres_gecer`.
  - `yasak_adrese_cozulen_host_baglanmaz`: çözümleyici `127.0.0.1` ve
    `10.0.0.1` döndürür; biri yasaksa hiçbirine bağlanılmaz.
  - `ad_bir_kez_cozulur`: sayan çözümleyici; bağlantı dönen adrese kurulur.
  - `sni_farkliysa_kesilir`, `sni_yoksa_kesilir`, `tls_olmayan_akis_tasinir`.
  - `mutlak_http_yalnizca_80_listedeyse_gecer`.
  - `azami_baglanti_asilinca_yeni_baglanti_kapanir`.
  - `bosta_kalan_baglanti_kapanir` (testte kısa süreyle).
  - `bitir_acik_baglantilari_kapatir`.
- [ ] **Step 2:** Testlerin başarısız olduğunu gör.
- [ ] **Step 3:** Uygula. Prototipin `tls.rs`'i ve `vekil.rs`'i başlangıç
  olabilir; ikisi de ölçüm içindi, ürün kuralları bu görevdedir.
- [ ] **Step 4:** Global Constraints'teki denetimler.
- [ ] **Step 5:** Commit `sandbox: çıkış vekili`.

### Task 4: Linux — köprü, kanal ve `--unshare-net`

**Files:**
- `crates/kalfa-sandbox/src/bin/kalfa-kopru.rs` (yeni),
  `crates/kalfa-sandbox/src/vekil/kanal.rs` (yeni)
- `crates/kalfa-sandbox/src/linux/{mod,bwrap,seccomp}.rs`,
  `crates/kalfa-sandbox/src/{lib,profil}.rs`
- Testler: `crates/kalfa-sandbox/tests/yalitim.rs` (ağlı senaryolar),
  `bwrap.rs`'in birim testleri

**Önce ölçüm (G5'in kalanı, WSL'de `kalfatest` ile):**
- [x] bwrap `--unshare-net` ile, kanal ucu `CLOEXEC`'siz verilince tanımlayıcı
  kaptaki köprüye geçiyor mu? Prototipin köprüsü bwrap'ın içinde
  çalıştırılarak ölçülür. Kalfa'nın gerçek seccomp filtresi ve `prlimit`
  sarmalayıcısı da ölçüme dahildir.
- [x] bwrap `lo`'yu açıyor mu (köprü `--lo-ac`'sız dinleyebiliyor mu)?
- [x] Sonuç bulgu raporuna eklenir. Olumsuzsa görev durur; gerekçe PR'a
  yazılır ve tasarım yeniden konuşulur.

**Sonuç (2026-10-05): olumlu.** Kanal bwrap'tan köprüye geçiyor, bwrap `lo`'yu
açıyor, köprü ölünce komut da ölüyor
([G5 bulguları, ek](../bulgular/2026-09-28-kalfa-3d-g5-linux-kopru.md#ek-bwrapla-ölçüm-görev-4ün-ön-ölçümü-2026-10-05)).
Ek bulgu: devredilmeyen tanımlayıcılar da kaba geçiyor; aşağıdaki
`linux::komut_kur` maddesine bakın.

**Interfaces:**
- **Köprü** (`kalfa-kopru --kanal <FD> -- <program> …`):
  - `127.0.0.1:0`'da dinler.
  - Komuta şu ortamı ekler (tasarım §8.5):
    - `HTTPS_PROXY`, `HTTP_PROXY`, `https_proxy`, `http_proxy`, `ALL_PROXY`.
    - `NO_PROXY` ve `no_proxy`.
    - `NODE_USE_ENV_PROXY=1`.
    - `npm_config_https_proxy`, `npm_config_proxy`, `CARGO_HTTP_PROXY`,
      `PIP_PROXY`.
    - `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=http.proxy`,
      `GIT_CONFIG_VALUE_0`.
  - Kanalı ve dinleyiciyi `CLOEXEC` yapar.
  - Komutun çıkış koduyla çıkar; sinyalle ölürse `128 + sinyal`.
  - Yalnızca `std` ve `libc` kullanır.
- **`vekil/kanal.rs`:**
  - `kanal_cifti() -> io::Result<(OwnedFd, OwnedFd)>` (`SOCK_SEQPACKET`,
    `CLOEXEC`).
  - `al(fd) -> io::Result<Option<OwnedFd>>`: tek tanımlayıcılı olmayan ya
    da kesik ileti reddedilir.
  - `dogrula(&OwnedFd) -> Result<(), String>`: bağlı bir TCP soketi
    (tasarım §8.5).
- **`CalistirmaIstegi.ag_listesi: Option<IzinListesi>`.** `ag: true` ise
  zorunludur.
- **`Yalitici::profil_ozeti(ag, liste_ozeti: Option<&str>)`:** liste özeti
  profil özetine girer.
- **`bwrap::secenekler`:**
  - `--unshare-net` her profilde verilir. Ağlı profilde `resolv.conf`
    bağlanmaz.
  - Ağlı profilde `--ro-bind <kalfa-kopru> /run/kalfa-kopru` eklenir.
- **`linux::komut_kur`:** ağlıysa komut `/run/kalfa-kopru --kanal <N> --`
  ile başlar; `prlimit` köprüden sonradır. Kanalın ucu devredilen
  tanımlayıcılara eklenir. Vekil komutla başlar, komut bitince `bitir` ile
  kapanır.
- **İkilinin yeri:** Kalfa, `kalfa-kopru`'yu kendi ikilisinin dizininde arar.
  Bulamazsa ağlı yalıtımlı komut başlamaz ve ileti bunu söyler. Paketleme
  (Tauri `externalBin`) Görev 8'le birlikte yapılır.
- **seccomp:** `ENGELLI`'ye `SYS_pidfd_getfd` eklenir (tasarım §8.5).
- **Devredilmeyen tanımlayıcılar** (ön ölçümün b9'u): `pre_exec` önce
  `close_range(3, ~0, CLOSE_RANGE_CLOEXEC)` çağırır, sonra yalnızca
  devredilenlerin `CLOEXEC`'ini kaldırır. Ağsız profil için de geçerlidir.
  `close_range` yoksa (`ENOSYS`) 3'ten `RLIMIT_NOFILE`'a kadar `fcntl`
  döngüsü kullanılır.

- [x] **Step 1: Başarısız testleri yaz**
  - `bwrap.rs`: `her_profilde_unshare_net`,
    `agli_profilde_resolv_baglanmaz_kopru_baglanir`.
  - `seccomp.rs`: `pidfd_getfd_engelli`.
  - `kanal.rs`: `boru_ve_dosya_tanimlayicisi_reddedilir`,
    `dinleyen_ve_bagli_olmayan_soket_reddedilir`,
    `iki_tanimlayicili_ileti_reddedilir`, `bagli_tcp_soketi_kabul_edilir`.
  - `yalitim.rs` (gerçek bwrap, tasarım §11.3):
    - `agli_komut_izinli_hosta_vekilden_ulasir` (yerel sahte sunucu).
    - `izinsiz_host_403_alir_dogrudan_baglanti_ve_ad_cozumu_yok`.
    - `vekil_ozel_ag_adresine_baglanmaz` ve özel ağ işaretinin kuralları.
    - `kanal_ve_dinleyici_komuta_gecmez`.
    - `kopru_oldurulunce_komut_aga_cikamaz`.
    - `kabin_icindeki_sunucu_vekilsiz_calisir` (`NO_PROXY`).
    - `profil_ozeti_liste_degisince_degisir`.
    - `devredilmeyen_tanimlayici_kaba_gecmez`: test sürecinde `CLOEXEC`'siz
      açılan bir tanımlayıcı kaptaki `/proc/self/fd`'de görünmez.
  - Uyumluluk matrisi (`kalfa-tools/tests/uyumluluk.rs`): cargo, npm, pip,
    git ve curl yerel sahte vekile `CONNECT` gönderiyor (prototipin o7'si
    gibi; indirme yapılmaz).
- [x] **Step 2:** Testlerin başarısız olduğunu gör.
- [x] **Step 3:** Uygula.
- [x] **Step 4:** Global Constraints'teki denetimler (WSL, `kalfatest`).
- [x] **Step 5:** Commit `sandbox: Linux'ta ağlı komut vekilden çıkar`.

### Task 5: `ag_hostlari`, tarif, mühür ve kart

**Files:**
- `crates/kalfa-core/src/{kabuk,izin,protokol}.rs`
- `crates/kalfa-tools/src/kabuk/mod.rs`
- `ui/src/ana/OnaySayfasi.svelte`, `ui/src/ipc/tipler.ts`, `ui/src/dil/tr.ts`
- Testler: `crates/kalfa-core/tests/{izin,arac_muhur}.rs`,
  `crates/kalfa-tools/tests/kabuk.rs`, `ui/src/**/*.test.ts`

**Interfaces:**
- **`KabukTarifi.ag_hostlari: Vec<String>`** (`#[serde(default)]`,
  normalleştirilmiş `host:port`). Tarif bütünüyle mühürdedir.
- **Kabuk aracı:**
  - `ag_hostlari`: en çok 16 öğe, `ag_hostu_coz` ile doğrulanır. Yalnızca
    `ag: true` ile verilebilir; aksi hâlde `Gecersiz`. Tekrarlar ayıklanır.
  - Açıklama `ag_hostlari`'yı anlatır. `github.com`'un varsayılan listede
    olmadığını ve Git ile GitHub indirmeleri için istenmesi gerektiğini
    söyler.
- **Kartın ağ satırı** (`izin.rs`): "Ağ: açık; paket kayıtları (crates.io,
  npm, PyPI), GitHub indirmeleri ve şu adresler: …". Her ek host ayrı bir
  satırdır; özel ağ işaretli kullanıcı hostunun yanında "(özel ağ)" yazar.
  Yalıtımsızda "Ağ: sınırsız (yalıtımsız)" (tasarım §8.7).
- **Ağlı kabuk için kısayol yoktur** (üst belge §7.6); 3e'nin kısayolları bu
  kuralı korur.

- [x] **Step 1: Başarısız testleri yaz**
  - `ag_hostlari_ag_olmadan_gecersiz`, `on_altidan_fazla_host_gecersiz`,
    `joker_host_gecersiz`, `tekrarlanan_host_bir_kez`.
  - `ag_hostlari_muhre_girer`: tek host eklemek mührü değiştirir.
  - `eski_kayittaki_tarif_ag_hostlari_olmadan_acilir`.
  - `kartta_her_ek_host_ayri_satir`.
  - Arayüz: kart ağ satırını ve ek hostları gösterir.
- [x] **Step 2:** Testlerin başarısız olduğunu gör.
- [x] **Step 3:** Uygula.
- [x] **Step 4:** Global Constraints'teki denetimler.
- [x] **Step 5:** Commit `çekirdek: ag_hostlari mühre ve karta girer`.

### Task 6: Sonuçta `[ağ]` satırları ve tur özetinde ağ özeti

**Files:**
- `crates/kalfa-tools/src/kabuk/mod.rs`
- `crates/kalfa-core/src/tur_ozeti.rs`
- `ui/src/ana/TurOzeti.svelte`, `ui/src/dil/tr.ts`
- Testler: `crates/kalfa-tools/tests/kabuk.rs`, `crates/kalfa-core/tests/tur_ozeti.rs`

**Interfaces:**
- **Komutun sonucu:** `AgOzeti`'nden her reddedilen bağlantı için bir satır,
  modele de gider:
  - "[ağ] `evil.com:443` engellendi (izin listesinde yok)"
  - "[ağ] `x:443` engellendi: özel ya da yerel bir adrese çözüldü"
  - "[ağ] `x:443` engellendi: SNI farklı"
  - "[ağ] `x:443` çözülemedi" ya da "… bağlanılamadı". Kalfa'nın kendi
    ortamında `HTTPS_PROXY` varsa satır, kurumsal vekilin bu sürümde
    desteklenmediğini de söyler (G16).
  - Aynı host ve karar için tek satır, yanında sayı.
- **`KomutOzeti.ag: Option<AgKomutOzeti>`:** host başına bağlantı sayısı,
  bayt ve reddedilenler. Tur özetinde görünür (tasarım §8.2).

- [x] **Step 1: Başarısız testleri yaz**
  - `reddedilen_baglanti_sonuca_satir_ekler`, `ayni_ret_tek_satir`,
    `kurumsal_vekil_notu`.
  - `tur_ozetinde_komutun_ag_ozeti`.
- [x] **Step 2–5:** uygula, denetle, commit `araçlar: ağ özeti sonuçta ve tur
  özetinde`.

### Task 7: Uçtan uca (S13)

**Files:** `e2e/testler/` (Linux)

- [x] S13 (tasarım §7.1): ağlı komut izinsiz bir hosta bağlanmaya çalışıyor.
  Bağlantı reddedilir; sonuçta "[ağ] … engellendi" yazar.
- [x] İzinli host yerel sahte sunucuya çözülür; komut yanıtı alır ve tur
  özetinde ağ özeti görünür.
- [x] Commit `e2e: ağ çıkışı`.

### Task 8: Windows — G6 ve biçim

**Önce ölçüm (G6, kurulu makinede Claude):**
- [x] Aynı AppContainer'daki iki süreç `127.0.0.1` üzerinden konuşabiliyor mu?
- [x] Kap, paket SID'ine açılmış bir boruya bağlanabiliyor mu?
- [x] `internetClient`'sız kapta cargo, npm, pip ve git vekille çalışıyor mu?
  Schannel (A8) vekille de çalışmalı.
- [x] Köprünün ikilisi kapta okunabiliyor mu? Program Files'taki kurulumda ve
  geliştirme derlemesinde ayrı ayrı.
- [x] Sonuçlar `docs/superpowers/bulgular/`'a yazılır.

**Sonuç (2026-10-05): olumlu, köprü yolu.**
[G6 bulguları](../bulgular/2026-10-05-kalfa-3d-g6-windows-kopru.md).
Boru iki uçta da overlapped açılmalı. Köprünün ikilisi geliştirme
derlemesinde kaba okunur bir yere kopyalanmalı. Köprünün alt planı bu PR'a
eklenecek.

**Sonra köprü:** [Windows alt planı](2026-10-05-kalfa-3d-windows-kopru.md),
Görev W1–W6. Yedek yol (K6) gerekmedi.

### Task 9: Belgeler

- [x] `docs/GELISTIRME_DURUMU.md`: 3d'nin durumu, G5 ve G6'nın sonuçları,
  bilinen sınırlar (Java, ECH kullanan araçlar, kurumsal vekil).
- [x] Aşama 3 tasarımı §8 ve §13: ölçümlerin sonuçları.
- [x] Bu planın "Uygulama sonucu" bölümü.
- [x] Commit `docs: 3d'nin sonucu`.

## Kabul ölçütleri

- Linux'ta ağlı yalıtımlı komut yalnızca izin listesindeki hostlara, yalnızca
  vekil üzerinden çıkar. Doğrudan bağlantı ve ad çözümü yoktur.
- Listede olmayan host, IP ile istek, yasak adrese çözülen host ve SNI
  uyuşmazlığı reddedilir. Her birinin testi vardır.
- Kanaldan gelen, bağlı bir TCP soketi olmayan tanımlayıcı reddedilir.
- `ag_hostlari` mühre ve karta girer; liste değişirse bekleyen onay
  geçersizleşir.
- Sonuçta `[ağ]` satırları, tur özetinde ağ özeti vardır.
- Windows'ta G6'nın sonucu yazılmış ve seçilen dal uygulanmıştır.
- Uyumluluk matrisinde cargo, npm, pip, git ve curl vekille çalışır.
- Bütün denetimler Windows'ta ve WSL'de yeşildir.

## Uygulama sonucu

Dal `3d/ag-cikisi`, PR #20. Görev 1 ve 8'in ölçümünü Claude; Görev 2, 3, 5, 6
ve Windows alt planının W1–W6'sını Antigravity uyguladı. Görev 4, 7 ve 9'u
Claude bitirdi.

- **Görev 4 (Linux köprüsü):** plandaki arayüzle. Köprü `--kanal` ile
  başlar, kanalı ve dinleyiciyi `CLOEXEC` yapar; vekil kanaldan gelen
  tanımlayıcıyı `dogrula` ile denetler. `pre_exec` önce `close_range` ile
  bütün tanımlayıcıları kapatılacak işaretler, sonra yalnızca devredilenleri
  açar (ön ölçümün b9'u). `pidfd_getfd` seccomp'ta engellidir.
- **Görev 7 (S13):** `e2e/testler/09-ag-cikisi.test.mjs` iki platformda.
- **Uyumluluk matrisi:** Linux'ta curl, git, npm, Node (`fetch`), Python ve
  cargo vekile `CONNECT` gönderiyor. WSL'de pip kurulu değil; Python'un kendi
  istemcisi sınanır, pip vekili aynı değişkenlerden okur. Windows'ta cargo
  A15, sistem Python'unun pip'i A16 yüzünden düşer (Windows alt planı).
- **İnceleme (Claude, 2026-10-06).** Antigravity Görev 4'ün ortasında
  durdu; Claude devraldı. İncelemede bulunup düzeltilenler:
  - **Güvenlik:** e2e'nin sahte ad çözümü (`KALFA_E2E_SAHTE_DNS`) sürüm
    derlemesinde de okunuyordu. Değişken tanımlıysa vekil sahte çözümleyiciyi
    kullanıyor ve adres denetimini bütünüyle kapatıyordu (geri döngü, bulut
    meta veri adresi, özel ağ). Artık `kalfa-sandbox`'ın `e2e` özelliğinin
    arkasındadır; özelliği yalnızca `kalfa-uygulama/e2e` açar. Sürüm
    derlemesinde değişkenin yok sayıldığını bir test sınar.
  - **Windows ortamı:** köprü vekil değişkenlerini iki yazımla veriyordu
    (`HTTPS_PROXY` ve `https_proxy`). Windows'ta ad büyük/küçük harf ayırmaz;
    ortamda aynı ad iki kez oluyor, PowerShell `env:`'i okurken hata
    veriyordu. Windows'ta yalnızca büyük harfli yazım verilir; köprü
    kaptaki aynı adlı değişkeni ezer.
  - **Windows yalıtım testlerinden 5'i kırmızıydı:** üç eski test ağlı
    profili köprüsüz kuruyordu (biri internete doğrudan bağlanmayı
    bekliyordu; artık tersini sınar). İki köprü testi `PATHEXT`'siz ortam
    yüzünden `curl.exe`'yi hiç çalıştırmıyordu; biri de düz HTTP'yi 80
    dışındaki porta gönderiyordu (`curl -p` ile tünele çevrildi).
  - **Uçtan uca:** derleme komutu `--bin kalfa-kopru` yüzünden uygulamayı
    derlemiyordu; testler eski bir ikiliyle koşuyordu. S13 hiç koşmamıştı:
    yardımcıları yoktu, onay akışı ağlı komutun riskli sayıldığını (#16)
    bilmiyordu ve `[ağ]` satırını terminalde arıyordu (satır modele giden
    sonuçtadır).
  - **Görev 4'ün yarım kodu:** `bwrap.rs`'in sıralama testinden silinen
    doğrulamalar geri geldi; üretim kodundaki `unwrap()` ve eksik `SAFETY`
    açıklamaları giderildi; vekil kanalı okuyucuyla kapatıcı arasında
    paylaşılan sahiplikle tutulur (ham numarayla kapanış yarışı vardı);
    bozuk ileti okuyucuyu durdurmaz.
- **Doğrulama:**
  - Windows: çalışma alanı, iki clippy, `e2e` özellikli testler; yalıtım
    43/43, uyumluluk 19/19; arayüz (`check` 0/0, Vitest 292, paket denetimi).
  - WSL (`kalfatest`): çalışma alanı, iki clippy, `e2e` özellikli testler;
    yalıtım 26/26, uyumluluk 12/12.
  - Uçtan uca: Windows ve WSL'de 33/33 (S13 dahil).
- **Bilinen sınırlar:** Java ortamdaki vekili okumaz; ECH kullanan araçlar
  SNI denetimine takılır (ölçülen araçların hiçbiri kullanmıyor); kurumsal
  vekil arkasındaki makinede vekilin dış bağlantısı kurumsal vekilden geçmez
  (G16).
