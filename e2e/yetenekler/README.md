# G5 Bütünleşik Yetenek Kabul Senaryoları (`e2e/yetenekler/`)

Bu klasör, Kalfa yetenek programının **Hat 01 (Ölçüm ve Bütünleşik Kabul — G0 ve G5)** kapsamında tanımlanan uçtan uca kabul senaryolarının sözleşmesini tutar.

## Sınır ve Durum

- Mevcut `e2e/calistir.mjs`, `e2e/package.json` ve `e2e/testler/*.test.mjs` dosyaları paylaşılan alandır; Hat 01 tarafından değiştirilmez.
- Bu klasördeki G5 senaryoları, paralel ürün hatlarının (**Hat 02 `#38`**, **Hat 03 `#39`**, **Hat 04 `#40`**, **Hat 05 `#41`**) tamamlanıp bütünleştirilmesinin ardından gerçek Tauri kabuğunda (`tauri-driver` + WebdriverIO) koşulmak üzere hazırlanmıştır.
- Hat 02–05 tamamlanmadan önce bu senaryoların durumu ölçüm sözleşmesine göre **`Çalıştırılmadı`** (`bagimli_hat_bekleniyor`) olarak kaydedilir; sahte sağlayıcı veya eksik ürün kodu üzerinde "geçti" olarak raporlanmaz.

## G5 Kabul Matrisi

| G5 Kimliği | Kapsanan Sabit Görevler | Bağımlı Hat | Doğrulanan Arayüz ve Çekirdek Davranışı |
|---|---|---|---|
| `G5-E2E-01` | `K06`, `S05` | Hat 02 (`PR #38`) | `KALFA.md` hiyerarşik kapsam rozeti, yapılandırılmış `plan_guncelle` görünümü ve `kullaniciya_sor` etkileşim kartı. |
| `G5-E2E-02` | `S01`, `S02`, `S03`, `S04`, `S06` | Hat 03 (`PR #39`) | Arka plan süreç kartı, canlı çıktı akışı, süreç grup sonlandırma, PTY terminal oturumu ve çalışma alanı yazma kilidi. |
| `G5-E2E-03` | `M01`, `M02`, `M03`, `M04`, `M05`, `M06` | Hat 04 (`PR #40`) | Uzak Streamable HTTP MCP kartı, mühürlü yazma onayı, OAuth PKCE durumu (belirteç sızdırmadan) ve dinamik araç keşfi. |
| `G5-E2E-04` | `W01`, `W02`, `W03`, `W04`, `W05`, `W06` | Hat 05 (`PR #41`) | `web_getir` ve `web_arama` kaynak atıf kartları, SSRF ret bildirimi ve sarmalanmış dış içerik uyarısı. |
