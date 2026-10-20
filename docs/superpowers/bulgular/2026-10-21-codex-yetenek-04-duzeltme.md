# Codex — EOF sözleşme aktarımı

21 Ekim 2026. Kullanıcı kalan uygulamayı Codex'e devretti. R3 incelemesindeki dar kusur düzeltildi; merge yetkisi devredilmedi.

#39 uygulama commit'i 8073eb1acc4999b028ee96ac8c41b20692242138 cherry-pick ile aktarıldı (70bbb84). #39/#41 ile surec.rs blob eşdeğerliği bdf61e704f9e64d98bcf1e612bdd289d7cf2071f; ham terminale tekrarlanan EOF baytı üretilmez, desteklenmeyen girdi kapatma açık hata verir ve kilit korunur.

Windows: kalfa-sandbox ve kalfa-uygulama lib testleri geçti. WSL kanonik EOF/raw EOF/setsid torun kanıtları son ürün başında ayrıca doğrulanır. Önceki MCP/TLS/OAuth tasarımı değiştirilmedi.

Kırmızı/yeşil yerel kanıtlar ana çalışma alanının .yerel/yetenek-duzeltme klasöründedir. Gerekli geniş Windows/WSL kontrolleri ve son baş CI'ı ayrıca kaydedilecektir; koşulmayan kontrol tamam sayılmaz. Yeni paket/indirme/sistem ayarı veya canlı API bu kod düzeltmelerine eklenmedi. Ortak dört Cargo audit açığı kullanıcı onaylı ayrı güvenlik yaması dalındadır.
