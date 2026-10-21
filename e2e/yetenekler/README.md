# G5 Bütünleşik Yetenek Kabul Senaryoları (`e2e/yetenekler/`)

Bu klasör, Kalfa yetenek programının **Hat 01 (Ölçüm ve Bütünleşik Kabul — G0 ve G5)** kapsamında tanımlanan uçtan uca kabul senaryolarının sözleşmesini tutar.

## Sınır ve Durum

- Mevcut `e2e/calistir.mjs`, `e2e/package.json` ve `e2e/testler/*.test.mjs` dosyaları paylaşılan alandır; Hat 01 tarafından değiştirilmez.
- Bu klasördeki G5 senaryoları, paralel ürün hatlarının (**Hat 02 `#38`**, **Hat 03 `#39`**, **Hat 04 `#40`**, **Hat 05 `#41`**) tamamlanıp bütünleştirilmesinin ardından gerçek Tauri kabuğunda (`tauri-driver` + WebdriverIO) koşulmak üzere hazırlanmıştır.
- Hat 02–05 tamamlanmadan önce bu senaryoların durumu ölçüm sözleşmesine göre **`Çalıştırılmadı`** (`bagimli_hat_bekleniyor`) olarak kaydedilir; sahte sağlayıcı veya eksik ürün kodu üzerinde "geçti" olarak raporlanmaz.

## G5 Kabul Matrisi

| G5 Kimliği | Kapsanan Sabit Görevler | Bağımlı Hat | Doğrulanan Arayüz ve Çekirdek Davranışı |
|---|---|---|---|
| `G5-E2E-01` | `K06`, `S05` | Hat 02 (`PR #38`) | `AGENTS.md` hiyerarşik kapsam rozeti, yapılandırılmış `plan_guncelle` görünümü ve `kullaniciya_sor` etkileşim kartı. |
| `G5-E2E-02` | `S01`, `S02`, `S03`, `S04`, `S06` | Hat 03 (`PR #39`) | Arka plan süreç kartı, canlı çıktı akışı, süreç grup sonlandırma, PTY terminal oturumu ve çalışma alanı yazma kilidi. |
| `G5-E2E-03` | `M01`, `M02`, `M03`, `M04`, `M05`, `M06` | Hat 04 (`PR #40`) | Uzak Streamable HTTP MCP kartı, mühürlü yazma onayı, OAuth PKCE durumu (belirteç sızdırmadan) ve dinamik araç keşfi. |
| `G5-E2E-04` | `W01`, `W02`, `W03`, `W04`, `W05`, `W06` | Hat 05 (`PR #41`) | `web_getir` ve MCP araç keşfi/arama kaynak atıf kartları, SSRF ret bildirimi ve sarmalanmış dış içerik uyarısı. |

## Çalıştırılabilir ilk kabul dilimi

`g5-proje-akisi.test.mjs`, gerçek Tauri penceresinde `plan_guncelle` adımlarını,
`kullaniciya_sor` kartından seçilen yanıtın doğru tura bağlanmasını ve her iki
sonucun JSONL'e yazılmasını doğrular. Yalnız model sağlayıcısı senaryoludur;
IPC, çekirdek araçları ve arayüz gerçek ürün yoludur.

Doğrulanmış bütünleşik kaynakta:

```sh
node calistir.mjs yetenekler/g5-proje-akisi.test.mjs
```

Bu iki test tek başına G5'in tamamı veya 20/24 görev başarısı değildir. K06 kapsamı,
süreç/terminal, uzak MCP/OAuth ve web ailelerinin bütün masaüstü kabulü ile 24 sabit
görevin son ürün uyarlaması ayrıca açık kalır. Çevrimdışı kabul GitHub PR merge'ünü
veya canlı sağlayıcı seçimini gerektirmez; aynı kaynakta gerekli ürün sözleşmelerinin
bulunmasını gerektirir. Canlı model kıyası ayrı bütçe/kapsam onayına tabidir.

21 Ekim doğrulaması: bu iki test ve mevcut konuşma takımındaki dört test,
aynı bütünleşik kaynakta Windows ve WSL Ubuntu 24.04 üzerinde 6/6 geçti.
[Ayrıntılı sonuç ve açık görevler](../../docs/superpowers/bulgular/2026-10-21-kalfa-g5-cevrimdisi-kabul.md).

## G5 v2 gerçek araç kabulü

`g5-v2.json`, G0'ın24 sabit JSON SHA'sını gerçek son ürün testlerine bağlar.
`g5-v2.mjs` her görev ve alt test için iki gerçek tekrar koşturur; her çağrıda
tam1 başarılı ve0 atlanmış test ister. Eksik/yanlış filtre, timeout ve eksik
önkoşul Geçti sayılamaz. S06 güvenli seri yolu doğrulansa da eşzamanlı özellik
Desteklenmiyor kalır. En az20/24 ve bütün zorunlu güvenlik kapıları gerekir.
M04 gerçek PKCE/refresh ve gerçek OAuth oturumunun JSONL denetimini birlikte içerir.
Bu kabul senaryolu model kullanır; canlı LLM/rakip kalite kıyası değildir.

Depo kökünden, mevcut çevrimdışı bağımlılıklar ve platform önkoşullarıyla:

```sh
node e2e/yetenekler/g5-v2.mjs --cikti /mutlak/makbuz/dizini
```

Windows M01 geliştirici makinesinde başlamaz. Mevcut Sürüm paketleri akışı
`g5_m01=true` ile geçici CI'da kurulum içinde, standart kullanıcıyla iki tekrar
çalıştırır; sonuç ve ham günlük aynı başarılı GitHub run/job'dan okunur:

```sh
node e2e/yetenekler/g5-v2.mjs --cikti C:/mutlak/makbuz --m01-ci-run KOSU_KIMLIGI
```

Bu seçenek GitHub CLI ile salt okunur metadata/günlük doğrulaması yapar; yerel
JSON başarı bayrağı kabul etmez. Kurucu dahil belge dışındaki tüm kaynak ağacı
CI başıyla eşleşmeli; yeni kaynakta eski kurucu kanıtı kullanılamaz. Makbuz
verilmediyse Windows M01 Çalıştırılmadı ve zorunlu güvenlik kapısı açık kalır.
K02/K03 mevcut ui/node_modules ister; başka konum için KALFA_UI_NODE_MODULES ve
KALFA_TS_LIB_PATH kullanılır. Hiçbir paket otomatik kurulmaz/indirilmez.

24 görev v2 koşumu, yukarıdaki gerçek Tauri masaüstü kabulünden ayrı kapıdır.
Sonuç JSON'unda platform, kaynak başı, manifest/koşucu/ham günlük SHA'ları ve
her tekrar görünür. Çalışma ağacındaki kaynak commit edilmeden kabul makbuzu
üretilmez. Ücretli model/arama denemeleri kullanıcı kararıyla bu teslimatta yoktur.