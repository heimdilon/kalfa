# Yetenek 02 (Proje Akışı: G1a + G1b + Arayüz) — Uygulama ve Doğrulama Raporu

**Tarih:** 2026-10-20
**Dal:** `codex/plan-yetenek-proje-akisi` ([PR #38](https://github.com/heimdilon/kalfa/pull/38))
**Uygulayıcı:** `Antigravity-Hat-02`
**Ortak Çekirdek/IPC Sözleşme Commit'i:** `d627f6d51eb705e0ec39ecf3e9c021004c07914d` (`d627f6d`)

## 1. Kapsam ve Ortak Sözleşme Özeti

Bu hat ([PR #36](https://github.com/heimdilon/kalfa/pull/36) koordinasyon sırasındaki ilk çekirdek/IPC entegrasyon hattı) üç bileşeni tek sözleşmede birleştirir:

1. **G1a — Sınırlı Proje Talimatları (`AGENTS.md`):**
   - `crates/kalfa-core/src/proje_talimatlari.rs`: `ProjeTalimatKapsami` (`Kok`, `AltDizin { dizin }`), `ProjeTalimatKaynagi`, `ProjeTalimatUyariTuru`, `ProjeTalimatUyarisi`, `ProjeTalimatDegisimi`, `ProjeTalimatGoruntusu`, `ProjeTalimatOkuyucu` yetenek arayüzü.
   - Sabit sınırlar: dosya başına **16 KiB** (`DOSYA_BOYUT_SINIRI`), tur bağlamı başına **64 KiB** (`BAGLAM_BOYUT_SINIRI`) ve **16 kaynak** (`BAGLAM_KAYNAK_SINIRI`). Sınır aşımı veya güvenlik reddi (`DosyaBoyutuAsildi`, `BaglamBoyutuAsildi`, `KaynakSayisiAsildi`, `GecersizUtf8`, `SembolikBaglantiReddedildi`, `GizliVeyaKorumaliYolReddedildi`, `Okunamadi`) sessizce kırpılmaz; `uyarilar` listesiyle hem modele hem arayüze bildirilir.
   - Kapsam ve öncelik: Kök `AGENTS.md` geneldir (öncelik `0`); alt dizin `AGENTS.md` dosyaları yalnız kendi alt dizinlerindeki hedefler için geçerlidir ve o alt dizindeki hedeflerde daha derin kapsam önceliklidir (`oncelik` = derinlik).
   - Tur anlık görüntüsü ve sürüm değişimi: Tur içinde yüklenen kaynağın sürümü (`icerik_ozeti` ve `yuklendigi_tur`) sabit tutulur; sonraki turda değişen veya silinen kaynaklar `degisen_kaynaklar` altında raporlanır.
   - Yetki ve güvenlik sınırı: Proje talimatları `Kip` değerini yükseltemez, onay veya yalıtım sınırlarını gevşetemez, dış `@include`/URL/üst dizin referanslarını kendiliğinden izlemez. Ham dosya metni olay/kayıt/UI'ye sızmaz; yalnız sistem mesajına ve mevcut `GonderimOnizleme` akışına girer.
   - `crates/kalfa-uygulama/src/proje_talimatlari.rs` (`CalismaAlaniTalimatOkuyucu`): Çekirdek doğrudan dosya sistemi açmaz; `kalfa_tools::CalismaAlani` (`cap-std`) üzerinden sembolik bağlantıları, çoklu sabit bağlantıları (`nlink > 1`), gizli/korumalı yolları ve 16 KiB üstü dosyaları belleğe almadan reddeder.
   - Bağlam sıkıştırma (`compact`) ve kurtarma (`durum_ozeti`): `ProjeTalimatGoruntusu` üstverisi (`kaynaklar`, `uyarilar`, `degisen_kaynaklar`, `toplam_bayt`) JSONL kaydında (`KayitOlayi::ProjeTalimatlariYuklendi`), `KayitOzet::proje_talimat_goruntusu`'nda ve compact sonrasında korunur.

2. **G1b — `plan_guncelle` ve `kullaniciya_sor` Çekirdek Araçları:**
   - `crates/kalfa-core/src/plan.rs` (`PlanGuncelleAraci`, `PlanDurumu`, `PlanAdimi`, `PlanAdimDurumu`):
     - Oturum planını bütünüyle değiştirir; `1..=32` adım, benzersiz kimlik (`≤64` karakter), adım başlığı `≤200` karakter, aynı anda en çok 1 `Suruyor` adımı.
     - Disk veya kabuk yetkisi oluşturmaz (`kilitsiz: true`, `Risk::Dusuk`).
     - `Olay::PlanGuncellendi` ve `KayitOlayi::PlanGuncellendi` üretir; kurtarmada `KayitOzet::plan` olarak geri yüklenir.
   - `crates/kalfa-core/src/soru.rs` (`KullaniciyaSorAraci`, `KullaniciSorusu`, `SoruSecenegi`, `KullaniciSorusuYaniti`, `SoruIptalNedeni`):
     - Tek etkin soru; başlık `≤500` karakter, `2..=4` benzersiz kimlikli seçenek (`≤200` karakter), isteğe bağlı serbest metin (`≤4000` karakter).
     - Araç onayından (`OnayGerekiyor`) tamamen ayrıdır; yanıtlanması izin kipi yükseltmez veya araç/şart onayı vermez.
     - Soru beklerken çalışma alanı veya dosya kilidi tutulmaz (`kilitsiz: true`). Süre dolması yanıt veya onay sayılmaz.
     - Yanıt yalnız `(tur, soru_kimligi)` eşleştiğinde kabul edilir (`Komut::KullaniciSorusuYaniti`); yinelenen veya eski tura ait yanıtlar reddedilir.
     - Kullanıcı soru beklerken normal mesaj gönderirse (`Komut::KullaniciMesaji`) bekleyen soru `SoruIptalNedeni::YeniKullaniciMesaji` olarak kapatılır ve yeni kullanıcı yönlendirmesi yeni tur olarak işlenir.
     - Tur iptali veya oturum kapanışında bekleyen soru `SoruIptalNedeni::TurIptalEdildi` / `OturumKapandi` olarak sonlanır. Kurtarmada yanıtlanmamış soru (`KayitOzet::kesintide_kalan_soru`) eski aracı yeniden çalıştırmaz; arayüzde `Kurtarildi` durumuyla gösterilir.

3. **Görev 5 & 6 — IPC, Arayüz ve Kodlama Çalışma Talimatı:**
   - `crates/kalfa-core/src/oturum/mod.rs`: `varsayilan_sistem_mesaji()` içine proje keşfi → hedefli araç seçimi → küçük değişiklik → doğrulama → kanıtlı son yanıt iş akışı ve `plan_guncelle` / `kullaniciya_sor` kullanım kuralları eklendi.
   - `crates/kalfa-uygulama` ve `crates/kalfa`: `kullanici_sorusu_yaniti` Tauri komutu, `ANA_PENCERE_KOMUTLARI` ve `capabilities/ana-pencere.json` yetkisi (`allow-kullanici-sorusu-yaniti`) eklendi.
   - `ui/`: `PlanListesi.svelte` ve `SoruKarti.svelte` bileşenleri (sabit `style` veya ham HTML olmadan), `tipler.ts`, `ana.ts`, `oturum.ts`, `anaDurum.svelte.ts`, `Konusma.svelte`, `KonusmaOgesi.svelte`, `OrtamBolumu.svelte` ve `tr.ts` metinleri eklendi.

## 2. İki Platform Doğrulama Kanıtları (Windows + WSL Ubuntu 24.04)

### Windows (`C:/Users/kor_a/kalfa-yetenek-02`)
- `cargo fmt --all -- --check`: **0 hata**
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo test -p kalfa-core --test proje_akisi --locked`: **13 geçti, 0 başarısız** (`k06_kok_ve_alt_dizin_agents_kapsam_onceligi_ve_dis_include_reddi`, `agents_surum_degisimi_yeni_turda_algilanir_tur_icinde_sabit_kalir`, `agents_boyut_sinirlari_ve_bozuk_dosyalar_uyari_uretir`, `agents_compact_ve_kurtarmada_kaynak_ve_kapsam_korunur`, `proje_talimati_izin_kipini_veya_guvenlik_sinirini_gecersiz_kilamaz`, `plan_guncelle_kurallari_dogrular_ve_kurtarmada_korunur`, `plan_guncelle_arac_veya_kabuk_yetkisi_vermez`, `s05_kullaniciya_sor_secenek_ve_serbest_yanit_dogru_kimlikle_eslesir`, `kullaniciya_sor_yanlis_kimlik_veya_yinelenen_yaniti_reddeder`, `kullaniciya_sor_beklerken_normal_kullanici_mesaji_soruyu_iptal_eder_ve_yeni_tur_baslatir`, `kullaniciya_sor_beklerken_iptal_komutu_ve_dosya_kilitsizligi_dogrulanir`, `kurtarmada_bekleyen_soru_eski_araci_yeniden_baslatmaz_ve_yeni_soru_sorulabilir`, `varsayilan_sistem_mesaji_kodlama_calisma_talimatlarini_icerir`)
- `cargo test --workspace --locked`: **Tüm workspace testleri geçti (0 başarısız)**
- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: **`kalfa` izin testleri (9/9), `kalfa-uygulama` birim testleri (121/121) ve tümleştirme testleri (`oturum_tablosu` 19/19, `uygulamada_proje_talimatlari_plan_ve_kullanici_sorusu_calisir` dahil) geçti**
- `ui/`:
  - `npm run check`: **0 hata, 0 uyarı**
  - `npm test`: **32 test dosyası, 496 test geçti**
  - `npm run build`: **Üretim paketi ve `paket-denetimi.mjs` temiz geçti**

### Linux — WSL2 `Ubuntu-24.04` (`kalfatest` kullanıcısı)
- `cargo fmt --all -- --check`: **0 hata**
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo test --workspace --locked`: **Tüm workspace testleri geçti (`proje_akisi` 13/13 dahil, 0 başarısız)**
- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: **Tüm `e2e` özellikli testler geçti (0 başarısız)**
- `ui/` (`npm ci && npm run check && npm test && npm run build`): **0 hata/uyarı, 32 test dosyası / 496 test geçti, `Paket denetimi temiz.`**
- Geçici WSL çalışma ve derleme dizinleri (`/tmp/kalfa-yetenek-02-wsl`, `/tmp/kalfa-yetenek-02-target`) doğrulama sonunda silindi.
