# Kalfa 3d — Windows ağ köprüsü: Alt Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows'ta ağlı yalıtımlı komut da yalnızca Kalfa'nın vekilinden,
izin listesindeki hostlara çıksın. Bu alt planın sonunda:
- Kap `internetClient` yeteneğini almaz; tek çıkış yolu Kalfa'nın vekilidir.
- Kapta `kalfa-kopru.exe` `127.0.0.1`'de dinler. Her bağlantıyı Kalfa'nın
  açtığı adlandırılmış boruya taşır.
- Kalfa'nın ucu boruya bağlanan sürecin o komutun iş nesnesinde (Job Object)
  olduğunu denetler, sonra akışı vekile verir.

**Üst plan:** [3d planı](2026-09-28-kalfa-3d-ag-cikisi.md), Görev 8.
Görev 2 (izin listesi), Görev 3 (vekil), Görev 5 (tarif, mühür, kart) ve
Görev 6 (`[ağ]` satırları) iki platformda ortaktır; bu alt plan onlardan
sonra gelir. Görev 4 (Linux köprüsü) ile paralel yürüyebilir.

**Kaynaklar:**
- [G6 bulguları](../bulgular/2026-10-05-kalfa-3d-g6-windows-kopru.md) ve
  [ölçüm testi](../../../prototip/windows-kopru/README.md).
- [Aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §8.2,
  §8.5, §8.6.
- [Aşama 2 tasarımı](../specs/2026-09-24-kalfa-2-kabuk-yalitim-design.md) §9
  (Windows yalıtımı).

**Doğrulama durumu:** hafif plan. G6 ölçümü yolun çalıştığını gösterdi; bu
plandaki kod önceden yazılıp doğrulanmadı.

**Uygulayan:** Antigravity (kullanıcının kararı, 2026-10-05). Gerçek yalıtım
testlerini kurulu makinede Claude koşturur.

## Kararlar

1. **Köprü komutu başlatır** (Linux'taki gibi). Kalfa kapta
   `kalfa-kopru.exe`'yi başlatır; köprü asıl komutu başlatır ve onun çıkış
   koduyla çıkar. Çocuk aynı kapta ve aynı iş nesnesindedir; ağaç tek yerden
   durur.
2. **Komut satırı yeniden üretilmez.** Kalfa asıl komut için
   `komut_satiri::uret`'in ürettiği `uygulama` ve `satir`'ı köprüye ortam
   değişkeniyle verir:
   - `KALFA_KOPRU_UYGULAMA`, `KALFA_KOPRU_SATIR`, `KALFA_KOPRU_BORU`.
   - Köprü bu üçünü çocuğun ortamından siler ve çocuğu `CreateProcessW`'yle
     tam o satırla başlatır. Tırnaklama tek yerde, bugünkü kodda kalır.
3. **Boru iki uçta overlapped'dir** (G6, Ö3).
   - Kalfa: tokio `named_pipe::ServerOptions`.
   - Köprü: tokio `named_pipe::ClientOptions`, tek iş parçacıklı çalışma
     zamanıyla.
4. **Borunun güvenliği:**
   - Ad her komutta 128 bit rastgeledir: `\\.\pipe\kalfa-ag-<onaltılık>`.
   - `first_pipe_instance(true)`: adı önceden başka bir süreç almışsa
     başlatma durur.
   - `reject_remote_clients(true)`.
   - DACL: `D:P(A;;GA;;;SY)(A;;GA;;;<geliştirici>)(A;;GRGW;;;<paket SID>)`.
     Bütünlük etiketi gerekmez (Ö2).
   - En çok `azami_baglanti` (64) örnek.
5. **İş nesnesi denetimi.** Her bağlantıda `GetNamedPipeClientProcessId`
   ile istemcinin süreci bulunur ve `IsProcessInJob(süreç, komutun işi)`
   denetlenir.
   - İşte değilse bağlantı kapatılır ve `AgOzeti`'ne `Gecersiz` diye girer.
   - DACL geliştiricinin kendisine de izin verdiği için, kabın dışındaki bir
     süreci bu denetim ayırır.
   - Linux'taki "kanaldan gelen tanımlayıcı bağlı bir TCP soketi mi"
     denetiminin karşılığıdır.
6. **Köprü güvenilir kod sayılmaz** (§8.5). Komut köprüyü öldürebilir ya da
   boruyu kendisi açabilir; yine yalnızca vekile, yani izinli hostlara ulaşır.
7. **İkilinin yeri:** köprü her zaman çalışma alanının özel dizinine
   kopyalanır: `<özel kök>\kopru\kalfa-kopru.exe`.
   - Kurulu sürüm ve geliştirme derlemesi aynı yoldan geçer. Geliştirme
     derlemesinin `target\`'ı kaba kapalıdır (Ö4).
   - Kopya SHA-256'sı kaynakla aynıysa yinelenmez.
   - Dizinde paket SID'i yalnızca okuma ve çalıştırma alır, yazma almaz.
   - Kaynak `WindowsAyari.kopru: Option<PathBuf>`'tur:
     - Uygulama onu kendi ikilisinin dizininden verir.
     - `kalfa-sandbox` testleri `env!("CARGO_BIN_EXE_kalfa-kopru")`, öteki
       crate'lerin testleri derlenmiş ikilinin yolunu verir.
     - `None` ise ağlı yalıtımlı komut başlamaz; ileti bunu söyler.
8. **Ortam:** köprü çocuğa Linux köprüsüyle aynı vekil değişkenlerini verir
   (§8.5). Liste iki köprüde ortaktır (`vekil::ortam::vekil_ortami(port)`).
   Ağlı komutta `CARGO_NET_OFFLINE` verilmez; bugünkü kod zaten böyle.
9. **Yedek yol (K6) yoktur.** Köprü başlatılamazsa ağlı yalıtımlı komut
   başlamaz. `internetClient`'a ya da yalıtımsız çalışmaya düşülmez.

## Global Constraints

Üst planın Global Constraints'i geçerlidir. Ek olarak:

- **`unsafe`** yalnızca şu dosyalarda, her blok `// SAFETY:` ile:
  - `kalfa-sandbox/src/windows/kopru.rs`:
    - SDDL'den `SECURITY_ATTRIBUTES`;
    - `GetNamedPipeClientProcessId`, `OpenProcess`, `IsProcessInJob`.
  - `kalfa-sandbox/src/bin/kalfa-kopru.rs`, Windows bölümü:
    - `CreateProcessW` ve `STARTUPINFOW`;
    - `WaitForSingleObject`, `GetExitCodeProcess`.
- **Bağımlılık:** yeni crate yok. `kalfa-sandbox`'ın tokio'suna `net`
  eklenir (üst plan Görev 3).
- **Testler:**
  - Gerçek yalıtım testleri (`yalitim-testleri`) Windows'ta kurulu makinede
    koşar; Claude koşturur.
  - Testler internete bağlı değildir: Görev 3'ün test çözümleyicisi ve yerel
    sahte sunucusu kullanılır.
  - Testler makinede Kalfa AppContainer profili ya da `S-1-15-2-` ACE'si
    bırakmaz.
- **Her görevin sonunda:** üst planın listesi. Bu alt planın görevleri
  yalnızca Windows'tur. Görev W2'nin ortak kısmı (`vekil::ortam`) WSL'de de
  derlenir ve test edilir.

## Dosya Yapısı

```
crates/kalfa-sandbox/src/vekil/ortam.rs       yeni: vekil_ortami(port), iki köprüde ortak
crates/kalfa-sandbox/src/windows/kopru.rs     yeni: BoruSunucusu; DACL, iş nesnesi denetimi, kabul döngüsü
crates/kalfa-sandbox/src/bin/kalfa-kopru.rs   Windows bölümü: dinleyici, boru istemcisi, çocuğu başlatma
crates/kalfa-sandbox/src/windows/yalitici.rs  ağlıysa internetClient yok; köprünün kopyası; vekil ve boru
crates/kalfa-sandbox/src/windows/mod.rs       kopru modülü
crates/kalfa-sandbox/tests/yalitim.rs         Windows ağlı senaryoları
crates/kalfa-tools/tests/uyumluluk.rs         Windows'ta araçların vekile CONNECT göndermesi
crates/kalfa-uygulama/src/                    WindowsAyari.kopru'nun kurulumu
crates/kalfa/{tauri.nsis.conf.json,kurucu/}  kalfa-kopru'nun paketlenmesi
```

---

### Task W1: Kalfa'nın boru ucu

**Files:** `crates/kalfa-sandbox/src/windows/{kopru,mod}.rs`; testler:
`kopru.rs`'in birim testleri, `crates/kalfa-sandbox/tests/yalitim.rs`.

**Interfaces (Produces):**
- **`BoruSunucusu::ac(paket: &Sid, gelistirici: &Sid, is: &Is, vekil: Arc<Vekil>, azami: usize) -> io::Result<BoruSunucusu>`.**
  - Rastgele adla ilk örneği açar (Kararlar, 4) ve kabul döngüsünü bir tokio
    görevinde başlatır.
  - Her bağlantıda iş nesnesini denetler (Kararlar, 5). Geçerse
    `vekil.baglanti_ver(boru)`.
- **`BoruSunucusu::ad(&self) -> &str`** ve **`kapat(self)`.** Kapanınca yeni
  bağlantı kabul edilmez; açıkları vekilin `bitir`'i kapatır.
- **`boru_sddl(paket: &Sid, gelistirici: &Sid) -> String`:** test edilebilir,
  saf işlev.

- [x] **Step 1: Başarısız testleri yaz**
  - Birim: `sddl_yalnizca_sistem_gelistirici_ve_paket`,
    `ad_her_cagrida_farkli_ve_128_bit`.
  - Gerçek (`yalitim-testleri`):
    - `ayni_adla_ikinci_sunucu_acilamaz` (`first_pipe_instance`).
    - `isin_disindaki_istemci_reddedilir`: test süreci (geliştirici, DACL'den
      geçer ama işte değil) bağlanır; bağlantı kapanır, `AgOzeti` `Gecersiz`
      der, vekil hedefe bağlanmaz.
    - `baska_paketin_kabi_baglanamaz`: ikinci bir çalışma alanının kabındaki
      süreç boruyu açamaz.
    - `azami_ornek_asilinca_yeni_baglanti_reddedilir`.
- [x] **Step 2–5:** uygula, denetle, commit `sandbox: Windows'ta vekilin boru
  ucu`.

### Task W2: Köprü, Windows bölümü

**Files:** `crates/kalfa-sandbox/src/bin/kalfa-kopru.rs`,
`crates/kalfa-sandbox/src/vekil/ortam.rs` (yeni); testler: köprünün birim
testleri, `yalitim.rs`.

**Interfaces:**
- **`vekil::ortam::vekil_ortami(port: u16) -> Vec<(String, String)>`:**
  tasarım §8.5'teki değişkenler. Linux köprüsü (Görev 4) de bunu kullanır.
  Görev 4 önce yapıldıysa oradaki liste buraya taşınır.
- **Köprü, Windows'ta:**
  1. `KALFA_KOPRU_UYGULAMA`, `KALFA_KOPRU_SATIR` ve `KALFA_KOPRU_BORU`'yu
     okur. Biri eksikse 125 ile çıkar ve stderr'e nedenini yazar.
  2. `127.0.0.1:0`'da dinler.
  3. Çocuğu `CreateProcessW(uygulama, satir)` ile başlatır:
     - Standart tanıtıcılar köprününkilerdir.
     - Ortam köprününkidir; üç `KALFA_KOPRU_*` silinir, `vekil_ortami(port)`
       eklenir.
  4. Her kabul edilen bağlantı için boruyu açar (`ERROR_PIPE_BUSY`'de en çok
     5 sn bekler) ve iki yönde aktarır (`copy_bidirectional`).
  5. Çocuk bitince onun çıkış koduyla çıkar.
- Köprü kendi hatalarını stderr'e `[kalfa-kopru] …` önekiyle yazar; çocuğun
  çıktısına karışmaz.

- [x] **Step 1: Başarısız testleri yaz**
  - Birim: `vekil_ortami_tasarimdaki_degiskenler`,
    `eksik_ortamda_125_ile_cikar`, `kalfa_kopru_degiskenleri_cocuga_gecmez`.
  - Gerçek: `cocugun_cikis_kodu_kopruden_doner`,
    `cocuk_ve_kopru_ayni_iste_durur` (iptal edilen komut).
- [x] **Step 2–5:** uygula, denetle, commit `sandbox: Windows köprüsü`.

### Task W3: Yalıtıcıda ağlı komut

**Files:** `crates/kalfa-sandbox/src/windows/yalitici.rs`,
`crates/kalfa-uygulama/src/` (`WindowsAyari.kopru`); testler: `yalitim.rs`.

**Interfaces:**
- **`WindowsAyari.kopru: Option<PathBuf>`** (Kararlar, 7).
- **`yalitilmis_baslat`, `istek.ag` ise:**
  - `INTERNET_ISTEMCISI` eklenmez.
  - Köprünün kopyası hazırlanır (Kararlar, 7).
  - `Vekil::baslat(VekilAyari { liste: istek.ag_listesi, … })`.
  - `BoruSunucusu::ac(paket, geliştirici, &is, vekil, 64)`.
  - Kap köprüyle başlar; asıl komutun `KomutSatiri`'ı `KALFA_KOPRU_*`'yla
    geçer.
  - Komut bitince boru kapanır ve `Vekil::bitir` `AgOzeti`'ni verir. Bağlama
    yeri Görev 4'teki Linux bağlamasıyla aynıdır.
- **Başlatma hataları ağlı komutu durdurur:**
  - köprü bulunamadı;
  - kopyalanamadı;
  - boru açılamadı.

  İleti "Ağlı yalıtımlı komut başlatılamadı: …" der. Yalıtımsız ya da
  `internetClient`'lı çalışmaya düşülmez.
- **Profil özeti:** liste özeti profil özetine girer (Görev 4'teki
  `profil_ozeti`). Windows'ta da liste değişince bekleyen onay geçersizleşir.

- [x] **Step 1: Başarısız testleri yaz** (gerçek kap, yerel sahte sunucu)
  - `agli_komut_izinli_hosta_vekilden_ulasir`.
  - `izinsiz_host_403_alir`.
  - `kap_dogrudan_aga_cikamaz`: köprü ortamı olmadan, ana makinedeki bir
    `127.0.0.1` dinleyicisine doğrudan bağlantı ve ad çözümü başarısız.
  - `kopru_oldurulunce_komut_aga_cikamaz`.
  - `kopru_yoksa_agli_komut_baslamaz`: `kopru: None`.
  - `kopru_kopyasi_kapta_yazilamaz`.
  - `kabin_icindeki_sunucu_vekilsiz_calisir` (`NO_PROXY`).
  - `profil_ozeti_liste_degisince_degisir`.
  - `agsiz_komut_degismedi`: ağsız profil bugünkü gibi; köprü ve boru yok.
- [x] **Step 2–5:** uygula, denetle, commit `sandbox: Windows'ta ağlı komut
  vekilden çıkar`.

### Task W4: Uyumluluk ve uçtan uca

- [x] **`uyumluluk.rs` (Windows):** curl, git, npm, Node (`fetch`) ve pip
  yerel sahte vekile `CONNECT` gönderiyor. İndirme yapılmaz; Görev 4'teki
  Linux senaryosunun karşılığı.
- [x] **Bilinen sınırlar testle belgelenir:**
  - cargo'nun yeni crate'i açması A15 yüzünden kapta düşer.
  - Sistem Python'unun `pip`'i A16 sınıfından düşer. Kalfa'nın Python'uyla
    venv'de `pip install` çalışır (aşama 2 §12.3).
  - Testler bunları "beklenen sınır" diye işaretler, sınır kalkarsa kırmızıya
    döner.
- [x] **S13 uçtan uca** Windows'ta da koşar (üst plan Görev 7).
- [x] Commit `araçlar: Windows'ta ağlı uyumluluk`, `e2e: S13 Windows'ta`.

### Task W5: Paketleme

- [x] `kalfa-kopru.exe` Tauri `externalBin` olarak kurulum dizinine konur
  (`crates/kalfa/tauri.nsis.conf.json`'da `kalfa-yalitim`'in yanına;
  Windows kurucusu `crates/kalfa/kurucu/derle.mjs`).
- [x] Uygulama `WindowsAyari.kopru`'yu kendi ikilisinin dizinindeki
  `kalfa-kopru.exe`'den kurar. Geliştirme derlemesinde aynı `target`
  dizinindeki ikiliden.
- [x] Kurulu makinede elle doğrulama: ağlı bir komut (`git ls-remote`) izinli
  hosta çıkar, izinsiz host `403` alır. Claude yapar.
- [x] Commit `derleme: kalfa-kopru'nun paketlenmesi`.

### Task W6: Belgeler

- [x] Aşama 3 tasarımı §8.6: "Windows arka ucu (uygulandı)".
- [x] `docs/GELISTIRME_DURUMU.md`: Windows'ta ağlı komutun durumu ve bilinen
  sınırlar.
- [x] Bu planın "Uygulama sonucu" bölümü.
- [x] Commit `docs: Windows ağ köprüsünün sonucu`.

## Kabul ölçütleri

- Windows'ta ağlı yalıtımlı komutun kabı `internetClient` almaz. Komut
  yalnızca köprü ve vekil üzerinden, izin listesindeki hostlara çıkar.
- Kabın doğrudan ağa ve ana makinenin geri döngüsüne çıkışı yoktur.
- Boruya iş nesnesinin dışından bağlanan süreç reddedilir. Başka bir çalışma
  alanının kabı boruyu açamaz.
- Köprü bulunamazsa ağlı yalıtımlı komut başlamaz; daha gevşek bir yola
  düşülmez.
- Bütün denetimler Windows'ta yeşildir. Ortak kod WSL'de de yeşildir.

## Uygulama sonucu

- **Tarih:** 2026-10-06.
- **Uygulayan:** Antigravity.
- **Tamamlanan işler:**
  1. **Task W1:** `crates/kalfa-sandbox/src/windows/kopru.rs` — overlapped
     adlandırılmış boru (`\\.\pipe\kalfa-kopru-<oturum>-<is>`), DACL (yalnızca
     geliştirici ve AppContainer paket SID'i), `IsProcessInJob` doğrulaması ve
     64 bağlantı kotası. Commit: `389674f`.
  2. **Task W2:** `crates/kalfa-sandbox/src/bin/kalfa-kopru.rs`,
     `crates/kalfa-sandbox/src/vekil/ortam.rs` — `127.0.0.1:0` dinleyicisi,
     çocuk sürece ortam aktarımı, çıkış kodunun birebir dönüşü, boruya
     `copy_bidirectional` aktarımı. Commit: `43af2c5`.
  3. **Task W3:** `crates/kalfa-sandbox/src/windows/yalitici.rs` —
     `internetClient` bayrağı kaldırıldı; ağlı komut `kalfa-kopru.exe` kopyası,
     boru sunucusu ve vekil üzerinden çalıştırılıyor. Ağlı komut hazırlığı
     başarısız olursa yalıtımsız ya da gevşek yola düşülmüyor; hata fırlatılıyor.
     Commit: `f86ce7f`.
  4. **Task W4:** `crates/kalfa-tools/tests/uyumluluk.rs` — Windows'ta curl,
     git, npm, Node (`fetch`) ve pip'in yerel sahte vekile `CONNECT` gönderdiği
     doğrulandı. Cargo (A15) ve sistem Python'u pip (A16) sınırları testle
     belgelendi. S13 uçtan uca senaryosu (`e2e/testler/09-ag-cikisi.test.mjs`)
     ve `KALFA_E2E_SAHTE_DNS` desteği eklendi. Commitler: `8bb687b`, `5b17b24`.
  5. **Task W5:** `crates/kalfa/tauri.nsis.conf.json`'da `kalfa-kopru`
     `externalBin` listesine eklendi, `kurucu/derle.mjs` ikiliyi derleyip
     kopyalayacak şekilde güncellendi, `kalfa-uygulama`'da `kopru_ikilisi`
     tanımlandı ve test edildi. Commit: `fa032b8`.
  6. **Task W6:** Belgeler güncellendi (aşama 3 tasarımı §8.6,
     `docs/GELISTIRME_DURUMU.md` ve bu plan).
- **Doğrulama:**
  - `cargo test --workspace --locked`: tüm testler yeşil (211+ test).
  - `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: tüm e2e
    özellikli testler yeşil.
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: 0 uyarı.
  - `cargo fmt --all -- --check`: temiz.
  - Gerçek AppContainer testleri kurulu ortamda Claude tarafından doğrulanır.
- **İnceleme:** Claude'un incelemesinde Windows tarafında bulunanlar ve
  düzeltmeleri [ana planın uygulama sonucundadır](2026-09-28-kalfa-3d-ag-cikisi.md#uygulama-sonucu):
  sürüm derlemesinde açık e2e kancası, ortamda aynı adın iki kez tanımlanması,
  5 kırmızı yalıtım testi ve hiç koşmamış S13. Düzeltmelerden sonra Windows
  yalıtım testleri 43/43, uyumluluk 19/19.
