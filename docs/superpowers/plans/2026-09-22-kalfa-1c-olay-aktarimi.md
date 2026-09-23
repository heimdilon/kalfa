# 1c ilk adım — çekirdek olay aktarımı

Bu çalışma, masaüstü kabuğundan önce spec §11.1'deki sınırsız çekirdek
kuyruğunu ele alır. Tüm 1c'nin uygulama planı veya tamamlanma kaydı değildir.
Görsel kararların kaynağı `docs/GELISTIRME_DURUMU.md` ve `design/README.md`'dir.

## Kapsam

- Çekirdekten tüketiciye giden kuyruk hem olay sayısıyla hem serileştirilmiş
  JSON baytlarıyla sınırlanır. UTF-8 ve JSON kaçışları hesaba katılır.
- Kapasite sorunu veya alıcının kapanması, süren turun iptal yolunu tetikler.
  İptal, dolu olay kuyruğunun boşalmasını bekleyemez.
- Taşıma hatası tüketici tarafından gözlemlenebilir olmalıdır. Eksik bir
  aktarım başarılı tamamlanmış gibi sunulamaz.
- Onay ve araç olayları metin parçası gibi birleştirilemez. Teslim edilemeyen
  onay, işlemi yetkilendiremez; başlamış işin iptali geri alma sayılmaz.
- Mevcut kayıt ve kurtarma davranışı korunur. İptal edilen araç çağrılarının
  sonuçları bağlamda kapatılır.

## Doğrulama

1. Duraklayan tüketiciyle olay ve bayt sayısını ölç; sınırların aşılmadığını
   ve iptal yolunun çalıştığını doğrula.
2. Çok büyük tek olayın sınırsız kuyruk tahsisine yol açmadığını doğrula.
3. Türkçe metin ve JSON kaçışlarında bayt hesabını doğrula.
4. Alıcı kapanırken sağlayıcı akışı veya araç hazırlığı bekliyorsa iptali
   doğrula; yeni olay gelmesini şart koşma.
5. Gösterilemeyen onaydan sonra yazma yapılmadığını doğrula.
6. Kuyrukta kabul edilen kritik olayların sıra ve içeriğini doğrula.
7. Workspace testlerini, Clippy ve biçim kontrollerini çalıştır. Canlı EVREN
   testleri çalıştırılmaz.

## Sonraki işler ve sınırlar

Bu adımın bayt bütçesi olay taşıma kuyruğuna aittir; bütün sürecin RSS sınırı
değildir. Model yanıtı, konuşma geçmişi, araç hazırlığı ve JSONL okuması için
bellek yönetimi ayrıca değerlendirilmelidir.

Tauri `Channel<T>` köprüsü, 16 ms başlangıç penceresiyle metin birleştirme,
webview tarafındaki tüketim onayı ve uçtan uca p95 < 50 ms ölçümü henüz bu
adıma dahil değildir. Rust kuyruğunu sınırlamak Tauri/webview kuyruğunu
kendiliğinden sınırlamaz; gerçek arayüzle §11.1 kabul kapısı ayrıca geçilmelidir.

1c'de ayrıca kimlik deposu, EVREN şart penceresi, çalışma alanı seçimi,
dosya sistemi kimliğine göre ortak kilit yöneticisi, kayıt başına tek yazıcı,
pencere yetkileri, CSP, güvenli Markdown ve Svelte 5 ekranları kalır.
