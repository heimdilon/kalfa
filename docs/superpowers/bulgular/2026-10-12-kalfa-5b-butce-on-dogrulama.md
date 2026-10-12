# 5b oturum bütçesi — Kabul modelinin ön doğrulaması

**Tarih:** 12 Ekim 2026. **Taban:** `3133248`.
**Kaynak:** `prototip/oturum-butcesi/cekirdek.rs` (workspace dışında).
**SHA-256:** `5dd19dbf289bcad7c55602a18b19b03f06cf61888c97aa718fc409f2944a4c22`.

Amaç: [5b tasarımının](../specs/2026-10-12-kalfa-5b-oturum-butcesi-design.md)
tek sahipli istek kabulü ve tüketim muhasebesinin uygulanabilirliğini
koşabilir Rust koduyla sınamak. Yeni paket/ağ çağrısı ve ürün değişikliği yok.

| Ortam | Araç zinciri | Sonuç |
|---|---|---|
| Windows | Rust 1.98.1, `48a229cea` | 11 geçti / 0 düştü / 0 atlandı |
| WSL Ubuntu 24.04, `kalfatest` | Rust 1.98.1, `48a229cea` | 11 geçti / 0 düştü / 0 atlandı |

[Çalıştırma komutları](../../../prototip/oturum-butcesi/README.md).
Windows test ikilisi ve iki platform logları git dışında yerelde saklandı.
Linux ikilisi doğrulama bitince kaldırıldı. Büyük bir `target`, yeni
worktree, AppContainer profili veya yeni test kullanıcısı oluşturulmadı.

## Kanıtlanan davranış

- Sekiz iş parçacığı gerçek kanalla tek sayaç sahibinden izin ister:
  dört kabul, dört ret; sonuçlar `join` ile toplanır, zamanlama tahminiyle
  testin geçtiği varsayılmaz.
- Kullanım eksikliği yeni izni keser; başlamış sonuçlar işlenebilir.
  Kullanıcı kararı aktif denemeler bitmeden verilemez. Devam geçmişi
  sıfırlamaz; sonraki eksik kullanım yeniden karar ister.
- Sonuçsuz başlangıç kurtarmada kaybolmaz; çift/olmayan kimlik ve eski
  karar reddedilir. İzin teslim edilirken alıcının kapanması, sonuç
  kanalının kapanışıyla bilinmeyen tüketim olarak görülebilir.
- Token sonradan öğrenildiği için paralel sonuçlar token eşiğini aşabilir;
  sonraki izin kesilir. Tam eşikte ve sayısal taşmada kapı açılmaz.

## Kanıtın sınırı

Bu **ürün entegrasyonu doğrulaması değildir**. Prototipte gerçek sağlayıcı,
Tokio/oneshot, kalıcı JSONL, serde kullanım ayrıştırması, eski kayıt göçü,
denetçi, eklenti host beklemesi ve Tauri/Svelte uygulanmamıştır. Windows
ve Linux'ta çalışan kod, planın bütün API değişikliklerinin derlendiği
anlamına gelmez. [Plan](../plans/2026-10-12-kalfa-5b-oturum-butcesi.md)
bu sınırların her biri için ayrı regresyon ve iki platform doğrulaması ister.

11 test gerçek EVREN'de token/harcama tavanı veya önerilen varsayılanların
ürün uygunluğu hakkında kanıt vermez. Canlı EVREN deneyi yapılmadı.
