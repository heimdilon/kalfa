# Kalfa geliştirme durumu

**21 Ekim Codex düzeltmesi — Hat 04:** EOF sözleşme aktarımı uygulandı. [Davranış ve doğrulama](superpowers/bulgular/2026-10-21-codex-yetenek-04-duzeltme.md); G5/CI ve merge ayrı kapılardır.

Güncelleme: 2026-10-14; Windows sürüm akışı ve PR #8 arayüzü birleşti, depo herkese açık. Bu dosya uygulamanın devam noktasıdır; tasarım
belgelerindeki tarihsel inceleme kayıtlarının yerine geçmez.

**Otomatik izin kipi (19 Ekim, PR A tamam):** Kalfa'ya Claude Code'unkine
benzer beşinci bir kip, Otomatik, eklenecek. Bu kipte onay isteyen yalıtımlı
komutları ve MCP çağrılarını bir hakem model değerlendirir. Hakem `clef`'tir
ve EVREN'in `/v1/systemone` ucunda çalışır. Kullanıcı tasarımı onayladı. İş
iki PR'a bölündü:

- **PR A, hakem ve ölçüm** ([#34](https://github.com/heimdilon/kalfa/pull/34)):
  Birleşti (`4041a39`).
  - **Bileşenler:** `kalfa-evren` karar istemcisi, `Saglayici::karar`,
    ortak bütçeli yeniden deneme ve `kalfa-core/src/hakem.rs`.
  - **Ölçüm** (42 öğe): yanlış izin 0, güvenli öğelerde otomatik izin %95,
    p95 gecikme 0,70 sn.
  - **Eşikler:** izin 0,80, ret 0,40.
- **PR B, kip ve arayüz** ([#35](https://github.com/heimdilon/kalfa/pull/35)):
  `otomatik-kip/b-kip` dalında uygulandı (Görev 1–8).
  - **Bileşenler:** `Kip::Otomatik` ve `hakeme_uygun` izin listesi (`izin.rs`),
    araç aşaması hakem kapısı, bağlı grup ve geri düşme sayaçları
    (`arac_asamasi.rs`), `KayitOgesi::OtomatikKarar` / `Olay::OtomatikKarar` /
    `GorunumOgesi::OtomatikKarar`, kurtarmada Otomatik → Sıkı dönüşümü,
    uygulama katmanı yükseltme sırası ve açıklaması (`guvenlik.rs`), arayüzde
    beşli kip menüsü ve konuşmada tek satırlık karar notları.
  - **Sınır testleri:** `crates/kalfa-core/tests/otomatik_kip.rs` içinde tasarım
    §9'daki bütün sınırlar (mühür testi dahil 18 oturum testi) doğrulandı.

**Sıradaki iş:** PR B (#35) incelemesi ve WSL doğrulaması.

- **Dal:** `otomatik-kip/b-kip`.
- **Uygulayıcı:** Claude (inceleme ve birleştirme).

- [Tasarım](superpowers/specs/2026-10-19-kalfa-otomatik-kip-design.md)
- [PR A planı](superpowers/plans/2026-10-19-kalfa-otomatik-kip-a-hakem.md)
- [Yoklama](superpowers/bulgular/2026-10-19-kalfa-otomatik-kip-yoklama.md)
- [Ölçüm](superpowers/bulgular/2026-10-19-kalfa-otomatik-kip-olcum.md)
- [PR B planı](superpowers/plans/2026-10-20-kalfa-otomatik-kip-b-kip.md)

**PR #8 — Codex ek düzeltmeleri (14 Ekim):** JSON boyutuna göre diff bütçesi,
gerçek `--`/`++` içerik satırlarının korunması ve yeni oturumun katalog
ağ isteğini beklememesi düzeltildi. Üç regresyon önce düştü, sonra geçti;
arayüz 429/429, check/build temiz; Windows workspace 1 214/0/19, Ubuntu
workspace 1 158/0/19, iki platformda e2e özellikli Rust 191/0/0. Workspace
ve e2e özellikli clippy ile fmt temiz. Gerçek yalıtım/WebDriver yeniden koşmadı.
[Doğrulama kaydı](superpowers/bulgular/2026-10-14-pr8-ek-duzeltmeler.md).

**PR #8 birleşti (14 Ekim, `286a991`):** Arayüz Claude Code/Codex masaüstü
düzenine geçti: tek kenar çubuğu (Ctrl+N yeni oturum, Ctrl+K arama, Ctrl+B
gizle), ince başlık, ortalanmış konuşma, tek satır araç çağrısı ve düşünce,
yüzen composer, nötr yüzeyler; durum çubuğu inceltildi. Rust ve IPC
değişmedi. Arayüz 431/431, check 0 hata/uyarı, build temiz; CI dört işiyle
yeşil. Gerçek WebDriver uçtan uca takımı bu düzen için koşmadı.

## Güncel devam noktası — 2026-10-12 18:44 TSİ

Aşama 1–3 tamamlandı. Aşama 4'te tasarım #49, çalışma zamanı #50
(`d1aa225`), EVREN yolu #51 (`4807265`), RAG #52 (`7ada5fa`) ve eklenti
arayüzü/dağıtımı #53 (`98e7c0d`) ana dala birleşti. **Aşama 4 tamamlandı.**

| İş | Yayınlanmış baş / taban | Durum |
|---|---|---|
| Ana dal | `main`, `3133248` | Aşama 4 ve 5a birleşmiş kod |
| [#2 — 5a](https://github.com/heimdilon/kalfa/pull/2) | `9dfdde5` → `3133248` | 12 Ekim squash birleşti; [CI](https://github.com/heimdilon/kalfa/actions/runs/38264533087) dört işiyle yeşil |
| [#52 — 4c](https://github.com/heimdilon/kalfa-arsiv/pull/52) | `ffdf65b` → `7ada5fa` | 12 Ekim squash birleşti; G5 kapandı, [CI 149](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38222907378) yeşildi |
| [#53 — 4d](https://github.com/heimdilon/kalfa-arsiv/pull/53) | `a51abd0` → `98e7c0d` | 12 Ekim squash birleşti; yeni ana dala taşındıktan sonra [CI 153](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38226258768) yeşildi |

Arşiv deposuna bağlanan kayıtlardaki commit kimlikleri ve PR/CI numaraları arşive aittir;
açık depoda geçmiş yeniden yazıldığı için kimlikler farklıdır. Aşama 4
worktree'leri ve WSL kopyaları 12 Ekim'de kaldırıldı. Ana klasördeki
ilgisiz `tasks.png`'ye dokunulmaz.

**Ölçüm kararları:** G4/G6/G8 ve G5'in aday/dosya, talimat dili, boyut,
parça penceresi kıyasları tamam. **İngilizce / 50 aday / 1024 boyut /
60 satır–2400 karakter–10 satır örtüşme** korunur. Tam ilk indeksleme
566 dosya/4 056 parçada yaklaşık **2,62 milyon token**: 2 157 582 doğrudan
sayılmış, ilk 747 parçanın yaklaşık 460 600 tokenı tahmin edilmiştir
(toplamın yaklaşık %18'i).

**G5 kapandı (12 Ekim):** Tam indeks artımlı güncellendi (568 dosya,
76 140 token); aynı 32 sorgu gerçek wasm/host üzerinden ilk beş parçada
**26/32 (%81, eşik %80)**, ilk beş farklı dosyada **28/32** verdi. Sorgu
ort. 2,0 sn, p95 2,3 sn; toplam 1 339 234 sayılmış token, hız sınırı ve
başarısız istek yok. Ürün ayarları değişmez.
[Ölçüm](superpowers/bulgular/2026-10-12-kalfa-g5-tam.md).

**4d incelemesi:** `9980516`, aynı adlı yerleşik ve eski kurulum kaydının
Svelte listelerini bozmasını düzeltir. İki regresyon önce düştü, sonra
geçti. Windows **1090 + 175**, WSL **1033 + 175** Rust testi; workspace'te
18 atlanan canlı test; iki platformda üç clippy/fmt temiz. Arayüz **390
test**, 0 hata/uyarı, paket denetimi temiz. Bu arayüz düzeltmesinde gerçek
yalıtım/e2e yeniden koşmadı; Claude'un `017aacd` kaydı Windows AppContainer
uyumluluk 24/24 ve e2e 38/38, WSL yalıtım 26, uyumluluk 16/16 ve e2e 38/38'dir.

**429'da toplu istek küçültme (12 Ekim):** Gömme isteği hız sınırına
çarpınca `kalfa-rag` toplu parça sınırını yarıya indirir (256 → en az 16),
her başarılı istekte iki katına çıkarır; sınır `meta`'da isteğe bağlı
`toplu_sinir` alanında kalır. Windows 1092, WSL 1035 test; üç clippy/fmt
temiz; CI yeşil. Kullanıcının onayıyla squash birleşti
([#1](https://github.com/heimdilon/kalfa/pull/1), `1738b8e`). Canlı EVREN
koşusu yapılmadı.

**5a — Bağlam yönetimi (12 Ekim, tamamlandı):**
[Tasarım](superpowers/specs/2026-10-12-kalfa-5a-baglam-yonetimi-design.md),
[plan](superpowers/plans/2026-10-12-kalfa-5a-baglam-yonetimi.md). Bağlamın
tahmini istem tokenı 64 bini (ya da modelin ölçülen penceresinin %80'ini)
aşınca en eski araç çıktıları tek tek `qwen3.8-flash-next` ile özetlenir;
özet araç rolünde, aynı `tool_call_id` ve `özet:` etiketiyle kalır,
`BaglamOzeti` olarak kayda girer ve kurtarmada aynı bağlamı verir.
Özetleyici başarısız olursa yer tutucu konur. EVREN'in
`context_length_exceeded`'ı `EvrenError::BaglamAsimi` oldu; bir kez sıkı
özetlemeyle yeniden denenir. Kurtarma artık yalnız ana istekleri sayar
(denetçi isteği gönderim sayacını bozuyordu). Pencereler üç küçük canlı
istekle ölçüldü (262 144 / 524 288 / 1 048 576). Son incelemeden sonra özet,
çıktının `[güvenlik]`/`[denetçi]` satırlarını korur; günlük sınır, ağ yok,
anahtar ya da ikinci 429 gibi geçici hatalarda özetleme durur ve özgün
çıktılar kalır. Son PR başında Windows **1124 + 175**, Linux **1067 + 175**
test, 19 atlanan; iki platformda üç clippy/fmt temiz; arayüz 393 test, 0
hata/uyarı, build temiz. Canlı özet testi (`baglam_canli`) koşmadı; ayrı
onay ister.

Codex incelemesi (`f9ee299`): durdurucu bir hata gelince başlamış özetleme
istekleri artık bırakılmıyor, beklenip günlüğe yazılıyor; Windows
`kalfa-core` 434 test, clippy/fmt temiz. WSL Ubuntu 24.04 (`kalfatest`)
workspace 1067/0/19, düz ve e2e clippy/fmt temiz; son regresyon geçti.

[#2](https://github.com/heimdilon/kalfa/pull/2), kullanıcının Linux ve PR CI'ı
yeşil olduğunda birleştirme onayıyla, doğrulanan `9dfdde5` başına sabitlenerek
squash birleşti (`3133248`, 18:08 TSİ). Birleşen ağaç CI'da doğrulanan
ağaçla birebir aynı. 5a'nın Linux doğrulama kopyası ve yardımcı dosyaları
kaldırıldı; loglar yerelde korundu. Kullanıcının tercihiyle `5a/tasarim`
yerel ve uzak dalları korunur.

Birleşme sonrası [ana dal CI'ı](https://github.com/heimdilon/kalfa/actions/runs/38268030807)
da `3133248`'de dört işiyle yeşil.

**5b — Oturum bütçesi (12 Ekim, tasarım incelemesi):**
[Tasarım](superpowers/specs/2026-10-12-kalfa-5b-oturum-butcesi-design.md),
[plan](superpowers/plans/2026-10-12-kalfa-5b-oturum-butcesi.md). Ana model,
denetçi, özetleme ve eklentilerin her üretim denemesi aynı oturum kapısından
geçer; iç 429 tekrarları ayrı sayılır. Kullanıcı kararı: tüketim bildirilmiyorsa
yeni istekten önce durup sor. Başlatma izni/karar kalıcı yazılır; sahipli izin
bırakıldığında tüketim kaybolmaz. Bekleyen eklenti çağrısı yeniden yürütülmez.
Token eşiği önceden kesin harcama tavanı değildir.

Claude incelemesi (12 Ekim): çağrı/kayıt haritası koda uyuyor; prototip
Windows'ta yeniden 11/11. Kullanıcı kararları: varsayılan **200 istek /
5 000 000 bilinen token** (64 bin bağlam eşiğinde 1 milyon ~16 turda
biterdi); EVREN'in üretimden önce reddettiği denemeler (429, günlük sınır,
bağlam aşımı, anahtar/şart/model, 4xx, gövdesi gitmemiş bağlantı) istek
sayılır, tokenı bilinen 0'dır ve soru sordurmaz. Kayıtlı EVREN hata
yanıtlarında `usage` yok; katı sayım 5a'nın otomatik tekrarını her seferinde
soruya çevirirdi. Reddedilen isteğe kredi tutulmadığı canlı doğrulanmadı. Kabul modeli Rust 1.98.1
ile Windows ve WSL'de **11/11** geçti; ürün async/JSONL/IPC entegrasyonu
henüz uygulanmadı. [Ön doğrulama](superpowers/bulgular/2026-10-12-kalfa-5b-butce-on-dogrulama.md).

**5b uygulaması (13 Ekim):** Plan, kullanıcının talimatıyla Haiku 5.5
alt ajanlarıyla dalgalar hâlinde uygulandı; Claude inceledi, birleştirdi ve
doğruladı (dal `5b/uygulama`, `7db70c7`). Ana istek, denetçi, özetleme ve
eklenti her denemede oturum kapısından geçer; başlangıç ve karar kalıcı
yazılır; kullanımı bilinmeyen ya da sınırı dolan oturumda bütçe kartı
"Bütçeyle devam et / Turu durdur" sorar; kurtarma sayaçları sıfırlamaz;
Ayarlar'da yeni oturum varsayılanı. Windows Rust 1 396 / 0 / 19, Linux
1 150 + 189, Linux yalıtım 26 ve uyumluluk 16, iki platformda clippy/fmt
temiz, arayüz 420, e2e Windows ve Linux 41/41. Sapmalar tasarım §4.1 ve
§11'de. Not: kullanıcının iptal ettiği akıştan sonra (kullanımı bilinmez)
bir sonraki istekten önce bütçe kararı istenir.

**5b birleşti (13 Ekim):** PR [#4](https://github.com/heimdilon/kalfa/pull/4)
kullanıcının açık onayıyla squash birleşti (`1c3a6e1`). Codex incelemesinin
yedi bulgusundan dördü önce düşen testlerle düzeltildi (iptal sırasında
kuyruktaki bitiş, salt okunur açılışın bütçesi, eski kayıt devrinde
birikimli kullanım, kayıttan oynatılan karar); üçü gerekçeyle değiştirilmedi
(tasarım §11). Son kod Windows 1 209 + 190, Linux 1 152 + 190, yalıtım 26,
uyumluluk 16; CI yeşil.

**Linux paketleri (13 Ekim, PR [#6](https://github.com/heimdilon/kalfa/pull/6), `0e0949d`):** `.deb` ve
`.rpm` tek betikle (`kurucu/derle-linux.mjs`) üretilir. `.deb` Kalfa'nın
kendi bwrap kopyasını (`/usr/lib/kalfa/bwrap`) ve yalnız ona ad alanı
izni veren, başlattığı süreçleri yeteneksiz alt profile yığan AppArmor
profilini (`kalfa-bwrap`) taşır; sistem bwrap'ı etkilenmez. `.rpm` sistemin
`bubblewrap`'ına bağlıdır. Araç zinciri sistemdendir. WSL'de paket
mekaniği doğrulandı (içerik, izinler, bwrap denemesi, profil sözdizimi);
Ubuntu 24.04.5 VM'inde gerçek AppArmor kabulü 19/19 geçti (kurulum,
profiller, kopyanın yeteneksiz çocuğu, sistem bwrap'ının etkilenmediği,
yeniden kurulum, kaldırma; uygulamada ağsız ve ağlı komut). Sürüm paketleri
glibc nedeniyle Debian 12 tabanında derlenmeli.
[Tasarım](superpowers/specs/2026-10-13-kalfa-linux-paketleri-design.md).

Kullanıcının açık onayıyla squash birleşti; Codex incelemesinin üç bulgusu
(rpm WebKitGTK/GTK gereksinimi, kabul betiğinde veri dizini ve elle
denetim) birleşmeden önce düzeltildi.

**Linux sürüm derlemesi (13 Ekim, PR [#9](https://github.com/heimdilon/kalfa/pull/9), `fd42cb0`):**
`.github/workflows/linux-paket.yml` (sonra `surum.yml`) paketleri Debian 12 kabında derler
(en yeni glibc sembolü `GLIBC_2.34`), `SHA256SUMS` üretir, paketleri Debian
12, Ubuntu 24.04 (`.deb`) ve Fedora 44 (`.rpm`) kaplarında kurar, kitaplık
çözümünü denetler ve kaldırır; ilk koşu tümüyle geçti. `v*` etiketinde
taslak sürüm oluşturur; yayımlamayı kullanıcı yapar. Kullanıcının açık
onayıyla squash birleşti. Codex incelemesinin üç bulgusu birleşmeden önce
düzeltildi: etiket `tauri.conf.json` sürümüyle denetlenir, sürüm notu
paketteki bwrap kopyasının Debian sürümünü yazar, simge değişikliği de iş
akışını tetikler.

**Windows kurucusu sürüm akışında (14 Ekim, PR [#11](https://github.com/heimdilon/kalfa/pull/11), `bfc3f83`):**
İş akışı `surum.yml` ("Sürüm paketleri") adını aldı. Windows makinesinde
`kurucu/derle.mjs` NSIS kurucusunu derler; geçici CI makinesinde
`.github/scripts/windows-kurucu-sinama.ps1` sessiz kurar, dosyaları ve
yalıtımın kök izinlerini denetler, sessiz kaldırır ve izinlerin geri
alındığını doğrular. Etikette bütün paketler tek `SHA256SUMS` ile taslak
sürüme eklenir. Temiz makine, `derle.mjs`'in `kalfa-yalitim.exe`'yi
derlemediğini gösterdi; düzeltildi. Codex incelemesinin iki bulgusu (izin
hakkı ve miras denetimi, kaldırmada bütün dosyalar) birleşmeden önce
düzeltildi; koşu [38466119004](https://github.com/heimdilon/kalfa/actions/runs/38466119004)
tümüyle geçti. Kullanıcının açık onayıyla squash birleşti.

**RAG sonuçları dosyaya göre tekil (14 Ekim, PR [#13](https://github.com/heimdilon/kalfa/pull/13), `238b7c7`):**
`kalfa-rag` yeniden sıralamada bütün adayların sırasını alır ve her dosyayı
en iyi parçasıyla bir kez gösterir; liste sonraki dosyalarla dolar. Kosinüs
geri dönüşünde de aynı kural geçerlidir. G5 tam ölçümünde bu sayım 26/32
yerine 28/32 vermişti; canlı ölçüm yeniden koşulmadı (kredi, ayrı onay).
Tasarım §11.3 adım 5 ve §11.4. Codex incelemesinin bulgusu (ilk 50 aday tek
dosyadan gelirse sonuç istenenden aza düşüyordu) birleşmeden önce düzeltildi:
o durumda sonraki dosyaların en iyi parçası da adaya girer. Kullanıcının
açık onayıyla squash birleşti.

**Bağımlılık denetimi ve güvenlik politikası (14 Ekim, PR
[#14](https://github.com/heimdilon/kalfa/pull/14), `ee86d02`):** Üst belge
§12.1'in dağıtım satırındaki iki eksik: CI'a sabit sürümlü `cargo audit` işi
ve kökte `SECURITY.md`; README bildirimi bu dosyaya bağlar. Kullanıcının
isteğiyle repoda özel açık bildirimi açıldı. İlk denetim: açık yok; geri
çekilen `yoke-derive` 0.8.3 PR [#15](https://github.com/heimdilon/kalfa/pull/15)
(`22faf16`) ile 0.8.4'e güncellendi. Kalan uyarılar Tauri zincirinden
dolaylı: `glib` 0.18 unsound (RUSTSEC-2024-0429; Linux GTK, Tauri gtk 0.18'de
kaldıkça yükseltilemez), `proc-macro-error` ve beş `unic-*` bakımsız.

**Canlı düşünce kuyruğu (14 Ekim, dal `ui/canli-dusunce-kuyrugu`):** Windows
e2e (msedgedriver 155) PR #8 sonrası 40/41 verdi: "duraklayan arayüz" testi
kalıcı düşüyordu. Kök neden: PR #8'de düşünce paneli akarken açık kalıyor ve
her karede bütün düşünceyi Markdown'a çözüyordu; milyonlarca karakterde
arayüz kuyruğu eritemiyor, aktarım uyarısı 20 sn yerine ~77 sn'de geliyordu.
Akarken yalnızca son 2 000 karakter düz metin çizilir; tamamı akış bitince
açılınca bir kez çözülür. Bileşen testi önce düştü, sonra geçti; Windows e2e
**41/41**. Linux e2e bu düzeltmede koşmadı.

PR [#16](https://github.com/heimdilon/kalfa/pull/16) birleşti (`0eab672`).
Linux e2e (WSL Ubuntu 24.04, `kalfatest`, `xvfb-run`) bu başta **41/41**.

**Onaylı canlı işler (14 Ekim):** Kullanıcının onayıyla:
- `baglam_canli` geçti: 1 612 baytlık çıktı 247 bayta özetlendi; `E0308`,
  `lib.rs` ve satır/sütun korundu (1 121 token).
- Canlı G5, dosya tekilleştirmeyle **28/32 (%88)**. 20 aday en iyi durumda
  26/32 verdiği için **50 aday kalır**. Ayrıntı ve maliyet (~2,9 M token):
  [G5 tekil](superpowers/bulgular/2026-10-14-kalfa-g5-tekil.md).
- Değerlendirme düzeneğinin "İndeks eksik" döngüsü düzeltildi: durum artık
  yalnız başlıktan okunuyor (sonuç parçası bu metni içerebiliyordu).

**rustls ve ilk sürüm (14 Ekim):** Aynı gün yayımlanan dört `rustls`
0.23.45 duyurusu (RUSTSEC-2026-0346…0349) `cargo audit`'i kırdı;
[#18](https://github.com/heimdilon/kalfa/pull/18) 0.23.46'ya yükseltti
(`efa2ec2`). `v0.1.0` etiketi kullanıcının isteğiyle `efa2ec2`'ye atıldı.
`surum.yml` yeşil: Windows ve Linux derlemeleri ile Debian 12, Ubuntu 24.04,
Fedora ve Windows kurulum sınamaları geçti. Taslak sürümde `.deb`, `.rpm`,
NSIS kurucusu ve `SHA256SUMS` var.

Kullanıcı `v0.1.0`'ı ön sürüm olarak yayımladı (14 Ekim 18:14 TSİ).

**MCP 1.1a (14 Ekim):** Kullanıcı MCP desteğini seçti; yerel stdio (1.1a)
ve uzak HTTP/OAuth (1.1b) olarak bölündü. 1.1a tasarımı bölüm bölüm
onaylandı: [tasarım](superpowers/specs/2026-10-14-kalfa-11a-mcp-yerel-design.md).
Codex incelemesinin (`dd641a3`) altı bulgusu r2'de çözüldü:
- başlatma izni,
- uzun yaşayan süreçte koruma yenilemesi ve kalan risk,
- ham stdio sınırları,
- iptalde süreç ağacının durdurulması,
- tarif ve mühür,
- araç tanımı denetimi.

Bulgu 5'in bir alt maddesine kaynaklı gerekçeyle kısmen katılınmadı:
kısayollar aynı tarifle yeniden başlatmada düşmez. PR #19'daki 15 satır içi
Codex yorumu r3'te ele alındı:
- tam erişimde ağ süzgeci yok,
- iki MCP protokol dönemi,
- yenilenebilir araç kaynağı,
- `outputSchema` doğrulaması,
- yolların arayüzden gelmemesi ve yerel onaylı kayıt,
- açık seçilen çalışma kipi,
- ortam birleştirme,
- anahtarlı kimlik kasası.

Yanıtlar tasarımın §13'ünde.

Codex'in r3 yeniden incelemesinden (`5dd79a4`) gelen beş bulgu r4'te
çözüldü:
- iptal ve zaman aşımında süreç ağacı her durumda durdurulur,
- kalan riskin üst sınırı mutlak denetim yaşıyla tanımlandı ve stdout
  üzerinden EVREN'e çıkış yolu da riske yazıldı,
- çalışma alanındaki sunucu kodu için yazma kuşağı,
- araç tanımında sır geçerse araç kapatılır,
- kısayol ve başlatma izni politika özeti Π'ye bağlandı.

Codex'in r4 yeniden incelemesinden (`e5080c2`) gelen iki bulgu r5'te
çözüldü:
- Kod güveni artık modelin yazabildiği bütün kökleri (W) kapsıyor: proje,
  kabuğun özel ev, geçici ve önbellek dizinleri. Her sunucunun kendi özel
  kökü var; Windows'ta bu kök yetenek SID'iyle ayrılıyor.
- Doğrulanamayan koruma taraması (hata, eksik yürüyüş, 30 sn aşımı) t_K'yı
  yenilemiyor, süreci durduruyor.

Kullanıcı, iptal ve zaman aşımında süreç ağacının her durumda
durdurulmasını onayladı (14 Ekim). Yalnız G1'i yetkilendirdi. G1 Windows'ta
geçti: borulu AppContainer süreci, 62 sn yaşam ve Job Object ile ağacın
kapanışı, sunucuya özel yetenek SID'i ayrımı ölçüldü; temizlik doğrulandı.
[G1 raporu](superpowers/bulgular/2026-10-14-kalfa-11a-g1.md).

Kullanıcı 14 Ekim'de üç karar verdi:
- G1 sonucunu kabul etti.
- §4.3'teki kalan riski, güvendiği yerel MCP sunucuları için kabul etti.
- Plan 1.1a-1'in yazılmasını onayladı.

Plan şurada: [1.1a-1](superpowers/plans/2026-10-14-kalfa-11a-1-etkilesimli-surec.md).
Kapsamı:
- `kalfa-sandbox`'a etkileşimli süreç,
- sunucunun özel kökü ve Windows yetenek SID'i,
- G1 kontrollerinin ürün başlatıcısının kabul testlerine taşınması,
- Windows ve WSL doğrulaması.

Codex planı inceledi (`INCELEME-PLAN-9d6b6b5`, dört bulgu). Plan r2'de
dördü de işlendi:
- `kapat()` ağacın bittiğini doğrular ve `Result` döner. Linux'ta kök
  torunlarının alt süreç toplayıcısıdır. Linux tam erişimin kalan sınırı
  tasarım r6 §4.1'de açıkça yazıldı.
- Windows'ta ağlı sunucu köprüyle ve aynı Job Object'te başlar.
- Windows'ta sunucu kökü korumalı DACL'le kurulur.
- stdin'i bir aktarma görevi yazar; `kapat()` dışarıda tutulan yazıcıya
  rağmen stdin'i kapatır.

Etkileşimli ağ testleri iki platformda eklendi. Codex r2'yi inceledi
(`INCELEME-PLAN-6205095`) ve üç açık buldu; plan r3'te düzeltildi:
- İptal edilen ya da hata veren `kapat()` ve doğrudan bırakma: `Drop` stdin
  görevini keser, torunları ve ağacı öldürür.
- Linux torun taraması tam okunamazsa `Err` verir; torunlar `pidfd` ile
  açılıp başlangıç zamanıyla doğrulanır.
- Windows'ta var olan kök ilk değişiklikten önce denetlenir; ret hiçbir
  şeyi değiştirmez.

Plan kodu iki platformda yeniden ön doğrulandı
(`deneme/11a-1-on-dogrulama`, `0eab968`, yalnız yerel): Windows yalıtım
50/50, WSL yalıtım 32/32; AGENTS'ın bütün komutları iki platformda geçti.
[Ön doğrulama raporu](superpowers/bulgular/2026-10-14-kalfa-11a-1-on-dogrulama.md).
Bu işin Windows koşularının %TEMP%'e eklediği üç test dizini silindi; eski
44 dizin ayrı görev.

**Ürün uygulaması (Codex):** Kullanıcı 14 Ekim'de §4.1 için doğrudan
“Açık uyarıyla kabul et (önerim)”, sahiplik için “Codex devralsın (önerim)”
yanıtını verdi. Tam erişimde doğal çıkışın `setsid` torun bırakabilmesi
§4.3'ten ayrı kabul edildi; arayüz uyarısı tasarım §9'da. Son alt ajan
tercihi GPT-6 Luna / max; önceki Haiku seçiminin yerine geçer.

Codex'in r3 incelemesindeki kalan ara ebeveyn PID hatası düzeltildi.
Yaprağın pidfd'si ve köke kadar bütün ara ebeveynlerin başlangıç kimliği
doğrulanır; kayıp/değişmiş halka ilgisiz alt dalı sinyalden korur. Eski
algoritmayla iki yeni regresyon düştü; yeni algoritmayla geçiyor.

Ürün kodu `codex/mcp-etkilesimli` dalındaki `c1d464f` commit'inde.
Dokuz sandbox dosyası değişti; yeni bağımlılık yok. Ön doğrulanmış
0eab968 kodu ürün kaynaklarına taşındı; kalan PID düzeltmesi ayrı alt
ajana, bağımsız inceleme başka bir alt ajana verildi. KALFA-19 sahipliği
Codex'te. Windows ve WSL ürün doğrulaması tamamlandı:
Windows yalıtım 50/50, uyumluluk 24/24; WSL yalıtım 32/32, uyumluluk
16/16. Sandbox birimleri 109/109 ve 75/75; fmt, workspace testleri ve
iki clippy iki platformda geçti. Bağımsız incelemede açık ürün engeli yok.
[Ürün raporu](superpowers/bulgular/2026-10-14-kalfa-11a-1-urun.md).

**Wasmtime güvenlik yaması (14–15 Ekim):** CI, bugün yayımlanan 13
Wasmtime duyurusundan kırmızı oldu. Kullanıcı indirme/güncelleme ve
Windows/WSL doğrulamasını açıkça onayladı. İki mevcut sabitleme 49.0.3;
693 kayıt korunarak yalnız 29 Wasmtime/Pulley/Cranelift aile üyesi
patch sürümü değişti. Claude'un yayımladığı `2203bf9`, Codex'in yerel
`f6be234` karşılığıyla birebir; yayımlanan commit korundu.

Codex doğrulaması: eklenti iki platformda 84/84, workspace Windows
1.234/0/19 ve WSL 1.187/0/19, e2e özellikli Rust 191/0/0. fmt,
workspace/WASM/e2e özellikli clippy iki platformda geçti. Audit işi
13 Wasmtime bulgusunu artık göstermiyor; mevcut 7 uyarı aynı.
`2203bf9` [CI koşusu](https://github.com/heimdilon/kalfa/actions/runs/38634667316)
beş işiyle yeşil. Son belge başının denetimleri PR üzerinden izlenir.
[Dar yama raporu](superpowers/bulgular/2026-10-14-wasmtime-4903-guvenlik-yamasi.md).
Bu koşunun Windows test kökü (230 artık) ve WSL kaynak kopyası da silindi.

**1.1a-1 birleşti (15 Ekim):** PR #19, kullanıcının açık squash onayıyla
`cf4e5f3` olarak main'e birleşti; birleşen ağaç doğrulanan `7a2dbf8` ile
birebir aynı. PR CI `38640288792` beş işiyle yeşil. Codex'in ürün
worktree'si arşivlenip kaldırıldı; KALFA-19 Bitti.

**1.1a-2 ürün uygulaması (15 Ekim, Codex; PR #20 taslak):**
[12 görevli planın](superpowers/plans/2026-10-15-kalfa-11a-2-mcp.md)
inceleme ve G2 kapısı tamamlandı; kullanıcı devam talimatı verdi.
Görev 1 (KALFA-21) `kalfa-mcp` ürün crate'ini ekler: rmcp 3.5.1 istemcisi,
ayrıştırma öncesi 4 MiB sınırı, 64 öğeli kuyruklar, ölümcül hata kanalı,
abonelik yaşamı, cache/log kapıları ve gerçek yerel test sunucusu.
Bağımsız incelemedeki iki kapanış bulgusu önce düşen testlerle düzeltildi.
MCP henüz araç defterine/arayüze bağlı değildir.

Windows ve WSL hedefli **37/37**; workspace Windows **1271/0/19**,
WSL `kalfatest` **1224/0/19**; client-only derleme, workspace clippy/fmt
iki platformda temiz. [Ürün doğrulama raporu](superpowers/bulgular/2026-10-15-kalfa-11a-2-gorev1-urun.md).
Bu görevde gerçek yalıtım, arayüz ve WebDriver yeniden koşmadı; sonraki
görevlerin kabul kapıları korunur. Canlı EVREN ya da PR merge'i yok.

Kullanıcının rmcp indirme ve “Yerel onaydan sonra dosya özeti alınsın”
kararları geçerli; tasarım r9 §13.8 kaynaklıdır. Kayıt onayı öncesinde
argüman/ortam dosyası içeriği okunamaz.

**Görev2 tamamlandı (15 Ekim, `c4f12ab`):** Tanım, son ortam, W kökleri,
seçici kimliği ve T/Π/K/P/N/R/D tipleri; native kayıt onayından sonra
akış içerik özeti eklendi. NODE_OPTIONS, NODE_PATH ve Windows junction/..
karşı örnekleri önce düştü, düzeltildi; son bağımsız inceleme kapalı.
Windows workspace **1300/0/19**, WSL **1254/0/19**; e2e özellikli Rust
**195/195** iki platformda. Client-only lib+bins, iki clippy/fmt temiz.
[Görev2 raporu](superpowers/bulgular/2026-10-15-kalfa-11a-2-gorev2-urun.md).
Görev1'in `1ede4c3` CI38745987451 koşusu yedi işiyle yeşil; yeni başın
CI sonucu ayrıca izlenir. Gerçek yalıtım/UI/WebDriver bu görevde koşmadı.

**Görev 3–5 tamamlandı (16 Ekim):** Antigravity'nin ayar/kasa/maske ve
ortak koruma uygulamaları `e2962a9` / `e75822f`. Kullanıcının doğrudan
devriyle Codex (KALFA-24), süreç yöneticisini bağımsız inceleme ve önce
düşen regresyonlarla tamamladı: RPC/başlatma/drop yarışları, gerçek
zamanlayıcı, korumalı kök ve yazma sonrası denetim beklemesi düzeltildi.
Windows/WSL workspace **1354/1309**, e2e özellikli Rust **234/235**;
gerçek yalıtım **50/32**, uyumluluk **24/16**, client/clippy/fmt temiz.
[Görev 5 raporu](superpowers/bulgular/2026-10-16-kalfa-11a-2-gorev5-urun.md).
İki eski A15 ret beklentisi indirmeyen gerçek arşiv açma testine çevrildi;
son koşuda yeni paket gerekmedi. Bu Rust kontrolleri UI/WebDriver değildir.

**Görev 6 tamamlandı (16 Ekim):** Gerçek actor izin matrisi, R/D mührü,
T/D/Π kısayolu ve manager-owned çağrı kilidi bağlandı. Cleanup/drop/hata
yolları doğrulanan kapanış ve son taramayı bekler; hatada alan engellenir.
İptal bildirimi flush makbuzu alır; aynı iki saniyelik kapanma payı korunur.
Bağımsız bulgular kapalı. Windows/WSL workspace **1389/1344**,
e2e özellikli Rust **250/251**, gerçek yalıtım **50/32**,
uyumluluk **24/16**, gerçek stdio kabulü **4/4** iki platformda; clippy/fmt temiz.
[Görev 6 raporu](superpowers/bulgular/2026-10-16-kalfa-11a-2-gorev6-urun.md).

**Görev 7 tamamlandı (16 Ekim):** MCP araç tanımı sır taraması (5 konum, yalnız
sıra numarasıyla ret), boyut/UTF-8/görünmez Unicode/harici `$ref` ve `$id`
kapıları, `D` özeti (çıkış şeması dahil), `(D, denetçi_modeli)` kalıcı önbelleği,
ilk sunan oturumun `ButceTalepci`/iptal bağlantısı ile sonuç/hata/stderr
maskeleme ve `outputSchema` doğrulama kapıları bağlandı. Ayrıca yönetici
olmayan Windows CI kullanıcısında görülen `gercek_surec_testleri` derleme kilidi
ve DACL okuma bayrağı düzeltildi. Bağımsız bulgular kapalı. Windows/WSL
workspace **1427/1382**, e2e özellikli Rust **255/256**, WSL gerçek yalıtım
**32/32**, uyumluluk **16/16**, gerçek stdio kabulü **4/4** iki platformda;
clippy/fmt temiz.
[Görev 7 raporu](superpowers/bulgular/2026-10-16-kalfa-11a-2-gorev7-urun.md).

**Görev 8 tamamlandı (17 Ekim, Claude):** Her model turu başında tek araç
görüntüsü (`AracAnlikGoruntusu`) alınır; istek, `AracTanimlari` günlüğü,
429/413 tekrarları, ad çözümlemesi ve argüman doğrulaması aynı görüntüyü
kullanır. Core `AracKaynagi` dikişi `kalfa-mcp`'yi tanımaz; tanım denetimleri
yalnız aktif turda, oturumun bütçesiyle bağımsız görev olarak sahiplenilir ve
oturumu kilitlemez. Uygulama alan başına `McpAlanAracKaynagi` yayımlar (tek
sahip, sonuçsuzda geri verme, nesil). Keşfin bu kaynağa bağlanması Görev 9.
Windows/WSL workspace **1438/1393**, e2e özellikli Rust **260/261**;
fmt/clippy temiz.
[Görev 8 raporu](superpowers/bulgular/2026-10-17-kalfa-11a-2-gorev8-urun.md).

**Görev 9 tamamlandı (17 Ekim, Claude):** Keşif zinciri alan kaynağına
bağlandı: her model turu başında etkin sunucular başlatılır/doğrulanır, ilk
keşifte ya da `R` değişince liste denetlenir, yoksa gerekiyorsa yenilenir;
`McpArac`'lar yayımlanır. Başlamayan ya da kapatılan sunucunun araçları düşer;
izinsiz sunucu turu bekletmez ve başlatma kartı birikmez. Yarım MCP çağrısı
kurtarmada yeniden çalıştırılmaz. Ayrıca G7'nin görünmez Unicode kapısı
açıklama ve şemalara genişletildi (`c8a8a39`). Windows/WSL workspace
**1445/1400**, e2e özellikli Rust **265/266**; fmt/clippy temiz.
[Görev 9 raporu](superpowers/bulgular/2026-10-17-kalfa-11a-2-gorev9-urun.md).

**Görev 10 tamamlandı (17 Ekim, Claude):** Ana pencereye 15 MCP komutu
(durum, ayrıntı, komut seç, taslak, içe aktarma, etkinlik, sil, yeniden
başlat, başlatma kartı yanıtı, izin geri alma) ve sıralı durum kanalı
(`mcp_dinle`, artan `revizyon`) eklendi; komut adları, capability ve TS
sözleşmesi aynı commit'te. Kayıt ve silme yerel pencereyle onaylanır; tam
erişimli başlatma arayüzün "kabul"üne ek olarak yerel onay ister; arayüz yol
göndermez. Başlatma artık tura bağlı değil: kısa turda kartın kaybolması
düzeltildi. Windows/WSL workspace **1447/1402**, e2e özellikli Rust
**267/268**, arayüz 0/0 ve 435/435; fmt/clippy temiz.
[Görev 10 raporu](superpowers/bulgular/2026-10-17-kalfa-11a-2-gorev10-urun.md).

**Görev 11 tamamlandı (17 Ekim, Claude):** Ayarlar'da Eklentiler'den
ayrı MCP sunucuları bölümü eklendi: rozet ve açıklaması, alan başına durum,
araç sayısı, izin geri alma ve yeniden başlatma. Form yalıtımlı başlar;
Gözat kimlik döner. Tam erişimde ağ alanı gizlenir ve kalıcı uyarı çıkar.
Gizli değer parola alanındadır ve geri gelmez. JSON içe aktarmada metin
gönderilince silinir. Ayrıntıda tanım denetimi durumları görünür.
Sohbette model dışı başlatma kartı yalnız aynı alanın oturumunda çıkar ve
sohbeti bekletmez. Onay sayfasında MCP kaynağı, rozet ve erişim satırı;
MCP kısayolu ve araç kartı rozeti eklendi. Arka uca üç sözleşme alanı
eklendi: `OnayOgesi.mcp`, kartın `kip`'i ve
`OturumAcilisi.calisma_alani_kimligi`. Arayüz 0/0 ve **459/459**;
Windows/WSL workspace **1447/1402**, e2e özellikli Rust **267/268**;
fmt/clippy temiz.
[Görev 11 raporu](superpowers/bulgular/2026-10-17-kalfa-11a-2-gorev11-urun.md).

**Görev 12 tamamlandı (17 Ekim, Claude):** Test sunucusu gerçek
yalıtımda (AppContainer, bwrap) ağsız/ağlı/sonradan gelen gizli dosya
senaryolarıyla sınandı; tam erişime çevrilince üçü de düştü. §11 matrisi
testlere bağlandı. Sonradan gelen gizli dosyanın çağrı sırasında sonuç
yoluyla sağlayıcıya ulaştığı (kabul edilmiş risk), çağrılar arasında
tanım yoluyla ulaşmadığı ölçüldü. Kabul testi bir eşzamanlılık hatası
buldu: süren koruma taraması başlatma doğrulamasını düşürüyor, araçlar
sonraki turda yayımdan çıkıyordu; bekleyen denetimle düzeltildi. Gerçek
WebDriver e2e Windows/WSL **43/43** (yeni `13-mcp`); workspace
**1454/1410**, e2e özellikli Rust **273/275**, MCP yalıtımı 3/3;
fmt/clippy temiz. PR incelemesindeki yedi bulgu (izin kalıcılığı, uyuyan
izin, silme/yeniden adlandırma temizliği, kasa temizliği, içe aktarma ve
form) testleriyle düzeltildi.
[Görev 12 raporu](superpowers/bulgular/2026-10-17-kalfa-11a-2-gorev12-kabul.md).

**1.1a-2 birleşti (17 Ekim):** PR #20, kullanıcının açık squash onayıyla
ve CI'ın yedi işi yeşilken birleşti: main `bfe198a`. Birleşmeden önce Codex
PR incelemesinin yedi bulgusu `20ec2ce` ile düzeltildi. PR worktree'si ve
WSL kopyası kaldırıldı. **Yerel MCP (1.1a) tamamlandı.**

**Masaüstü arayüzü tamamlandı (18 Ekim, KALFA-25):** Kullanıcının ilk
denemesindeki geri bildirimle şunlar birleşti:
- açılır liste düzeltmesi (#23);
- araç kartında dosya diff'i (#25);
- konuşmadaki onay kartı (#26);
- Windows'ta Kalfa'ya özel başlık çubuğu (#27);
- sağ panel: Ortam, Değişiklikler, Dosyalar (#28);
- kalan üç ayrıntı (#29, main `756c613`).

Dosyalar bölümü salt okurdur ve yol değil oturuma bağlı kimlik alır.
[Tasarım](superpowers/specs/2026-10-18-kalfa-masaustu-arayuzu-design.md),
[plan](superpowers/plans/2026-10-18-kalfa-masaustu-arayuzu.md). Terminal,
tarayıcı, yan sohbet ve plan bölümleri sonraya kaldı.

**`git_koruma` ağ yolu sertleştirmesi (18 Ekim):** Masaüstü arayüzünün
güvenlik incelemesi salt okur dosya görünümünde yakalanan açığın aynısının
`kalfa-core`'un `git_koruma`'sında durduğunu göstermişti. Çalışma alanından
okunan git hedefleri (`.git` dosyasının `gitdir:` satırı, `commondir`,
git dizininin `core.worktree`'si, `core.hooksPath` ve `goreli_yap`'taki ara
bağlantı hedefleri) doğrudan birleştirilip `canonicalize` ediliyordu; hedef
UNC (`\\pay\…`), `\\?\` ya da aygıt yazımındaysa Windows bu yolu açarken
ağa kimlik bilgisi gönderebiliyordu.

Artık tek bir `yerel_hedef_mi`/`yerel_birlestir` çifti bütün bu noktalarda
hedefin kendi önekini bir **izin listesine** göre denetler: yalnız göreli
yollar ve sürücü harfi yazımları (`C:\…`, `\\?\C:\…`) açılır. Ağa
çıkabilen bütün önekler — `\\pay\…`, `\\?\UNC\…`, `\\.\UNC\…`,
`\\?\GLOBALROOT\Device\Mup\…` — reddedilir ve dosya sistemine hiç
dokunulmaz. Ayrıca `git config` çalıştırılmadan önce git'in depo keşfinde
izleyeceği zincir (`.git`'in `gitdir:` satırı, sonra onun `commondir`'i)
aynı listeden geçirilir; zincirde reddedilen bir yol varsa git hiç
çalıştırılmaz. `include.path` çözümü **kapatılmadı**: `--no-includes`
denendi ve geri alındı, çünkü include edilen dosyadaki `core.hooksPath`
görülmeyince o kanca dizini korumasız kalıyordu.

Reddedilen hedefte **kapalı düşülür**: dosya araçlarının korumasını
hesaplayan `goreli_yap` çalışma alanının tamamını korur. "Reddedilen yol
zaten çalışma alanının dışındadır" varsayımı yanlıştı; hem çalışma alanı
bir paylaşımda olabilir hem de aygıt ad alanı alanın içini başka yazımla
adlandırabilir.

Bilinen sınırlar modül belgesinde: yoldaki bir bağlantının ağ paylaşımını
göstermesi, ağa bağlanmış sürücü harfi (`Z:`) ve reddedilen hedefli bir
`.git` dosyasının yönlendirdiği dizinin yalıtımda salt okunur
yapılmaması — sonuncusunu `kalfa-sandbox`'ın `yeni_git_denetle` izin
listesi karşılıyor: yalıtımda yeni oluşturulan `.git` dosyası ve
`commondir` zaten tehlikeli sayılıyor.

Üç yeni test: izin listesinin kendisi (birim; dolaylı ağ yazımları dahil),
yerel `\\?\C:\…` yönlendirmesinin yalıtımda salt okunur kalması ve ağ
hedefinde korumanın alanın tamamına düşmesi. Windows workspace **1479/0/19**,
Ubuntu 24.04 (WSL) workspace **1431/0/19**; iki platformda workspace clippy
ve `cargo fmt` temiz. Güvenlik sınırına dokunduğu için gerçek yalıtım da
koştu: Windows AppContainer uyumluluk **24/24**, Linux yalıtım **32/32** ve
uyumluluk **16/16** (köprü derlendi, `köprü bulunamadı` yok),
`yalitim-testleri` özellikli clippy temiz. Uçtan uca takım bu değişiklikte
koşmadı.

**Bütçesiz oturum ve compact (19 Ekim, KALFA-27, PR [#32](https://github.com/heimdilon/kalfa/pull/32)):**
[Tasarım](superpowers/specs/2026-10-19-kalfa-compact-butcesiz-design.md),
[plan](superpowers/plans/2026-10-19-kalfa-compact-butcesiz.md). Oturum
bütçesinin istek/token sınırları, bilinmeyen kullanımda durdurma ve bütçe
karar kartı kaldırıldı; deneme defteri (`ButceTalepci`, `ButceDenemesi`,
gönderim günlüğü satırları ve sayaçlar) korundu. 5a'nın çıktı başına
özetlemesinin yerini bütün geçmişi özetleyen otomatik (`T > 0,9 × min(pencere(model), etkin_pencere)`),
zorunlu (`BaglamAsimi`) ve `/compact [talimat]` ile elle compact aldı. Özet
oturumun kendi modeliyle üretilir, `<oturum_ozeti sinir="…">` sarmalında
kullanıcı rolünde veri olarak sarılır ve `OturumOzetlendi` kaydıyla
kurtarmada bayt bayt aynı bağlamı verir. Eski `ButceAyarlandi`,
`ButceDevamKarari`, `BaglamOzeti` kayıtları ve `oturum_butcesi` ayar alanı
uyumluluk için okunur. Arayüzde bütçe kartı ve ayarı kalktı; durum çubuğuna
sınırsız `İstek · Token` ve `Bağlam %N`, konuşmaya açılır özet bölücüsü,
composer'a `/compact` önerisi ve Ayarlar'a etkin bağlam penceresi (varsayılan
200 000, alt sınır 32 000) eklendi.

Windows doğrulaması (Antigravity): workspace **1 453 / 0 / 19**, e2e
özellikli Rust **282 / 0 / 0**, `kalfa-mcp --lib --bins` check, üç clippy
(workspace, `e2e` özellikli, `wasm32-wasip2` eklentiler) ve `cargo fmt`
temiz; arayüz `npm run check` 0 hata/0 uyarı, Vitest **488 / 488** (32
dosya), `npm run build` ve paket denetimi temiz. WebDriver e2e'de yeni
`12-butce.test.mjs` (`/compact` senaryosu) **1/1**, güncellenen
`02-konusma.test.mjs` **4/4** ve `04-kabul.test.mjs` **8/8** dahil **37/41**
geçti; kalan 4 alt test (`05-kabuk`, `09-ag-cikisi`) Antigravity'nin
etkileşimsiz işçi masaüstünde iç içe AppContainer `node.exe` (`0xC0000142`)
sınırı nedeniyle kurulu makinede Claude tarafından koşturulacaktır. WSL
doğrulaması ayrı kopya/hedef dizini gerektirdiğinden Claude incelemesine
bırakıldı.

Claude incelemesi ve doğrulaması (19 Ekim, `926b136`): kapsam, sarmal,
kurtarmada ortak `compact::uygula` ve sistem isteminin `oturum_ozeti`
kuralı plana uygun. İnceleme düzeltmeleri: bağlam yüzdesi %100 ile sınırlı
(testli), sınıf gövdesindeki fazla `;` kalktı, etkin pencere sabitleri
`kalfa-uygulama`'da çekirdekten okunur. Windows: workspace **1 458 / 0 / 19**,
e2e özellikli **282 / 0 / 0**, fmt, üç clippy, `kalfa-mcp` check temiz;
arayüz **489 / 489**, check 0/0, build temiz; WebDriver e2e **41/43**.
WSL Ubuntu 24.04 (`kalfatest`): workspace **1 410 / 0 / 19**, e2e özellikli
**284 / 0 / 0**, fmt, üç clippy, `kalfa-mcp` check ve köprü temiz; arayüz
**489 / 489**; WebDriver e2e **40/42**.

Düşen e2e'ler bu PR'dan değil, ikisi de tabanda aynı biçimde düşüyor:
Windows'ta `05-kabuk` iptal ve `09-ag-cikisi` S13, AppContainer'da `node`
başlatılamıyor (`Erişim engellendi (os error 5)`); aynı iki test
`main` kodunda da aynı hatayla düştü. WSL'de `04-kabul` gecikme p95'i
50 ms eşiğinin kıyısında dalgalanıyor: aynı makinede art arda taban
(`47f3abc`) ve PR başı üçer koşuda üçer düşüş verdi (p95 50–58,6 ms).
Kök nedenler ([e2e ortam bulguları](superpowers/bulgular/2026-10-19-kalfa-e2e-ortam.md)):
Windows'ta 0.1.1 yükseltmesi `C:\Program Files\nodejs` ve `C:\Python314`
araç zinciri izinlerini tasarım gereği geri aldı; "Onar" ya da
`kalfa-yalitim izin-ver` ile açılır (kullanıcı, yönetici). Linux'ta gerileme
yok: Windows'ta yük yokken 14 Ekim `main`'i, taban ve PR başı p95
35–46 ms ile geçiyor; düşüşler eşzamanlı yükten. Makinedeki yedi artık
`Kalfa.Alan.*` profili (17 Ekim) kullanıcı onayıyla silinecek.

**Sıradaki tek iş:** Kullanıcının açık onayıyla PR
[#32](https://github.com/heimdilon/kalfa/pull/32) squash birleştirilir.
**Engel:** Merge onayı.
**Devam koşulu:** Kullanıcı "birleştir" der; ardından worktree
(`C:/Users/kor_a/kalfa-compact`) ve WSL kopyası
(`/home/kalfatest/kalfa-compact*`) silinir.

**Açık depo ve CI (12 Ekim):** Depo `heimdilon/kalfa` adıyla herkese açık
oldu; varsayılan dal `main`. Geçmiş taşındı; PR #1–#53 ve eski CI kayıtları
özel arşivdedir (`heimdilon/kalfa-arsiv`), belgelerdeki eski PR/CI
bağlantıları oraya gider. Özel depoda GitHub Actions dakikası dolmuştu;
açık depoda standart çalıştırıcılar ücretsizdir. CI kapsamı ve iki platform
şartı değişmedi.

**Birleştirme:** #52 ve #53 kullanıcının ayrı açık onaylarıyla,
doğrulanan başlara sabitlenerek squash birleşti; birleşen ağaçlar CI'da
doğrulanan ağaçlarla birebir aynıdır.

**Korunan veri:** 4c indeksleri (ilk indeks, kesinti öncesi kopya,
değerlendirme), G5 makbuz ve logları ile 4d'nin Windows ve WSL inceleme
logları `C:\Users\kullanici\kalfa-yedek\asama-4-veri-20261012`'dedir.

**Sonraki iyileştirme adayları:** Sonuçları dosyaya göre tekilleştirmek
(G5'te +2 sorgu; 14 Ekim'de uygulandı); Linux paketlemesi (13 Ekim'de
eklendi).

Ayrıntılı yerel handoff: `C:\Users\kullanici\kalfa-yedek\HANDOFF-asama-4.md`;
[PR handoff'u](https://github.com/heimdilon/kalfa-arsiv/pull/52#issuecomment-6078860023).
Aşağıdaki tarihli doğrulamalar geçmişi korur; güncel devam yukarıdadır.

## Mevcut uygulama

- **1a — EVREN istemcisi:** `kalfa-evren`; istek/yanıt tipleri, SSE ve araç
  çağrısı akışı, tipli hatalar, şart uçları, kredi gözlemi ve fixture testleri.
- **1b — çekirdek:** `kalfa-core`; kimlikler ve yol doğrulama, dört izin kipi,
  şema doğrulama, mühürlü onay, çalışma alanı ortak kilitleri, JSONL kayıt ve
  kurtarma, sağlayıcı soyutlaması ve UI'sız oturum döngüsü. Araç aşaması
  (izin, mühürlü onay, kilitli paralel yürütme), iptal (akış/onay/hazırlık/
  yürütme sırasında), tur ve tekrar sınırları, hız sınırında tek yeniden
  deneme, şart akışı ve kayıttan devam tamamlandı.
- **1b — dosya araçları:** `kalfa-tools`; `dosya_oku`, `dosya_listele`,
  `icerik_ara`, `dosya_yaz`, `dosya_duzenle`. Çalışma alanına `cap-std` dizin
  tutamacıyla erişilir; üst dizin ve dosya bileşenlerinde sembolik
  bağlantılar (iç ya da dış, ayrıca Windows junction) izlenmez; yazma/
  düzenleme onaydan sonra içerik ya da hard link sayısı değişirse çalışmaz.
- **1c — masaüstü uygulaması:** `kalfa-uygulama`, Tauri kabuğu `kalfa`,
  Svelte arayüzü `ui/`; ayrıntısı aşağıda.
- **2a — kabuk aracı:** `kabuk` (`kalfa-tools`) ve komut çalıştırma, süreç
  sınırları (`kalfa-sandbox`). Ayrıntısı aşağıda.
- **2b — Linux yalıtımı:** kabuk komutları Linux'ta bubblewrap ve seccomp ile
  yalıtımda çalışır (`kalfa-sandbox`); Ayarlar'da yalıtım durumu ve araç
  zinciri dizinleri. Ayrıntısı aşağıda.
- **2c — Windows prototipi:** ürün kodu yok; ölçüm aracı `prototip/windows-yalitim/`.
  Kısıtlı belirteç yolu Dal B'de kaldı; AppContainer ek ölçümüyle Dal A'ya
  ulaşılıyor. Ayrıntısı aşağıda.
- **2d-1 — Windows arka ucu:** `kalfa-sandbox`'ta AppContainer yalıtımı,
  yönetici yardımcısı `kalfa-yalitim.exe`, çalışma alanı hazırlığı ve ACE
  kaydı. Kurulumu yapılmış makinede uygulama kabuk komutlarını yalıtımda
  çalıştırır; kurulumu uygulamadan başlatmak ve Ayarlar'ın Windows bölümü
  2d-2'dedir. Ayrıntısı aşağıda.
- **2d-2 — son kullanıcı akışı (tamamlandı):** yalıtım yokken kapalı kabuk,
  sade ileti ve BT raporu, yalıtımsız anahtarı, "Onar", Kalfa Projeleri
  klasörü, Ayarlar'da hazırlanmış alanlar, Windows kabuk e2e'si gerçek
  yalıtımda ve makine geneli NSIS kurucusu. Kurucu kurulu makinede UAC ile
  kabul edildi. Ayrıntısı aşağıda.
- **2d-3 — Kalfa'nın getirdiği araç zinciri (tamamlandı; kurucu UAC ile
  kabul edildi):** kurucu Python ve Node'u sabit sürüm ve SHA-256'yla getirir,
  kabın `PATH`'inde önce gelirler; yöneticiye ait araç dizini UAC ile açılır,
  Onar yükseltmede kaybolan izinleri geri getirir; Windows'ta ağsız cargo
  geliştiricinin registry'siyle derler; sınır matrisi. Ayrıntısı aşağıda.
- **Güvenlik sertleştirmesi (PR #16):** [güvenlik analizinin](superpowers/bulgular/2026-09-27-guvenlik-analizi.md)
  1–7. bulguları: `.git`'in tamamı korumalı ve yeni `.git` karantinası,
  kendiliğinden çalışan yapılandırma sınıfı, kabuk onayında onaysız
  değişiklikler, genişleyen gizli kalıplar, araç çıktısının rastgele sınırlı
  sarmalı, yönetici yardımcısının kayıt dizini denetimi ve görünmez
  karakterler. Ayrıntısı aşağıda.
- **3a — tur özeti ve geri alma:** PR #19 (`kalfa-asama-1`'de birleşti). Tur
  özeti, dosya ve komut meta veri yakalamaları, oturum silme.
- **3b — gönderim günlüğü ve önizleme (uygulandı; PR #22):** İstek denemelerinin
  kaydı (`IstekSonucu`, `IstekAsamasi`), kaynak etiketleri, sistem iletisi ve
  araç tanımlarının kaydı, göndermeden önce göster (`Komut::OnizlemeAyarla`,
  `Olay::GonderimOnizlemesi`, `Komut::GonderimKarari`), onay kartında veri çıkışı
  gösterimi, arayüz (`GonderimGunlugu.svelte`, `OnizlemeKarti.svelte`), S12 ve
  e2e testleri. G8 ölçümü kapandı. Ayrıntısı aşağıda.
- **3c — senaryolar ve güvenlik denetçisi (uygulandı; PR #47):** S1–S12
  prompt injection senaryoları; EVREN üzerinde denetçi (`qwen3.8-flash-next`,
  JSON şema); kapalı, uyar ve katı kipleri; kip ayarı (`ayarlar.json`,
  düşürmek onay ister); araç kartında denetim rozetleri; katı kipin kartı.
  G3 ve G4 ölçümleri kapandı. Ayrıntısı aşağıda.
- **3d — ağ çıkışı (uygulandı; PR #20):** ağlı yalıtımlı komut yalnızca
  izin listesindeki hostlara, Kalfa'nın vekili üzerinden çıkar. Linux'ta
  bwrap `--unshare-net` ve `SCM_RIGHTS` köprüsü; Windows'ta `internetClient`'sız
  AppContainer ve adlandırılmış boru köprüsü. Ayrıntısı aşağıda.
- **3e — küçük işler (uygulandı; PR #23):** "bu oturumda izin ver"
  kısayolları, ek gizli kalıplar ve gizli sayılmayanlar (K7), `ayarlar.json`'da
  gizli kalıplar ve ağ listesi, onay kartının "Kapsam" satırı ve Ayarlar
  sayfası. G9 kapandı. Aşama 3'ün son planıdır. Ayrıntısı aşağıda.
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
  gerçek EVREN'e karşı `#[ignore]` testte de var (spec §4.2).

1b uygulama planındaki (`docs/superpowers/plans/2026-09-21-kalfa-1b-cekirdek.md`)
15 görevin tümü ayrı commit'ler halinde repoya işlendi (`git log --oneline` ile
görülebilir). Plandaki kutular tarihsel uygulama adımlarını tarif eder; bu
geçişte her göreve karşılık gelen kodun ve testlerin plan niyetini karşıladığı
yeniden doğrulandı (bkz. Doğrulama).

## 1b doğrulama kaydı

Windows üzerinde Rust 1.98.1 ile (bu geçişte yeniden koşturuldu):

| Kontrol | Sonuç |
| --- | --- |
| `cargo test --workspace --quiet` | 173 geçti, 0 başarısız, 4 canlı test atlandı |
| `kalfa-core` | 95 test geçti |
| `kalfa-evren` | 51 test geçti, 3 canlı test atlandı |
| `kalfa-tools` | 27 test geçti, 1 canlı test atlandı |
| `cargo clippy --workspace --all-targets -- -D warnings` | Temiz |
| `cargo fmt --all -- --check` | Temiz |
| `git diff --check` | Temiz (yalnızca LF/CRLF uyarıları) |
| `cargo tree -p kalfa-core -e normal \| grep -E "kalfa-tools\|tauri"` | Boş — çekirdek `kalfa-tools`'a ya da bir UI kütüphanesine bağlı değil |
| `git status research/` | Temiz — fixture'lar değişmedi |
| `git grep` ile kütüphane kodunda `unwrap()`/`expect()` taraması (`test_destek.rs` hariç, `--untracked` dahil) | Sonuç yok |

Windows symlink, bağlantılı üst dizin, hard link ve junction testleri
atlanmadan geçti (`baglantili_ust_dizinden_git_hook_yazilamaz`,
`junction_uzerinden_git_hook_yazilamaz`, `hard_link_onaydan_sonra_eklenirse_*`,
`baglanti_takma_adlari_serbest_kipte_bile_yetki_atlatamaz` dahil).

**Linux doğrulaması yapıldı.** Ubuntu 24.04.5 LTS (WSL2, `Ubuntu-24.04`
dağıtımı, native ext4 — Docker Desktop'ın iç VM'i değil), Rust 1.98.1
(`rustc 1.98.1 (48a229cea 2026-09-01)`, workspace'in `rust-toolchain.toml`
pinini karşılıyor), repo `/root/kalfa`'ya `git clone` ile kopyalanarak:

| Kontrol (Linux) | Sonuç |
| --- | --- |
| `cargo test --workspace` | 172 geçti, 0 başarısız, 4 canlı test atlandı |
| `kalfa-tools` | 26 geçti, 1 canlı test atlandı |

Windows'a göre tek fark: toplam 173 yerine 172 — `junction_uzerinden_git_hook_yazilamaz`
`#[cfg(windows)]` ile işaretli olduğu için Linux derlemesine hiç girmiyor
(beklenen davranış, eksik/başarısız değil). Diğer bütün testler, canlı
EVREN testleri hariç, atlanmadan koştu ve geçti. Unix'e özgü bağlantı
senaryoları ada göre doğrulandı (`cargo test -p kalfa-tools`, `--quiet`
olmadan, tek tek test adlarıyla):

- `sembolik_baglanti_ile_disari_cikilamaz` — dış symlink ile çalışma
  alanından çıkılamıyor
- `ic_baglanti_gizli_dosyayi_okutamaz_ve_aramaya_sokamaz` — iç symlink
  gizli dosyayı okutmuyor/aratmıyor
- `okuma_hazirligindan_sonra_baglanti_konursa_acilmaz`,
  `yazma_hazirligindan_sonra_konulan_baglanti_izlenmez` — onay/hazırlıktan
  sonra konan bağlantı izlenmiyor
- `baglantili_ust_dizinden_git_hook_yazilamaz` — bağlantılı üst dizin
  üzerinden `.git/hooks`'a yazılamıyor
- `hard_link_onaydan_sonra_eklenirse_dosya_yazilmaz`,
  `hard_link_onaydan_sonra_eklenirse_duzenleme_yapilmaz` — onaydan sonra
  eklenen hard link, yazma/düzenlemeyi `OnayEskidi` ile durduruyor
- `baglanti_takma_adlari_serbest_kipte_bile_yetki_atlatamaz` — uçtan uca,
  Serbest kipte bile bağlantı takma adlarıyla izin atlatılamıyor
- `disari_cikma_ve_gizli_dosya_uctan_uca` — dışarı çıkan yol ve gizli
  dosya reddi uçtan uca

Sonuç: plan sonu kontrol listesindeki "Linux ve Windows'ta bağlantı
testleri çalışır" maddesi artık her iki platformda da karşılanıyor.

Canlı EVREN testleri (`#[ignore]`) bu geçişte de çalıştırılmadı — depo
politikası gereği ajan bu testleri hiç koşturmaz (`.env` var olsa da).
Kullanıcı isterse: `set -a && . ./.env && set +a && cargo test --workspace -- --ignored`.

## Devam eden aşama — 1c (Tauri + Svelte kabuğu)

1c'nin tasarım belgesi yazıldı:
[`2026-09-22-kalfa-1c-design.md`](superpowers/specs/2026-09-22-kalfa-1c-design.md).
1c uygulama planı parçalara ayrıldı; çekirdek ve arayüz işleri ilgili plan
dosyalarında izleniyor.
Bu bölüm, konuşmada şimdiye kadar verilen kararları kaydeder; böylece başka bir
oturum ya da araç kaldığı yerden devam edebilir.

### 1c-1 — çekirdek eklemeleri (tamamlandı)

Plan: [`2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md`](superpowers/plans/2026-09-22-kalfa-1c-1-cekirdek-eklemeleri.md).
`kalfa-core`'a olay paketleyici ve kredi sınırı (`aktarim`), terminal
`AktarimDurdu` olayı, tek şart kapısı (`sart_kapisi`), çalışma alanı kilit
yöneticisi, kayıt başına tek yazıcı, kayıtta çalışma alanı / yönlendirilen model
/ token kullanımı, geçmiş görünümü ve oturum listesi (`gecmis`) eklendi.

Doğrulama (2026-09-23):

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --quiet` | 254 geçti, 0 başarısız, 4 yok sayıldı | 253 geçti, 0 başarısız, 4 yok sayıldı |
| Clippy / fmt | Temiz | Temiz |
| `cargo tree` normal bağımlılık izolasyonu | `kalfa-tools` / `tauri` yok | `kalfa-tools` / `tauri` yok |
| `unwrap()` / `expect()` üretim taraması | Eşleşmeler yalnızca `#[cfg(test)]` modüllerinde | Aynı |

`research/` çalışma ağacı temizdi. Task 11 push'undan sonra CodeRabbit yeni
incelemeyi yürütüyordu; önceki inceleme limit nedeniyle satır satır
tamamlanamamıştı.

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2, native ext4),
Rust 1.98.1, `4f26f9d`. Windows'tan tek fark `junction_uzerinden_git_hook_yazilamaz`
testinin `#[cfg(windows)]` olmasıdır.

Uygulama sınırları: eski kayıtta `calisma_alani: None` ise çekirdek kurtarma
API'si yine kullanılabilir; bu tür kayıtları yalnızca geçmiş görünümünde açma
kapısı uygulama/UI katmanında, `oturum_ac` sırasında zorunlu tutulmalı. Kayıt
kilidi yan dosyası (`.kilit`) oturum açıkken silinmemelidir; uygulama kendi
temizliğinde bu dosyayı silmez. Kök kilit yöneticisi pasif yol girdilerini bir
sonraki yeni yol erişiminde toplar; sessiz dönemde tablo geçici olarak en yüksek
eşzamanlı yol sayısına kadar girdi tutabilir. Şart reddi yerel bekleyenleri
iptal eder, ancak başlamış EVREN kabul POST'unu uzaktan geri alamaz; yerel kapı
Ret durumunu korur. Bekleyen çağrı birden fazla karar neslini atlarsa fail-closed
olarak Ret alır; iki hızlı Kabul de atlanırsa aynı korumacı sonuç oluşabilir.
Büyük kayıt listesi 64 KiB bloklarla geriye taranır; tek bir çok uzun JSONL
satırını satır sınırında çözmek daha büyük bir okuma gerektirebilir.

### İlk altyapı adımı: olay kuyruğu

`kalfa-core` olay kanalı artık olay sayısı ve serileştirilmiş JSON baytları
üzerinden sınırlanıyor. `OturumAyarlari::olay_butcesi` varsayılanı 16.384 olay
ve 8 MiB; bunun bir olaylık ve 512 baytlık bölümü terminal aktarım hatasına
ayrılmıştır. Kuyruğa kabul edilen olaylar sırayla korunur. Yeni olay bütçeye
sığmazsa oturum iptal edilir ve kapanır; tüketici `transport_hatasi()` ile
nedeni görebilir. Bu durumda tüm olayların teslim edildiği veya `Tamamlandi`
olayının geleceği varsayılamaz; tüketici aktarım hatasını ayrıca ele almalıdır.

`OlayAlici` artık `UnboundedReceiver` takma adı değil, `recv()` / `try_recv()`
sunabilen bir sarmalayıcıdır. Alıcının kapanması süren turu iptal eder.
Bu sınır yalnızca olay kuyruğunun JSON yüküne aittir; bütün oturumun RAM
kullanımını veya gelecekteki webview kuyruğunu sınırladığı iddia edilmez.

Kapsam ve kabul ölçütleri:
[`2026-09-22-kalfa-1c-olay-aktarimi.md`](superpowers/plans/2026-09-22-kalfa-1c-olay-aktarimi.md).
Metin birleştirme, Tauri köprüsü ve gerçek arayüzde p95 gecikme ölçümü henüz
yapılmadı; §11.1 kabul kapısı bütünüyle tamamlanmış değildir.

Bu değişiklik için Windows üzerinde yeniden doğrulama (2026-09-22):

| Kontrol | Sonuç |
| --- | --- |
| `cargo test --workspace --quiet` | 182 geçti, 0 başarısız, 4 canlı test atlandı |
| Yeni olay aktarımı testleri | 2 birim + 7 entegrasyon testi geçti |
| `cargo clippy --workspace --all-targets -- -D warnings` | Temiz |
| `cargo fmt --all -- --check` | Temiz |
| `git diff --check` | Temiz; yalnızca LF/CRLF uyarıları |

Yeni testler olay ve bayt bütçelerini ayrı ayrı, UTF-8/JSON kaçışlarını,
terminal hata rezervini, gösterilemeyen onayda yazmanın engellenmesini,
sessiz akışta alıcı kapanışını, kritik olay sırasını ve geçersiz kapasiteyi
sınar. Bu yeni değişiklik Linux'ta yeniden çalıştırılmadı; yukarıdaki Linux
sonuçları tamamlanmış 1b sürümüne aittir. Canlı EVREN çağrısı yapılmadı.

### Kapsam (2026-09-22'de kesinleşti)

Tasarım belgesinin §11 ve §12.3 bölümlerinden:

1. Anahtar girişi ve kimlik deposu, şart metni/onayı, çalışma alanı seçimi.
2. `Komut`/`Zarf<Olay>` köprüsü, akış görünümü, iptal, araç onay kartları ve
   kayıttan oturum açma.
3. Dosya sistemi kimliği üzerinden çalışma alanı başına ortak kilit yöneticisi;
   oturum kaydı başına tek yazıcı sahipliği.
4. Olay birleştirme ve sert bayt/olay bütçeli IPC. Çekirdek kuyruğundaki sınırın
   Tauri/webview boyunca korunması ve gerçek tüketiciyle ölçülmesi gerekir.
5. Pencereye göre komut yetkileri, sıkı CSP ve güvenli Markdown gösterimi.

### Verilen kararlar (2026-09-22)

1. **Görsel temel:** Claude Design'daki "Kalfa Design System" projesi. Repodaki
   anlık kopyası `design/kalfa-ds/`. Kaynağı, kopyada eksik olanlar ve spec'le
   çelişen içerikler (şart ekranı, onay kapsamı vb.) `design/README.md`'de.
2. **Tokenlar aynen kullanılır:** `design/kalfa-ds/tokens/*.css` Svelte
   uygulamasına olduğu gibi girer. Tek istisna `fonts.css` (madde 4).
3. **Bileşenler Svelte 5'te yeniden yazılır.** Tasarım sistemindeki bileşenler
   React (`.jsx`). Her bileşenin `.d.ts` dosyası arayüz tanımı, `.prompt.md`
   dosyası kullanım notu olarak kullanılır. `_ds_bundle.js` ve
   `_adherence.oxlintrc.json` React'e özgüdür, uygulamaya girmez.
4. **Dış kaynak yok (§11.8, §11.9):** Tasarım sistemi fontları Google Fonts'tan,
   ikonları unpkg CDN'inden yüklüyor. Uygulamada Source Sans 3, Source Serif 4
   ve JetBrains Mono (hepsi OFL lisanslı) uygulamayla birlikte paketlenir;
   Lucide ikonları npm paketinden gelir.
5. **Üç sütunlu yerleşim:** menü (213 px), oturum listesi (295 px), konuşma.
   Tasarım sistemindeki sağ inspector sütunu (proje dosya ağacı ve dosya
   önizlemesi) 1c'ye girmez. Bu sütun, arayüzün çalışma alanını ajan döngüsü
   dışında okumasını ve gizli dosya kuralını (§7.4) arayüz tarafında da
   uygulamayı gerektirir; sonraki bir aşamada ele alınır.
6. **1c yalnızca kendi payını uygular.** Tasarım sisteminde olup sonraki
   aşamalara kalanlar:
   - `TurnSummary`'deki "Geri al" ve gönderim günlüğü ekranı: aşama 3.
   - Onay kartındaki "bu oturumda" kapsam seçimi: aşama 3. 1c'de kapsam her
     zaman "yalnızca bu çağrı".
   - Menüdeki "Eklentiler": aşama 4.
   - "Paylaş" düğmesi: v1 dışında (§12.2).
7. **Proje yapısı:** Tauri uygulaması `crates/kalfa/` altında (spec §4'teki
   `kalfa` crate'i); `tauri.conf.json` ve pencere yetkileri (`capabilities/`)
   orada. Arayüz `ui/` altında: Vite + Svelte 5 + TypeScript, SvelteKit yok.
   Paket yöneticisi npm; `package-lock.json` commit'lenir, kurulum `npm ci`
   ile. Ana pencere (`index.html`) ve şart penceresi (`sartlar.html`) ayrı
   giriş noktalarıdır; şart penceresinin paketinde sohbet ekranının kodu yer
   almaz. Tokenlar `ui/src/stiller/`'e kopyalanır (`fonts.css` hariç), fontlar
   `ui/src/fontlar/`'da paketlenir. Bileşen adları tasarım sistemindekilerle
   aynıdır. `kalfa` ince tutulur (pencere kurulumu, Tauri komutları, kanal
   bağdaştırıcısı, kimlik deposu); test gerektiren mantık Tauri'ye bağlı
   olmayan crate'lerde kalır. Sonuçları: Linux'ta `cargo test --workspace`
   WebKitGTK geliştirme paketlerini ister; `cargo test`'in önce
   `npm run build` gerektirip gerektirmediği plan aşamasında doğrulanacak.
8. **Olay köprüsü (§11.1): Tauri `Channel` + kredi.** Paketleyici ve kredi
   hesabı `kalfa-core`'da, oturum döngüsünden ayrı bir `aktarim` modülünde.
   İlk olay 16 ms'lik pencere açar (1c-4'te ölçümle 8 ms'ye indi); pencerede
   biriken olaylar tek pakette
   gider; art arda gelen aynı türden metin parçaları birleştirilir. Onay,
   hata, tamamlanma, araç ve şart olayları birleştirilmez, düşürülmez ve
   pencereyi beklemeden hemen gönderilir. Arayüzün "işledim" demediği veri en
   çok 4 paket ya da 256 KiB (başlangıç değerleri, ölçümle ayarlanır). Kredi
   bitince paketleyici çekirdek kuyruğundan okumaz; uzun süre okumayan
   arayüzde çekirdek bütçesi dolar ve iptal yolu çalışır. Krediden büyük tek
   olay, yolda paket yokken tek başına gider. Arayüz "işledim"i paketi
   durumuna uyguladığında gönderir, çizimi beklemez. `kalfa` yalnızca
   Channel'a yazar ve "işledim" komutunu karşılar. Kopukluğu paket numarası
   yakalar: `Zarf.sira` kayıt satırlarıyla ortak sayaçtan arttığı için olay
   akışında boşluk olağandır; `sira` yalnızca sıralama içindir (spec §4.1'deki
   boşluk kuralı buna göre güncellenecek).
9. **Oturum yaşam döngüsü, kilitler, tek yazıcı.** Açık oturum tablosu
   `kalfa`'da (her oturumun çekirdek tutamacı ve aktarım görevi); aynı oturum
   ikinci kez açılmaz, açık olana geçilir.
   - Çalışma alanı yalnızca yerel klasör seçiciden gelir; arayüz yol
     gönderemez, seçilmiş alanlara kimlikle başvurur.
   - Tek yazıcı: kaydın yanındaki kilit dosyasında işletim sistemi kilidi
     (`File::try_lock`), kayıt açık kaldıkça tutulur; kayıttan açarken önce
     kilit alınır, sonra okunur. `kalfa-core`'daki `OturumKaydi`'de. Kaydın
     kendisi kilitlenmez: Windows'ta dosya kilidi aynı süreçteki okumaları da
     engeller.
   - Tek Kalfa süreci: ikinci açılış mevcut pencereyi öne getirir (Tauri
     single-instance eklentisi); kilitler süreç içidir.
   - Kilit yöneticisi çalışma alanı kökünü dosya sistemi kimliğiyle tanır
     (Windows: birim seri numarası + dosya dizini; Linux: aygıt + inode);
     `kalfa-core`'da; son oturum kapanınca silinir. Bilinen sınır: iç içe
     çalışma alanları ayrı yönetici alır.
   - Arayüz kopması (sayfa yeniden yüklenmesi, pencere kapanması): o pencereye
     bağlı oturumlar kapanır, süren tur iptal edilir, kayıt kilidi bırakılır;
     arayüz son açık oturumu kayıttan açar. Yakalanamayan donmada kredi ve
     çekirdek bütçesi devreye girer.
   - Oturum listesi kayıt dizininden çıkarılır, dizin dosyası tutulmaz:
     kaydın başından çalışma alanı ve başlık, sonundan son durum okunur.
     Kayıttan açılan oturumun geçmişi kayıt öğelerinden üretilir; canlı
     olaylar `sira` ile devam eder.
   - `OturumAcildi` kaydına çalışma alanı yolu eklenecek (şu an yalnızca model
     ve kip var).
10. **Pencereler ve komut yetkileri (§11.9).** İki pencere; ikisini de yalnızca
    Rust açar.
    - Ana pencere: oturumlar, konuşma, onay kartları. Komutları: mesaj gönder,
      iptal, araç onayı, model ve kip değiştir, oturum aç/kapat, çalışma alanı
      seç, "işledim".
    - Şart penceresi: anahtar girişi ve EVREN şart metni. Komutları: anahtarı
      kaydet, şart metnini al, kabul et, reddet.
    - Genel bir "komut gönder" yok; her işlemin kendi Tauri komutu var.
      `SartlariKabulEt` yalnızca şart penceresindeki "Kabul ediyorum"
      düğmesinden doğar. Komutlar uygulama manifest'inde tanımlanır, yetkiler
      pencere etiketine göre dağıtılır.
    - Anahtar ana pencereye hiç girmez: şart penceresinde yazılır, bir kez
      Rust'a gider, arayüze geri dönmez.
    - Ana pencere yeni pencere açamaz (açabilseydi `sartlar` etiketli sahte
      pencereyle şart yetkilerini alabilirdi). Tauri'nin kendi izinlerinden
      yalnızca gerekenler verilir.
    - İki pencerede de uygulama sayfaları dışına gezinme kapalı.
    - Bağlantılar: tıklanınca Rust adresin tamamını yerel iletişim kutusunda
      gösterir, onayla sistem tarayıcısında açar; yalnızca `http`/`https`.
    - İzin kipini yükseltmek yerel iletişim kutusunda onaylanır; düşürmek onay
      istemez. Yalıtımsız kabuk onayı kabukla birlikte sonraki aşamada.
    - CSP: yalnızca uygulamanın kendi betik, stil, font ve görselleri; satır
      içi betik ve uzak kaynak yok.
11. **İlk açılış, şart akışı ve kimlik deposu (§11.8).**
    - Açılış: anahtar yoksa yalnızca şart penceresi açılır ve anahtar ister.
      Anahtar varsa `GET /v1/terms/status`: kabul edilmiş ve güncelse ana
      pencere; kabul gerekiyorsa metin şart penceresinde, ana pencere kabulden
      sonra; 401'de yeni anahtar istenir; EVREN'e ulaşılamıyorsa ana pencere
      bağlantı yok uyarısıyla açılır, şart gerekiyorsa ilk istekte 403 ile
      yakalanır.
    - Anahtar önce şart durumu sorgusuyla denenir; 401 dönerse ya da EVREN'e
      ulaşılamazsa saklanmaz. Geçerliyse işletim sistemi kimlik deposuna
      yazılır (`keyring`: Credential Manager / Secret Service). Secret Service
      yoksa pencere bunu söyler, anahtar yalnızca Kalfa açıkken bellekte
      tutulur. Anahtar değişirse açık oturumlar da yenisini kullanır.
    - Şart penceresi: başlıkta sürüm ve değişikliğin esaslı olup olmadığı;
      metin özetlenmeden güvenli Markdown ile; uzak görseller (EVREN
      metnindeki shields.io rozetleri) yüklenmez, alt metinleri görünür. Onay
      kutusu yok; yalnızca "Kabul ediyorum" ve "Kabul etmiyorum". Ret: anahtar
      saklı kalır, pencere nedenini söyleyen kilitli görünüme geçer ve metni
      yeniden gösterme düğmesi sunar; bekleyen turlar iptal edilir.
    - **Tek şart kapısı** (`kalfa-core`, Tauri'siz): oturumlar 403 alınca
      kapıya kaydolup bekler; kapı metni her sürüm için bir kez alır, gösterilen
      sürümü doğrular, kabul isteğini bir kez gönderir ve bekleyen oturumların
      hepsini uyandırır; ilk açılış da aynı kapıyı kullanır. Gerekçe: bugün her
      oturum kabulü kendisi gönderiyor; iki oturum beklerken tek tık iki
      `POST /v1/terms/accept` olur ve EVREN'in aynı sürümün tekrar kabulüne
      yanıtı bilinmiyor (kullanıcı adına kabul yasak olduğundan canlı
      denenmez).
12. **Ekranlar.**
    - 1c'de yazılacak bileşenler (tasarım sistemindeki adlarla): Badge, Button,
      Card, Icon, IconButton, IconTile; Checkbox, Input, Select, Textarea;
      Composer, Message, ThoughtPanel, ToolCallCard; CodeBlock, DiffView,
      MetaList, TokenMeter; NavItem, NavRail, PanelHeader, SessionRow,
      StatusBar. Sonraki aşamalara kalanlar: TurnSummary, FileTree, LogTable,
      Pagination, SourceTag. Menüde 1c'de yalnızca Oturumlar ve anahtar
      değiştirme var.
    - Onay isteği konuşmada küçük bir kart olarak görünür; "İncele" tasarım
      sistemindeki onay sayfasını açar (dosya listesi, dosya başına diff, "ne"
      ve "nerede", uyarılar, ret gerekçesi, üç karar düğmesi). Sayfa karar
      verilmeden kapatılabilir, istek kartta bekler; onay bekleyen oturum
      varken diğer oturumlara geçilebilir. Veri çıkışı, yalıtım,
      geri alınabilirlik ve kapsam alanları 3. aşamadaki tam kartla gelir.
      "Bu değişiklikler geri alınabilir" cümlesi geri alma gelene kadar
      yazılmaz.
    - Durum çubuğu: son gözlenen bakiye ve zamanı, oturumun token kullanımı
      (düşünce dökümüyle), yönlendirilen model, izin kipi. Bağlam doluluğu
      yok (EVREN kataloğu bağlam penceresini vermiyor). Tahmini maliyet
      yalnızca katalog fiyatı sıfırdan büyükse görünür (§11.5); şu an bütün
      fiyatlar sıfır.
    - Arayüz durumu saf bir TypeScript işleviyle güncellenir (paketi oturumun
      durumuna uygular, DOM'a dokunmaz, Vitest ile test edilir); Svelte
      yalnızca bu durumu çizer.

### 1c-3a — arayüz temeli (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3a-arayuz-temeli.md`](superpowers/plans/2026-09-23-kalfa-1c-3a-arayuz-temeli.md).
`ui/` altında Vite + Svelte 5 + TypeScript iskeleti; tasarım tokenları ve
paketlenmiş fontlar; Türkçe katalog; komut sözleşmesinin tipleri ve
çağrıları; oturum durumu işlevi; güvenli Markdown ve Shiki; tasarım
sisteminin 1c bileşenleri ve geliştirmede açılan bileşen vitrini. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | 208 paket eklendi; 209 paket denetlendi, 0 güvenlik açığı | Başarılı |
| `npm run check` | 0 hata, 0 uyarı | 0 hata, 0 uyarı |
| `npm test` | 10 dosya, 104 test geçti | 10 dosya, 104 test geçti |
| `npm run build` + paket denetimi | Başarılı; `Paket denetimi temiz.` | Başarılı; `Paket denetimi temiz.` |

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2), Node v24.20.0,
`56f5759`.

### 1c-2 — Tauri kabuğu (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-2-tauri-kabugu.md`](superpowers/plans/2026-09-23-kalfa-1c-2-tauri-kabugu.md).
`kalfa-uygulama` (Tauri'siz uygulama mantığı: komut sözleşmesi, şart akışı,
oturum tablosu, olay köprüsü, kimlik deposu) ve `kalfa` (Tauri kabuğu: iki
pencere, 22 komut, pencere izinleri, yerel iletişim kutuları) eklendi. İki
pencerenin ekranları 1c-3b'de.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --all-features --locked` | 301 geçti, 0 başarısız, 4 yok sayıldı | 295 geçti, 0 başarısız, 4 yok sayıldı (`kalfa` hariç) |
| Clippy / fmt | Başarılı | Başarılı (`kalfa` hariç) |
| `cargo tree -p kalfa-uygulama` içinde `tauri`/`wry`/`webkit` | Yok | Yok |
| `unwrap()` / `expect()` taraması | Yalnızca `test_destek.rs` | Aynı |

Linux sütunu PR incelemesinde dolduruldu: Ubuntu 24.04 (WSL2), Rust 1.98.1,
`767e8f5`. `kalfa` crate'i WebKitGTK geliştirme paketleri istediği için Linux'ta
derlenmedi; fark onun 5 izin testi ve Windows'a özel junction testidir.

Gerçek pencere denemesi (Windows, WebView2, sahte EVREN sunucusu; planın
İnceleme bölümü): şart penceresinde anahtar kaydı ve tek `POST` ile kabul, ana
pencerenin açılması ve şart penceresinin kapanması, pencereler arası ve
Tauri/eklenti komutlarının reddi, `window.open` ve dış adrese gezinme engeli,
kaydı olmayan oturumda `OturumYok` beklenen gibi.

Plandan sapma (Codex, PR yorumu): aynı oturumdaki izin kipi değişiklikleri
async kilitle sıraya alındı. Plandaki kodda eşzamanlı iki düşürme (ör. Serbest'ten
Proje ve İnceleme) ters sırayla çekirdeğe ulaşırsa kip onaysız yükselebiliyordu;
`eszamanli_kip_degisimleri_ayni_oturumda_sirayla_islenir` testi sınar.

### 1c-3b — ekranlar (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3b-ekranlar.md`](superpowers/plans/2026-09-23-kalfa-1c-3b-ekranlar.md).
İki pencerenin ekranları yazılıp komutlara bağlandı: ana pencerede menü,
oturum listesi, konuşma, onay kartı ve onay sayfası, durum çubuğu; şart
penceresinde anahtar formu, EVREN şart metni ve kilitli görünüm. Testler ve
geliştirmedeki ekran önizlemesi (`ui/onizleme.html`) sahte bir arka uçla
(`ui/src/sahte/arka.ts`) çalışır; uygulama paketine girmez.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | 208 paket eklendi; 209 paket denetlendi, 0 güvenlik açığı | Başarılı |
| `npm run check` | 0 hata, 0 uyarı | 0 hata, 0 uyarı |
| `npm test` | 14 dosya, 167 test geçti | 14 dosya, 167 test geçti |
| `npm run build` + paket denetimi | Başarılı; `Paket denetimi temiz.` | Başarılı; `Paket denetimi temiz.` |

Linux sütunu PR incelemesinde, 1c-2 ile birleşmiş ağaçta dolduruldu: Ubuntu
24.04 (WSL2), Node v24.20.0.

Birleşik uygulama denemesi (Windows, WebView2, sahte EVREN sunucusu): 1c-2'nin
kabuğu ile bu ekranlar ilk kez birlikte çalıştırıldı. Anahtar formdan girildi,
alan boşaldı, şart metni geldi. "Kabul ediyorum" tek `POST` gönderdi; ana pencere
boş liste ve karşılama ekranıyla açıldı; "Anahtarı değiştir" şart penceresini
anahtar formuyla açtı. Denemede görülen bir hata incelemede düzeltildi: açık
temadaki şart penceresinde not ve uyarı zemini koyu çıkıyordu, çünkü tasarım
sistemindeki `--amber-tint` ve `--blue-tint` yalnızca koyu temada tanımlı. Zemin
artık durum renginden `color-mix` ile türetilir.

### 1c-4 — kabul (tamamlandı)

Plan: [`2026-09-24-kalfa-1c-4-kabul.md`](superpowers/plans/2026-09-24-kalfa-1c-4-kabul.md).

Neler eklendi:
- Uygulama gerçek pencerede, sahte EVREN'li bir test derlemesiyle ve
  `tauri-driver` üzerinden sınanıyor. Kapsam: ilk açılış, konuşma, onay, iptal,
  kayıttan açma ve güvenlik sınırları.
- Üst belge §11.1'in kabul kapısı iki platformda geçti.
- Test derlemesi `e2e` özelliğidir ve sürüm derlemesine girmez. Özellik şunları
  getirir:
  - Sahte sağlayıcı, sahte iletişim kutuları ve aktarım sayaçları.
  - Test komutları ve arayüzün test kancası.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 303 geçti, 0 başarısız, 4 yok sayıldı | 302 geçti, 0 başarısız, 4 yok sayıldı |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | Başarılı | Başarılı |
| Clippy (normal ve `e2e` derlemesi) / fmt | Başarılı | Başarılı |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 16 dosya, 184 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 21 test geçti | 21 test geçti (Xvfb) |

Kabul ölçümü (plan doğrulaması, 2026-09-24):
- **Windows:** Windows 11, WebView2 153.
- **Linux:** Ubuntu 24.04, WSL2, Xvfb (yazılım çizimi), WebKitGTK 2.52.
- **Gecikme:** parçanın üretilmesinden ekrana çizilmesine kadar geçen süre.

| Ölçüm | Windows | Linux |
| --- | --- | --- |
| Gecikme p50 / p95 / p99, 20 ms'de bir parça | 21,9 / 25,2 / 25,9 ms | 26,8 / 40,1 / 44,3 ms |
| Gecikme, 5–10 parçalık yığınlar | 21,9 / 26,0 / 31,8 ms | 21,2 / 31,3 / 32,9 ms |
| Gecikme, düşünce ve metin paralel | 21,9 / 26,0 / 27,2 ms | 20,9 / 29,2 / 31,9 ms |
| Duraklayan arayüz: yoldaki en çok veri (kredi 4 paket, 256 KiB) | 4 paket, 246 889 bayt | Aynı |
| Duraklayan arayüz: kuyruğun en yükseği (bütçe 16 384 olay, 8 MiB) | 1 973 olay, 8 387 088 bayt | Aynı |
| Duraklarken Kalfa sürecinin belleği | 29 → 38 MB | 205 → 232 MB |
| Uzun oturum açılışı, 20 / 500 tur | 36,6 / 66,3 ms | 54 / 111 ms |
| Ekrandaki DOM düğümü, 20 / 500 tur | 635 / 639 | 635 / 639 |

- **Paketleme penceresi 16 ms'den 8 ms'ye indi.** 16 ms'de p95 Windows'ta 41,
  Linux'ta 47 ms'ydi; hedef 50 ms'ye çok yakındı.
- **Kredi ve paket sınırı değişmedi.**
- **Duraklayan arayüzde tur iptal edildi, sınırlar aşılmadı.** Arayüz yeniden
  işlemeye başlayınca `AktarimDurdu` uyarısı geldi. Kuyruk bütçeye kadar doldu,
  ama ne o ne de yoldaki veri sınırı aştı.
- **Linux'ta bellek daha yüksek görünüyor:** WebKitGTK'nin arayüz süreci
  Kalfa'nın kendi sürecidir. Windows'ta WebView2 ayrı süreçlerdedir.
- **Uzun oturumda tam sanallaştırma gerekmedi (§9.3).** Açılış 20 turluk
  kayıttakiyle aynı mertebede kaldı.

Plan yazılırken bulunanlar:
- **Pencere kapanınca oturum kaydı süreç çıkışıyla yarışıyordu.** Süren tur
  iptal edilip kayda yazılırken süreç çıkabiliyordu. Artık süreç kapanan
  oturumları bekliyor (`kapananlari_bekle`).
- **WebDriver'ın pencere kapatması uygulamanın kapanış yolunu sınamıyor.** Son
  pencerede süreci doğrudan sonlandırıyor. Test bu yüzden pencereyi test
  komutuyla kapatıyor.
- **Chromium, WebDriver'ın betik bağlamında `eval`'i engellemiyor.** Yalnızca
  ihlali bildiriyor; dizeden kod bu yüzden dize zamanlayıcısıyla sınanıyor.
- **WebKit'te pencereyi kapatan tıklama "no such window" dönebiliyor.** Test
  yardımcısı bunu beklenen sonuç sayıyor.

Açık madde: `same-file`'ın ReFS'teki (Dev Drive) kimliği doğrulanamadı;
makinede ReFS birimi yok.

### Tasarım belgesi

Bütün kararlar, yukarıdakilere ek olarak çizim ve dil, hata durumları, test ve
kabul ölçümü dahil,
[`2026-09-22-kalfa-1c-design.md`](superpowers/specs/2026-09-22-kalfa-1c-design.md)
belgesindedir. Yukarıdaki liste onun özetidir; ayrılırlarsa belge geçerlidir.
1c'nin dört parçası (1c-1 çekirdek eklemeleri, 1c-2 Tauri kabuğu, 1c-3 arayüz,
1c-4 kabul) tamamlandı. Sıradaki aşama üst belge §12.3'e göre ikincisidir:
kabuk aracı ve yalıtım. Tasarımı
[`2026-09-24-kalfa-2-kabuk-yalitim-design.md`](superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md)
belgesindedir. Aşama dört plana bölünür: 2a kabuk aracı ve süreç sınırları, 2b
Linux arka ucu, 2c Windows prototipi, 2d Windows arka ucu.

## Devam eden aşama — 2 (kabuk aracı ve yalıtım)

Tasarım:
[`2026-09-24-kalfa-2-kabuk-yalitim-design.md`](superpowers/specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md).
Dört plana bölünür: 2a kabuk aracı ve süreç sınırları, 2b Linux arka ucu, 2c
Windows prototipi, 2d Windows arka ucu.

### 2a — kabuk aracı ve süreç sınırları (tamamlandı)

Plan: [`2026-09-24-kalfa-2a-kabuk.md`](superpowers/plans/2026-09-24-kalfa-2a-kabuk.md).

Neler eklendi:
- **`kabuk` aracı.** İki biçimi var: `{program, argumanlar}` ve `{betik}`.
  Betik Linux'ta sh, Windows'ta PowerShell ile çalışır. Seçenekler: çalışma
  dizini, ağ isteği, zaman aşımı.
- **`kalfa-sandbox`.** Komutu başlatır ve süreç sınırlarını uygular:
  - Ortam değişkenleri izin listesiyle geçer.
  - Varsayılan süre 120 sn, en çok 1800 sn.
  - En çok 256 süreç çalışabilir (Windows'ta).
  - İptal, zaman aşımı ya da komutun bitişi bütün süreç ağacını durdurur:
    Linux'ta süreç grubu, Windows'ta Job Object.
- **İzin kararı.** Yalıtım olmadığı için her komut, her kipte yerel pencerede
  onaylanır. Webview'daki "Onayla" komutu çalıştırmaz; Rust yerel pencereyi
  açar ve gösterdiği komutu çekirdekten okur.
- **İlk çağrı uyarısı.** Bir çalışma alanındaki ilk komut, projenin betiklerinin
  kod olarak çalışacağını söyler. Bu onay uygulamanın veri dizinine kaydedilir.
- **Canlı çıktı.** Komut çalışırken çıktı araç kartındaki terminal görünümünde
  akar; ANSI renkleri çizilir. Model, komut bitince kırpılmış çıktıyı alır:
  baştan 10 000, sondan 20 000 bayt ve çıkış kodu.
- **Durum çubuğu.** Yalıtım göstergesi ("Yalıtımsız") eklendi.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 348 geçti, 0 başarısız | 346 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 63 geçti | 63 geçti |
| Clippy (normal ve `e2e` derlemesi) / fmt | Başarılı | Başarılı |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 18 dosya, 205 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda) | 24 test geçti (Xvfb) |
| Gecikme p95 (düzenli / yığın / paralel) | 24,2–24,4 / 24,8–26,1 / 24,5–24,6 ms | 37,8 / 31,5 / 37,9 ms |

Plan yazılırken bulunanlar:
- **Çıktı seli Linux'ta arayüzü boğuyordu.** Terminal görünümü megabaytlarca
  metin tutarken WebKitGTK (Xvfb, yazılım çizimi) saniyede 50–60 KB
  işleyebildi; tur 60 saniyede bitmedi. Görünüm artık en çok 200 000 karakter
  tutuyor, tek satır 4 000 karakterde kesiliyor ve araçtan arayüze en çok
  1 MiB gidiyor. Aynı test 3,6 saniyede bitiyor.
- **Canlı çıktının kayıplı olması iki yerde gerekli.** Araçtan oturuma giden
  kanal doluyken parça atılır. Oturum da çekirdeğin olay kuyruğu yarıdan fazla
  doluyken parçayı iletmez. Yoksa yavaş bir ekran kuyruğu doldurup turu iptal
  ettirirdi. Atılan her bayt sayılır ve ekranda "… N bayt gösterilmedi …" diye
  görünür.
- **Aynı çağrının üç kez başarısız olma sınırı kabuğu durdururdu.** Model testi
  düzeltip aynı komutu yeniden koştukça sınır dolardı. Sıfırdan farklı çıkış
  kodu ve zaman aşımı bu sınıra sayılmıyor.
- **Yerel onay penceresinin gösterdiği komut webview'dan gelmemeli.** Gelseydi
  sızmış bir betik zararsız bir metinle zararlı komutu onaylatabilirdi. Komut
  çekirdeğin bekleyen onayından okunuyor. Çekirdek de webview'ın gönderebildiği
  onay komutuyla yerel onaylı öğeyi onaylamıyor.

### 2b — Linux yalıtımı (tamamlandı)

Plan: [`2026-09-25-kalfa-2b-yalitim.md`](superpowers/plans/2026-09-25-kalfa-2b-yalitim.md).

Neler eklendi:
- **Linux arka ucu.** Komut bubblewrap'ın içinde çalışır:
  - Sistem yolları ve araç zinciri (`~/.cargo/bin`, `~/.rustup`, Ayarlar'dan
    eklenenler) salt okunurdur. Ev dizini, diğer çalışma alanları ve sistemin
    geri kalanı görünmez.
  - Çalışma alanı yazılabilir, dışına yazılamaz. Her çalışma alanının kendi
    ev, geçici ve önbellek dizini vardır (veri dizinindeki `yalitim/`).
  - Gizli dosyalar ve hard link'leri okunamaz. `.git/hooks`, `.git/config` ve
    `core.hooksPath` hedefi salt okunurdur; worktree'nin dışarıdaki git dizini
    yazılabilir bağlanır.
  - Ağ varsayılan olarak kapalıdır. seccomp `io_uring`, `keyctl`, `bpf` ve
    `ptrace` gibi çağrıları engeller; soket aileleri izin listesiyle
    denetlenir (Unix ve vsock soketleri açılamaz).
  - Süreç sayısı sınırı (256) yalıtımın içinde konur. İptal ve Kalfa'nın ölümü
    bütün ağacı durdurur.
- **Algılama.** bwrap yoksa ya da kullanıcı ad alanları kısıtlıysa Kalfa bunu
  durum çubuğunda ve Ayarlar'da talimatıyla söyler; komutlar yalıtımsız ve
  yerel onaylı çalışır.
- **Ayarlar.** Menüye Ayarlar eklendi: yalıtım durumu, talimat, "Yeniden dene"
  ve araç zinciri dizinleri (Linux; yerel klasör seçiciyle eklenir).
- **İzin kararı.** Yalıtımlı komut Sıkı ve Proje kipinde olağan onayla, Serbest
  kipte ağsızsa onaysız çalışır. Çalışma alanındaki ilk komut ve ağlı komut her
  kipte onay ister.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 360 geçti, 0 başarısız | 369 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 66 geçti | 66 geçti |
| Clippy (normal, `e2e`; Linux'ta ayrıca `yalitim-testleri`) / fmt | Başarılı | Başarılı |
| Yalıtım testleri (root olmayan kullanıcıyla) | — | `yalitim` 17, `uyumluluk` 4 geçti |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 20 dosya, 218 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda) | 25 test geçti (iki koşuda; Xvfb, birleştirme kapalı) |
| Gecikme p95 (düzenli / yığın / paralel) | 24,4–24,6 / 25,6–26,1 / 24,2–24,6 ms | 35,9–37,5 / 33,0–35,5 / 36,8–37,6 ms |

Plan yazılırken bulunanlar:
- **Süreç sayısı sınırı yalıtımın içinde konmalı.** Sınır bwrap'a dışarıda
  verilince ad alanının üst sınırı oluyor ve kullanıcının bütün süreçleri
  sayılıyor; komut hiç başlamıyor. İçeride (`prlimit`) konunca yalnızca
  yalıtımdaki süreçler sayılıyor.
- **Salt okunur cargo registry'si ağsız derlemeye yetiyor.** Yeni crate ise
  salt okunur registry'ye inemez; ağlı çalıştırmada kullanıcının registry'si
  bağlanmaz, cargo çalışma alanına özel registry'ye indirir.
- **Rustup vekili her çalışmada `~/.rustup/update-hashes`'i istiyor.**
  `~/.rustup`'ın tamamı salt okunur açıldı; dizinde kimlik bilgisi yok.
- **x32 çağrıları seccomp listesini atlatabilirdi.** `seccompiler` bunları
  kapsamıyor; elle yazılmış ikinci bir filtre `ENOSYS` döndürüyor.
- **Konuşma en alttan kopuyordu (1c'den kalan hata).** Kendi kaydırma olayı,
  onay sayfası kapanınca odağın geri dönmesi ya da konuşmadaki bir düğmeye
  tıklamak alta yapışmayı bırakıyordu; yeni onay kartı görünüm alanının altında
  kalıyordu. Artık yalnızca kullanıcının kaydırması bırakıyor. Linux'ta kabuk
  testleri yalıtımla koşunca görünür oldu.
- **Linux gecikme ölçümü Xvfb'de birleştiriciyi ölçüyordu.** Alta yapışma
  düzelince akan metin görünür oldu ve p95 56–64 ms'ye çıktı. Karenin çizimi
  ~5 ms sürüyor; asıl bekleme, GPU'suz Xvfb'de yazılımla (llvmpipe) çalışan GL
  birleştiricisinde: görünür her değişiklikte bir sonraki kare ~25 ms gecikiyor.
  Linux uçtan uca testleri `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile koşar; ürün
  kodu değişmez. 1c'deki Linux ölçümü, akan metin görünüm alanının dışındayken
  alınmıştı.

PR incelemesinde düzeltilenler (Codex'in inceleme botu buldu):
- **Onaylanan kip çalıştırmada korunmuyordu.** Araç yalıtımı o anki durumdan
  çıkarıyordu. Mühür denetiminden sonra yeniden algılama durumu "yok"a
  çevirirse yalıtımlı onaylanan komut yalıtımsız başlayabilirdi. Kip artık
  onaylanan tariften alınıyor; yalıtımsız arka uç da yalıtımlı onaylanan
  isteği reddediyor.
- **`AF_VSOCK` engellenmiyordu.** seccomp soket kuralı engel listesiydi ve ağ ad
  alanı vsock'u kesmez; WSL2'de bu, Windows'taki servislere giden yoldur. Soket
  aileleri artık izin listesiyle denetleniyor.

### 2c — Windows prototipi (tamamlandı)

Bulgular: [`2026-09-25-kalfa-2c-windows-prototip.md`](superpowers/bulgular/2026-09-25-kalfa-2c-windows-prototip.md).
Prototip: [`prototip/windows-yalitim/`](../prototip/windows-yalitim/README.md). Ürün
kodu değildir; Kalfa'nın workspace'ine girmez.

Tasarımdaki P1–P10 geliştiricinin makinesinde (Windows 11 Home) ölçüldü:
- Yönetici adımlarını kullanıcı çalıştırdı: `kur.ps1`, `guvenlik-duvari.ps1`, `geri-al.ps1`.
- Deneme ağacını da kullanıcı kurdu: `agac.ps1`.
- Ölçümleri Claude koştu: `kalfa-prototip olc`.

- **Sonuç Dal B.**
  - Windows, S-1-15 SID'lerini (ALL APPLICATION PACKAGES dahil) kısıtlayıcı SID olarak kabul
    etmiyor.
  - Sistem dizinlerini okutan tek kullanılabilir SID Users. Bu yüzden Users'ın okuyabildiği
    klasörler de okunuyor.
  - Çalışma alanları arası yazma ayrımı, profil koruması, gizli dosya ve korumalı yollar tutuyor.
  - Araç zincirlerinin tamamı çalışıyor: cmd, PowerShell, git, node, npm, python, cargo, rustc.
- **Tasarıma işlenen düzeltmeler:**
  - Kısıtlayıcı küme: çalışma alanı + ortak okuma + Everyone + Users + RESTRICTED.
  - Varsayılan DACL: kullanıcı, SYSTEM ve çalışma alanı SID'i.
  - `SetErrorMode`: yükleyici hata kutuları UI kısıtlarına rağmen masaüstünde açılıyordu.
  - Güvenlik duvarı kuralının kullanıcı koşuluna ortak okuma SID'i eklendi. Yoksa kural kısıtlı
    belirteçlere uygulanmıyordu.
  - Users grubundan çıkarma adımı kalktı: etkisiz.
  - İzinler API'yle veriliyor: `icacls` hesaba karşılık gelmeyen SID'leri reddediyor.
- **Kısıtlı belirteç yolunda açık kalanlar:**
  - A8: Schannel kısıtlı belirteçte çalışmıyor; ağlı cargo crates.io'ya ulaşamıyor.
  - A9: Geri döngü süzülmüyor.
  - A10: Varsayılan izinli borular salt okunur açılıyor.

**AppContainer ek ölçümü:**

- Bulgular: [`2026-09-25-kalfa-2c-appcontainer.md`](superpowers/bulgular/2026-09-25-kalfa-2c-appcontainer.md).
- Yönetici adımlarını (`kap-kur.ps1`) kullanıcı çalıştırdı; ölçümleri
  (`kalfa-prototip kap olc`) Claude koştu.

- **Sonuç Dal A.**
  - Okunamayanlar: kullanıcı klasörleri, `C:\Users` ve profil.
  - Araç zincirlerinin tamamı çalışıyor.
  - Schannel çalışıyor, geri döngü kapalı, varsayılan izinli borular
    açılamıyor (A8–A10).
  - Ayrı kullanıcı ve güvenlik duvarı kuralı gerekmiyor.
  - Çağrı başına 22–29 ms.
- **Bedeli:**
  - `C:\` köküne kurulumda yayılmayan bir listeleme izni.
  - Korumalı yollar yalnızca izinle; ret izinleri AppContainer'da işlemiyor.
  - MSVC ortamını Kalfa verir.
  - Yönetici sahipli araç zinciri dizinlerine kurulumda izin.
- **Açık:** A11 (profil altındaki çalışma alanının üst dizinleri), A12 (C
  derleyen `build.rs`), A13 (LAN, ssh-agent ve docker boruları).

- **Tasarım:** §9 AppContainer'a göre yeniden yazıldı; ayrı kullanıcı yolu
  §9.7'de yedek. Üst belge r3.5.
- **Güvenlik:** Kabın içinden başka bir paket SID'iyle süreç başlatılamıyor
  (A14).
- **Makine:** Prototipin kurulumu geri alındı.

### Güvenlik düzeltmesi: yalıtımda `.git` salt okunur (2026-09-26)

2d-1 planlanırken 2b'nin Linux yalıtımında üç açık bulundu. Üçü de
Windows arka ucunu da ilgilendirdiği için 2d'den önce ayrı bir PR'la kapatıldı
(aşama 2 tasarımı §7.2–§7.4, üst belge r3.6):

- **`.git` yalnızca kısmen korunuyordu.** Kancalar ve yapılandırma salt
  okunurdu, ama yalıtımdaki komut `.git`'in kendisini yeniden adlandırıp yerine
  yenisini kurabiliyordu (WSL'de doğrulandı). Artık `.git`'in tamamı (dizin ya
  da worktree'nin `.git` dosyası) salt okunur bağlanır. Kullanıcı kararıyla
  yalıtımda `git status`, `git diff` ve `git log` çalışır; `git add` ve
  `git commit` çalışmaz.
- **Dış git dizini doğrulanmıyordu.** `.git` dosyasının `gitdir:`'i ve
  `commondir` çalışma alanındadır; gösterdikleri dizin doğrulanmadan
  yazılabilir bağlanıyordu. Artık geri başvurusu tutmayan dizin açılmaz,
  tutan da salt okunur açılır.
- **Kalfa'nın özel dizinlere yazması bağlantı izliyordu.** `ev/.gitconfig` ve
  `ev/.npmrc` her çalıştırmadan önce yazılır; bu dizinlere yalıtımdaki komut da
  yazabilir. Artık bağlantılar (Windows'ta junction'lar da) izlenmez.
  Windows'ta junction'lı test eski kodda düştü.

Bilinen sınır: `.git` sembolik bağlantıysa, çalışma alanı git deposu değilse ya
da alt dizinlerde, yalıtımdaki komut yeni bir `.git` oluşturabilir.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 365 geçti, 0 başarısız | 375 geçti, 0 başarısız |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 66 geçti | 66 geçti |
| Clippy / fmt | Başarılı | Başarılı (üç derleme) |
| Yalıtım testleri (`kalfatest`) | — | `yalitim` 17, `uyumluluk` 4 geçti |
| Uçtan uca | 24 test geçti (iki koşuda); p95 24–27 ms | 25 test geçti (iki koşuda); p95 35–38 ms |

### Hata düzeltmesi: aynı turdaki kabuk çağrıları ve alan hazırlığı (2026-09-28)

Windows'ta hazırlanmamış çalışma alanında model aynı turda birden çok `kabuk`
çağrısı yapınca yalnızca ilki çalışıyor, ötekiler "Dosya onaydan sonra
değişti" ile düşüyordu. Sebep şuydu: ilk çağrı alanı hazırlayınca ötekilerin
yürütmeden önceki taze tarifinde alan hazırlığı ve ilk çağrı artık yoktu, bu
yüzden mühür tutmuyordu. Tükenen bu tek seferlik koşullar artık onayla
kapsanır ve araç onaylanan tarifle çalışır. Onaylanmayan hazırlık sonradan
belirirse komut yine başlamaz; ileti nedeni söyler (aşama 2 tasarımı §3.3).

| Kontrol | Windows | WSL Ubuntu-24.04 |
| --- | --- | --- |
| `cargo test --workspace --locked` ve `e2e` özellikli testler | Geçti | Geçti |
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| Gerçek yalıtım testleri | `yalitim` 29/29, `uyumluluk` 7/7 (AppContainer) | `yalitim` 17/17, `uyumluluk` 4/4 (bwrap) |
| 2d-3 birleştikten sonra (ana dal PR dalına birleştirildi) | Çalışma alanı 464, `e2e` özellikli 91, `yalitim` 30/30, `uyumluluk` 11/11 | Çalışma alanı 414, `e2e` özellikli 90, `yalitim` 17/17, `uyumluluk` 6/6 |

### Hız düzeltmesi: git korumaları yanıt başına bir kez (2026-09-28)

Araç planlaması git korumalarını (`git config --get core.hooksPath`) yazma
hedefi başına ve oturum görevini engelleyerek çözüyordu. Yürütme de aynı
çözümü her çağrı için, sırayla tekrarlıyordu. Artık planlama korumaları
model yanıtı başına bir kez, engellemeden çözer. Yürütme ise onları kilit
altında yeniden çözmeye devam eder (spec §7.3), ama bunu engelleyen iş
parçacığında yapar.

Windows'ta 20 dosya yazan bir yanıtta ölçülen süreler:

- **Planlama:** 0,5–0,6 s'den 31–40 ms'ye indi.
- **Toplam:** 1,1–1,3 s'den 0,18–0,33 s'ye indi.

`.git/config`'in doğrudan okunmaması bilinçli bir karardır. Ölçüm ve
gerekçe [bulgular belgesindedir](superpowers/bulgular/2026-09-28-git-koruma-cozum-olcumu.md).

### 2d-1 — Windows arka ucu (PR #12)

Plan: [`2026-09-26-kalfa-2d-1-windows.md`](superpowers/plans/2026-09-26-kalfa-2d-1-windows.md).
2d iki plana bölündü; 2d-2 uygulama tarafı ve Ayarlar'dır. Görev 1–4'ü
Codex yazdı. Görev 5'i Codex'in taslağı üzerine, Görev 6'yı ve inceleme
düzeltmelerini Claude yazdı (Codex'in kotası bitti).

Neler eklendi:
- **AppContainer yalıtımı** (aşama 2 tasarımı §9):
  - Komut, geliştiricinin hesabıyla, çalışma alanına özel bir AppContainer'da
    (`Kalfa.Alan.<kimlik>`) ve bir Job Object'te çalışır.
  - Araç zinciri dizinlerine ortak bir yetenek SID'i okuma izni taşır. Ağlı
    çalıştırmada `internetClient` verilir.
  - Kabın ortamı çalışma alanına özel ev, geçici ve önbellek dizinlerini
    gösterir; MSVC ortamını Kalfa verir.
- **`kalfa-yalitim.exe`** (yönetici): `kur` sürücü köklerine ve `C:\Users`'a
  listeleme izni verir; `kaldir` geri alır; `izin-ver` / `izin-geri-al`
  geliştiricinin sahibi olmadığı araç zinciri dizinlerini açar ya da kapatır
  (örneğin `C:\Program Files\nodejs`, `C:\Python314`).
- **Çalışma alanı hazırlığı:**
  - Paket SID'ine çalışma alanında değiştirme izni verilir.
  - Gizli dosyaların mirası kesilir; `.git` ve kancalar salt okunurdur.
  - Üst dizinlere mirassız listeleme verilir (A11).
  - Her değişiklik ACE kaydına yazılır, kaldırmada kayda göre geri alınır.
- **Hazırlığın onayı:** hazırlanmamış çalışma alanındaki ilk yalıtımlı komutun
  onayı her kipte yerel penceredir.
- **Yalıtım dizini:** Windows'ta `%LOCALAPPDATA%\Kalfa\yalitim`.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `cargo test --workspace --locked` | 414 geçti, 0 başarısız | 380 geçti, 0 başarısız (4 atlandı) |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | 68 geçti | 68 geçti |
| Clippy (normal, `e2e`, `yalitim-testleri`) / fmt | Başarılı | Başarılı |
| Yalıtım testleri | `yalitim` 27, `uyumluluk` 6 geçti (kurulumdan sonra; `yalitim` üç koşuda) | `yalitim` 17, `uyumluluk` 4 geçti (`kalfatest`) |
| `npm run check` / `npm test` / `npm run build` | 0 hata, 0 uyarı; 20 dosya, 218 test; paket denetimi temiz | Aynı |
| Uçtan uca (`cd e2e && npm test`) | 24 test geçti (iki koşuda); p95 24–25 ms | 25 test geçti (iki koşuda); p95 33–37 ms |

Linux uçtan uca testleri Windows'takilerle aynı anda koşunca gecikme testleri
yük altında düştü (p95 63–108 ms); tek başına koşuda geçti.

Kurulumu yapılmış makinede Windows uçtan uca testlerinin kabuk senaryoları
gerçek yalıtımla koşar ve iki koşuda iki AppContainer profili bıraktı
(çalışma alanları kaldırılmıyor). 2d-2'nin Windows uçtan uca işinde
kapatılır.

A11–A15 (aşama 2 tasarımı §14):
- **A11:** üst dizinlerin listelenmesi gerekiyor. Git for Windows (`getcwd`)
  ve PowerShell (`Set-Location`) istiyor. PSDrive yalnızca PowerShell'i
  kurtarıyor. Hazırlık üst dizinlere paket SID'ine mirassız listeleme verir;
  `C:\Users` yöneticiye ait olduğundan izni kurulum verir. Kap bu dizinlerdeki
  adları görür, içeriklerini göremez.
- **A12:** C derleyen `build.rs` çalışıyor. MSVC dizini `PATH`'in başında
  olmalı ve kabın `TEMP`'i kurulmalı (aşağıda).
- **A13:** ölçülmedi; makinede LAN hedefi, ssh-agent ve Docker yok.
- **A15 (bilinen sınır):** çalışma alanına özel cargo registry'si kapta
  doldurulamıyor. cargo paketi açarken hedefi kanonikleştiriyor; DOS yol adı
  Mount Manager'a erişim istiyor, AppContainer'da "Erişim engellendi". Windows
  yalıtımında crates.io bağımlılığı indirilemez; bağımlılıksız derleme
  çalışır. Yol 2d-2'de seçilir; en olası aday geliştiricinin registry'sinin
  salt okunur verilmesi.

Uygulanırken ve incelemede bulunanlar:
- **Gerçek yalıtım testleri Codex'in ortamında koşamıyordu.** Codex'in kendi
  yalıtımında `CreateAppContainerProfile` açılamıyor. Görev 4'ün testleri ilk
  kez kurulu makinede koştu: 24 testten 4'ü test hatasıyla düştü (betikler
  kendi alanına yazıyordu, `mklink` yanlış kaçışla çağrılıyordu, kabın
  yazamadığı yere işaret konuyordu).
- **Kabın `TEMP`'i yoktu.** Windows, AppContainer süreçlerinde `TEMP`'i
  `%LOCALAPPDATA%\Packages\<paket>\AC\Temp`'e çeviriyor ama dizini kurmuyor.
  PowerShell kendisi kurduğu için görünmüyordu; `link.exe` LNK1104 veriyordu.
- **rustc Git'in `link.exe`'sini çalıştırıyordu.** Kapta Visual Studio'yu COM
  ile bulamayınca `PATH`'teki ilk `link.exe`'ye düşüyor. MSVC dizini sondaydı;
  Git'in MSYS `link.exe`'si kapta 0xC0000142 ile başlayamıyor.
- **`SetNamedSecurityInfoW` üst dizinde dakikalar sürüyordu.** Her yazımda
  mirası bütün alt ağaca yeniden yayıyor. Mirassız izinler artık kök
  iznindeki gibi `SetFileSecurityW` ile yalnızca nesneye yazılıyor.
- **Node ve Python dizinleri uygulama paketlerine kapalı.** Kurucuları DACL'i
  korumalı yazıyor; `izin-ver` gerekiyor.
- **Bir test her koşuda makinede AppContainer profili bırakıyordu** (bozuk ACE
  kaydı testi, profili bilerek korunan durum). Test artık profilini siliyor.
- **`kaldir` bazı hatalarda başarı bildirip kaydı silebiliyordu** (bozuk
  kayıt, okunamayan kök ACL'i, "bulunamadı" dışındaki erişim hatası). Codex
  düzeltti; PR'da onaylanan plandan sapma.
- **Bozuk ACE kaydında eksiksiz geri alma güvenli değil.** Kaydı kaybolmuş bir
  yolun kesik mirası Kalfa'nın mı geliştiricinin mi, ayırt edilemiyor.
  Yalnızca kanıtlanan değişiklikler geri alınıyor; kayıt ve profil korunuyor
  (kullanıcı kararı).

Makine: `kalfa-yalitim kur` kurulu (C:\, D:\ ve C:\Users); `izin-ver` ile
`C:\Program Files\nodejs` ve `C:\Python314` açık. Geri alma `kaldir` ve
`izin-geri-al`'dır.

### Ayrı kullanıcı karşılaştırması ve son kullanıcı kararları (2026-09-27)

Bulgular: [`2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md`](superpowers/bulgular/2026-09-27-kalfa-2d-ayri-kullanici-karsilastirma.md).

- **Ölçüm:** yönetici gerekmedi.
  - Kısıtlı belirteçte DOS yol adı çözülüyor ve cargo paketi açıyor (A15
    yok). Ama Schannel `SEC_E_NO_CREDENTIALS` veriyor, cargo indiremiyor (A8).
  - AppContainer'da A15 bir sınıf. Python `realpath` sessizce yanlış yol
    döndürüyor; Node `realpathSync.native` EPERM veriyor (A16).
- **Kullanıcı kararları.** Hedef, teknik olmayan birinin kolay kurulumu:
  - AppContainer kesinleşti.
  - Araç zincirini (Python, Node, Git) Kalfa'nın kurucusu getirir.
  - Rust ve MSVC gelişmiş özelliktir.
  - Yalıtım yoksa kabuk kapalıdır. Kullanıcıya sade ileti, BT'ye teknik
    ayrıntı verilir. Yalıtımsız çalıştırma yalnızca Ayarlar'dan açılır.
- **Tasarım:** aşama 2 tasarımı §9.8 ve §15; üst belge r3.7.

### Dış kaynak incelemesi: awesome-harness-engineering (2026-09-27)

Bulgular: [`2026-09-27-awesome-harness-engineering-inceleme.md`](superpowers/bulgular/2026-09-27-awesome-harness-engineering-inceleme.md).
Liste bir okuma dizini; alınacak kod ya da bağımlılık yok. 2d-2'nin kapsamı
değişmez. Aşağıdakiler 2d-3 ve aşama 3 planlarına girecek:

- **Sınır matrisi (2d-3):** onaylanan tarife × çalışma anı durumu (yalıtım
  var/yok, izin kipi, ağ, yalıtımsız anahtarı) için tablo biçiminde test.
  Uyuşmazlık olan her hücrede süreç başlamaz. 2d-2 incelemesinde kontrol
  listesi olarak da kullanılır.
- **A15/A16 için önce okuma (2d-3):** OpenAI'nin Codex Windows yalıtımı yazısı
  AppContainer'ı kendi iş yükü için uygun bulmuyor. Tasarım kararı değişmez;
  plan yazılmadan önce hangi araçlarda benzer sorun çıkacağını görmek için
  okunur.
- **Senaryo testleri (aşama 3):** sahte EVREN ile çok turlu, betikli model
  yanıtları. Her senaryo iki şeyi ayrı sınar: son durum (çalışma alanı, JSONL)
  ve iz (onay, tarife, yalıtım). İlk adaylar: sunulmayan kabuğun çağrılması,
  onaydan sonra durum değişmesi, onay beklerken kapanma, kurtarmada yan etkinin
  yinelenmemesi. Komut dizisi değil güvenlik değişmezleri sınanır. Bağlam
  özetlemenin onay kapsamını genişletmediği de bu altyapıyla sınanır.
- **Araç sonuçları (aşama 3):** modele dönen hata ve kırpma sonuçları "ne oldu,
  şimdi ne yapabilirsiniz" biçiminde yazılır; kullanıcıya sade ileti, BT'ye
  teknik rapor ayrımı korunur.
- **Uymayanlar:** Claude API'nin sunucu tarafı özetlemesi (EVREN üzerinden
  gidiyoruz; özetleme bizde), yeni ajan çatıları, vektör veritabanı, onayı
  sınıflayıcıyla gevşetmek.

### 2d-2 devam noktası (2026-09-27)

Plan: [son kullanıcı akışı](superpowers/plans/2026-09-27-kalfa-2d-2-son-kullanici.md).
Görev 1–8'in kodu PR dalında. Görev 6 hazırlanmış alanları, Görev 7 profil
sayımı ve alan temizliğini, Görev 8 NSIS kancalarını ve kullanıcı kabul/geri
alma betiklerini ekledi. Görev 9 bu doğrulama kaydını güncelledi.

Claude'un incelemesi ve iki platformdaki doğrulaması (kurulu Windows makinesi):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked`, `e2e` özellikli testler | Geçti | Geçti |
| Gerçek yalıtım testleri | 29/29 (AppContainer) | 17/17 (bwrap); uyumluluk 4/4 |
| Arayüz denetim, test ve üretim derlemesi | 0 hata/uyarı; 229/229 test; paket denetimi temiz | — |
| Uçtan uca | 25/25; "kabuk yalıtımı" gerçek AppContainer'da koştu; `Kalfa.Alan.*` profil sayısı koşudan önce ve sonra aynı | 25/25 (Codex) |

İncelemede düzeltilenler plandaki inceleme notunda. Sürüm yükseltmede araç
zinciri dizinlerinin izinlerinin gitmesi kullanıcı kararıyla bilinen sınırdır
(2d-3'te yeniden ele alınır). §9.8'deki araç zinciri kurucuda kararı plan
gereği 2d-3'tedir.

Kullanıcının açık izniyle `@tauri-apps/cli@2.11.4` ve Tauri'nin NSIS araçları
indirildi. Windows x64 kurucusu
`target/release/bundle/nsis/Kalfa_0.1.0_x64-setup.exe` üretildi.

**Kurucu kabulü (kullanıcı, UAC):** kurulum, kurulu sürümde `Hazir` durumu,
bozulan kurulumu "Onar"la düzeltme ve kaldırma denendi. Kaldırmadan sonra
kök ve araç zinciri izinleri, kurulum kaydı ve Program Files dizini kalmadı.
Ayrıntı plandaki kabul notunda. 2d-2 tamamlandı.

### 2d-3 devam noktası (2026-09-27)

Plan: [araç zinciri](superpowers/plans/2026-09-27-kalfa-2d-3-arac-zinciri.md),
PR #15 (`2d-3/arac-zinciri`). Kodu Claude yazdı; G1–G6 commit'lendi, her
görevin sonucu ve sapmaları plandaki "Sonuç" notlarında.

- **G1:** A16 ölçümü ([bulgular](superpowers/bulgular/2026-09-27-kalfa-2d-3-a16.md));
  Python 3.14.7 (NuGet) ve Node 24.21.0 seçildi; Python Kalfa'nın
  `sitecustomize.py`'siyle çalışıyor.
- **G2 / G2b:** onaylanan kip ile çalışma anı `kip_uyusmasi`'nda, 16 hücreli
  sınır matrisi; bağımlılık dizinleri gizli kalıpların dışında.
- **G3:** kurucu araçları indirir, özeti denetler, açar (5,9 → 42,7 MB).
- **G4:** kurulu Kalfa'nın araçları kabın `PATH`'inde kullanıcınınkilerden önce.
- **G5:** `kalfa-yalitim kur --izin-ver`, `okunur_yonetici_gerekli`, Onar ve
  "Ekle"de UAC; Windows'ta araç zinciri dizinleri Ayarlar > Gelişmiş'te.
- **G6:** ağsız cargo geliştiricinin `.cargo`'suyla; kapa yalnızca `registry`
  ve `git` açılır, kimlik dosyaları okunamaz.
- **G7:** belgeler; yükseltmeden sonra yalıtım "Hazır" görünürken kaybolan
  dizin izinleri için Ayarlar > Gelişmiş'te Onar (kabul adımını yazarken
  bulundu).
- Git dağıtılmaz; kaldırmada oturum kayıtları silinmez (tasarım §9.8).

Son doğrulama (tam, PR dalının son hâli):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked` | 459 | 409 (Onar düzeltmesinden sonra uygulama crate'leri yeniden: 81/81) |
| `e2e` özellikli testler | 90 | 89 |
| Gerçek yalıtım testleri | 30/30 (AppContainer) | 17/17 (bwrap, `kalfatest`) |
| Uyumluluk | 10/10 (`KALFA_TEST_ARAC_ZINCIRI` ile) | 6/6 |
| Kurucu betik testleri | 5/5 | 3/3 (2 zip testi atlandı) |
| Arayüz denetim, test ve derleme | 0/0; 231 test; paket denetimi temiz | — |
| Uçtan uca | 25/25; `Kalfa.Alan.*` profili kalmadı | — (arayüzün Linux yolu değişmedi) |

**Kurucu kabulü (kullanıcı, UAC, 2026-09-28):** kurulum, Kalfa'nın Python,
Node ve Git'iyle yalıtımda komutlar, Gelişmiş'ten UAC ile dizin ekleme,
yükseltmenin izinlere etkisi ve Onar'la geri gelmesi, temiz kaldırma ve
geliştirme ortamının geri kurulması denendi. Ayrıntı plandaki kabul notunda.
Kabulde bulunan `\\?\` önekli yol görüntüsü düzeltildi. Aynı turdaki paralel
kabuk çağrılarının ilk hazırlıkta düşmesi 2d-3'ten önceki bir hatadır; ayrı
işe ayrıldı. 2d-3 tamamlandı; sıradaki PR #15'in birleştirilmesi.

### Güvenlik sertleştirmesi (PR #16, 2026-09-28)

[Güvenlik analizi](superpowers/bulgular/2026-09-27-guvenlik-analizi.md) ve
[plan](superpowers/plans/2026-09-27-kalfa-guvenlik-sertlestirme.md); kararlar
ve plandan sapmalar planın "Kararlar" ve "Uygulama sonucu" bölümlerinde.

- **Bulgu 1 (yüksek):** dosya araçları `.git/commondir` yazabiliyordu; git bununla
  yalıtım dışında kod çalıştırıyordu. Artık her derinlikteki `.git`'in tamamı
  korumalı (onay ister), 8.3 kısa adla da yazılamaz. Yalıtımda bütün `.git`
  girdileri salt okunur; komutun oluşturduğu tehlikeli yeni `.git` izin
  listesiyle bulunur, `.git.kalfa-karantina-<zaman>` olarak kenara alınır ve
  "Güvenlik uyarısı" olarak gösterilir.
- **Bulgu 2:** `.vscode/`, `.cargo/config*`, `.envrc`, CI dosyaları gibi
  kendiliğinden çalışan yapılandırmaya yazmak her kipte onay ister; yalıtımda
  salt okunur, yenisi bildirilir.
- **Bulgu 3:** kabuk onayı oturumdaki onaysız değişiklikleri ve derleme
  dosyalarını gösterir; aynı adımdaki yazmalar da onaya çıkar; riskli öğeler
  "Tümünü onayla" ile geçmez.
- **Bulgu 4:** gizli dosya kalıpları genişledi.
- **Bulgu 5:** araç çıktısı çağrı başına rastgele sınırlı sarmalla gelir.
- **Bulgu 6:** yönetici yardımcısı kayıt dizininin sahibini ve reparse
  noktasını denetler, DACL'i tutamaçla yazar.
- **Bulgu 7:** görünmez ve yön değiştiren karakterler onay yüzeylerinde
  `⟨U+XXXX⟩` olarak görünür ve onay ister.

Son doğrulama (tam, PR dalının son hâli):

| Doğrulama | Windows | WSL Ubuntu-24.04 |
|---|---|---|
| Biçim ve Clippy (tam, `e2e`, yalıtım özellikli) | Geçti | Geçti |
| `cargo test --workspace --locked` | 513 | 458 |
| `e2e` özellikli testler | 92 | 91 |
| Gerçek yalıtım testleri | 30/30 (AppContainer) | 17/17 (bwrap, `kalfatest`) |
| Uyumluluk | 14/14 | 9/9 |
| Arayüz denetim, test ve derleme | 0/0; 242 test; paket denetimi temiz | — |
| Uçtan uca | 25/25 | 25/25 (`xvfb-run`) |

Testler makinede fazladan Kalfa AppContainer profili bırakmadı. Uçtan uca
`05-kabuk`'taki etiket beklentisi yeni sarmala taşındı; ilk koşuda yalnız o
test eski beklentiyle düştü, düzeltmeden sonra iki platformda geçti.

**Açık işler:**
- Görev 3'ün kurulu makinede yönetici olarak kabulü (kullanıcı; UAC): kurulum,
  "Onar" ve kaldırma kayıt dizininin yeni denetimiyle.
- Bulgu 8, ek gizli kalıplar ve güvenlik denetçisi aşama 3'e taşındı
  (aşağıda).

Bağlam özetleme, oturum bütçesi ve WASM eklentileri sonraki aşamalardadır.
Masaüstü uygulaması Windows ve Linux'ta çalışıyor; tamamlanmış bir v1 sürümü
yoktur.

## Devam eden aşama — 3 (görünürlük)

Tasarım:
[`2026-09-28-kalfa-3-gorunurluk-design.md`](superpowers/specs/2026-09-28-kalfa-3-gorunurluk-design.md);
üst belge r3.9. Aşama 3 beş plana bölünür:

- **3a:** tur özeti ve geri alma.
- **3b:** gönderim günlüğü ve önizleme.
  - Plan: [`2026-09-28-kalfa-3b-gonderim-gunlugu.md`](superpowers/plans/2026-09-28-kalfa-3b-gonderim-gunlugu.md)
    (hafif; uygulayanı kullanıcı seçecek). 3a'dan sonra yürür.
- **3c:** prompt injection senaryo testleri ve güvenlik denetçisi.
  - Plan: [`2026-09-28-kalfa-3c-senaryolar-denetci.md`](superpowers/plans/2026-09-28-kalfa-3c-senaryolar-denetci.md)
    (hafif; uygulayanı kullanıcı seçecek).
  - Uygulandı; PR #47. Ayrıntısı aşağıda.
- **3d:** ağ çıkışının kısıtlanması (Bulgu 8).
  - Plan: [`2026-09-28-kalfa-3d-ag-cikisi.md`](superpowers/plans/2026-09-28-kalfa-3d-ag-cikisi.md).
  - Ortak vekil, izin listesi ve arayüz uygulandı (Görev 1, 2, 3, 5, 6; PR #20).
  - Windows alt planı [`2026-10-05-kalfa-3d-windows-kopru.md`](superpowers/plans/2026-10-05-kalfa-3d-windows-kopru.md)
    tamamlandı (Görev W1–W6):
    - `BoruSunucusu` (`crates/kalfa-sandbox/src/windows/kopru.rs`) ile overlapped
      adlandırılmış boru, paket SID'i ve Job Object doğrulaması.
    - `kalfa-kopru` ikilisi (`crates/kalfa-sandbox/src/bin/kalfa-kopru.rs`):
      `127.0.0.1:0` dinleyicisi, `vekil_ortami` değişkenleri ve boruya köprüleme.
    - `WindowsYalitici`'da ağlı komut: `internetClient` kaldırıldı; komut
      yalnızca köprü ve vekil üzerinden izinli hostlara çıkar.
    - `uyumluluk.rs` (Windows): curl, git, npm, node (`fetch`) ve pip vekille
      CONNECT gönderiyor.
    - Bilinen sınırlar testle belgelendi: cargo'nun yeni crate'i açması A15
      yüzünden kapta düşer; sistem Python'unun `pip`'i A16 sınıfından düşer;
      Kalfa'nın Python'uyla venv'de `pip install` çalışır.
    - S13 uçtan uca senaryosu (`09-ag-cikisi.test.mjs`) ve `KALFA_E2E_SAHTE_DNS`
      desteği.
    - `kalfa-kopru.exe` `tauri.nsis.conf.json`'da `externalBin` olarak paketlenir;
      `kalfa-uygulama` ikiliyi kurulu ve geliştirme dizinlerinden bulur.
  - Linux köprüsü (Görev 4) ve S13 (Görev 7) uygulandı; inceleme bulguları
    düzeltildi. Ayrıntısı aşağıda.
- **3e:** "bu oturumda izin ver" kısayolları, ek gizli kalıplar ve onay
  kartının kalan alanları.
  - Plan: [`2026-09-28-kalfa-3e-kucuk-isler.md`](superpowers/plans/2026-09-28-kalfa-3e-kucuk-isler.md).
    Gizli sayılmayanlar (K7) ve `ayarlar.json` bu plandadır; aşama 3'ün son
    planıdır.
  - Uygulandı; PR #23. Ayrıntısı aşağıda.

Bağlam özetleme ve oturum bütçesi aşama 3'ün dışındadır.

### Kullanıcı kararları (2026-09-28)

1–4 tasarım yazılmadan önce, 5–10 PR'da verildi (tasarım §2.1, K1–K10).

1. **Önceki içerik oturumla birlikte saklanır.**
   - Veri dizininde, kaydın dışındaki bir depoda durur. Dosyaların adı
     içeriklerinin özetidir; kayda yalnızca özet girer.
   - Oturum silinene kadar kalır; 3a'ya "Oturumu sil" eklenir.
   - Sınırlar: dosya başına 8 MiB, oturum başına 512 MiB. Aşılınca en eski
     turların verisi silinir.
2. **Kabuğun değiştirdiği dosyalar geri alınamaz, listelenir.** Komuttan önce
   ve sonra meta veri karşılaştırılır.
3. **Sonradan değişmiş dosyaya geri almada dokunulmaz;** dosya çakışma olarak
   listelenir. Önizleme hangi dosyanın döneceğini önceden gösterir.
4. **Varsayılan listede olmayan host çağrıda istenir** (`ag_hostlari`); onay
   kartında görünür ve mühre girer. Varsayılan liste crates.io, npm, PyPI ve
   GitHub'ın indirme uçlarıdır.
5. **`github.com` varsayılan listede değildir.** git'in `push` ucudur; git
   işlemleri onu çağrıda ister.
6. **Windows'ta vekilin ölçümü olumsuz çıkarsa** ağlı komut bugünkü gibi
   `internetClient` ile çalışır; kart "Ağ: sınırsız (Windows'ta vekil yok)"
   der.
7. **Varsayılan gizli kalıplar kapatılamaz.** Yanlış pozitif (örneğin
   `src/credentials.rs`) Ayarlar'daki "gizli sayılmayanlar" listesiyle
   çözülür; eklemek yerel pencerede onaylanır.
8. **Gizli dosyanın önceki hâli de saklanır;** oturum silinince o da silinir.
9. **Geri alınamayacak kadar büyük dosyaya yazmak her kipte onay ister;**
   kayıtsız oturum hariç.
10. **3a'yı Claude yerel makinede uygular.**

Tasarımın kendi verdiği kararlar ve gerekçeleri tasarım belgesinin
§2.2'sindedir. Windows'a ve Linux'a özgü davranışlar (AppContainer'da geri
döngü, bwrap'ta köprü, Windows'ta tarama maliyeti) bu belge yazılırken
ölçülmedi; açık noktalar G1–G18'dir.

### 3a — tur özeti ve geri alma (uygulandı; PR incelemede)

Plan: [`2026-09-28-kalfa-3a-geri-alma.md`](superpowers/plans/2026-09-28-kalfa-3a-geri-alma.md).
Dal `3a/geri-alma`, PR #19 (tabanı `3/tasarim`, PR #18). Hafif bir plandı;
platformdan bağımsız görevleri bir bulut oturumu Linux'ta, kalanları ve iki
platformdaki doğrulamayı Claude yerel makinede yaptı.

- **Tur özeti:** her tur tasarım sistemindeki `TurnSummary` ile biter:
  dosyalar, komutların değiştirdiği dosyalar, EVREN istekleri, güvenlik
  uyarıları.
- **Geri alma:**
  - Dosya araçları yazmadan önce önceki hâli oturumun deposuna koyar
    (Windows'ta `%LOCALAPPDATA%\Kalfa\geri-alma`) ve "başladı" kaydına yazar.
  - "Turu geri al…" önce planı gösterir, sonra mühürle yürütür. Sonradan
    değişen, bir komutun değiştirdiği ya da hard link'li dosyaya dokunmaz;
    nedenini söyler.
  - Kayıttan açılan oturumda da çalışır. Model bir Kalfa notuyla bilgilenir.
- **Kabuk:** komuttan önce ve sonra meta veri görüntüsü alınır. Değişen
  dosyalar tur özetinde ve sonraki kabuk onayında listelenir, geri alınmaz.
- **Onay kartı:** her yazan öğe için "Geri alınabilir mi". 8 MiB'tan büyük
  dosyaya yazmak kayıtlı oturumda her kipte onay ister.
- **Oturumu silme:** yerel pencerede onaylanır; kayıt, kilit dosyası ve geri
  alma verisi birlikte silinir.
- **Ölçümler:**
  - G1: 100 000 dosyada görüntünün eki Windows'ta 17 ms, WSL'de 45 ms.
  - G10 kapandı: depo yalıtımın içinden görünmüyor.
  - G2 kapandı: üç model art arda iki kullanıcı mesajını kabul ediyor
    (canlı test).

Ayrıntılar, plandan sapmalar ve doğrulama tablosu planın "Uygulama sonucu"
bölümünde.

Araç planlamasındaki aynı maliyet (yazılan hedef başına `git config`) PR
#25'te giderildi; aşağıdaki "Hız düzeltmesi" bölümüne bakın.

### 3b — gönderim günlüğü ve göndermeden önce gösterme (uygulandı; PR #22 incelemede)

Plan: [`2026-09-28-kalfa-3b-gonderim-gunlugu.md`](superpowers/plans/2026-09-28-kalfa-3b-gonderim-gunlugu.md).
Dal `3b/gonderim-gunlugu`, PR #22 (tabanı `kalfa-asama-1`). Uygulama Antigravity tarafından yapıldı.

- **Her istek denemesinin kaydı:** Başarılı ve başarısız (kopan, ağ hatası, HTTP hatası vb.) bütün istekler JSONL'de birer `KayitOgesi::Istek` olarak kaydedilir. `asama: Baglanti` (gövde gitmedi) ve `asama: Yanit` (gövde gitti) ayrımı yapılır. Bağlantı hatasında `gonderilen` sayacı ve son sistem/araç tanımları ilerlemez; sonraki istekte ve kurtarmada öğeler yeniden `yeni` sayılır.
- **Kaynak etiketleri:** Mesajlara ve araç çıktılarına kaynak etiketleri (`mesaj:kullanıcı`, `model`, `kalfa:sonuç`, `kalfa:not`, `kalfa:sistem`, `kalfa:araçlar`, `dosya:...`, `kabuk:...` vb.) eklenir. `KayitOgesi::Mesaj.kaynak`'a yazılır.
- **Sistem iletisi ve araç tanımları:** İlk istekte ve değiştiklerinde `KayitOgesi::SistemIstemi` ve `KayitOgesi::AracTanimlari` olarak kaydedilir.
- **Günlüğün türetimi:** Kayıttan sayfalanarak okunan `GunlukSayfasi` ve `GunlukSatiri` türetilir. Eski kayıtlar da geriye dönük uyumlu açılır ("ayrıntı yok"). `gonderim_ogesi` ile tam metin çekilir.
- **Göndermeden önce göster (önizleme):** Oturum ayarı `onizleme_acik` açıkken dış kaynaklı araç sonuçları sonraki istekten önce `Olay::GonderimOnizlemesi` ile bekletilir. Çıkarılan öğeler model bağlamına `[kullanıcı bu içeriği göndermedi]` yer tutucusuyla girer ve asıl içerik kayda girmez. Önizleme iptalinde `ONIZLEME_IPTALI` yer tutucusu girer. Kurtarmada kapanan önizleme için `ONIZLEME_KAPANISI` yer tutucusu üretilir. Hata eşiği aşıldığında da önizleme kartı gösterilir; kip değişikliği önizleme beklemesini bozmaz.
- **Onay kartında "Veri çıkışı":** Her öğe, onaylanırsa EVREN'e ne gideceğini söyler: dosya içeriği, komut çıktısı, arama sonuçları, dosya adları ya da yalnızca "yazıldı" bilgisi. Araç bunu hazırlıkta verir (`Hazirlik.veri_cikisi`); bir risk hesabı değildir.
- **Arayüz:** Svelte 5 ile `GonderimGunlugu.svelte` (modal diyalog, log tablosu, ayrıntı paneli, tam metin görüntüleyici), `OnizlemeKarti.svelte` ("Gönder" ve "Seçilenleri gönderme" butonları), konuşma durum çubuğunda "Göndermeden önce göster" ayarı, `OnayKarti`'nda "Veri çıkışı" rozeti.
- **Ölçümler:**
  - G8 kapandı: Önizleme kartında 50 × 64 KiB öğe durumunda kart gövdesinde içerik taşınmaz; yalnızca meta veriler yer alır. 50 öğelik zarf serileştirildiğinde ~7.5 KiB tutar, 256 KiB olay bütçesine ve 32 KiB test eşiğine rahatça sığar (`g8_onizleme_karti_olay_butcesine_sigar`). Bekleyen sonucun içeriği karardan önce kayda yazılmaz; kullanıcı onu konuşmada araç sonucu olarak görür. `gonderim_ogesi` yalnızca kayda girmiş öğeleri açar.
- **Senaryo ve e2e:** S12 senaryosu (`s12_onizlemede_cikarilan_oge_sonraki_istekte_yoktur`) ve uçtan uca e2e testi (`07-gonderim.test.mjs`).

### 3c — senaryolar ve güvenlik denetçisi (uygulandı; PR #47 incelemede)

Plan: [`2026-09-28-kalfa-3c-senaryolar-denetci.md`](superpowers/plans/2026-09-28-kalfa-3c-senaryolar-denetci.md).
Dal `3c/denetci`, PR #47. Görev 2–5'i Antigravity, 1'i ve 6–11'i Claude uyguladı.

- **Senaryolar:** `kalfa-tools/tests/senaryolar.rs`'de S1–S12. Her biri
  tasarımdaki bir değişmezi gerçek dosya araçları ve sahte sağlayıcıyla
  sınar. S9 katı denetçinin denetleyemediği içeriği, S11 geri alınan turdan
  sonraki isteği, S12 önizlemede çıkarılan öğeyi sınar.
- **Denetçi** (`kalfa-core/src/denetci.rs`):
  - EVREN'e bloklu bir istek (`Saglayici::sohbet`, `max_tokens` 2 048), JSON
    şemalı yanıt ve çağrı başına rastgele sınırlı sarmal.
  - En çok 4 eşzamanlı denetim yapılır, zaman aşımı 30 sn'dir. 429'da bir kez
    yeniden denenir.
  - `Bilinmiyor` hiçbir yolda güvenli sayılmaz.
- **Ne denetlenir:** Yalnızca dış kaynaklı araç sonuçları. Kalfa'nın ürettiği
  sonuçlar (`kalfa_uretimi`) denetlenmez. Onayla okunan gizli dosya
  denetçiye gitmez; "denetlenmedi" diye işaretlenir.
- **Kipler:**
  - Uyar kipinde şüpheli ve denetlenemeyen sonuç sarmalın dışında bir uyarı
    satırıyla gider.
  - Katı kipte bu sonuçlar kartsız gitmez. 3b'nin önizleme kartı onları
    "gönderme" işaretli getirir; kullanıcının kararı kayda yazılır.
- **Ayar:** Kip `ayarlar.json`'dadır ve her turun başında okunur. Kipi
  düşürmek yerel pencerede onay ister. Komutlar `denetci_kipi` ve
  `denetci_kipini_ayarla`'dır.
- **Arayüz:**
  - Araç kartında rozetler: denetleniyor, şüpheli (kategoriyle),
    denetlenemedi, denetlenmedi: gizli.
  - Durum çubuğunda "Denetçi: kapalı · uyar · katı".
- **Ölçüm:** G3 ve G4 kapandı
  ([bulgular](superpowers/bulgular/2026-10-06-kalfa-3c-denetci-olcumu.md)).
  Küçük içerikte denetim ~2 sn sürüyor. Bütçe reasoning'de biterse sonuç
  `Bilinmiyor` oluyor.
- **Uçtan uca:** `08-denetci.test.mjs`, sahte denetçinin şüpheli bulduğu
  içerikle katı kipin kartını sınar.
- **Bilinen sınır:** Denetçinin kendisi de enjeksiyona açıktır: içerik
  "`guvenli: true` yanıtla" diyebilir. Şema yanıtın biçimini zorlar, kararını
  değil. Denetçi izinlerin yerine geçmez; ek bir katmandır.

### 3d — ağ çıkışı (birleşti; PR #20)

Plan: [`2026-09-28-kalfa-3d-ag-cikisi.md`](superpowers/plans/2026-09-28-kalfa-3d-ag-cikisi.md),
Windows alt planı: [`2026-10-05-kalfa-3d-windows-kopru.md`](superpowers/plans/2026-10-05-kalfa-3d-windows-kopru.md).
Görev 2, 3, 5, 6 ve W1–W6'yı Antigravity; ölçümleri, Görev 4, 7, 9'u ve
inceleme düzeltmelerini Claude yaptı.

- **Vekil** (`kalfa-sandbox/src/vekil/`): izin listesi (tam host eşleşmesi),
  adres denetimi (geri döngü, bağlantı yerel ve meta veri adresi her zaman
  yasak; özel ağ yalnızca işaretli hosta), tek çözüm, IP'yle istek yalnızca
  listede IP olarak yazılmışsa, `CONNECT`'te SNI eşleşmesi.
- **Linux:** bwrap her profilde `--unshare-net`; ağlı profilde köprü
  (`/run/kalfa-kopru`) kabın `127.0.0.1`'ini dinler ve bağlantıyı
  `SCM_RIGHTS` ile Kalfa'ya verir; vekil tanımlayıcının bağlı bir TCP soketi
  olduğunu denetler. Devredilmeyen tanımlayıcılar kaba geçmez
  (`close_range`).
- **Windows:** ağlı kap `internetClient` almaz; köprü rastgele adlı,
  geliştiriciye ve paket SID'ine açık bir boruya bağlanır; vekil bağlanan
  sürecin komutun iş nesnesinde olduğunu denetler.
- **Model ve arayüz:** `ag_hostlari` mühre ve onay kartına girer; sonuçta
  `[ağ]` satırları, tur özetinde ağ özeti.
- **İnceleme bulguları** (ayrıntısı planın uygulama sonucunda): sürüm
  derlemesinde açık kalan e2e kancası adres denetimini kapatabiliyordu
  (artık yalnızca `e2e` özelliğiyle derlenir); Windows'ta ortamda aynı ad iki
  kez tanımlanıyordu; 5 Windows yalıtım testi kırmızıydı; S13 hiç
  koşmamıştı.
- **Bilinen sınırlar:** Java vekili okumaz; ECH kullanan araçlar SNI
  denetimine takılır; kurumsal vekil arkasında vekilin dış bağlantısı
  kurumsal vekilden geçmez (G16); Windows'ta cargo (A15) ve sistem
  Python'unun pip'i (A16).
- **İptal denetimi (2026-10-07):** Windows'ta Schannel'ın iptal denetimi
  kaptan CRL/OCSP'ye ulaşamıyor. Bu yüzden stapling yapmayan hostlarda cargo
  ve curl düşüyordu. Ağlı komutta cargo'ya `CARGO_HTTP_CHECK_REVOKE=false`,
  curl'e `.curlrc` ile `ssl-revoke-best-effort` verilir
  ([bulgular](superpowers/bulgular/2026-10-07-kalfa-3d-iptal-denetimi.md)).

### 3e — küçük işler (birleşti; PR #23)

Plan: [`2026-09-28-kalfa-3e-kucuk-isler.md`](superpowers/plans/2026-09-28-kalfa-3e-kucuk-isler.md).
Dal `3e/kucuk-isler`, PR #23. Görev 1–6'yı Claude uyguladı.

- **Kısayollar** (`kalfa-core/src/kisayol.rs`): Teklifi çekirdek yapar;
  webview yalnızca kabul bitini gönderir. Kısayol yalnızca kararı uyarısız
  onaylı olan öğeye verilir:
  - Dosya kısayolu araç ve dizindir; alt dizinleri kapsar.
  - Kabuk kısayolu program ve dizindir; yalnızca program biçiminde,
    yalıtımlı, ağsız ve ilk olmayan çağrıya verilir.
  - Kısayolla yürüyen çağrı onaysız değişiklik listesine girer. Kip
    değişince bütün kısayollar silinir; kayda yazılmaz.
- **Gizli ayarları** (`GizliAyarlari`, `kalfa-core/src/izin.rs`):
  - Ek gizli kalıplar ve gizli sayılmayanlar uygulama genelinde
    paylaşılır; değişiklik açık oturumlarda ve yalıtımda hemen geçerlidir.
  - Varsayılan kalıplar kapatılamaz. İstisna yalnızca adın eşleşmesini
    kaldırır; gizli dosyanın hard link'i yalıtımda gizli kalır.
- **`ayarlar.json`:** ek gizli kalıplar, gizli sayılmayanlar, ağ listesi
  ve denetçi kipi.
  - Gizliliği azaltan ya da erişimi genişleten her değişiklik yerel
    pencerede onaylanır: ek kalıbı kaldırmak, istisna eklemek, ağ hostu
    eklemek.
  - Elle bozulmuş bir gizli kalıp yalnızca kendisi düşer; öteki kalıplar
    geçerli kalır.
- **Arayüz:** Onay sayfasında "Kapsam" satırı; konuşmanın üstünde "Bu
  oturumdaki izinler (N)"; Ayarlar'da gizli dosyalar, ağ ve denetçi.
- **G9:** 100 000 yolda sınıflandırma Windows'ta 68–98 ms, WSL'de
  46–59 ms (sürüm derlemesi; üst uç 50 ek kalıp ve 50 istisnayla). Önbellek
  gerekmedi.
- **Uçtan uca:** `10-kisayol.test.mjs`. Bir kısayol verilir ve sonraki
  yazma onaysız geçer; kip değişince kısayol kalkar. Gizli sayılmayanlara
  eklenen dosya onaysız okunur.
- **İnceleme düzeltmesi:** `ayarlar.json`'daki tek bir geçersiz kalıp bütün
  gizli listeleri düşürüyordu. Kullanıcının geçerli ek kalıpları böylece
  sessizce kapanıyor, sonraki ayar yazımında da dosyadan siliniyordu. Artık
  yalnızca geçersiz kalıp ayıklanır.
- **Doğrulama:**
  - Windows: çalışma alanı 815 ve `e2e` özellikli 114 test; iki clippy ve
    fmt temiz. Yalıtım 43/43. Arayüz: Vitest 303/303, `npm run check` 0 hata
    0 uyarı. Uçtan uca 35/35.
  - Windows uyumluluk 21/22: `cargo_ozel_registry_kapta_doldurulamaz`
    ana dalda (`f1c0e32`) da aynı nedenle düşüyor. Ağlı kapta Schannel
    sertifika iptalini denetleyemiyor (`CRYPT_E_REVOCATION_OFFLINE`).
    Nedeni bulundu ve #48 ile düzeldi (3d'nin "İptal denetimi" maddesi);
    birleşmiş #23'te Windows uyumluluk 23/23.
  - WSL (`kalfatest`): çalışma alanı 757 ve `e2e` özellikli 113 test; üç
    clippy temiz; yalıtım 26/26, uyumluluk 15/15.
  - WSL uçtan uca 35/35. İlk iki tam koşuda gecikme ölçümü eşiği aştı
    (p95 52–61 ms, eşik 50). Tek başına beş koşuda 34–43 ms; üçüncü tam koşu
    geçti.

**Aşama 3** 3e'nin birleşmesiyle tamamlanır: 3a–3e uygulandı. Kalan açık
noktalar (G11–G17) kullanımla izlenir ya da aşama 3'ün dışındadır (tasarım
§13).

**Aşama 3 tamamlandı** (#23 → `1b79d1d`, 2026-10-07).

## Devam eden aşama — 4 (eklenti ve RAG)

Tasarım:
[`2026-10-07-kalfa-4-eklenti-rag-design.md`](superpowers/specs/2026-10-07-kalfa-4-eklenti-rag-design.md);
üst belge r3.10. Tasarımdan önce bir ön ölçüm yapıldı:
[bulgular](superpowers/bulgular/2026-10-07-kalfa-4-on-olcum.md).

- **Kullanıcı kararları (K1–K4):**
  - `kalfa-rag` Kalfa'yla birlikte gelir.
  - Üçüncü taraf eklenti klasörden kurulur.
  - İndeks arama çağrısında oluşur; çalışma alanındaki ilk EVREN gönderimi
    yerel onaylıdır.
  - Ön ölçüm yapıldı.
- **7 Ekim ön ölçümü (tarihsel):**
  - İlk denemede gömme ~7–9 sn sürdü. Sonraki G4/G8 ölçümünde sorgu gömme
    0,092–0,266 sn; sabit gecikme varsayımı korunmadı. `dimensions` ve
    `base64` destekleniyor. Güncel kararlar aşağıdaki 4c kaydındadır.
  - wasmtime 49.0.2: örnekleme 0,15 ms. Rust std eklentisi
    `wasi:filesystem`'i her zaman import ediyor; bu yüzden import izin
    listesi ve ön açık dizinsiz dosya sistemi kullanılıyor.
  - Yeni EVREN kayıtları `crates/kalfa-evren/tests/kayitlar/`'da.
- **Planlar:**
  - **4a:** sözleşme ve çalışma zamanı. Kodu önceden iki platformda
    doğrulanır.
  - **4b:** EVREN yolu.
  - **4c:** `kalfa-rag`.
  - **4d:** arayüz ve dağıtım.
  - 4a önce gelir; 4b ile 4c ondan sonra paralel yürüyebilir; 4d en
    sondadır.

Tasarım PR #49 ile birleşti (`a091927`). Codex incelemesinin dört bulgusu
işlendi: kayıtlar `research/`'ten taşındı; indeks yazma sırası her kesilmede
tutarlı; aynı boyut ve zamanla yeniden yazılan dosya yakalanıyor; eklentinin
EVREN isteklerine oturum başına sınır.

### 4a — sözleşme ve çalışma zamanı (birleşti; PR #50)

Plan: [`2026-10-07-kalfa-4a-sozlesme-calisma-zamani.md`](superpowers/plans/2026-10-07-kalfa-4a-sozlesme-calisma-zamani.md).
Kodu önceden yazıldı ve Windows ile WSL'de doğrulandı (yerel dal `deneme/4a`).
Plan PR #50'nin dalında uygulandı: Görev 1–9, görev başına bir commit. Beş
Haiku 5.5 alt ajanı dosyaları paralel yazdı; Claude çıktıyı doğrulanmış ağaçla
karşılaştırdı (fark yok), derledi ve iki platformda doğruladı:

| | Windows | WSL |
|---|---|---|
| `cargo test --workspace` | 860 (10 atlandı) | 802 (10 atlandı) |
| `--features e2e` | 114 | 113 |
| `kalfa-plugin` | 41 | 41 |
| Yalıtım / uyumluluk | 43 / 23 | 26 / 15 |
| Üç clippy, `fmt` | temiz | temiz |


- **Yeni crate'ler:** `kalfa-plugin-sdk` (WIT 0.1, SDK), `kalfa-plugin`
  (manifest, `Motor`, host, depo, `EklentiAraci`), üç test eklentisi.
- **Çekirdek:** `Hazirlik.eklenti` (mühre girer), `Hazirlik.kilitsiz`,
  `VeriCikisi::EklentiCiktisi`.
- **G1, G3, G7 kapandı** (sonuçlar planın "Ölçümler" bölümünde). Test
  eklentilerini testler kendileri derler; `cargo test --workspace` yeterli.
- **Tasarımdan farklar** (planda tablo): `EklentiOrtami` trait'i yok, host
  gerçek okuma yolunu ve depoyu kullanır; depo `kalfa-plugin`'de, yeri 4d'de.
- `evren-cagir` 4a'da `izin-yok("… aşama 4b")` döndürür.

PR #50 `d1aa225` ile `kalfa-asama-1`'e birleşti.

### 4b — EVREN yolu (birleşti; PR #51)

[PR #51](https://github.com/heimdilon/kalfa-arsiv/pull/51) 11 Ekim'de `4807265`
ile `kalfa-asama-1`'e birleşti. Plan ve uygulama kaydı
`docs/superpowers/plans/2026-10-08-kalfa-4b-evren-yolu.md`'dedir.

- Eklentinin gömme ve yeniden sıralama çağrısı gerçek EVREN yolundan geçer.
  Gövde izin listesiyle doğrulanır; modelin görevi uca uymalıdır.
- İlk gönderim her kipte yerel onaylı; önizleme ve gönderim günlüğü eklenti
  isteğini taşır. Oturum başına eklenti sınırı kurtarmada yeniden sayılır.
- Gönderim izni 4b'de bellekte; `eklentiler.json`, Eklentiler ekranı ve
  “Metni göster” 4d'dedir.
- Devralınan doğrulama: Windows 918 ve e2e özellikli 114, WSL 861 ve 113
  test; üç clippy/fmt temiz. Arayüz 309 test, 0 hata/uyarı; WSL
  yalıtım/uyumluluk 26/15. Ayrıntı #51'in kaydında.

### 4c — Türkçe anlamsal arama (uygulandı; PR #52 taslak)

Plan ve uygulama kaydı:
[`2026-10-09-kalfa-4c-rag.md`](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/plans/2026-10-09-kalfa-4c-rag.md).
Dal `4c/plan`, [PR #52](https://github.com/heimdilon/kalfa-arsiv/pull/52),
ilk incelenen kod `003035e`. #51 birleşince yeniden tabanlandı;
tabanı `kalfa-asama-1`, 11 Ekim canlı ölçüm kaydının başı `171908c`'dir.
[CI 145](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38128314461)
bu başta biçim, arayüz, Linux ve Windows işleriyle yeşil. Yayınlanmış son
baş `f64a413` için [CI 147](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38145248319)
dört işiyle yeşildir.

- `kalfa-rag`: satır pencereleri, artımlı ve kesintiye dayanıklı indeks;
  gömme (1024 boyut/base64), ilk 50 adayın yeniden sıralanması; dosya yolu,
  satır aralığı ve puan içeren sonuç. İndekste metin saklanmaz.
- SDK'nın EVREN/depo yardımcıları; gerçek wasm/host bütünleşmesi ve S14.
- Canlıda görülen günlük token sınırı ayrı `EvrenError::GunlukSinir` oldu;
  sohbet/denetçi yeniden denemez. Günlük sınırın ayrı arayüz durumu 4d'dedir.
- **8–9 Ekim tarihsel canlı değerlendirmesi:** 26/32 (%81,25), 547 dosya,
  yaklaşık 4120 parça; arama 1,7–2,0 sn, yaklaşık 39 000 token. O ilk indeks
  iki güne bölünmüştü. 11 Ekim'de ayrı boş indeksin maliyeti ölçüldü;
  aşağıdaki rapor bunu kapatır, tam/güncel kalite tekrarını kapatmaz.
- `e0d44bf` için devralınan sonuç: Windows 979 ve 114, WSL 921 ve 113
  test. `003035e` için bu oturumda iki platformda tam testler, e2e özellikli
  testler, üç clippy ve fmt doğrulandı; kesin sonuç
  [inceleme kaydındadır](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-4c-inceleme.md).
- **İnceleme düzeltmeleri:** Boyutu uygun dosya birden çok toplu isteğe
  bölünür; tamamlanan parçalar `meta.bekleyen` ile kesintiden sonra yeniden
  kullanılır. Dosya bütünü bitmeden tamamlandı sayılmaz. Sorgu indekslemeden
  önce gömülür; yeniden sıralama 429/günlük sınıra takılırsa doğrulanmış
  kaynaklar kosinüs puanı ve açıklama notuyla döner. Sorgu gömmesi baştan
  reddedilirse hata korunur. Windows'ta 994 + 114, WSL'de 936 + 113 test
  geçti; üç clippy ve fmt temiz. Ayrıntı inceleme kaydındadır.
- **CI:** G5 aday/dosya ölçüm commit'i `76afebe` için
  [koşu 137](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37947611609)
  dört işiyle yeşil. Talimat dili `0ffef46` için
  [koşu 138](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37954272157)
  ilk Windows denemesinde değişmemiş iptal süre testinde düştü;
  yerelde iki tam takım ve üç ayrı Windows tekrarında geçti. Test sınırı
  değiştirilmedi; CI 138 tekrarı dört işiyle yeşil. Boyut `b3488e2` için
  [CI 139](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37961173852)
  dört işiyle yeşil. Parça deneyi `1aa1ac4` için
  [CI 140](https://github.com/heimdilon/kalfa-arsiv/actions/runs/37969507051)
  dört işiyle yeşil; sonuç kaydının yeni başı ayrıca kontrol edilir.
- **G6 kapandı:** İki platformda yerel SDK ve gerçek wasm/host ölçümü;
  256 × 1024 base64 yanıt wasm'da yaklaşık 2 ms, float yaklaşık 18 ms.
  Base64 JSON yaklaşık %75 küçük; crate basit elle çözücüden hızlı.
  Mevcut seçim korunur; kaynaklar, ham sonuçlar ve sınırlar
  [G6 kaydındadır](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g6-cozme.md).
- **G4/G8 ölçümü ve v1 kararı kapandı:** Kullanıcının açık onayıyla canlı
  ölçüm; sorgu gömme 0,092–0,266 sn. 256 gerçek parçada tek toplu istek
  medyan 5,855 sn, dört paralel 4,893 sn; her iki kip 193 012 token.
  Mevcut toplu istek/WIT 0.1 korunur. Bir hız sınırı ve devam dahil
  3 047 820 token; iki platformda 994 + 114 / 936 + 113 test ve üç clippy/fmt
  temiz. Varsayılan takımda yeni canlı testle 14 test atlanır. Yöntem ve
  sınırlar [G4/G8 kaydındadır](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g4-g8-gomme.md).
- **G5 aday/dosya karşılaştırması ölçüldü:** Mevcut indeksin güncel kaynakla
  eşleşen 411 dosya/3564 parçasında, onaylı 32 sorgu. 20 ve 50 aday
  26/32; 20 aday sıralama tokenını %60,33 azalttı ama bir sorgu kazandırıp
  birini kaybettirdi. Sonuçta dosya tekilleştirme 28/32; aday seçerken dosya
  başına iki parça sınırı 25/32. Ürün 50 adayda kalır. 127 HTTP isteği
  2 251 896 token; bir aynı gövde yeniden gönderilmedi, indeksin 18 dosyası
  değişmedi. Bu kontrollü alt küme tam/güncel ürün kalite testi değildir.
  İki platformda 1000 + 114 / 942 + 113 test, üç clippy/fmt temiz; atlanan
  sayı 15. [Yöntem, ham kayıt ve karar](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g5-aday.md).
- **G5 talimat dili ölçüldü:** Aynı 32 sorgu ve kontrollü kümede İngilizce
  talimat ilk beş parçada 26/32, Türkçe 25/32; ilk beş farklı dosyada ikisi
  de 28/32. Türkçe bir sorgu kazanıp ikisini kaybetti; ürün İngilizce
  talimatta kalır. 128 HTTP isteği 2 548 682 token; indeksin 18 dosyası
  değişmedi. Windows 1002 + 114, WSL 944 + 113 test, üç clippy/fmt temiz;
  varsayılan takımda 16 test atlandı.
  [Yöntem, ham kayıt ve karar](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g5-dil.md).
- **G5 boyut karşılaştırması ölçüldü:** Onaylı 32 sorgu, kaynakla eşleşen
  411 dosya/3555 parça; 1024 ve 4096 ilk beş parçada aynı sorgularda 26/32,
  ilk beş farklı dosyada 28/32. Kaynak ve sorgu ön ek uyumu geçti.
  4096 kalite kazandırmadı, ham vektör alanı dört kat büyüdü; ürün
  İngilizce/50 aday/1024 boyutta kalır. 184 yeni HTTP isteği 4 821 637 token;
  indeksin 18 dosyası değişmedi. Windows 1007 + 114, WSL 949 + 113 test,
  üç clippy/fmt temiz; varsayılan takımda 17 test atlandı.
  [Yöntem, ham kayıt ve karar](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g5-boyut.md).
- **İlk indeksleme maliyeti ölçüldü (11 Ekim):** Ayrı boş indekste
  566 dosya/4 056 parça; yaklaşık **2,62 milyon token**. 3 309 parçanın
  2 157 582 tokenı doğrudan sayıldı; ilk kesintideki 747 parçanın tüketimi
  ölçülen oranla tahmindir (toplamın yaklaşık %18'i). Dakikalık pencerede
  429 tekrarları 30 dakika ilerlemedi; gövde görülmeden toplu isteği
  küçültme kararı verilmedi. Eski ve yeni indeks korunur.
  [Ölçüm ve sınırlar](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-11-kalfa-4c-ilk-indeks.md).
- **Açık ölçüm:** G5 tam/güncel indeksle gerçek wasm/host kalite doğrulaması;
  32 sorgu günlük kota sıfırlandıktan sonra (12 Ekim 03:00 TSİ), mevcut
  50 adayla; aynı sonuçtan ilk beş parça ve ilk beş farklı dosya ölçülür.
- **G5 parça kıyası ölçüldü (11 Ekim):** Aynı 408 tam doğrulanmış dosyada
  30/1200/5, 60/2400/10 ve 120/4800/20 satır/karakter/örtüşme pencereleri;
  ilk beş parçada **24/26/26**, ilk beş farklı dosyada **25/28/27** isabet
  (32 sorgu). Küçük pencere sıralama tokenını %43,94 azalttı ama kaliteyi
  düşürdü; büyük pencere %81,84 fazla tokenla net kalite kazandırmadı.
  Mevcut **60/2400/10** korunur. 9 Ekim kotadan ertelenen tarif 11 Ekim
  devam onayıyla koştu; üç hız sınırı ve ayrıca onaylı sınırlı devamla
  **320 başarılı/323 toplam HTTP**, **10 934 194 bilinen token**.
  325 deneme/16 milyon raporlanan bütçenin altında; başarısız tüketim bilinmez.
  İndeksin 18 dosyası değişmedi; tam/güncel ürün kalitesi ayrıca açık.
  Windows 1025 + 114, WSL 967 + 113 test; 18 test atlandı, üç clippy/fmt
  iki platformda temiz. Kaynak özetleri aynı, ürün kodu değişmedi.
  [Yöntem, ham kayıt ve karar](https://github.com/heimdilon/kalfa-arsiv/blob/f64a413af110006a258a3a283f50a52622190eaf/docs/superpowers/bulgular/2026-10-09-kalfa-g5-parca.md).

### 4d — Eklenti arayüzü ve dağıtım (uygulandı; PR #53 taslak)

[PR #53](https://github.com/heimdilon/kalfa-arsiv/pull/53), `4d/plan` → `4c/plan`.
Kod ve doğrulama kaydı bu PR'daki
`docs/superpowers/plans/2026-10-11-kalfa-4d-arayuz-dagitim.md`'dedir.

- Eklentiler ekranı; klasörden doğrulanan baytlarla yerel onaylı kurulum,
  güncelleme/kaldırma, alana göre açma ve gönderim iznini kaldırma.
- Atomik `eklentiler.json`; izin ve kurulu sürüm her turda denetlenir.
  İlk gönderim kartı, metin önizlemesi ve günlük sınırın arayüz durumu.
- NSIS'e yerleşik `kalfa-rag`; Linux paketlemesi henüz depoda yok.
- Claude'un `017aacd` kodunda Windows 1090 + 175 test, arayüz 388,
  gerçek AppContainer uyumluluk 24/24, e2e 38/38; WSL test/clippy/fmt,
  yalıtım 26, uyumluluk 16/16, e2e 38/38 geçti.
- **Codex inceleme düzeltmesi (`9980516`):** Aynı adlı yerleşik/eski kurulum
  Eklentiler ekranı ve izin panelinde iki ayrı anahtarla gösterilir;
  kullanılamayan kurulum kaldırılırken yerleşik korunur. İki regresyon
  önce Svelte hatasıyla düştü, düzeltmeyle geçti. Windows 1090 + 175,
  WSL 1033 + 175 test; iki platform üç clippy/fmt temiz; arayüz 390 test,
  0 hata/uyarı ve build temiz. Gerçek yalıtım/e2e bu arayüz düzeltmesinde
  yeniden koşmadı. [CI 148](https://github.com/heimdilon/kalfa-arsiv/actions/runs/38145500135)
  bu başta dört işiyle yeşil. G1: Windows sürüm kalfa.exe 41,85 MB (+17,2 MB).

**Sıradaki işler:**

1. Kota sıfırlandıktan sonra onaylı G5 tam/güncel kalite ölçümünü
   tamamlayın; mevcut indeksleri silmeyin. Bu koşuda 20 aday ölçülmez;
   50 adaydan dönen sonuçta dosya tekilleştirme de değerlendirilir.
2. G5 ve gerekli kayıtlar tamamlanınca #52 için açık squash onayı alın.
   #53'ün mevcut incelemesi ve iki platform/CI doğrulaması tamamlandı;
   #52 birleşince yeni tabana taşınır, yeni CI ve ayrı merge onayı gerekir.
3. Yalnız bu belge güncellemesi için push/CI başlatmayın: kullanıcının
   bildirdiği 281 dakikayı korumak için yerel belge farklarını sonraki
   yetkili değişiklikle birleştirin. CI kapsamı ve gerekli testler korunur.
