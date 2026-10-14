# Kalfa 1.1a-1 — Planın iki platformda ön doğrulaması (2026-10-14)

**Soru:** Plan 1.1a-1'in kodu, Codex'in plan incelemesindeki
(`INCELEME-PLAN-9d6b6b5`) dört düzeltmeyle birlikte Windows'ta ve Linux'ta
derleniyor ve testleri geçiyor mu?

**Onay:** Kullanıcı 14 Ekim'de şunları istedi:

- dört bulgunun, etkileşimli ağ testlerinin ve test komutlarının plana
  işlenmesi,
- ayrı worktree'de Windows ve WSL ön doğrulaması,
- çıkan değişikliklerin plana geri yazılması.

Bu bir ürün uygulaması değildir. Kod yalnız yerel bir denetim dalındadır.

## Ortam

- **Windows:** Windows 11 Home 10.0.26200, Kalfa yalıtımı kurulu makine,
  Rust 1.98.1. Worktree `C:\Users\kor_a\kalfa-11a1-on`.
- **Linux:** WSL2, Ubuntu 24.04, `kalfatest` kullanıcısı. Ayrı kopya
  `/home/kalfatest/kalfa-11a1-on`; `CARGO_TARGET_DIR` ayrı. Kopya
  Windows worktree'siyle dosya dosya karşılaştırıldı.
- **Dal:** `deneme/11a-1-on-dogrulama`, commit `a6a3a17`
  (taban `9d6b6b5`). Yalnız yerel; push edilmedi.
- Paket kurulmadı, sistem ayarı değişmedi, yönetici gerekmedi.

## Sonuç

AGENTS'ın iki platform komutları:

| Komut | Windows | WSL (`kalfatest`) |
|---|---|---|
| `cargo fmt --all -- --check` | Geçti | Geçti |
| `cargo test --workspace --locked` | Geçti | Geçti |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Temiz | Temiz |
| `cargo clippy -p kalfa-sandbox -p kalfa-tools --features "… yalitim-testleri" --all-targets --locked -- -D warnings` | Temiz | Temiz |
| `cargo build -p kalfa-sandbox --bin kalfa-kopru --locked` | — | Geçti |
| `cargo test -p kalfa-sandbox --features yalitim-testleri --locked --test yalitim` | 50/50 (92 sn) | 32/32 (17 sn) |
| `cargo test -p kalfa-tools --features yalitim-testleri --locked --test uyumluluk` | 24/24 | 16/16 |

Windows'ta yalıtım testleri `--test-threads=1` ile koştu. Arayüz ve e2e
komutları gerekmedi: değişiklik yalnız `kalfa-sandbox`'ta.

### Yeni testler

| Test | Windows | Linux | Ne gösterir |
|---|---|---|---|
| `etkilesimli::tests` (4) | Geçti | Geçti | Sunucu adı, özel kök, ortam, kip tablosu |
| `etkilesimli_tam_erisim_borulari_birebir_yankilar_ve_eofta_cikar` | Geçti | Geçti | G1.1, tam erişim |
| `etkilesimli_kapat_disarida_tutulan_yaziciya_ragmen_eof_verir` | Geçti | Geçti | Codex 4a |
| `etkilesimli_kapat_okumayan_sureci_ve_bloke_yazmayi_bitirir` | Geçti | Geçti | Codex 4b, Review Focus 2 |
| `etkilesimli_kapat_donunce_torun_yasamaz` | Geçti | Geçti | Codex 1a |
| `etkilesimli_birakilan_surecin_agaci_olur` | Geçti | Geçti | Review Focus 3 |
| `etkilesimli_setsid_torun_kapat_ve_birakmada_olur` | — | Geçti | Codex 1c |
| `etkilesimli_gecersiz_ad_ve_tam_erisimde_ag_listesi_reddedilir` | Geçti | Geçti | Review Focus 1 |
| `etkilesimli_tam_erisim_ozel_kokte_calisir` | Geçti | Geçti | §3 çalışma dizini |
| `etkilesimli_borular_birebir_yankilanir` | Geçti | Geçti | G1.1, yalıtımlı ürün başlatıcısı |
| `etkilesimli_uzun_yasar_kapat_agaci_durdurur` | Geçti | Geçti | G1.2; Linux'ta torun `setsid` ile |
| `etkilesimli_sunucu_koku_yetenek_sidiyle_ayrilir_ve_izinler_geri_alinir` | Geçti | — | G1.3, Codex 3, Review Focus 4–5 |
| `etkilesimli_sunucu_ozel_koku_kabuktan_ve_obur_sunucudan_ayri` | — | Geçti | G1.3 karşılığı, Review Focus 4 |
| `etkilesimli_guvenli_olmayan_mevcut_kok_reddedilir` | Geçti | — | Codex 3 |
| `etkilesimli_ortam_ozel_koke_ve_sisteme_yonelir` / `…_yonelir` | Geçti | Geçti | G1 ortam bulgusu, ayrılmış adlar |
| `etkilesimli_tam_erisim_yalitim_hazirken_yalitimsiz_baslar` | Geçti | Geçti | §4.1 kip tablosu |
| `etkilesimli_agli_sunucu_vekilden_cikar_ve_kapanista_temizlenir` | Geçti | Geçti | Codex 2, ağlı süreç |

G1.2'nin ürün testi 5 ping × 3 sn ile yaklaşık 15 sn yaşar. 10 dakikalık
mutlak koruma yaşı 1.1a-2'nin kabul sınırıdır.

### Mutasyon denetimleri

Düzeltmeler geçici olarak geri alındı; testler düzeltmeyi gerçekten
ölçüyor mu diye bakıldı.

| Geri alınan | Sonuç |
|---|---|
| `kapat()`'ta stdin aktarma görevinin kesilmesi | `…disarida_tutulan_yaziciya_ragmen_eof_verir` düştü (Windows). |
| Linux'ta torun taraması (boş liste) | `…setsid_torun_kapat_ve_birakmada_olur` düştü; temizlik koruyucusu kalan süreci aldı (Linux). |
| Öldürmeden sonra grubun boşalmasını bekleme | Linux'ta `…kapat_donunce_torun_yasamaz` düştü. Windows'ta geçti: torun `TerminateJobObject`'ten sonra, `tasklist` sorgusundan önce ölüyor. Windows'ta bu adımı ayırt eden bir test yok; sözleşme gereği kalır. |

Her mutasyondan sonra kod geri yüklendi ve testler yeniden geçti. WSL
eşitlemesi ilk sürümünde dosyaların eski zaman damgasını koruyordu; geri
yüklenen dosyayı cargo değişmemiş sayıyordu. Eşitleme içerik
karşılaştırmalı `rsync -c`'ye çevrildi, sandbox dosyaları tazelendi ve
testler yeniden koşturuldu.

## Ön doğrulamada bulunanlar

Plana işlendi (planın "Ön doğrulamada plana işlenen değişiklikler"
bölümü):

1. **Windows'ta kök iki kipte korumalı kurulmalı.** Tam erişim önce
   başlayınca `mcp` mirası açık oluşuyor, sonraki yalıtımlı başlatma
   reddediliyordu.
2. **AppContainer `LOCALAPPDATA`'yı çevirir.** Kapta değer
   `ev\Packages\<profil>\AC` oluyor; özel kökün içinde kalıyor.
3. **Windows ortam birleştirmesi.** Taslak, sunucu ortamının bütün
   değerlerini yalıtımın ortamının üstüne yazıyordu. Bu, kullanıcının
   `CARGO_HOME`'u gibi değerleri sunucunun özel değerinin üstüne
   yazabilirdi. Artık yalıtımın koyduğu değerler kalıyor; yalnız yalıtımın
   düşürdükleri (`GITHUB_TOKEN` gibi) geri ekleniyor.
4. **`Yalitimsiz` değişmedi.** Taslaktaki alan eklemesi altı crate'in
   testlerini kırardı. Tam erişim testleri `surec.rs` birim testine
   taşındı.
5. **Linux:** `deneme` işlevi de `komut_kur`'u çağırıyordu. G1.2
   testindeki işaretler bwrap'ın komut satırında da göründüğü için `$0`'dan
   kuruldu.

## Temizlik

- **Windows:** Testlerden önce makinede kullanıcıya ait bir Kalfa profili
  vardı (`kalfa.alan.28af…`); ona dokunulmadı. Testlerden sonra:
  - Kalfa profili 1 (aynı kullanıcı profili), `Packages\kalfa*` 1,
    çalışan `kalfa-kopru` ve `timeout` süreci 0.
  - Yeni testlerin `mcp` dizini ve sunucu yetenek SID'i makinede kalmadı.
  - Ayrı bulgu: Mevcut bir kabuk testi her koşuda %TEMP%'te silinemeyen
    bir `.tmp*` dizini bırakıyor. İçinde araç zinciri yeteneğinin açık
    ACE'si var (`ev\.cargo\bin`). Eylül'den beri 44 tane birikmiş; bu iki
    koşu 2 ekledi. Yeni testlerden gelmiyor; ayrı görev olarak önerildi.
- **WSL:** Testlerden sonra `sleep 300`, `kalfa-g1-*`, `kalfa-agli-*` ya da
  `kalfa-kopru` süreci kalmadı.
- Worktree ve WSL kopyası, planın yeniden incelemesi bitene kadar
  tutuluyor: inceleme yeni bir değişiklik isterse ön doğrulama yeniden
  koşar. İnceleme bitince ikisi de kaldırılır.

## Karar ve sınırlar

- Plan 1.1a-1 r2'nin kodu iki platformda derlenip testleri geçti. Plan
  kod blokları bu commit'teki dosyalardan birebir üretildi.
- **Ölçülmeyenler:**
  - `kapat()`'ın `Err` yolu: öldürme ya da bitiş doğrulaması hatası,
    çekirdek düzeyinde bir hata gerektirir; zorlanmadı.
  - Linux tam erişimde, kök kendiliğinden çıktıktan sonra yaşayan `setsid`
    torunu (tasarım §4.1'de açıkça kapsam dışı).
  - Gerçek bir MCP sunucusu ve `rmcp` (1.1a-2).
