# 5b oturum bütçesi kabul modeli

Bu prototip ürün kodu değildir, Rust workspace'ine ve kurucuya girmez.
Yeni bağımlılık, ağ isteği, sistem ayarı veya kalıcı test kullanıcısı yoktur.
5b tasarımının tek sahipli sayaç geçişlerini sınar. Gerçek sağlayıcı, JSONL,
Tauri, eklenti beklemesi ve kullanıcı kartı burada uygulanmamıştır; bunların
entegrasyon testleri uygulama planında zorunludur.

Repo kökünde, kurulu Rust 1.98.1 ile:

```sh
mkdir -p target/5b-on-dogrulama
rustc --edition 2024 --test prototip/oturum-butcesi/cekirdek.rs -o target/5b-on-dogrulama/butce-test
target/5b-on-dogrulama/butce-test
```

Windows'ta dizini `New-Item -ItemType Directory -Force` ile oluşturun;
çıktıya `.exe` ekleyin. Rust'un mevcut MSVC ortamı gerekir. Linux'ta
WSL Ubuntu 24.04'ün `kalfatest` kullanıcısıyla koşulur. Rustup bir indirme
isterse durun; bu prototip için kurulum yetkisi verilmiş değildir.

Kabul ölçütü: 11 test geçer. Yarış testi sekiz eşzamanlı talebi gerçek
kanalla tek sahibe taşır ve yalnız dört iznin verildiğini denetler.
Eksik kullanım, başlamış isteklerin tamamlanması, eski karar, kurtarma,
çift sonuç, izin alıcısının kapanması, mutlak limit yükseltme ve taşma sınanır. Token sınırının
önceden kesin harcama tavanı olduğu iddia edilmez.
