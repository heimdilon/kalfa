# Kalfa 3d — G6 ölçümü: Windows ağ köprüsü

**Tarih:** 2026-10-05
**Plan:** [3d, Görev 8](../plans/2026-09-28-kalfa-3d-ag-cikisi.md)
**Tasarım:** [aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §8.6, §13
**Ölçüm:** [`prototip/windows-kopru/`](../../../prototip/windows-kopru/README.md)

**Sonuç:** G6 olumlu. Köprü yolu Windows'ta çalışıyor; yedek yola (K6) gerek
yok. Köprü ve Kalfa boruyu asenkron (overlapped) kullanmalı (Ö3).

## Düzenek

- **Kap:** Kalfa'nın gerçek yalıtımı (`WindowsYalitici`). Kabuk aracının bütün
  yolundan, `ag: false` ile, yani `internetClient` yeteneği olmadan.
- **Kaptaki taraf:** çalışma alanına yazılan bir Python betiği. Kalfa'nın
  ürettiği bir ikili yok; ölçümün sorusu kabın yeteneği.
- **Kabın dışındaki taraf:** test sürecinde adlandırılmış boru sunucuları ve
  bir CONNECT vekili. Vekil ölçüm için izin listesi uygulamaz.
- **Makine:** Windows 11, normal kullanıcı (yönetici yükseltmesi yok), Python
  3.14.7, Node 24.20.0, Git 2.55.
- **Temizlik:** her koşudan sonra ACE kaydı boş, Kalfa profili kalmadı.

## Sonuçlar

| # | Soru | Sonuç |
|---|---|---|
| Ö1 | Aynı kaptaki iki süreç `127.0.0.1` üzerinden konuşur mu? | **Evet.** Dinleyen süreç ve onun başlattığı ikinci süreç veri alıp verdi. Kaptan dışarıya geri döngünün kapalı olması (A9) kabın içini etkilemiyor. |
| Ö2 | Kap, paket SID'ine açık bir boruya bağlanır mı? | **Evet.** Paket SID'i yalnızca okuma ve yazma (`GRGW`) aldı. Düşük bütünlük etiketi (`S:(ML;;NW;;;LW)`) gerekmedi; etiketli ve etiketsiz boru ikisi de açıldı. Paket SID'i olmayan boru `PermissionError` verdi: DACL uygulanıyor. |
| Ö3 | `internetClient`'sız kapta araçlar köprü ve vekil üzerinden çalışır mı? | **Evet, boru asenkron kullanılınca.** Ayrıntı aşağıda. |
| Ö4 | Köprünün ikilisi kapta çalışır mı? | **Program Files'tan evet** (`git.exe`). Çalışma alanı dışındaki bir kullanıcı dizininden **hayır** ("erişim engellendi"). Geliştirme derlemesinin `target\`'ı kaba kapalıdır. |

### Ö3: araçlar

Köprü, kapta `127.0.0.1`'de dinler. Her bağlantı için boruyu açar ve baytları
iki yönde aktarır. Araçlara `HTTPS_PROXY` ve tasarım §8.5'teki öteki
değişkenler verildi.

| Araç | Sonuç |
|---|---|
| curl, vekilsiz (denetim) | ✖ `Could not resolve host`: kap ağsız. |
| curl (Schannel, iptal denetimi açık) | ✔ 200, 0,4 sn. Schannel'ın iptal denetimi vekil üzerinden geçti (A8'in endişesi tutmadı). |
| `git ls-remote` (github.com) | ✔ 1,1 sn |
| `npm view` | ✔ 7,1 sn |
| Node `fetch`, `NODE_USE_ENV_PROXY=1` | ✔ 200 |
| `cargo fetch` | Ağ ✔ (`index.crates.io` ve `static.crates.io` tünelleri). Paket açılırken ✖ "erişim engellendi … while canonicalizing": bilinen A15. |
| `pip download` | Ağ ✔ (`pypi.org` ve `files.pythonhosted.org` tünelleri). Açarken ✖ `PermissionError`, kabın `AC\Temp`'inde: A16 sınıfından. |

- **cargo ve pip'in hataları köprüden bağımsızdır.** Aynı iki komut bugünkü
  ağlı yolda (`ag: true`, `internetClient`, vekilsiz) aynı hatalarla düştü
  (Ö5). Ağ kısmı iki yolda da çalışıyor.
- **Kalfa ağlı komuta `CARGO_NET_OFFLINE` vermemeli.** Ağsız çalıştırma onu
  veriyor (2d-3, A15'in gelişmiş yolu). Ölçümde ağlı denemeler için kaldırıldı.

### Ö3: asenkron boru zorunlu

İlk denemede tünel kuruldu ama veri akmadı. curl 30 saniyede zaman aşımına
uğradı, git takıldı.

- **Neden:** eşzamanlı açılan bir tutamaçta Windows aynı dosya nesnesindeki
  G/Ç'yi sıraya koyar. Bir iş parçacığı boruyu okurken bekledikçe ötekinin
  aynı boruya yazması da bekler. İki yönlü aktarım böylece kilitlenir.
- **Kapsam:** hem köprü (`open(...)` ile Python) hem vekil (`File` ile Rust)
  aynı hatayı yapıyordu.
- **Düzeltme:** iki uçta da overlapped G/Ç.
  - Köprüde Python `asyncio`'nun Proactor döngüsü
    (`loop.create_pipe_connection`).
  - Vekilde tokio'nun `net::windows::named_pipe::ServerOptions`'ı
    (`create_with_security_attributes_raw`).

  Bundan sonra bütün araçlar geçti.

## Tasarıma etkisi (§8.6)

1. **Köprü yolu seçilir; K6'nın yedek yolu gerekmez.** Ağlı yalıtımlı komut
   Windows'ta da `internetClient`'sız çalışır, tek çıkışı vekildir.
2. **Boru iki uçta da overlapped açılır.** `kalfa-kopru.exe` ya tokio
   (`named_pipe::ClientOptions`) ya da overlapped Win32 G/Ç kullanır.
   Kalfa'nın ucu tokio'nun `ServerOptions`'ıdır. Workspace'teki tokio'ya `net`
   özelliği zaten 3d'de ekleniyor; Windows'ta adlandırılmış boru da onunla
   gelir. Bu ölçümde kilit dosyası değişmedi.
3. **Borunun DACL'i:** SYSTEM, geliştirici ve o çalışma alanının paket SID'i
   (`GRGW`). Bütünlük etiketi gerekmez. Her komut için yeni, rastgele adlı bir
   boru açılır.
4. **Köprünün ikilisi kaba okunur olmalı.**
   - Kurulu Kalfa'da Program Files'tadır; ALL APPLICATION PACKAGES okur.
   - Geliştirme derlemesinde `target\` kaba kapalıdır. Kalfa ikiliyi
     çalışma alanının yalıtım dizinine (kaba okunur) kopyalar ya da test
     düzeneği bunu yapar. Hangisinin seçileceği alt planda.
5. **Bilinen sınırlar sürer:**
   - Yeni crate'ler A15 nedeniyle kapta açılamaz: ağlı `cargo fetch` vekil
     olsa da düşer.
   - Sistem Python'unun `pip download` / `pip install`'ı A16 sınıfından düşer.
     Kalfa'nın Python'uyla venv'de `pip install` daha önce çalışmıştı (aşama 2
     §12.3).

## Kalan sorular (alt plan)

- Köprü kabın Job Object'inde, komuttan önce mi başlar, yoksa komutu köprü mü
  başlatır (Linux'taki gibi)? Linux'taki gibi köprünün başlatması, çıkış
  kodunu ve süreç ağacını tek yerde tutar.
- Kalfa'nın vekili kaç eşzamanlı boru örneği tutar; sınır ve zaman aşımı.
- Köprü güvenilir kod sayılmaz (§8.5'teki gibi). Sınır Kalfa'daki vekildir;
  boruyu ele geçiren komut da yalnızca izinli hostlara ulaşır.
