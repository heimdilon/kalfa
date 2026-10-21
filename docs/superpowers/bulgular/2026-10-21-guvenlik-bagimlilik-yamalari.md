# RustSec güvenlik yamaları — 21 Ekim 2026

## Sorun ve değişiklik

CI audit işi 118881376626 dört açık bildirdi. Wasmtime ve wasmtime-wasi birlikte
49.0.3 → 49.0.4, rustls-webpki 0.103.15 → 0.103.16 ve webpki-root-certs
1.0.9 → 1.0.10 yamasına alındı. Cargo.lock toplam 31 mevcut paket sürümünü
değiştiriyor: Wasmtime/Pulley/Cranelift ailesi ve iki TLS paketi. Paket adı
eklenmedi veya çıkarılmadı; manifest özellik listeleri ve audit kuralları aynı.

Kaynak: RUSTSEC-2026-0378/0379/0382/0383 için mevcut CI logunun Solution alanları.
RustSec web sayfaları bu istemcide açılamadı; logda olmayan çözüm iddia edilmedi.
Kullanıcı gerekli güvenlik yaması indirmelerini açıkça yetkilendirdi.

## Doğrulama

Windows ve Ubuntu-24.04 / kalfatest üzerinde cargo fmt ve üç clippy (workspace,
e2e özellikli, wasm32-wasip2) geçti. Kilit dosyası karşılaştırması 31 patch,
0 yeni/çıkarılan paket adı gösterdi. Ayrı Standards ve Spec incelemelerinde
yeni bulgu çıkmadı. Workspace/e2e test sayıları ile son commit CI audit sonucu
PR açıklamasında ve yerel makbuzda ayrıca kaydedilir; clippy başarısı test veya
audit başarısı yerine kullanılmaz.

Linux doğrulaması WSL'nin kendi diskinde geçici kaynak/target kopyasıyla yürür;
Windows dosya sistemi üzerindeki copy-file izin hatası nedeniyle test atlanmadı.
Canlı API çağrılmadı, yeni araç kurulmadı, sistem ayarı veya audit istisnası eklenmedi.

## Geri alma ve kapsam

Cargo.toml ile Cargo.lock birlikte geri alınabilir. API, oturum/kayıt şeması,
ürün izinleri ve yalıtım politikası değişmez. Yetenek PR'larının kalan kod
düzeltmeleri ayrı dallardadır; bu dal ana dal tabanlı güvenlik yamasıdır.
