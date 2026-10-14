# Wasmtime 49.0.3 — PR #19 güvenlik yaması

**Tarih:** 14–15 Ekim 2026. **Ajan:** Codex. **Durum:** Yerel doğrulama ve yama CI'ı tamamlandı.

## Gerekçe ve onay

MCP ürün başı `57563bc` için CI run
[38631584692](https://github.com/heimdilon/kalfa/actions/runs/38631584692),
14 Ekim'de yayımlanan RUSTSEC-2026-0352–0364 duyurularından kırmızı oldu.
13 bulgu mevcut `wasmtime`/`wasmtime-wasi` 49.0.2 ailesini etkiliyor;
RUSTSEC-2026-0352 dosya sistemi yalıtımından çıkışı da içeriyor.

`Cargo.toml` ve `Cargo.lock`, bu başta `main` `04f16fc` ile aynıydı.
Bu bağımlılık farkı MCP uygulamasından gelmedi. Resmî
[Wasmtime v49.0.3](https://github.com/bytecodealliance/wasmtime/releases/tag/v49.0.3)
14 Ekim'de yayımlandı ve bildirilen düzeltmeleri içeriyor.

AGENTS'ın sapma kuralıyla
[PR'a dar öneri](https://github.com/heimdilon/kalfa/pull/19#issuecomment-6177785980)
yazıldı. Yerel önbellekte yama olmadığı için indirme izni istendi.
Kullanıcı bu oturumdaki Windows/WSL indirme, güncelleme ve doğrulama
sorusuna doğrudan “Güncelle ve doğrula” yanıtını verdi. Sistem kurulumu
ve PR merge'i bu izin kapsamında değil.

## Fark

Yayımlanan yama `2203bf9a8736b26811802ea2772243346614b402` (Claude).
Codex'in yerel `f6be234` yamasıyla bütün izlenen dosyaların birebir aynı
olduğu `git diff --exit-code` ile doğrulandı. Yayımlanmış commit korundu;
Codex dalı ona alındı. Yerel karşılık `codex/wasmtime-49-yerel-denetim`
dalında durur; doğrulama iki aynı kaynak ağacı için geçerlidir:

- İki mevcut sabitleme `=49.0.2` → `=49.0.3`.
- Aynı özellikler: Wasmtime runtime/cranelift/component-model/async/std;
  WASI p2. Rust kaynaklarına ve testlere dokunulmadı.
- Lock'ta 693 paket kaydı, 617 ayrı paket adı korunuyor.
- Yalnız 29 aile üyesi patch sürümü değişti: 14 Wasmtime ve 2 Pulley
  kaydı 49.0.3; 13 Cranelift kaydı 0.136.3.
- Sürüm/checksum alanları çıkarıldığında lock tabanla aynı;
  bağımlılık ilişkileri değişmedi. Yeni/kaldırılmış paket adı yok.
- Audit ignore listesi değiştirilmedi; yeni test atlama/kapatma kuralı
  eklenmedi. Varsayılan canlı API `#[ignore]` testleri aynı kaldı.

Bu fark GPT-6 Luna / max alt ajan tarafından salt okunur bağımsız
incelendi. Kaynak/test/yapılandırma içerikleri tabanla karşılaştırıldı;
manifest dışında içerik farkı bulunmadı. Bu inceleme çalışma zamanı
doğrulaması değildir.

## Doğrulama

Rust 1.98.1; Windows yönetici olmayan hesap; WSL Ubuntu-24.04
`kalfatest`. Gereken Windows/Linux ve wasm32-wasip2 paketleri açık izinle
indirildi; ardından doğrulama `--locked` ve
`CARGO_NET_OFFLINE=true` ile yapıldı. Testler tek iş parçacığıyla koştu.

| Kontrol | Windows | WSL |
|---|---|---|
| `cargo fmt --all -- --check` | Geçti | Geçti |
| `cargo test -p kalfa-plugin --locked` | 84/84 | 84/84 |
| `cargo test --workspace --locked` | 1.234 geçti / 19 ignore | 1.187 geçti / 19 ignore |
| Workspace clippy, `-D warnings` | Geçti | Geçti |
| SDK/RAG/test eklentileri wasm32-wasip2 clippy | Geçti | Geçti |
| Uygulama/Tauri e2e özellikli Rust testleri | 191/191 | 191/191 |
| Uygulama/Tauri e2e özellikli clippy | Geçti | Geçti |
| `2203bf9` CI bağımlılık denetimi | Geçti | Ortak iş |

MCP'nin gerçek yalıtım ve ağ kabul kodu bu yamada değişmedi;
[1.1a-1 ürün raporundaki](2026-10-14-kalfa-11a-1-urun.md) Windows 50/50,
WSL 32/32 ve uyumluluk 24/24–16/16 kanıtı ayrı tutuluyor. Bu yama için
onları yeniden koşturmak yerine değişen eklenti çalışma zamanı ve onu
kullanan workspace/e2e yolları yeniden sınanıyor. Konuk paketlerin
wasm32-wasip2 derlenmesi de doğrulanıyor.

Bildirilen 13 açığın her biri için bu depoda ayrı saldırı prototipi
yazılmadı. Kanıt, doğru sabit yama sürümü, aynı özellik/bağımlılık ilişkileri,
mevcut eklenti sınır testleri ve güncel audit sonucudur. Canlı EVREN
deneyi veya şart kabul çağrısı yok.

## Kayıt ve temizlik

Yerel kayıtlar:
`.yerel/mcp-tasarim-inceleme/WASMTIME-WINDOWS.log`,
`WASMTIME-WSL.log` ve `WASMTIME-TEMIZLIK.log`.
İlk MCP ürün günlükleri değiştirilmedi.

Windows test kökü `%TEMP%/kalfa-codex-wasmtime49`; WSL kopyası
`/home/kalfatest/kalfa-codex-wasmtime49`. Önceki derleme önbellekleri
kullanıldı. Windows'ta 230 üst düzey test artığı kaldırıldı. Önce normal
silme denendi; taban testinin kısıtlı `paylasilan` dizini reddedildiğinde
yalnız sahibi olunan test dizinlerinin DACL bölümü geri verildi ve
kök silindi. SACL/owner/group ve sistem izinleri değiştirilmedi. Eski
44 dizine ve 28 Eylül Kalfa profiline dokunulmadı.

WSL crate kaynakları ve Cargo.toml/Cargo.lock Windows kaynağıyla birebir
karşılaştırıldı. Tauri'nin Git tarafından göz ardı edilen `gen/` platform
çıktıları bu karşılaştırmanın dışında tutuldu; tek ilk fark
`windows-schema.json` üretimiydi. Son fmt geçti, bu WSL kopyası kaldırıldı.
Ortak önceki önbellekler korundu; yeni ayrı 18–20 GB target oluşturulmadı.

## Ara doğrulama bulgusu

WSL'nin ilk workspace koşusu `kalfa-core --test saglayici` içindeki
fixture okumada düştü. Fixture yeni kopyada vardı; ortak target içindeki
eski test ikilisi `CARGO_MANIFEST_DIR` olarak silinmiş
`/home/kalfatest/kalfa-codex-11a1/crates/kalfa-core` yolunu içeriyordu.
Yalnız yeni WSL kopyasının Rust girdilerinin zaman damgası yenilendi;
kaynak içeriği değişmedi. Yeniden derlenen ikilide güncel yol doğrulandı;
aynı 6 test geçti. İlk hata günlüğü korunuyor; ardından tam workspace,
clippy, WASM ve e2e özellikli Rust kontrolleri tekrar tamamlandı.

Yayımlanan yamadaki
[CI audit işi](https://github.com/heimdilon/kalfa/actions/runs/38634667316/job/116094968353)
693 bağımlılığı taradı ve geçti. 13 Wasmtime bulgusu yok; önceden var olan
7 izinli uyarı sürdü. Bu sonuç genel olarak bütün uyarıların giderildiği
iddiası değildir.

Yayımlanmış `2203bf9` başının
[CI koşusu](https://github.com/heimdilon/kalfa/actions/runs/38634667316)
beş işiyle yeşil: biçim, bağımlılık denetimi, arayüz, Windows Rust ve
Linux Rust. Son belge commit'inin kontrolleri PR'ın güncel denetim
sayfasından izlenir; bu rapordaki çalışma zamanı kodu `2203bf9` ile aynıdır.
