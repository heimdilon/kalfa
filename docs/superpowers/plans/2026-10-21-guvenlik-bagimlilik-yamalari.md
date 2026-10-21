# Dört Cargo audit açığı — güvenlik yaması planı

21 Ekim 2026. Uygulayıcı: Codex. Kullanıcı gerekli güvenlik yamalarını indirmeyi açıkça onayladı.
Taban: main 5260ed37d9e1dfcda94cce3d4f0d0f9357b9243f.

## Global Constraints

- Yeni özellik/üst sürüm ailesi eklenmez; mevcut bağımlılıkların güvenlik yaması alınır.
- Wasmtime ve wasmtime-wasi aynı kesin patch sürümünde kalır; özellik listeleri değişmez.
- Cargo audit'in #41 CI işi 118881376626 kaynak alınır: RUSTSEC-2026-0378/0379 için Wasmtime >=49.0.4; 0382 için rustls-webpki >=0.103.16; 0383 için webpki-root-certs >=1.0.10.
- RustSec web sayfaları bu istemcide açılamadı; mevcut CI logunun çözüm alanları esas alındı. Çözümlenemeyen sürüm uydurulmaz.
- Cargo.lock hedeflenen paketlerle güncellenir; ilgisiz yükseltme, audit istisnası veya kontrol kapatma yapılmaz.
- Kurulum onayı yalnız gerekli güvenlik yaması indirmeleridir; sistem ayarı, sürücü kurulumu, canlı API ve merge yoktur.

## Başarı ve doğrulama

Wasmtime =49.0.3 -> =49.0.4; rustls-webpki 0.103.15 -> 0.103.16; webpki-root-certs 1.0.9 -> 1.0.10. Kilit dosyasının aile değişimleri incelenir. Windows ve WSL workspace/e2e özellikli test, clippy, fmt ve WASI eklenti kontrolleri çalıştırılır. Son commit'in CI audit sonucu ayrıca okunur. Koşulmayan kontrol tamam sayılmaz.

Geri alma: bu PR'daki Cargo.toml/Cargo.lock yama değişikliğini birlikte geri almak. Kaynak API veya kayıt biçimi değişmez. Sonuç ve gerçek test sayıları ayrı bulgu raporuna kaydedilir.
