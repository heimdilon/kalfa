# PR #8 — ek inceleme bulgularının düzeltmesi

13–14 Ekim 2026. `2eaecd5` başındaki üç ek bulgu, kullanıcının isteğiyle
Codex tarafından aynı PR dalında giderildi.

## Değişiklikler

- Tur özetinin 2 MiB diff bütçesi artık JSON dizgisindeki tırnakları ve
  kaçışları da sayar. Tam satırlar tutulur; UTF-8 karakterleri bölünmez.
  Çekirdeğin 8 MiB olay sınırı korunur.
- Diff çözümleyicisi blok başlığındaki eski/yeni satır sayısını izler.
  Dosya başlığına benzeyen `-- eski` / `++ yeni` içerikleri korunur;
  ardışık yamaların gerçek dosya başlıkları blok dışında atlanır.
- Yeni oturum mevcut model kataloğunu yerel `uygulama_durumu` IPC'sinden
  alır; yeni bir `/models` ağ isteği beklemez. Açılıştaki katalog yenilemesi
  arka planda sürer ve elle yenileme kullanılabilir.

## Regresyon kanıtı

Üç test önce bildirilen hatayla düştü, sonra geçti:

- `kontrol_karakterli_diffler_json_butcesine_sigar`: U+0001/U+0002 içeren
  beş diffin JSON boyutu 2 MiB diff bütçesini ve 8 MiB olay sınırını aşmaz.
  Önceki kodda 2 097 143 bayt ham diff, 12 436 208 baytlık olay üretiyordu.
- `dosya başlığına benzeyen gerçek içerik satırlarını korur`: `-- eski`
  ve `++ yeni`, numaralı silinen/eklenen içerik olarak çizilir; sayı 1/1'dir.
- `arka planda bekleyen katalog isteği yeni oturumu engellemez`: katalog
  yanıtı beklerken yeni oturum mevcut modelle başlar. İlk durum okumasından
  sonra güncellenen katalog da yeni oturuma yansır.

Hedefli Windows kontrolleri: tur özeti **12/12**, ilgili arayüz **27/27**.
Tam arayüz: **429/429**, tür kontrolü **0 hata/uyarı**, üretim derlemesi ve
paket denetimi temiz. Windows workspace **1 214/0/19** (geçti/düştü/atlandı);
Ubuntu workspace **1 158/0/19**. İki platformda e2e özellikli Rust
**191/0/0**; workspace ve e2e özellikli clippy ile fmt temiz.

Gerçek yalıtım ve WebDriver uçtan uca takımı bu düzeltmede yeniden
çalıştırılmadı; yukarıdaki e2e sayıları Rust'ın e2e özellikli testleridir.

Canlı EVREN çağrısı, paket kurulumu ve sistem ayarı değişikliği yapılmadı.
Eklenti/yalıtım sınırları ve bağımlılıklar değişmedi.
