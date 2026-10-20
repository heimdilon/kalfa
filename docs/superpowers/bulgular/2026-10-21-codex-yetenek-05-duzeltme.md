# Codex — Bozuk JSON URL maskelemesi

21 Ekim 2026. Kullanıcı kalan uygulamayı Codex'e devretti. R3 incelemesindeki dar kusur düzeltildi; merge yetkisi devredilmedi.

web_getir argümanları JSON olarak ayrıştırılamadığında kayıt/sunum için tamamı [MASKELENDI] olur. Güvenilir URL sınırı olmayan ham metinde regex ile sır bulmaya çalışılmaz. Ham yürütme argümanları ve hedef/mühür bağı değişmez.

Windows: helper ve tam sahte sağlayıcılı oturum JSONL canary testleri önce düştü; düzeltmeyle web_getir 15/15 ve veri_cikisi 2/2 geçti. Mevcut doğru hedefe istek, değiştirilmiş hedefte eski mühür reddi, DTO ve hata sunumu testleri de geçti.

Kırmızı/yeşil yerel kanıtlar ana çalışma alanının .yerel/yetenek-duzeltme klasöründedir. Gerekli geniş Windows/WSL kontrolleri ve son baş CI'ı ayrıca kaydedilecektir; koşulmayan kontrol tamam sayılmaz. Yeni paket/indirme/sistem ayarı veya canlı API bu kod düzeltmelerine eklenmedi. Ortak dört Cargo audit açığı kullanıcı onaylı ayrı güvenlik yaması dalındadır.
