# G5 çevrimdışı kabul — 21 Ekim 2026

## Sonuç ve kaynak

R3'ün K03 CSS ve bozuk JSON maskeleme kusurları düzeltildi; Hat 04'e güncel EOF sözleşmesi aktarıldı. Ancak **G5 ürün kabulü tamamlanmadı; 20/24 eşiği ölçülmüş değildir**. Bu rapor çalışan ilk masaüstü kabul dilimini ve kalan ölçüm engelini kaydeder. Canlı model karşılaştırması yapılmadı.

Birlikte doğrulanan kaynak `codex/yetenek-son-kabul` dalının `dfdd90f30a009733a18fa0ea3f8dfb559bdd92ee` commit'idir: Hat 05 `d6222d3`, Hat 01 `66b3e11`, güvenlik PR #43 ile aynı Cargo yaması ve bu PR'daki ilk G5 arayüz testi. Ürün crate kaynakları, Cargo.toml ve Cargo.lock, Hat 05'in `d33f2393f5f30118b45e6fb98572c6ca99321153` başıyla aynıdır. Bu yerel bütünleştirme GitHub PR merge'ü değildir.

## Geçen kontroller

| Kontrol | Windows | WSL Ubuntu 24.04, kalfatest |
|---|---|---|
| K03 dahil ölçüm regresyonları | 10/10 | 10/10 |
| Web getirme / veri çıkışı | 15/15 + 2/2 | 15/15 + 2/2 |
| Gerçek oturum tablosu / uzak MCP uygulama akışı | 29/29 + 1/1 | 29/29 + 1/1 |
| Sandbox birim testleri | 115/115 | 85/85 |
| İlgili crate'lerde clippy ve yalıtım özellikli clippy | Geçti | Geçti |
| Gerçek yalıtım uyumluluğu | AppContainer 24/24 | Yalıtım 33/33 + uyumluluk 16/16 |
| Gerçek Tauri konuşma + ilk G5 kabulü | 6/6 | 6/6 |

Arayüzde `npm run check` 0 hata/0 uyarı, Vitest 35 dosya/515 test, normal derleme ve e2e derlemesi geçti. Güvenlik PR #43 için iki platformun workspace/e2e Rust testleri ve üç clippy kontrolü ayrıca yeşildir; bu sonuçlar bütünleşik ürün workspace'inin baştan sona test edildiği anlamına gelmez. Her PR'ın kendi başındaki CI ayrıca izlenir.

Masaüstü testi gerçek Tauri IPC'sini, çekirdek araçları ve Svelte arayüzünü kullanır; yalnız model yanıtları yerel sahte sağlayıcıdandır:

```sh
node --test --test-concurrency=1 --test-reporter=spec testler/02-konusma.test.mjs yetenekler/g5-proje-akisi.test.mjs
```

Linux'ta komut `xvfb-run -a` ile koştu. Test ikilisi `e2e,tauri/custom-protocol` özellikleriyle debug profilinde derlendi; sürüm paketi veya performans kabulü iddia edilmez. Windows'ta kullanıcı onaylı EdgeDriver 155.0.4283.62 test klasöründen kullanıldı. Windows derleme hedefinin farklı kaynaklarla paylaşılması eski Tauri komut manifest'ini tuttu; yalnız `cargo clean -p kalfa` sonrasında aynı kaynakla derleme ve altı test geçti. Ürün izinleri değiştirilmedi.

Yeni iki G5 testi planın gerçek adımlarını ve `PlanGuncellendi` JSONL kaydını; soru kartından TOML seçiminin doğru `(tur, soru_kimligi)` eşleşmesiyle araç sonucuna ve `KullaniciSorusuCozuldu` kaydına dönmesini doğrular. Bu testler tek başına K06/S05'in bütününü kapsamaz.

## Sabit 24 görev kapısındaki engel

G0 koşucusu, bugün de yalnız dosya araçlarını ve yerel MCP'yi bağlar. Geçen `g0_cevrimdisi_kosucu_iki_kosuda_ayni_sonucu_ve_dort_durumu_uretir` testi iki aynı G0 koşusunu doğrular: bağımlılıklar hazırken K01–K05 geçer; M01 varsayılan ortamda çalıştırılmaz; aşağıdaki 18 görev ölçüm adaptörü tarafından desteklenmez. Bu dağılım G5 veya canlı model başarısı değildir. Ayrıca M01 gerçek Linux stdio MCP + yalıtım yolu aynı bütünleşik kaynakta yeniden çalıştırıldı ve Geçti; alan dışı dosya ve .env erişimi reddedildi. M01 özel Windows koşusu ise çalışma alanı AppContainer ACL hazırlığı olmadığı için açık Çalıştırılmadı döndü; geçen 24/24 uyumluluk testi bu ayrı M01 önkoşulunun yerine yazılmadı.

| Aile | G5'e bağlanmamış sabit görevler |
|---|---|
| Yerel kodlama | K06 |
| Süreç ve oturum | S01, S02, S03, S04, S05, S06 |
| MCP | M02, M03, M04, M05, M06 |
| Web | W01, W02, W03, W04, W05, W06 |

Örneğin M02 fixture'ı uzak HTTP MCP isteği yerine `dosya_oku` çağırır ve sıfır onay bekler. Son ürün sözleşmesi uzak MCP çağrılarını her kipte kullanıcı onayına bağlar. Dolayısıyla yalnız yetenek bayraklarını açmak hem sahte başarı üretir hem onay sözleşmesiyle çelişir; sabit G0 fixture'ı değiştirilmedi ve bu çelişki gizlenmedi.

**Tek sonraki iş:** G0 başlangıç kümesini koruyarak sürümlü bir G5 görev adaptörü oluşturmak; önce M02'yi gerçek yerel HTTP MCP sunucusu, onaylı çağrı ve bağımsız sonuç denetimiyle bağlamak. Sonra kalan aileler aynı ürün yollarına bağlanarak 24 görev iki tekrarda çalıştırılmalı ve zorunlu güvenlik kontrolleriyle birlikte en az 20/24 eşiği ölçülmelidir. Tam masaüstü süreç/MCP/web kabulü ve sürüm paketleri ve canlı sağlayıcı deneyi ayrı kanıt ister. Merge ve canlı deney için yeni yetki kullanılmadı.

Yerel ham test logları ve sabit baş makbuzu ana checkout'un `.yerel/yetenek-duzeltme/` dizinindedir; açık depoya makineye özgü özel kayıtlar eklenmez.
