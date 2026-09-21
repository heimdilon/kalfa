# Kalfa tasarımı — Codex incelemesi, 2. tur

Tarih: 2026-09-21
İncelenen: tasarım belgesinin r2 revizyonu (`5d17311`)
Kaynak: Codex'in yanıtı, kullanıcı tarafından sohbete yapıştırıldığı hâliyle. Metin
değiştirilmedi; bağlantı adları yapıştırma sırasında URL'lerini kaybetmiş olabilir.
İşlenişi: tasarım belgesi §17, r3.

---

Büyük ölçüde olmuş; r2 belirgin biçimde daha sağlam. Kredi muhasebesi, SSE
açıklaması, WIT sözleşmesi, denetçi başarısızlıkları, onay bağlama ve oturum
kurtarma düzeltilmiş. Yeni fixture'ları da kontrol ettim: Qwen şema yanıtı ve
Qwen/DeepSeek araç çağrıları belgede anlatılanları destekliyor.

Ama güvenlik bölümünü tamamen kapanmış saymadan şu noktaları düzeltirdim:

1. [P1] ~/.cargo bütünüyle okunabilir olmamalı. Satır 604
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:604) bu dizini açıyor; Cargo
   burada credentials.toml içinde giriş bilgileri saklayabiliyor. Gizli dosya engeli
   ise çalışma alanıyla sınırlı tarif edilmiş. Kötü niyetli build betiği token'ı
   okuyup stdout üzerinden modele taşıyabilir; ağın kapalı olması bunu engellemez.
   Yalnızca gereken alt dizinleri açın veya kimlik bilgisi içermeyen ayrı CARGO_HOME
   kullanın. Cargo belgeleri

2. [P1] Windows'ta iki ortak kullanıcı, projeleri birbirinden ayırmıyor. Satır 640
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:640) aynı hesaplara çalışma
   alanlarının ACL izinlerini veriyor. A ve B projeleri aynı anda açıksa aynı hesapla
   çalışan süreç, ikisine verilen izinleri de kullanabilir. İzinlerin geri alınması ve
   eşzamanlılık modeli belirtilmemiş. Çalışma alanına özel kimlik veya süreç başına
   ayrıca zorlanan erişim sınırı gerekiyor. Bu, tarif edilen ortak kimlik modelinden
   çıkan bir tasarım riski. Microsoft erişim modeli

3. [P1] Linux'ta kökü salt okunur bağlamak, dış dosyaları gizlemiyor. Satır 620
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:620) yalnızca $HOME'u örtüyor.
   Ev dizini dışında, örneğin /srv veya /mnt altında kullanıcı tarafından okunabilen
   başka proje verileri görünür kalabilir. Boş kökten başlayıp gerekli sistem ve proje
   yollarını seçerek bağlamak daha tutarlı. Bubblewrap'ın güvenlik sınırı seçilen
   bağlamalara bağlı. Bubblewrap belgeleri

4. [P1] .git koruması dosya araçlarıyla aşılabiliyor. Satır 607
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:607) .git/hooks ve .git/config
   korumasını kabuk profiline koyuyor; fakat Proje/Serbest kipinde yerleşik yazma
   aracı için aynı yasak belirtilmemiş. Ajan kabuk yerine dosya düzenleme aracını
   kullanabilir. Korunan yollar ortak izin motorunda bütün yazma yollarına
   uygulanmalı; özel core.hooksPath hedefleri de hesaba katılmalı.

5. [P1] Plugin gönderimleri için önizlemenin zamanı belirsiz. Satır 1165
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:1165) önizlemeyi araç
   sonuçlarına bağlıyor. Oysa RAG, araç çalışırken evren-cagir üzerinden içeriği
   gönderebilir; sonuç kartı geldiğinde veri çoktan çıkmış olur. Önizleme açıksa
   gönderim kararı, plugin'in host çağrısındaki HTTP isteğinden önce alınmalı.

6. [P2] git status karşılaştırması değişiklik kaydı için yeterli değil. Satır 412
   (docs/superpowers/specs/2026-09-21-kalfa-design.md:412): Dosya önceden de
   değiştirilmişse, komut yeniden değiştirdiğinde durum yine M kalır. Ignore edilen
   dosyalar da varsayılan çıktıda görünmez. Önce/sonra içerik hash'i veya snapshot
   karşılaştırması gerekiyor; aksi hâlde tur özeti eksik kalır. Git belgeleri

Bunlar çalışan kodda doğrulanmış açıklar değil, tasarımda kalan boşluklar. Çekirdek
uygulama planına geçilebilir; özellikle ilk dört maddeyi izolasyon tasarımı
kesinleşmeden kapatırdım. Dosyalarda değişiklik yapmadım.
