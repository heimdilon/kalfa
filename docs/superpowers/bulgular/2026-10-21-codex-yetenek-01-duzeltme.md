# Codex — K03 CSS görünürlüğü

21 Ekim 2026. Kullanıcı kalan uygulamayı Codex'e devretti. R3 incelemesindeki dar kusur düzeltildi; merge yetkisi devredilmedi.

Svelte derleyicisinin external CSS çıktısı artık JSDOM document.head içine bağlanır. CSS ile gizlenen düğme Kaldı, CSS ile görünür düğme Geçti olur. Gerçek derleyici, mount ve kullanıcı tıklaması korunur. K03 SHA-256 ve iki ölçüm belgesi birlikte güncellendi.

Windows: yeni regresyon önce düştü (yanlış Geçti), düzeltmeden sonra bütün yetenek_programi takımı 10/10 geçti. Eksik Node paketlerinde Çalıştırılmadı beklentisi doğrulanır. Güncel G5 kaydı henüz tamamlanmadı; çevrimdışı G0 başarısı G5 yerine yazılmaz.

Kırmızı/yeşil yerel kanıtlar ana çalışma alanının .yerel/yetenek-duzeltme klasöründedir. Gerekli geniş Windows/WSL kontrolleri ve son baş CI'ı ayrıca kaydedilecektir; koşulmayan kontrol tamam sayılmaz. Yeni paket/indirme/sistem ayarı veya canlı API bu kod düzeltmelerine eklenmedi. Ortak dört Cargo audit açığı kullanıcı onaylı ayrı güvenlik yaması dalındadır.
