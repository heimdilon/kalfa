# Yetenek 02 (Proje Akışı: G1a + G1b + Arayüz) — Uygulama ve Doğrulama Raporu

**Tarih:** 2026-10-20
**Dal:** `codex/plan-yetenek-proje-akisi` ([PR #38](https://github.com/heimdilon/kalfa/pull/38))
**Uygulayıcı:** `Antigravity-Hat-02`
**Ortak Çekirdek/IPC Sözleşme Commit'i:** `bca07f6e38a60c0b8edb1872f28ae44cc19f46e4` (`bca07f6` — ilk sözleşme `d627f6d51eb705e0ec39ecf3e9c021004c07914d` üzerine `ST-03` ve `SP-03` düzeltmeleriyle doğrulanmış kararlı sözleşme)

## 1. Kapsam ve Ortak Sözleşme Özeti

Bu hat ([PR #36](https://github.com/heimdilon/kalfa/pull/36) koordinasyon sırasındaki ilk çekirdek/IPC entegrasyon hattı) üç bileşeni tek sözleşmede birleştirir:

1. **G1a — Sınırlı Proje Talimatları (`AGENTS.md`):**
   - `crates/kalfa-core/src/proje_talimatlari.rs`: `ProjeTalimatKapsami` (`Kok`, `AltDizin { dizin }`), `ProjeTalimatKaynagi`, `ProjeTalimatUyariTuru`, `ProjeTalimatUyarisi`, `ProjeTalimatDegisimi`, `ProjeTalimatGoruntusu`, `ProjeTalimatOkuyucu` yetenek arayüzü.
   - Sabit sınırlar: dosya başına **16 KiB** (`DOSYA_BOYUT_SINIRI`), tur bağlamı başına **64 KiB** (`TOPLAM_BOYUT_SINIRI`) ve **16 kaynak** (`KAYNAK_SAYISI_SINIRI`). Korunmuş tur içi kaynaklar (`tur_ici_koru == true`) dahil her yükleme yolu aynı `64 KiB` ve `16` kaynak bütçesine tabidir (`ST-03`). Sınır aşımı veya güvenlik reddi (`DosyaBoyutuAsildi`, `ToplamBoyutAsildi`, `KaynakSayisiAsildi`, `GecersizUtf8`, `SembolikBaglantiReddedildi`, `GizliVeyaKorumaliYolReddedildi`, `Okunamadi`) sessizce kırpılmaz; `eksik_baglam = true` ve `uyarilar` listesiyle hem modele hem arayüze bildirilir.
   - Kapsam, öncelik ve ilk yazma kapısı (`SP-03`): Kök `AGENTS.md` geneldir (öncelik `0`); alt dizin `AGENTS.md` dosyaları yalnız kendi alt dizinlerindeki hedefler için geçerlidir ve o alt dizindeki hedeflerde daha derin kapsam önceliklidir (`oncelik` = derinlik). Bir araç çağrısı planlanırken hedef kapsam için modelin henüz görmediği yeni bir alt dizin `AGENTS.md` kaynağı veya uyarısı keşfedilirse, yazma çağrısı (`dosya_yaz`, `dosya_duzenle` vb.) model bu talimatı sistem mesajında değerlendirmeden onaya veya yürütmeye geçmez; çağrı `AracDurumu::IptalEdildi` (`hata_sayilmaz: true`) ile kapatılır ve model yeni tur adımında taze hazırlık, taze `OnayMuhru` ve izin/onay akışıyla yeniden karar verir.
   - Tur anlık görüntüsü ve sürüm değişimi: Tur içinde yüklenen kaynağın sürümü (`icerik_ozeti` ve `yuklendigi_tur`) sabit tutulur; sonraki turda değişen veya silinen kaynaklar `degisen_kaynaklar` altında raporlanır.
   - Yetki ve güvenlik sınırı: Proje talimatları `Kip` değerini yükseltemez, onay veya yalıtım sınırlarını gevşetemez, dış `@include`/URL/üst dizin referanslarını kendiliğinden izlemez. Ham dosya metni olay/kayıt/UI'ye sızmaz; yalnız sistem mesajına ve mevcut `GonderimOnizleme` akışına girer.
   - `crates/kalfa-uygulama/src/proje_talimatlari.rs` (`CalismaAlaniTalimatOkuyucu`): Çekirdek doğrudan dosya sistemi açmaz; `kalfa_tools::CalismaAlani` (`cap-std`) üzerinden sembolik bağlantıları, çoklu sabit bağlantıları (`nlink > 1`), gizli/korumalı yolları ve 16 KiB üstü dosyaları belleğe almadan reddeder.
   - Bağlam sıkıştırma (`compact`) ve kurtarma (`durum_ozeti`): `ProjeTalimatGoruntusu` üstverisi (`kaynaklar`, `uyarilar`, `degisen_kaynaklar`, `toplam_bayt`, `eksik_baglam`) JSONL kaydında (`KayitOlayi::ProjeTalimatlariYuklendi`), `KayitOzet::proje_talimat_goruntusu`'nda ve compact sonrasında korunur.

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

## 2. Codex İnceleme Bulguları ve Kanıtları (`ST-03`, `SP-03`)

| Bulgu | Önce Düşen Test Kanıtı | Uygulanan Düzeltme ve Sonra Geçen Test Kanıtı |
|---|---|---|
| **ST-03 (`[P2] #38`)**: Korunan tur kaynakları `64 KiB` toplam bayt ve `16` kaynak sınırını aşabiliyor | `proje_talimatlari::tests::tur_ici_yeni_hedef_toplam_talimat_butcesini_ve_kaynak_sinirini_asamaz` düzeltme öncesinde `tur içi bağlam sınırı aşıldı: 81920 > 65536; eksik_baglam=false` ile düştü (`z0..z3` 4×16 KiB üzerine alfabetik erken `a/AGENTS.md` eklendiğinde). | `YuklemeBirikimi::kaynak_eklemeyi_dene` ile korunan tur kaynakları önce `16 KiB` / `16` kaynak / `64 KiB` toplam sınırlarından geçirildi; yeni adaylar kalan bütçeye göre eklendi veya `ToplamBoyutAsildi` / `KaynakSayisiAsildi` + `eksik_baglam = true` üretti. Birim testi ve `proje_akisi::tur_icinde_korunan_kaynaklar_64_kib_ve_16_kaynak_sinirini_asamaz` geçti. |
| **SP-03 (`[P2] #38`)**: Alt dizin `AGENTS.md` ilk yazma kararına katılmadan mevcut hazırlanmış yazma yürüyor | `ilk_cagrisi_dogrudan_yazma_olan_alt_dizin_talimati_model_kararina_katilmadan_yurumez` (`left: 1, right: 2` model isteği) ve `ayni_yanitta_farkli_alt_dizinlere_coklu_yazma_kurallari_onceden_alir_ve_eski_hazirlikla_calismaz` (`left: 2, right: 1` yazma sayısı) düzeltme öncesinde düştü. | `arac_asamasi` içinde çağrının hedef kapsamını etkileyen yeni keşfedilmiş bir `AGENTS.md` kaynağı/uyarısı varsa yazma çağrıları `hakem_kapisi`/`onay_topla`/`yurut` öncesinde `AracDurumu::IptalEdildi` (`hata_sayilmaz: true`) ile durduruldu; model yeni sistem mesajındaki kurallarla yeniden karar verip taze hazırlık/mühür/onay ile yazmayı gerçekleştirdi. Her iki regresyon testi geçti. |

## 3. İki Platform Doğrulama Kanıtları (Windows + WSL Ubuntu 24.04)

### Windows (`C:/Users/kor_a/kalfa-yetenek-02`)
- `cargo fmt --all -- --check`: **0 hata**
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo test -p kalfa-core --test proje_akisi --locked`: **16 geçti, 0 başarısız** (`kok_ve_alt_dizin_agents_kapsami_ve_onceligi`, `agents_icindeki_dis_include_ve_yetki_yukseltme_etkisizdir`, `tur_arasinda_degisen_agents_yeni_turda_bildirilir`, `agents_boyut_sinirlari_ve_bozuk_dosyalar_uyari_uretir`, `tur_icinde_korunan_kaynaklar_64_kib_ve_16_kaynak_sinirini_asamaz`, `ilk_cagrisi_dogrudan_yazma_olan_alt_dizin_talimati_model_kararina_katilmadan_yurumez`, `ayni_yanitta_farkli_alt_dizinlere_coklu_yazma_kurallari_onceden_alir_ve_eski_hazirlikla_calismaz`, `compact_ve_kurtarma_sonrasi_proje_talimatlari_korunur`, `plan_guncelle_gecerli_plani_kaydeder_ve_olay_yayar`, `plan_guncelle_iki_suruyor_adimi_reddeder`, `plan_kurtarma_sonrasi_korunur`, `kullaniciya_sor_secenek_ve_serbest_yanit_akisi_s05`, `kullaniciya_sor_gecersiz_ve_tekrar_yaniti_reddeder`, `kullaniciya_sor_normal_kullanici_mesaji_ile_iptal_olur_ve_yeni_mesaj_calisir`, `kullaniciya_sor_tur_iptali_ve_kurtarma_davranisi`, `kullaniciya_sor_beklerken_dosya_kilidi_tutmaz`)
- `cargo test --workspace --locked`: **Tüm workspace testleri geçti (`kalfa-core` birim testleri 76/76, `otomatik_kip` 18/18, `proje_akisi` 16/16 dahil, 0 başarısız)**
- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: **`kalfa` izin testleri (9/9), `kalfa-uygulama` birim testleri (121/121) ve tümleştirme testleri (`oturum_tablosu` 19/19, `uygulamada_proje_talimatlari_plan_ve_kullanici_sorusu_calisir` dahil) geçti**
- `ui/`:
  - `npm run check`: **0 hata, 0 uyarı**
  - `npm test`: **32 test dosyası, 496 test geçti**
  - `npm run build`: **Üretim paketi ve `paket-denetimi.mjs` temiz geçti**

### Linux — WSL2 `Ubuntu-24.04` (`kalfatest` kullanıcısı)
- `cargo fmt --all -- --check`: **0 hata**
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo clippy -p kalfa-sandbox -p kalfa-tools --features "kalfa-sandbox/yalitim-testleri kalfa-tools/yalitim-testleri" --all-targets --locked -- -D warnings`: **0 hata, 0 uyarı**
- `cargo test -p kalfa-core --test proje_akisi --locked`: **16 geçti, 0 başarısız**
- `cargo test --workspace --locked`: **Tüm workspace testleri geçti (`proje_akisi` 16/16 dahil, 0 başarısız)**
- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`: **Tüm `e2e` özellikli testler geçti (0 başarısız)**
- `cargo test -p kalfa-sandbox --features yalitim-testleri --test yalitim`: **32 geçti, 0 başarısız (`bwrap` + `seccomp` gerçek Linux yalıtımı)**
- `cargo build -p kalfa-sandbox --bin kalfa-kopru --locked && cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk`: **16 geçti, 0 başarısız**
- `ui/` (`npm ci && npm run check && npm test && npm run build`): **0 hata/uyarı, 32 test dosyası / 496 test geçti, `Paket denetimi temiz.`**
- Geçici WSL çalışma ve derleme dizinleri (`/tmp/kalfa-yetenek-02-wsl`, `/tmp/kalfa-yetenek-02-target`) doğrulama sonunda silindi.
