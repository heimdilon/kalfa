# Kalfa Yetenek Programı — `G0` Başlangıç Ölçüm Raporu

**Tarih:** 2026-10-20
**Hat:** Hat 01 — Ölçüm ve Bütünleşik Kabul (`PR #37`, dal: `codex/plan-yetenek-olcum`)
**Ölçülen Taban Commit:** `4041a39995e3bea3d56515e53ffc8af47f68e1ae` (`main`)
**Görev Kümesi Şeması:** `kalfa.yetenek.gorev.v1` (24 sabit görev)
**Sonuç Şeması:** `kalfa.yetenek.sonuc.v1`

> [!IMPORTANT]
> Bu rapordaki çevrimdışı koşular `crates/kalfa-tools/tests/yetenek_programi.rs` üzerinden `SahteSaglayici` ve gerçek `kalfa-tools` dosya araçlarıyla yürütülmüştür (`saglayici_turu: "sahte_saglayici"`, `canli_model_basarisi: false`). Betikli sağlayıcıyla geçen hiçbir senaryo gerçek model başarısı olarak raporlanmaz.

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

\* **Not (`M01`):** `yerel_mcp_stdio` yeteneği 1.1a ile ürün kodunda mevcuttur. Alt süreç başlatmayan varsayılan çevrimdışı birim koşusunda `M01` (`gercek_yalitim: true`, `yerel_surec: true`) **`Çalıştırılmadı`** olarak sınıflandırılır; önkoşul anahtarları açıldığında (`m01_ortam.gercek_yalitim_acik = true`, `m01_ortam.yerel_surec_acik = true`) gizli `.env` reddini ve `KLF-M01-STDIO-OK` okumasını doğrulayarak **`Geçti`** (`canli_model_basarisi: false`) sonucunu üretir.

---

## 2. Görev Bazında `G0` Başlangıç Tablosu

| Kimlik | Aile | Başlık | SHA-256 (İlk 16) | `G0` Çevrimdışı Sonuç | Canlı Model Başarısı | Bağımlı Hat / Durum Gerekçesi |
|---|---|---|---|---|---|---|
| `K01` | Yerel kodlama | Rust'ta sınır hatası | `83c0ec54fe8fc48a` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: `saturating_sub(n)` doğrulandı, komşu dosyalar değişmedi. |
| `K02` | Yerel kodlama | Çok dosyalı yeniden adlandırma | `161ce30ea2319f5d` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: 4 dosyada `oturum_baslat` güncellendi, `src/ilgisiz.rs` korundu. |
| `K03` | Yerel kodlama | Kök neden bulma | `7fbe04641213d066` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: `src/zaman_asimi.rs` düzeltildi, test dosyası değişmedi. |
| `K04` | Yerel kodlama | Büyük dosyada hedefli düzenleme | `888f4e7eeb50cba1` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: yalnız `bolum_0750_degeri` güncellendi. |
| `K05` | Yerel kodlama | Git diff inceleme ve geri alma | `7ff8c04c56808b2d` | **Geçti** | Hayır (`sahte_saglayici`) | Bağımsız denetleyici geçti: `src/hesap.rs` temiz yedeğe döndürüldü, `src/sabitler.rs` korundu. |
| `K06` | Yerel kodlama | `KALFA.md` kuralına uyum | `25403481639760ac` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `proje_talimatlari_kapsami` — Hat 02 (`PR #38`). |
| `S01` | Süreç ve oturum | Uzun test çalıştırma | `eff58aecb1155887` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec` — Hat 03 (`PR #39`). |
| `S02` | Süreç ve oturum | Yerel sunucu başlatma ve yoklama | `0855c7ea11294497` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec, surec_grup_sonlandirma` — Hat 03 (`PR #39`). |
| `S03` | Süreç ve oturum | Takılan komutu iptal etme | `0acc99c4a799dd4c` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `arka_plan_surec, surec_grup_sonlandirma` — Hat 03 (`PR #39`). |
| `S04` | Süreç ve oturum | Etkileşimli CLI aracı | `a788ff2b9afb447b` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `etkilesimli_terminal_pty` — Hat 03 (`PR #39`). |
| `S05` | Süreç ve oturum | Soru/plan/compact/kurtarma | `9a7ef3acf8d0c2fb` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `kullaniciya_sor, oturum_kurtarma` — Hat 02 (`PR #38`). |
| `S06` | Süreç ve oturum | Eşzamanlı iki oturum | `e567120f8fe965ec` | **Desteklenmiyor** | Hayır (`sahte_saglayici`) | Eksik yetenek: `eszamanli_alan_kilidi` — Hat 03 (`PR #39`). |
| `M01` | MCP | Yerel stdio MCP | `33c66a4d14fa9b10` | **Çalıştırılmadı** | Hayır (`sahte_saglayici`) | Önkoşul varsayılan birim koşusunda kapalı: `gercek_yalitim, yerel_surec` (önkoşul açıkken **Geçti**). |
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

## 3. Tüketim ve RAG Sıcak/Soğuk Kayıt Durumu (`G0`)

- **Jeton Tüketimi:** Çevrimdışı `SahteSaglayici` koşularında `tuketim_biliniyor: false`, `girdi_jetonu: null`, `cikti_jetonu: null` (`bilinmiyor`) olarak kaydedilmiştir; `0` yazılmamıştır.
- **RAG Soğuk ve Sıcak Ölçümü:** `G0` çevrimdışı temel koşusunda RAG eklentisi tetiklenmediği için `rag_soguk_indeks_ms: null` ve `rag_sicak_sorgu_ms: null` olarak ayrılmış alanlarda tutulmuştur.

---

## 4. Canlı Model Başlangıç Karşılaştırması (`Görev 4` — Durum: `Çalıştırılmadı`)

Planın Global Constraints ve kullanıcı talimatı uyarınca:
> *"Canlı model karşılaştırmalarını ayrıca kapsam ve bütçe onayı verilmeden başlatma."*

Bu nedenle canlı EVREN ve rakip araç karşılaştırma matrisi bu aşamada **`Çalıştırılmadı`** (`kullanici_kapsam_ve_butce_onayi_bekleniyor`) olarak kaydedilmiştir:

| Küme | Kapsam | Koşu Sayısı | Durum | Devam Koşulu |
|---|---|---:|---|---|
| İlk Tur (Duman Kümesi) | 6 görev (`K01`, `K04`, `S01`, `S05`, `M01`, `W01`) × 2 tekrar × 3 ürün (Kalfa + 2 referans) | 36 | **Çalıştırılmadı** | Kullanıcının hedef modelleri (`glm-5.3`, `minimax-m2.9`, `qwen3.8-coder-next` vb.), kredi/süre üst sınırını ve dış ürün erişimini açıkça onaylaması. |
| Tam `G0`/`G5` Matrisi | 24 görev × 2 tekrar × 3 ürün (Kalfa + 2 referans) | 144 | **Çalıştırılmadı** | Hat 02–05 (`PR #38`–`#41`) birleşimi ve kullanıcının canlı bütçe/kapsam onayı. |
