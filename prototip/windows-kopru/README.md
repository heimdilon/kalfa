# Kalfa 3d: Windows ağ köprüsü ölçümü (G6)

G6'nın ölçüm testi. Ürün kodu değildir; workspace'e girmez. Sonuçlar:
[bulgu raporu](../../docs/superpowers/bulgular/2026-10-05-kalfa-3d-g6-windows-kopru.md).

`g6_olcum.rs` Kalfa'nın gerçek Windows yalıtımını (`WindowsYalitici`) ve kabuk
aracını kullanır:
- Kapta bir Python betiği çalıştırır.
- Kabın dışında adlandırılmış boru sunucuları ve bir CONNECT vekili açar.

## Koşturma

Yalıtım kurulu olmalıdır. Python ve Node dizinleri kaba açılmış olmalıdır
(`kalfa-yalitim izin-ver <dizin>`, aşama 2 §9.4). Yönetici gerekmez. İnternet
gerekir.

1. Dosyayı test olarak kopyalayın:

   ```powershell
   Copy-Item prototip\windows-kopru\g6_olcum.rs crates\kalfa-tools\tests\
   ```

2. `crates\kalfa-tools\Cargo.toml`'da tokio satırını geçici olarak şöyle
   yapın (kilit dosyası değişmez):

   ```toml
   tokio = { workspace = true, features = ["net", "io-util"] }
   ```

3. Koşturun:

   ```powershell
   cargo test -p kalfa-tools --features yalitim-testleri --test g6_olcum -- --ignored --nocapture
   ```

4. Kopyayı ve `Cargo.toml` değişikliğini geri alın.

Test her koşuda yeni bir çalışma alanı açar ve sonunda onu kaldırır. Son satır
"ACE kaydı temiz: true" olmalıdır.
