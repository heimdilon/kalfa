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

## Sıradaki aşama

1c planı **henüz yazılmadı.** Kullanıcı önce tasarımı birlikte düşünmek
istiyor; bu yüzden bu geçişte 1c'nin uygulama planı oluşturulmadı. Tasarım
belgesinin §11 ve §12.3 bölümleri ile birlikte ele alınması gereken açık
noktalar (1c planına girdi olacak):

1. Anahtar girişi ve kimlik deposu, şart metni/onayı, çalışma alanı seçimi.
2. `Komut`/`Zarf<Olay>` köprüsü, akış görünümü, iptal, araç onay kartları ve
   kayıttan oturum açma.
3. Dosya sistemi kimliği üzerinden çalışma alanı başına ortak kilit yöneticisi;
   oturum kaydı başına tek yazıcı sahipliği.
4. Olay birleştirme ve sert bayt/olay bütçeli IPC. Mevcut çekirdeğin sınırsız
   olay kanalı üretim arayüzü için bellek sınırı sağlamaz.
5. Pencereye göre komut yetkileri, sıkı CSP ve güvenli Markdown gösterimi.

Kabuk yalıtımı, bağlam özetleme, geri alma, gönderim günlüğü, oturum bütçesi,
güvenlik denetçisi ve WASM eklentileri sonraki aşamalardadır. Henüz çalışır
bir masaüstü uygulaması veya tamamlanmış v1 sürümü yoktur.
