# Kalfa geliştirme durumu

Güncelleme: 2026-09-22. Bu dosya uygulamanın devam noktasıdır; tasarım
belgelerindeki tarihsel inceleme kayıtlarının yerine geçmez.

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
- **Uçtan uca:** gerçek dosya araçlarıyla, arayüz olmadan, okuma → onay →
  düzenleme → model yanıtı → kayıttan kurtarma akışı test edilir; aynı akış
  gerçek EVREN'e karşı `#[ignore]` testte de var (spec §4.2).

1b uygulama planındaki (`docs/superpowers/plans/2026-09-21-kalfa-1b-cekirdek.md`)
15 görevin tümü ayrı commit'ler halinde repoya işlendi (`git log --oneline` ile
görülebilir). Plandaki kutular tarihsel uygulama adımlarını tarif eder; bu
geçişte her göreve karşılık gelen kodun ve testlerin plan niyetini karşıladığı
yeniden doğrulandı (bkz. Doğrulama).

## Doğrulama

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

## Sıradaki aşama — 1c (Tauri + Svelte kabuğu)

1c'nin tasarımı konuşuluyor. 1c spec'i ve uygulama planı **henüz yazılmadı.**
Bu bölüm, konuşmada şimdiye kadar verilen kararları kaydeder; böylece başka bir
oturum ya da araç kaldığı yerden devam edebilir.

### Başlangıç kapsamı (henüz kesinleşmedi)

Tasarım belgesinin §11 ve §12.3 bölümlerinden:

1. Anahtar girişi ve kimlik deposu, şart metni/onayı, çalışma alanı seçimi.
2. `Komut`/`Zarf<Olay>` köprüsü, akış görünümü, iptal, araç onay kartları ve
   kayıttan oturum açma.
3. Dosya sistemi kimliği üzerinden çalışma alanı başına ortak kilit yöneticisi;
   oturum kaydı başına tek yazıcı sahipliği.
4. Olay birleştirme ve sert bayt/olay bütçeli IPC. Mevcut çekirdeğin sınırsız
   olay kanalı üretim arayüzü için bellek sınırı sağlamaz.
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

### Henüz konuşulmayanlar

- Beş maddelik kapsamın kesinleşmesi.
- Tauri/Svelte projesinin repodaki yeri ve yapısı.
- Olay köprüsü ve IPC bütçesinin tasarımı (§11.1'deki kabul kapısı).
- Kilit yöneticisi ve kayıt dosyası başına tek yazıcı.
- Pencere bazlı komut yetkileri ve şart penceresi.

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Henüz çalışır
bir masaüstü uygulaması veya tamamlanmış v1 sürümü yoktur.
