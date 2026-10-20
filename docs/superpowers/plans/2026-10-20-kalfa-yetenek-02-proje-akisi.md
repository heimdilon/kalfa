# proje talimatları, plan ve kullanıcı sorusu planı

**Tarih:** 2026-10-20. **Planlayan/inceleyen:** Codex. **Uygulayıcı:** Antigravity (`Antigravity-Hat-02`).
**Durum:** Uygulandı ve Codex inceleme düzeltme turu (`ST-03`, `SP-03`) tamamlandı; Windows ve WSL Ubuntu 24.04 doğrulamaları %100 yeşil.
**Plan tabanı:** main `4041a39995e3bea3d56515e53ffc8af47f68e1ae` (`#35` birleşimi sonrası `ad7fd8ee5bf2d265865d61557a1c1765f1bf24ba`). **Çalışma dalı:** `codex/plan-yetenek-proje-akisi`. **Ortak sözleşme commit'i:** `bca07f6e38a60c0b8edb1872f28ae44cc19f46e4`.
**İlk başlanacak iş:** Önce kök/alt AGENTS kapsamı ve kaynak güveni için davranış testlerini yazın; soru ve plan araçları aynı hattın ikinci adımıdır.
**Koordinasyon:** [iş sırası ve ortak dosyalar](https://github.com/heimdilon/kalfa/blob/codex/plan-yetenek-koordinasyon/docs/superpowers/specs/2026-10-20-kalfa-yetenek-programi.md).

## Global Constraints

- Kullanıcının 20 Ekim talimatı: Codex plan ve PR hazırlar; Antigravity uygular; kullanıcı haber verince Codex inceler. Bu belge ürün kodu veya derlenmiş örnek içermez; ön doğrulama yapılmış gibi sunulamaz.
- İlk teslimat yerel kodlama + web + MCP, Windows ve Linux'tur. EVREN, Türkçe ad/metin, bağlı onay, yalıtım, sır koruması, bütçe ve JSONL kurtarma korunur.
- Başlamadan güncel AGENTS.md, GELISTIRME_DURUMU, Jira sahipliği ve PR başını okuyun. Her hat ayrı worktree'dedir; aynı klasöre iki uygulayıcı girmez.
- İlk görevde planın varsayımlarını ve gerçek SDK/platform arayüzlerini doğrulayın. Yeni paket/indirme/sistem ayarı AGENTS iznine tabidir; planda adı geçmesi kurulum onayı değildir. Yeni unsafe gerekirse dosya listesi ve gerekçe uygulamadan önce tasarımda kesinleşir.
- Hazırlık → izin/onay → taze mühür/kilit → yürütme yolu korunur. Yeni işlem türü mevcut Otomatik kip izin listesine kendiliğinden eklenmez. Yalıtımlı iş sessizce tam erişime düşmez.
- Sonucu bilinmeyen yan etkili iş tekrar yürütülmez. İş başlatıldı kaydı, iş başarıyla bitti kaydı yerine geçmez.
- Üretim kontrolleri AGENTS.md'deki kapsamla iki platformda koşar. CI gerçek yalıtım ve WebDriver yerine geçmez. Canlı EVREN/arama servisi denemesi ve merge mevcut ayrı yetki kurallarına tabidir.
- Her görev: davranışı ölçen düşen test/deney → küçük uygulama → geçen test → ilgili diff incelemesi. Plandan sapma gerekiyorsa gerekçeyi PR'a yazın; güvenlik kuralını sessizce gevşetmeyin.
- Ortak dosyaları değiştirmeden kendi PR'ınıza dar bir sözleşme notu yazın: yeni alan/olay, kaynak sahibi, kayıt/kurtarma ve hata davranışı. Koordinasyon planındaki sıra uygulanır; diğer hattın bekleyen tiplerini uydurmayın.

## Amaç ve sıra

G1a proje bağlamı → G1b plan/soru. Talimat okuma araç izni oluşturmaz. Kullanıcının verdiği işi anlamak, küçük değişiklik, uygun doğrulama ve kanıtlı son yanıt sistem talimatında açık olur. Tam beceri sistemi, dış include/hook ve otomatik script bu işte yoktur.

## Dosya sahipliği

| Yol | Sorumluluk |
|---|---|
| `crates/kalfa-core/src/proje_talimatlari.rs` (yeni) | Kaynak/kapsam/öncelik ve sınırlı talimat verisi |
| `crates/kalfa-core/src/plan.rs`, `soru.rs` (yeni) | Plan/soru durum kuralları |
| `crates/kalfa-core/tests/proje_akisi.rs` (yeni) | Oturum kabul testleri |
| `crates/kalfa-uygulama/src/proje_talimatlari.rs` (yeni) | Çalışma alanı tutamacıyla güvenli dosya erişimi |
| `ui/src/bilesenler/PlanListesi.svelte`, `SoruKarti.svelte` (yeni) | Görünüm; mevcut gerçek bileşen konumunu ilk görevde doğrulayın |

Ortak dosyalar: core `lib.rs/arac.rs/protokol.rs/kayit.rs/kurtarma.rs/oturum/*`; uygulama `oturumlar.rs/sozlesme.rs`; UI IPC tipleri ve konuşma görünümü. Bu hat ortak core/IPC eklemelerinde ilk bütünleştirme sırasındadır; sonrasındaki hatlar bunun tamamlanan commit'ini alır.

## Ortak Dosya Veri Sözleşmesi Notu (Hat 02 → Hat 03/04/05)

- **Proje Talimatları (`kalfa_core::proje_talimatlari`):**
  - Kaynak sahibi: `Oturum` (`OturumAyarlari::proje_talimat_okuyucu: Option<Arc<dyn ProjeTalimatOkuyucu>>`). Uygulama katmanında `CalismaAlaniTalimatOkuyucu`, `kalfa_tools::CalismaAlani` güvenli sınırlarını (`KORUMALI_DIZINLER`, `GIZLI_DOSYA_DESENLERI`, symlink/hardlink/alan dışı reddi) kullanır.
  - Kayıt/Olay: `Olay::ProjeTalimatlariYuklendi { goruntu: ProjeTalimatGoruntusu }` ve `KayitOlayi::ProjeTalimatlariYuklendi { goruntu }`. Yalnızca kaynak meta verisi (`goreli_yol`, `kapsam_oneki`, `oncelik`, `bayt_uzunlugu`, `icerik_ozeti`), toplam bayt, değişen kaynaklar ve uyarılar olaya/kayda/UI'ye (`ProjeTalimatBilgisi`) girer; ham dosya metni yalnızca model sistem mesajına ve mevcut `GonderimOnizleme` akışına girer.
  - Sınırlar: Dosya başına `16 KiB`, bağlam başına `64 KiB` ve `16` kaynak. Sınır aşımı, bozuk UTF-8 veya symlink/gizli dosya reddi sessizce tam sayılmaz; `ProjeTalimatUyarisi` üretir.
  - Kapsam ve Öncelik: Tur başında kök `AGENTS.md` (öncelik `0`), araç çağrılarında hedef göreli yolun ata dizin zincirindeki `AGENTS.md` dosyaları (derinlik = öncelik, daha dar kapsam çatışan proje kuralında önceliklidir) yüklenir. Sürüm özeti (`SHA-256`) değişirse yeni turda `ProjeTalimatDegisimi` olarak algılanır. Compact ve JSONL kurtarma (`KayitOzet::proje_talimat_goruntusu`) son anlık görüntüyü korur.
- **Plan (`kalfa_core::plan`):**
  - Kaynak sahibi: `Oturum` (`PlanDurumu`). `plan_guncelle` aracı (`kilitsiz: true`, `Risk::Dusuk`) `OrtamIstegi::PlanGuncelle` ile oturuma başvurur.
  - Kayıt/Olay: `Olay::PlanGuncellendi { plan: PlanDurumu }` ve `KayitOlayi::PlanGuncellendi { plan }`. UI'de `PlanDurumuBilgisi` (`PlanListesi.svelte`) olarak gösterilir.
  - Doğrulama: `1..=32` benzersiz kimlikli adım, kimlik `≤64` karakter, başlık `≤200` karakter, en çok bir `PlanAdimDurumu::Suruyor` adımı. Disk veya kabuk izni vermez.
- **Kullanıcı Sorusu (`kalfa_core::soru`):**
  - Kaynak sahibi: `Oturum` (`EtkinSoru`). `kullaniciya_sor` aracı (`kilitsiz: true`, `Risk::Dusuk`) `OrtamIstegi::KullaniciSorusu` ile oturuma başvurur; beklerken çalışma alanı kilidi tutmaz.
  - Komut/Olay/Kayıt: `Komut::KullaniciSorusuYaniti { tur, soru_kimligi, secenek_kimligi, serbest_metin }` (Tauri komutu: `kullanici_sorusu_yaniti`), `Olay::KullaniciSorusu { tur, soru }`, `Olay::KullaniciSorusuYanitlandi { tur, yanit }`, `Olay::KullaniciSorusuIptal { tur, soru_kimligi, neden }` ve karşılık gelen `KayitOlayi` varyantları.
  - Doğrulama ve Yarışlar: Başlık `≤500` karakter, `2..=4` seçenek (`≤200` karakter), serbest yanıt `≤4000` karakter. Yanıt yalnız `(tur, soru_kimligi)` eşleştiğinde kabul edilir; süre dolması yanıt/onay sayılmaz. Yeni normal `Komut::KullaniciMesaji` bekleyen soruyu `SoruIptalNedeni::YeniKullaniciMesaji` ile iptal edip yeni tura geçer. Kurtarmada (`KayitOzet::kesintide_kalan_soru`) yarım kalan soru aracı yeniden çalıştırılmaz; `Durum::Kurtarildi` bilgisiyle kullanıcıya bildirilir.

## Kesin davranış sözleşmesi

- Talimat kaynağı yalnız kullanıcı tarafından açılmış çalışma alanı içindeki `AGENTS.md` dosyalarıdır. Kök genel; alt dizin yalnız kendi altındaki hedefleri yönetir. Daha dar kapsam çatışan proje kuralında önceliklidir; kaynak yolu korunur.
- Proje talimatı sistem/insan talimatını, izin kipini veya güvenlik sınırını geçersiz kılamaz. RAG/arama sonucu içinde bulunan bir metin otomatik proje talimatına yükseltilmez.
- Symlink/junction/alan dışı include izlenmez; gizli/korumalı dosya okumaya dolaylı yol açılmaz. Başlangıçta kök kaynak, hedef dosya seçildiğinde onun ataları yüklenir. Tarama bütün depoyu körlemesine dolaşmaz.
- Önerilen sabit sınırlar: dosya başına 16 KiB, bağlam başına toplam 64 KiB ve 16 kaynak. Sınır aşımı açık hata/eksik bağlam bilgisidir; sessiz kırpılmış talimat tam sayılmaz. İlk doğrulama bu sayıların mevcut bağlam/önizleme bütçesine uyduğunu kanıtlar.
- Değişen kaynağın özeti yeni turda algılanır; tur içinde yüklendiği sürüm etiketlenir. Compact kaynak ve kapsam bilgisini kaybetmez.
- `plan_guncelle`: oturum planını bütünüyle değiştirir; en çok 32 benzersiz kimlikli adım, adım başına 200 karakter, en çok bir `Suruyor` adımı. Durumlar `Bekliyor/Suruyor/Tamam`. Plan değişikliği disk/kabuk yetkisi değildir.
- `kullaniciya_sor`: tek etkin soru; başlık ≤500 karakter, 2–4 seçenek, seçenek ≤200 karakter; serbest yanıt ≤4000 karakter. Zaman geçmesi yanıt/onay sayılmaz.
- Yanıt yalnız etkin oturum/tur/soru kimliği eşleşince alınır; yinelenen/eski yanıt reddedilir. Yeni normal kullanıcı mesajı bekleyen soruyu açıkça iptal edip yeni yönlendirme olarak işlenir.
- Plan/soru onay kartı değildir. Soru cevabı araç/şart kabulü veya kip yükseltmesi oluşturmaz. Soru beklerken dosya kilidi tutulmaz.
- Kurtarmada plan korunur; bekleyen soru eski aracı yeniden başlatmaz. UI kullanıcıya soru bekliyor durumunu gösterir; yeni soru kimliğiyle güvenli yeniden sorma yolu tasarımda sabitlenir.

## Görevler

### 1. Mevcut bağlam yolunu doğrula

- [x] `varsayilan_sistem_mesaji`, bağlam/compact, gönderim önizlemesi ve kayıt yolunu inceleyin. Okuyucunun dosya içeriğini UI'ye veya gizli günlüğe aktarmadığını belgeleyin.
- [x] Üç fixture: kök/alt kural çatışması, dış include telkini, dosya sürümü değişimi. Eksik davranışta test düşsün.
- [x] Tamam: kaynak yükleme zamanını ve kural önceliğini açıklayan dar veri sözleşmesi PR'a yazılmıştır.

### 2. Sınırlı talimat okuma ve bağlam

- [x] Core dosya sistemi açmaz; uygulamanın yetenek tutamacından kaynak/özet/kapsam alır. Yeni genel dosya sistemi katmanı kurulmaz.
- [x] Kullanıcıya kaynaklar gösterilir; gönderilen metin mevcut gönderim önizlemesine/bütçeye girer.
- [x] Dosya/byte/kaynak sınırı, Unicode, okunamayan dosya ve symlink karşı örnekleri geçsin.
- [x] Tamam: K06 fixture ve compact/kaynak sürümü testi geçer.

### 3. Plan aracı ve kayıt

- [x] Plan durumunu ayrı modülde doğrulayın; araç girdisi ve olay/kayıt biçimini tek commit'te ekleyin.
- [x] İki etkin adım, yinelenen kimlik, aşırı büyük liste ve yanlış oturum testleri önce düşsün.
- [x] Tamam: plan UI'de ve JSONL kurtarmada aynı; araç izni verilmiş gibi davranış yoktur.

### 4. Soru isteği ve bekleme

- [x] Mevcut `AracOrtami/OrtamIstegi` yolunu doğrulayın; soru yaşamını oturum sahiplensin. Tauri bağımlılığı core'a girmez.
- [x] İptal/normal mesaj/eski yanıt/uygulama kapanışı yarışlarını sınayın. Bekleme ağı veya çalışma alanı kilidini tutmasın.
- [x] Tamam: S05 fixture doğru soru/yanıt kimliği ve kayıt sırasını kanıtlar.

### 5. IPC ve UI

- [x] Backend DTO + TypeScript tipleri + soru/plan bileşenlerini birlikte ekleyin. Ham HTML/sabit style, UI'den dosya yolu veya kendine onay yoktur.
- [x] Klavye/odak, serbest yanıt, bağlantı kopması, eski kart ve yeniden açılış için UI/e2e sınaması.
- [x] Tamam: kullanıcı soru yanıtlar; oturum devam eder; onay kartı sınırı değişmez.

### 6. Kodlama çalışma talimatı

- [x] Proje keşfi → hedefli araç → küçük değişiklik → uygun test → kanıtlı sonuç yönergelerini ekleyin.
- [x] Araç adı çoğaltmak veya yeni model router eklemek yerine mevcut RAG/dosya/kabuk kullanımı K01–K06 ile ölçülsün.
- [x] Tamam: fixture başarısı ve canlı ölçüm ayrıdır; model kalitesi iddiası uydurulmaz.

## Doğrulama ve kaynaklar

Yeni oturum testi: `cargo test -p kalfa-core --test proje_akisi --locked`; ardından core/uygulama/UI kapsamındaki AGENTS kontrolleri. Gerçek GUI akışı için e2e özellikli Rust tek başına yeterli değildir.

[Sistem talimatı](../../../crates/kalfa-core/src/oturum/mod.rs), [compact](../../../crates/kalfa-core/src/oturum/compact.rs), [araç ortamı](../../../crates/kalfa-core/src/arac.rs), [kurtarma](../../../crates/kalfa-core/src/kurtarma.rs).

## Bitirme ve devir

- [x] Windows ve WSL Ubuntu 24.04 için uygulanan AGENTS kontrollerini komut, HEAD ve sonuçla kaydedin. Koşmayan kontrolü ve nedenini ayrıca yazın.
- [x] Yeni güvenlik sınırları için olumlu/olumsuz test, iptal ve kurtarma kanıtı ekleyin; gerçek yalıtım kabulü gereken işte yalnız sahtelerle tamam demeyin.
- [x] Diff yalnız bu hattın kodu, testleri ve belgelerinden oluşsun. Yeni davranış aynı commit'te Rust/TypeScript sözleşmelerine ve Türkçe metinlere işlensin.
- [x] Kendi plan kutularınızı ve bu hat için ayrı bulgu raporunu güncelleyin. GELISTIRME_DURUMU/README'yi bütünleştirme sırası geldiğinde güncel diff'i okuyarak değiştirin.
- [x] PR'ı kullanıcı haber verdiğinde Codex'in inceleyeceği şekilde bırakın; otomatik merge veya başka hatta kendiliğinden geçiş yoktur.
- [x] Devirde baş/HEAD, yapılan görevler, test/loglar, açık kararlar, tek sıradaki adım ve varsa canlı deney bütçesi bulunsun.
