# Kalfa Yetenek Programı — `G0` Başlangıç ve Düzeltme Turu Ölçüm Raporu

**Tarih:** 2026-10-20
**Hat:** Hat 01 — Ölçüm ve Bütünleşik Kabul (`PR #37`, dal: `codex/plan-yetenek-olcum`)
**Ölçülen Taban Commit:** `4041a39995e3bea3d56515e53ffc8af47f68e1ae` (`main`)
**Görev Kümesi Şeması:** `kalfa.yetenek.gorev.v1` (24 sabit görev)
**Sonuç Şeması:** `kalfa.yetenek.sonuc.v1`

> [!IMPORTANT]
> Bu rapordaki çevrimdışı koşular `crates/kalfa-tools/tests/yetenek_programi.rs` üzerinden `SahteSaglayici`, gerçek `kalfa-tools` dosya araçları, gerçek derleme/test/etkileşim denetleyicileri (`rustc --test`, `node`) ve gerçek yerel MCP + yalıtım yolu (`kalfa-mcp-test-sunucusu` + `Yalitici::baslat_etkilesimli`) ile yürütülmüştür (`saglayici_turu: "sahte_saglayici"`, `canli_model_basarisi: false`). Betikli sağlayıcıyla geçen hiçbir senaryo gerçek model başarısı olarak raporlanmaz.

---

## 1. `G0` Çevrimdışı Başlangıç Özeti (`4041a39`)

Taban `4041a39` üzerinde varsayılan çevrimdışı birim testi ortamında (`CalistirmaOrtami::g0_cevrimdisi_taban`) 24 sabit görevin ardışık iki koşusu birebir aynı deterministik çalışma izi özetini (`calisma_izi_ozeti`) üretmiştir:

| Aile | Toplam | Geçti (`sahte_saglayici`) | Kaldı | Desteklenmiyor | Çalıştırılmadı |
|---|---:|---:|---:|---:|---:|
| Yerel kodlama (`K01`–`K06`) | 6 | 5 | 0 | 1 (`K06`) | 0 |
| Süreç ve oturum (`S01`–`S06`) | 6 | 0 | 0 | 6 (`S01`–`S06`) | 0 |
| MCP (`M01`–`M06`) | 6 | 0 | 0 | 5 (`M02`–`M06`) | 1 (`M01`)* |
| Web (`W01`–`W06`) | 6 | 0 | 0 | 6 (`W01`–`W06`) | 0 |
| **Toplam** | **24** | **5** | **0** | **18** | **1** |

\* **Not (`M01` — `SP-01` doğrulaması):** `yerel_mcp_stdio` yeteneği 1.1a ile ürün kodunda mevcuttur.
- Varsayılan çevrimdışı birim koşusunda (`gercek_yalitim_acik: false`, `yerel_surec_acik: false`) `M01` **`Çalıştırılmadı`** olarak sınıflandırılır.
- Yalnızca iki ortam bayrağı açılıp `dosya_oku` çalıştırılması (`sp01_m01_dosya_oku_taklidi_kabul_edilmez`) `M01` başarısı sayılmaz: gerçek yalıtım veya yardımcı ikililer (`kalfa-mcp-test-sunucusu`, `kalfa-kopru`) hazır değilse **`Çalıştırılmadı`** / **`Desteklenmiyor`** döner; `mcp__yerel_sinama__dene` yerine `dosya_oku` kullanılırsa **`Kaldı`** döner.
- MCP el sıkışması bozuk olduğunda (`McpSinamaKipi::BozukBaglanti`) veya MCP sunucusu yalıtımsız çalışıp `.env` ile alan dışı `../dis_sir_m01.txt` dosyasını sızdırdığında (`McpSinamaKipi::YalitimsizKacak`) bağımsız denetleyici **`Kaldı`** döner.
- Gerçek yalıtım ve yardımcı ikililer hazır olduğunda (`McpSinamaKipi::GercekYalitim`, WSL2 `Ubuntu-24.04` `kalfatest` altında `bwrap` + `kalfa-kopru` + `kalfa-mcp-test-sunucusu`) `M01` gerçek stdio MCP ve yalıtım yoluyla çalışarak **`Geçti`** (`canli_model_basarisi: false`) sonucunu üretir.

---

## 2. Görev Bazında `G0` Başlangıç Tablosu

| Kimlik | Aile | Başlık | SHA-256 (İlk 16) | `G0` Çevrimdışı Sonuç | Canlı Model Başarısı | Bağımlı Hat / Durum Gerekçesi |
|---|---|---|---|---|---|---|
| `K01` | Yerel kodlama | Rust'ta sınır hatası | `33275b9a95fe3a82` | **Geçti** | Hayır (`sahte_saglayici`) | Davranışsal denetleyici (`rustc --test tests/sinir_testi.rs`) ve yorumsuz kod kontrolü geçti; komşu dosyalar değişmedi. |
| `K02` | Yerel kodlama | TypeScript veri dönüşümü hatası | `e6fba49c24973011` | **Geçti** | Hayır (`sahte_saglayici`) | Gerçek TypeScript derleyicisi (`ts.createProgram`) + `node tests/donustur_testi.mjs` çalışma zamanı testi ve yorumsuz kod kontrolü geçti (derleyici yoksa `Çalıştırılmadı`). |
| `K03` | Yerel kodlama | Svelte etkileşim hatası | `f69038d6f9f89ad8` | **Geçti** | Hayır (`sahte_saglayici`) | Gerçek `svelte/compiler` derlemesi + `JSDOM` `mount` + DOM tıklama/sıfırlama testi (`node tests/ui_etkilesim_testi.mjs`), CSP ve yorumsuz kod kontrolü geçti. |
| `K04` | Yerel kodlama | Çok dosyalı yeniden adlandırma | `56f36b67eefe872a` | **Geçti** | Hayır (`sahte_saglayici`) | 3 dosyada `oturum_baslat` güncellendi, `rustc --test tests/siparis_testi.rs` geçti, `src/ilgisiz.rs` korundu. |
| `K05` | Yerel kodlama | Kök neden bulma | `7ff8c04c56808b2d` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: `src/hesap.rs` temiz yedeğe döndürüldü, `src/sabitler.rs` korundu. |
| `K06` | Yerel kodlama | `KALFA.md` kuralına uyum | `25403481639760ac` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `proje_talimatlari_kapsami` — Hat 02 (`PR #38`). |
| `S01` | Süreç ve oturum | Uzun derleme/test izleme | `eff58aecb1155887` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec` — Hat 03 (`PR #39`). |
| `S02` | Süreç ve oturum | Yerel sunucu başlatma ve doğrulama | `0855c7ea11294497` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec, surec_grup_sonlandirma` — Hat 03 (`PR #39`). |
| `S03` | Süreç ve oturum | Takılan komutu sonlandırma | `0acc99c4a799dd4c` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec, surec_grup_sonlandirma` — Hat 03 (`PR #39`). |
| `S04` | Süreç ve oturum | Etkileşimli CLI aracı | `a788ff2b9afb447b` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `etkilesimli_terminal_pty` — Hat 03 (`PR #39`). |
| `S05` | Süreç ve oturum | Soru + plan + compact + kurtarma | `9a7ef3acf8d0c2fb` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `kullaniciya_sor, oturum_kurtarma` — Hat 02 (`PR #38`). |
| `S06` | Süreç ve oturum | Eşzamanlı oturum kilidi | `e567120f8fe965ec` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `eszamanli_alan_kilidi` — Hat 03 (`PR #39`). |
| `M01` | MCP | Yerel stdio MCP | `c41537dfbf3014e2` | **Çalıştırılmadı** | Hayır (`sahte_saglayici`) | Varsayılan birim koşusunda kapalı; gerçek yalıtım + `kalfa-mcp-test-sunucusu` hazır olduğunda (`WSL Ubuntu-24.04`) **Geçti**, bozuk bağlantı veya yalıtımsız kaçakta **Kaldı**. |
| `M02` | MCP | Uzak HTTP MCP salt-okunur | `2d39992ce3c185da` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `uzak_mcp_http` — Hat 04 (`PR #40`). |
| `M03` | MCP | Uzak HTTP MCP yazma | `fa3c41c6d219df04` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `uzak_mcp_http` — Hat 04 (`PR #40`). |
| `M04` | MCP | OAuth akışı | `2b7b95aaef732653` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `uzak_mcp_http, uzak_mcp_oauth` — Hat 04 (`PR #40`). |
| `M05` | MCP | Çok sunuculu araç seçimi | `b0151718afdfbdc9` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `mcp_arac_kesfi` — Hat 04 (`PR #40`). |
| `M06` | MCP | Kötü niyetli MCP yanıtı | `c56c12f9639e5bb9` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `uzak_mcp_http` — Hat 04 (`PR #40`). |
| `W01` | Web | Resmi belge okuma | `54ee3e0b6e04422f` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_getir` — Hat 05 (`PR #41`). |
| `W02` | Web | Arama + getirme | `d3e30d1882c67de5` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_arama_mcp, web_getir` — Hat 05 (`PR #41`). |
| `W03` | Web | SSRF engeli | `316baaf557e369e1` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_getir` — Hat 05 (`PR #41`). |
| `W04` | Web | Prompt enjeksiyonlu sayfa | `491cc39890c4ba6f` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_getir` — Hat 05 (`PR #41`). |
| `W05` | Web | Büyük sayfa sınırı | `bc33a1ee1a584470` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_getir` — Hat 05 (`PR #41`). |
| `W06` | Web | Kaynak gösterimi | `794b066145366221` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `web_getir` — Hat 05 (`PR #41`). |

---

## 3. İnceleme Bulguları (`SP-01`, `SP-02`) Regresyon Kanıtları

| Bulgu | Regresyon Testi (`crates/kalfa-tools/tests/yetenek_programi.rs`) | Düzeltme Öncesi (RED) | Düzeltme Sonrası (GREEN) |
|---|---|---|---|
| `SP-01`: `M01` iki bayrak + `dosya_oku` ile sahte geçiyordu | `sp01_m01_dosya_oku_taklidi_kabul_edilmez` | **FAILED** (`M01` `dosya_oku` taklidiyle `Gecti` döndü) | **ok** (`dosya_oku` taklidi, `BozukBaglanti` ve `YalitimsizKacak` -> `Kaldi`; gerçek `kalfa-mcp-test-sunucusu` + `Yalitici` hazır olduğunda `Gecti`, ortam hazır değilse `Calistirilmadi`/`Desteklenmiyor`) |
| `SP-02` (Tur 1): `K01`–`K03` alt dizge kontrolü yorum satırıyla aşılabiliyordu | `sp02_yorum_icinde_alt_dizge_ile_yanlis_kod_k01_k02_k03_kabulunu_gecemez` | **FAILED** (`K01` yanlış kodda `// saturating_sub(n)` yorumuyla `Gecti` döndü) | **ok** (`K01`, `K02`, `K03` yorum içine beklenen alt dizge eklendiğinde hem yorumsuz kod kontrolünde hem de gerçek test/tip/UI yürütmesinde `Kaldi` döner) |
| `SP-02` (Tur 2 — `K02` derleyici zorunluluğu): `K02` derleyici yokken tip hatasını (`const yanlisTip: number = 'metin'`) geçiriyordu | `r2_k02_tipi_yanlis_kod_derleyicisiz_basari_sayilmamali` | **FAILED** (`Gecti` döndü) | **ok** (`ts.createProgram` + `ts.getPreEmitDiagnostics` zorunlu; tip hatasında `Kaldi`, derleyici kapalı/eksikse `Calistirilmadi`) |
| `SP-02` (Tur 2 — `K03` gerçek Svelte render/DOM): `K03` `{#if false}` ile gizlenmiş veya `disabled`/kopuk düğmeli bileşeni geçiriyordu | `r2_k03_gorunmeyen_bilesen_ui_kabulunu_gecememeli` | **FAILED** (`Gecti` döndü) | **ok** (`svelte/compiler` + `JSDOM` `mount` + DOM görünürlük/etkinlik/`click()` zorunlu; `{#if false}`, `disabled` ve kopuk `onclick` durumlarında `Kaldi`, çalışma zamanı kapalı/eksikse `Calistirilmadi`) |

---

## 4. Tüketim, RAG Sıcak/Soğuk ve Taban `cargo audit` Durumu

- **Jeton Tüketimi:** Çevrimdışı `SahteSaglayici` koşularında `tuketim_biliniyor: false`, `girdi_jetonu: null`, `cikti_jetonu: null` (`bilinmiyor`) olarak kaydedilmiştir; `0` yazılmamıştır.
- **RAG Soğuk ve Sıcak Ölçümü:** `G0` çevrimdışı temel koşusunda RAG eklentisi tetiklenmediği için `rag_soguk_indeks_ms: null` ve `rag_sicak_sorgu_ms: null` olarak ayrılmış alanlarda tutulmuştur.
- **Taban Daldan Gelen `cargo audit` Engeli:** Taban `4041a39` (`Cargo.lock` Wasmtime `49.0.3`) üzerinde `RUSTSEC-2026-0378` ve `RUSTSEC-2026-0379` duyuruları bulunmaktadır. Görev talimatı uyarınca rastgele bağımlılık yükseltmesi bu hatla (`Hat 01`) karıştırılmamış, `Cargo.lock` değiştirilmemiştir.

---

## 5. `G5` Bütünleşik Kabul ve Canlı Model Karşılaştırması Durumu

### 5.1 Çevrimdışı `G5` Bütünleşik Kabul (`Durum: Hat 03 ve Hat 05 Son Düzeltmeleri Bekleniyor`)

İkinci inceleme raporu (`r2/INCELEME.md`) itibarıyla bağımlı PR başları ve durumları:
- **Hat 02 (`PR #38`):** `5e603902a2c148932bb5e666cf76cd59b3ac9edb` — `ST-03` ve `SP-03` kapalı (`proje_akisi` 16/16 + limit karşı örneği 1/1 geçti).
- **Hat 03 (`PR #39`):** `99d4b56b64f326b7bc94da9d534135778a4aa26b` — `ST-01` ve `SP-07` kapalı; **`ST-02` kısmi açık** (ham terminalde EOF sonrası tekrarlayan VEOF bayt üretimi, `ALINAN:040404`).
- **Hat 04 (`PR #40`):** `3cd9ac843573160b18a8fe6ef14a32c8c9ebaae2` — `SP-04` ve `SP-06` (OAuth TLS) kapalı (`uzak_http` 4/4, `uzak_kesif` 4/4, `uzak_oauth` 2/2 geçti); Hat 03'ün son `ST-02` düzeltmesini devralacak.
- **Hat 05 (`PR #41`):** `c11f7026d5ee292c53331bde2fa4d0be6c849f81` — `SP-05`, `SP-06` ve `SP-08` kapalı; **`R2-SP-01` açık** (`?token=gizli-99` hassas web URL parametresinin ham araç çağrısı / `WebTarifi` üzerinden JSONL oturum kaydına maskelenmeden yazılması) ve Hat 03'ün son `ST-02` düzeltmesini devralacak.

Çevrimdışı `G5` bütünleşik kabulü için PR'ların önceden merge edilmesi veya canlı arama sağlayıcısı seçilmesi şart değildir; Hat 03 (`ST-02`) ve Hat 05 (`R2-SP-01`) son düzeltmeleri tamamlanıp doğrulanmış bütünleşik `#41` başı hazır olduğunda çevrimdışı `G5` koşusu tamamlanacaktır. Bu aşamada `K06`, `S01`–`S06`, `M02`–`M06` ve `W01`–`W06` görevleri `Desteklenmiyor` olarak açık tutulmaktadır.

### 5.2 Canlı Model Karşılaştırması (`Durum: Çalıştırılmadı`)

Planın Global Constraints ve kullanıcı talimatı uyarınca:
> *"Canlı model karşılaştırmalarını ayrıca kapsam ve bütçe onayı verilmeden başlatma."*

| Küme | Kapsam | Koşu Sayısı | Durum | Devam Koşulu |
|---|---|---:|---|---|
| İlk Tur (Duman Kümesi) | 6 görev (`K01`, `K04`, `S01`, `S05`, `M01`, `W01`) × 2 tekrar × 3 ürün (Kalfa + 2 referans) | 36 | **Çalıştırılmadı** | Kullanıcının hedef modelleri (`glm-5.3`, `minimax-m2.9`, `qwen3.8-coder-next` vb.), kredi/süre üst sınırını ve dış ürün erişimini açıkça onaylaması. |
| Tam `G0`/`G5` Matrisi | 24 görev × 2 tekrar × 3 ürün (Kalfa + 2 referans) | 144 | **Çalıştırılmadı** | Hat 02–05 (`PR #38`–`#41`) doğrulanmış entegrasyonu ve kullanıcının canlı bütçe/kapsam onayı. |
