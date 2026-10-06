# Kalfa 3d — G5 ve G7 ölçümü: Linux ağ köprüsü

**Tarih:** 2026-09-28
**Plan:** [3d, Görev 1](../plans/2026-09-28-kalfa-3d-ag-cikisi.md)
**Tasarım:** [aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md)
§8.2, §8.5, §13
**Prototip:** [`prototip/linux-kopru/`](../../../prototip/linux-kopru/README.md)

**Sorular:**

- **G5:** Ağ ad alanının içindeki köprü, kabul ettiği bağlantının
  tanımlayıcısını `SCM_RIGHTS` ile Kalfa'ya taşıyabilir mi? Kalfa bu soketle
  çalışabilir mi? Araçlar `HTTPS_PROXY`'ye uyuyor mu?
- **G7:** Araçlar ECH kullanıyor mu? SNI, CONNECT hostuyla aynı mı?

## Sonuç

| Konu | Sonuç |
|---|---|
| Tanımlayıcı taşıma (G5) | Çalışıyor. Vekil, ad alanında kabul edilen soketi okur ve yazar; 8 MiB indirme ve 1 MiB yükleme bayt bayt geçti. |
| Ad alanı | Komut dışarıya, DNS'e ve dış ad alanının geri döngüsüne ulaşamıyor. |
| Yaşam | Kanal komuta geçmiyor. Köprü komutun çıkış koduyla çıkıyor. Köprü çıkınca vekilin kanalı hemen kapanıyor. |
| Soket kuralı | Ağlı profilin kuralı (`socket(AF_UNIX)` yasak) köprünün tanımlayıcı göndermesini engellemiyor. |
| Araçların uyumu (G5) | curl, wget, git, cargo, pip, uv, npm, python ve go uyuyor. Node'un `fetch`'i ve `https`'i yalnızca `NODE_USE_ENV_PROXY=1` ile uyuyor. Java ortama hiç bakmıyor. |
| Yapılandırma önceliği | npm, cargo, git ve pip'te yapılandırma dosyasındaki vekil ortamı eziyor; aracın kendi ortam değişkeni geri eziyor. |
| `NO_PROXY` | wget yalnızca küçük harfli `no_proxy`'yi okuyor. |
| CONNECT sürümü | pip ve python `CONNECT … HTTP/1.0` gönderiyor. |
| ECH (G7) | Hiçbir araç ECH göndermedi. SNI her bağlantıda CONNECT hostuyla aynıydı. ClientHello 248–1605 bayt, tek TLS kaydı. |
| `403` iletisi | pip, uv, Node `fetch` ve go çıktısında `403` geçmiyor. |
| Maliyet | Bağlantı başına yaklaşık 0,3 ms ek süre. |

Tasarıma etkileri aşağıda, "Tasarıma etkileri" bölümündedir. bwrap'la yineleme
3d'nin Görev 4'ündedir.

## Düzenek

- **Makine:** Linux 6.18 (bulut kabı), `root`. bwrap kurulu değil; sistem
  paketi kurulmadı.
- **Ad alanı:** Kalfa'nın bwrap'ı yerine `unshare --user --map-root-user
  --net`.
  - Ağ ad alanı bwrap'ın `--unshare-net`'indekiyle aynıdır: yalnızca kabın
    kendi geri döngüsü vardır.
  - bwrap geri döngüyü kendisi açar. `unshare` açmadığı için köprü `lo`'yu
    `SIOCSIFFLAGS` ile açtı.
- **Kanal:** Ölçüm düzeneği (Kalfa'nın yerinde) bir `SOCK_SEQPACKET` çifti
  açar. Çiftin bir ucu `CLOEXEC`'siz olarak 3 numarayla `unshare`'e, oradan
  köprüye geçer. Köprü kanalı `CLOEXEC` yapar ve `127.0.0.1`'de rastgele bir
  portu dinler. Komutu tasarımın ortamıyla başlatır (§8.5) ve kabul ettiği her
  bağlantıyı tek baytlık bir iletiyle kanaldan gönderir.
- **Vekilin sahtesi:**
  - Gelen tanımlayıcıyı denetler: bağlı bir TCP soketi olmalı (`fstat`,
    `SO_DOMAIN`, `SO_TYPE`, `SO_PROTOCOL`, `SO_ACCEPTCONN`, `getpeername`).
  - İsteği okur. Varsayılan listedeki (§8.3) hostta `200` döner, aracın
    ClientHello'sunu okur ve kapatır. Listede olmayan host `403` alır.
  - **Dışarıya hiçbir bağlantı kurulmadı.** Yalnızca `yerel.test`, dış ad
    alanındaki yerel bir sunucuya aktarıldı. Araçlar bu yüzden izinli hostta TLS
    hatasıyla bitti; ölçülen, vekile ne gönderdikleridir.
- **Ortam:** komutun ortamı bwrap'taki `--clearenv` gibi boştu: `PATH`, geçici
  bir `HOME`, `LANG`, `TERM`, `TMPDIR` ve her aracın önbellek dizini.
  Makinenin kendi vekil ayarları geçmedi.
- **Sürümler:** curl 8.5.0 (OpenSSL 3.0.13), wget 1.21.4, git 2.43.0
  (libcurl-gnutls), cargo 1.98.1, pip 24.0 (Python 3.11.15), uv 0.8.17, npm
  10.9.7, Node 22.22.2, go 1.24.7, OpenJDK 21.0.10.

Komut: `cargo build --offline --release && ./target/release/kalfa-kopru-prototip
olc`. Bütün gruplar tek koşuda, hatasız geçti.

## o1–o6: köprü ve ad alanı

| Kod | Deneme | Gözlenen |
|---|---|---|
| o1.1 | Doğrudan bağlantı (192.0.2.1:443) | `Network is unreachable`, 0 ms |
| o1.2 | Ad çözümü (`index.crates.io`) | `Temporary failure in name resolution`, 0 ms |
| o1.3 | Dış ad alanının geri döngüsündeki sunucu | `Connection refused` |
| o2.1 | curl `-p` ile `yerel.test` | Yerel sunucunun yanıtı; vekilde `CONNECT yerel.test:80` aktarıldı |
| o2.2 | 8 MiB indirme | 8 388 608 bayt, yaklaşık 530 MB/sn |
| o2.3 | 1 MiB yükleme | Sunucu 1 048 576 bayt aldı |
| o3.1 | Komutun açık tanımlayıcıları | Yalnızca 0, 1, 2 ve listelemenin kendisi; kanal ve dinleyici yok |
| o4.1 | Komut 7 ile çıkar | Köprü 7 ile çıktı |
| o4.2 | Komut SIGTERM ile ölür | Köprü 143 ile çıktı |
| o4.3 | Köprü çıkınca vekilin kanalı | 0 ms'de kapandı; vekil bitti |
| o4.4 | Arka plandaki çocuk, köprü çıktıktan sonra vekile bağlanır | `Couldn't connect to server`; vekile istek gelmedi |
| o5.1 | Soket kuralı altında curl `-p` ile `yerel.test` | Çalıştı |
| o5.2 | Aynı kural altında `socket(AF_UNIX)` | `EPERM` (kural etkin) |
| o6.1 | Kabın içindeki sunucu, curl (`127.0.0.1`, `localhost`) | `200 200`, vekilsiz |
| o6.2 | Aynısı wget, yalnızca `NO_PROXY` ile | wget isteği vekile gönderdi (`GET http://127.0.0.1:18080/`), `403` aldı |
| o6.3 | Aynısı wget, `no_proxy` da verilince | Vekilsiz, başarılı |

o4.4'te arka plandaki süreç köprüden sonra yaşadı, çünkü prototipte süreç ad
alanı ayrı değildir. bwrap `--unshare-pid` ile köprü çıkınca kabın bütün
süreçleri ölür. Dinleyici köprüyle kapandığı için sonuç iki durumda da aynıdır:
vekile ulaşılamaz.

## o7: araçların uyumu (G5) ve ClientHello (G7)

"Vekile gelen" sütununda `200`, izinli hostta tünelin açıldığını gösterir.
Vekil ClientHello'yu okuyup bağlantıyı kapattığı için araçlar TLS hatasıyla
bitti.

| Kod | Araç | Vekile gelen | SNI | ECH | ALPN | ClientHello |
|---|---|---|---|---|---|---|
| a1 | curl | `CONNECT index.crates.io:443 HTTP/1.1` → 200 | = host | yok | h2, http/1.1 | 512 B |
| a2 | curl, listede olmayan host | `CONNECT izinsiz.example:443` → 403 | — | — | — | — |
| a3 | curl, düz HTTP | `GET http://izinsiz.example/` → 403 | — | — | — | — |
| a4 | wget | `CONNECT pypi.org:443 HTTP/1.1` → 200 | = host | yok | — | 397 B |
| a5 | git ls-remote, `github.com` listede yok | `CONNECT github.com:443` → 403 | — | — | — | — |
| a6 | git ls-remote, `github.com` eklendi | `CONNECT github.com:443 HTTP/1.1` → 200 | = host | yok | h2, http/1.1 | 390 B |
| a7 | cargo fetch | `CONNECT index.crates.io:443 HTTP/1.1` → 200 | = host | yok | h2, http/1.1 | 1559 B |
| a8 | pip download | `CONNECT pypi.org:443 HTTP/1.0` → 200 | = host | yok | http/1.1 | 512 B |
| a9 | uv pip compile | `CONNECT pypi.org:443 HTTP/1.1` → 200 | = host | yok | h2, http/1.1 | 248 B |
| a10 | npm view | `CONNECT registry.npmjs.org:443 HTTP/1.1` → 200 | = host | yok | — | 1590 B |
| a11 | node `fetch` | istek yok; `getaddrinfo EAI_AGAIN` | — | — | — | — |
| a12 | node `fetch`, `NODE_USE_ENV_PROXY=1` | `CONNECT registry.npmjs.org:443 HTTP/1.1` → 200 | = host | yok | http/1.1 | 1605 B |
| a13 | node `https.get` | istek yok; `getaddrinfo EAI_AGAIN` | — | — | — | — |
| a14 | node `https.get`, `NODE_USE_ENV_PROXY=1` | `CONNECT registry.npmjs.org:443 HTTP/1.1` → 200 | = host | yok | — | 1590 B |
| a15 | python `urllib` | `CONNECT pypi.org:443 HTTP/1.0` → 200 | = host | yok | http/1.1 | 512 B |
| a16 | go mod download, `proxy.golang.org` eklendi | `CONNECT proxy.golang.org:443 HTTP/1.1` → 200 | = host | yok | h2, http/1.1 | 1491 B |
| a17 | java `HttpClient` | istek yok; ad çözülemedi | — | — | — | — |
| a18 | java, `JAVA_TOOL_OPTIONS` ile vekil | `CONNECT repo.maven.apache.org:443` → 403 | — | — | — | — |

Araçlar vekile uymadığında (a11, a13, a17) ağa hiç çıkamadı; güvenli taraf
budur. Hataları ad çözümünden geldi.

## o8: yapılandırma dosyasındaki vekil

Dosyaya belgelere ayrılmış, yönlendirilmeyen bir vekil adresi
(`http://192.0.2.1:3128`) yazıldı.

| Kod | Araç | Vekile gelen | Son satır |
|---|---|---|---|
| c1 | npm, küresel `npmrc`'de başka vekil | istek yok | `npm error` (bağlanamadı) |
| c2 | aynısı + `npm_config_https_proxy` | `CONNECT registry.npmjs.org:443` | — |
| c3 | cargo, `config.toml`'da başka vekil | istek yok | `Failed to connect to index.crates.io:443 over proxy 192.0.2.1` |
| c4 | aynısı + `CARGO_HTTP_PROXY` | `CONNECT index.crates.io:443` | — |
| c5 | git, `gitconfig`'de başka vekil | istek yok | `Failed to connect to 192.0.2.1 port 3128` |
| c6 | aynısı + `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` ile `http.proxy` | `CONNECT github.com:443` → 403 | — |
| c7 | pip, `pip.conf`'ta başka vekil | istek yok | `No matching distribution found` |
| c8 | aynısı + `PIP_PROXY` | `CONNECT pypi.org:443` | — |

Kapta kullanıcının ev dizini ve `CARGO_HOME`'u özeldir; kullanıcı düzeyindeki
dosyalar görünmez. Sistem düzeyindeki dosyalar (`/etc/gitconfig`,
`/etc/pip.conf`, Node'un küresel `npmrc`'si) ve çalışma alanındaki proje
dosyaları (`.npmrc`, `.cargo/config.toml`) görünür. Oradaki bir vekil ayarı
kapta ulaşılamaz bir adrese yönlendirir. Sonuç güvenlidir (araç çıkamaz) ama
komut çalışmaz.

## o9: maliyet

| Deneme | Sonuç |
|---|---|
| 200 sıralı bağlantı, köprü ve vekil üzerinden (CONNECT, GET, yanıt) | ortalama 0,45 ms, medyan 0,42 ms |
| 200 sıralı bağlantı, doğrudan (karşılaştırma) | ortalama 0,19 ms, medyan 0,18 ms |
| 256 bağlantı, 32 paralel, köprü ve vekil üzerinden | toplam 146 ms; 256'sı da aktarıldı |

Vekilin sahtesi bağlantı başına bir iş parçacığı açar; Kalfa'nın vekili tokio
kullanacak. Fark TLS el sıkışmasının ve indirmenin yanında önemsizdir.

## o10: engellenen hostta araçların iletisi

Liste boşken:

| Kod | Araç | Son satır | Çıktıda `403` |
|---|---|---|---|
| e1 | cargo | `[7] Could not connect to server (CONNECT tunnel failed, response 403)` | evet |
| e2 | npm | `npm error 403 on a server you do not have access to.` | evet |
| e3 | pip | `ERROR: No matching distribution found for idna` | hayır |
| e4 | uv | `Caused by: tunnel error: unsuccessful` | hayır |
| e5 | python `urllib` | `Tunnel connection failed: 403 Forbidden` | evet |
| e6 | node `fetch`, `NODE_USE_ENV_PROXY=1` | `Error: Request was cancelled.` | hayır |
| e7 | go | JSON çıktısında hata; `403` geçmiyor | hayır |

curl (a2) ve git (a5) de `CONNECT tunnel failed, response 403` diyor.

## Tasarıma etkileri

1. **Köprünün ortamı genişler (§8.5):**
   - `no_proxy` de verilir; wget büyük harfli `NO_PROXY`'yi okumuyor (o6.2).
   - `NODE_USE_ENV_PROXY=1` verilir. Node 22.22.2'de `fetch` ve `https`
     onunla vekile uydu (a12, a14). Node betikleri (`postinstall`) bu
     yollarla indirir.
   - Aracın kendi vekil değişkenleri de verilir: `npm_config_https_proxy`,
     `npm_config_proxy`, `CARGO_HTTP_PROXY`, `PIP_PROXY`. Git'e `http.proxy`,
     `GIT_CONFIG_COUNT=1`, `GIT_CONFIG_KEY_0=http.proxy` ve
     `GIT_CONFIG_VALUE_0` ile verilir. Sistemdeki ya da çalışma alanındaki bir
     vekil ayarı böylece komutu bozamaz (o8).
   - Bu değişkenler Kalfa'nın ortam izin listesinde (`kalfa-sandbox/src/ortam.rs`)
     yoktur. Köprü kullanıcıdan gelen bir değeri ezmez.
2. **CONNECT ayrıştırıcısı `HTTP/1.0`'ı da kabul eder (§8.2):** pip ve
   python onu gönderiyor (a8, a15).
3. **G7 kapandı:** ölçülen araçlarda ECH yok. SNI her bağlantıda CONNECT
   hostuyla aynıydı. SNI denetimi bu araçları bozmaz. ECH kullanan bir araç
   çıkarsa bağlantısı kesilir; bu, tasarımın kabul ettiği sınırdır.
4. **`[ağ]` satırı zorunludur (§8.4):** pip, uv, Node `fetch` ve go
   çıktısında `403` geçmiyor (o10). Model engeli ancak Kalfa'nın eklediği
   satırdan öğrenir.
5. **Java vekile uymaz:** ortamdaki vekili okumuyor (a17).
   `JAVA_TOOL_OPTIONS` ile uyuyor (a18). Ancak JVM her açılışta "Picked up
   JAVA_TOOL_OPTIONS" yazıyor. Köprü bunu varsayılan olarak vermez. Java
   araçları ağlı yalıtımda ağa çıkamaz; bilinen sınır olarak yazılır.
6. **Go'nun modül vekili varsayılan listede yok:** go vekile uydu (a16) ama
   `proxy.golang.org` listede olmadığı için `403` aldı (e7). Modülün
   doğrulaması `sum.golang.org`'u da ister. Listeye eklenip eklenmeyeceği
   kullanıcının kararıdır (tasarım §13, G18).
7. **Bağlantı başına maliyet önemsiz** (o9).

## Doğrulanmayanlar

Bunlar bwrap'la, WSL'de (Ubuntu-24.04) `kalfatest` kullanıcısıyla ölçülecek
(plan, Görev 4):

- **Kanal:** tanımlayıcının bwrap'tan kaptaki köprüye geçmesi. Bugün bwrap'a
  verilen tanımlayıcıları (`--args`, seccomp) bwrap kendisi tüketir. Kanal ise
  kaptaki köprüye kadar geçmeli.
- **Geri döngü:** bwrap'ın `--unshare-net`'te `lo`'yu kendisinin açması.
- **Kalfa'nın filtresi:** prototipin küçük filtresi yerine Kalfa'nın gerçek
  seccomp filtresi altında köprü.
- **Kullanıcı:** root olmayan kullanıcı ve Ubuntu 24.04'ün AppArmor kısıtı.
  Kalfa'nın bugünkü bwrap düzeni bu kısıtla çalışıyor; köprü ek bir ad alanı
  açmaz.
- **Kalfa'nın vekili:** tokio ile yazılan gerçek vekil ve sınırları (64
  bağlantı, 5 dakika boşta kalma).
- **Kurumsal vekil (G16):** ölçülmedi. Prototip dışarıya hiç bağlanmadı.

## Ek: bwrap'la ölçüm (Görev 4'ün ön ölçümü, 2026-10-05)

Prototip `unshare` ile ölçmüştü. Bu ek, aynı yolu Kalfa'nın gerçek bwrap
düzeniyle yineler: WSL Ubuntu-24.04'te, `kalfatest` kullanıcısıyla, bwrap
0.9.0.

**Düzenek:**
- `linux::komut_kur`'a geçici bir ölçüm kancası eklendi; commit'lenmedi.
- Kalfa'nın ağlı profilinin seçenekleri kullanıldı: gerçek seccomp filtresi
  (`engel_filtresi(true)`, AF_INET ve AF_INET6 açık, AF_UNIX kapalı), `enosys`
  filtresi ve `prlimit` sarmalayıcısı.
- Görev 4'ün öngördüğü değişiklikler eklendi:
  - `--unshare-net`, `resolv.conf` bağlanmadan.
  - Prototipin köprüsü salt okunur `/run/kalfa-kopru`'ya bağlandı.
  - `SOCK_SEQPACKET` kanalın bir ucu, öteki devredilen tanımlayıcılar gibi
    `CLOEXEC`'siz geçirildi.
- Kanalın öbür ucunda test, gelen tanımlayıcıyı okuyup sahte vekil gibi yanıt
  verdi. İnternete çıkılmadı.

| # | Soru | Sonuç |
|---|---|---|
| b1 | Kanal bwrap'tan kaptaki köprüye geçiyor mu? | **Evet.** Kaptaki curl'ün `GET http://yerel.test/` isteği köprüden, kanaldan vekile ulaştı; yanıt komuta döndü. |
| b2 | bwrap `lo`'yu açıyor mu? | **Evet.** Kapta yalnızca `lo` var; köprü `--lo-ac`'sız `127.0.0.1`'de dinledi. |
| b3 | `CONNECT` vekile ulaşıyor mu? | **Evet.** Sahte vekil `403` verdi; curl `CONNECT tunnel failed, response 403`. |
| b4 | Komut vekilsiz ağa çıkabilir mi? | **Hayır.** `curl --noproxy '*' http://1.1.1.1/`: `Couldn't connect to server`. |
| b5 | Ad çözümü var mı? | **Hayır.** `getent hosts` başarısız; `/etc/resolv.conf` yok. |
| b6 | Süreç yapısı | pid 1 bwrap, pid 2 köprü, pid 3 komut (`prlimit` komuta `exec` eder). |
| b7 | Köprü ölünce ne olur? | Komut da ölür: `kill -9` köprüye, komutun çıkışı 137. bwrap süreç ad alanının ilk süreci olarak çıkınca kabı kapatır. |
| b8 | Kanal ve dinleyici komuta geçiyor mu? | **Hayır.** Komutun tanımlayıcıları: 0–2, komut ikamesinin borusu ve `/dev/ptmx` (b9). |
| b9 | Başka tanımlayıcı sızıyor mu? | **Evet:** `/dev/ptmx` (fd 7), testi başlatan `su -` oturumundan. |

**b9'un anlamı:**
- Kalfa'nın `pre_exec`'i yalnızca devrettiği tanımlayıcıların `CLOEXEC`'ini
  kaldırıyor; bwrap da devraldığını kapatmıyor.
- Kalfa sürecinde `CLOEXEC`'siz açık kalan her tanımlayıcı kaba geçer. Ağlı
  olsun olmasın bütün yalıtımlı komutlar için geçerlidir.
- Kalfa'nın kendi açtıkları Rust'ın varsayılanıyla `CLOEXEC`'tir. Ama Kalfa'yı
  başlatan süreçten kalanlar ve kitaplıkların açtıkları için güvence yoktur.
- Görev 4'e derinlemesine savunma olarak eklendi:
  - `pre_exec` önce 3'ten yukarıdaki bütün tanımlayıcıları `CLOEXEC` yapar
    (`close_range(3, ~0, CLOSE_RANGE_CLOEXEC)`), sonra yalnızca devredilenleri
    açar.
  - Testi `devredilmeyen_tanimlayici_kaba_gecmez`.

**Sonuç:** Görev 4'ün ön ölçümü olumlu; Görev 4 tasarımdaki gibi yürür.
