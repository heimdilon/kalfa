# Kalfa 2c: Windows yalıtım prototipi

Aşama 2 tasarımının §9.5'indeki P1–P10 ölçümleri. Ürün kodu değildir; Kalfa'nın
workspace'ine girmez. Sonuçlar:
[bulgu raporu](../../docs/superpowers/bulgular/2026-09-25-kalfa-2c-windows-prototip.md).

## Adımlar

1. **Kurulum (yönetici).** Yönetici PowerShell'de, bu dizinde:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\kur.ps1
   ```

   Betik şunları yapar:
   - İki yerel kullanıcı ve bir grup oluşturur.
   - Oturum açma haklarını ayarlar.
   - Kullanıcıları giriş ekranında gizler.
   - Ağsız kullanıcı için güvenlik duvarı kuralı ekler.
   - Araç zinciri dizinlerine okuma izni verir.

   Ayrıntısı betiğin başındadır.
2. **Deneme ağacı (yönetici gerekmez).** Prototipi derleyip ağacı kurun:

   ```powershell
   cargo build --offline
   powershell -ExecutionPolicy Bypass -File .\agac.ps1
   ```

   `C:\KalfaPrototip` altında iki çalışma alanı, onlara özel dizinler ve
   izinleri kurulur (ayrıntısı betiğin başında). İzinler .NET ile verilir:
   `icacls`, hiçbir hesaba karşılık gelmeyen SID'leri kabul etmiyor.
3. **Ölçüm (yönetici gerekmez):**

   ```powershell
   cargo run --offline -- olc
   ```

   Rapor `C:\KalfaPrototip\sonuc.md`'ye yazılır. Belirli ölçümler için önek
   verilebilir: `cargo run --offline -- olc p2 p5`.
4. **P8 (yönetici).** Kullanıcıları Users grubuna ekleyip ölçümü yineleyin:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\users-grubu.ps1 -Ekle
   ```

5. **Geri alma (yönetici).** Makineyi prototipten önceki hâline döndürür:

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\geri-al.ps1
   ```

## AppContainer ek ölçümü

Kısıtlı belirteç yolunda açık kalan P1 (Dal A), A8 (Schannel) ve A9 (geri
döngü) AppContainer'da ölçülür; P2–P10 aynı senaryolarla yinelenir. Yerel
kullanıcı, grup ya da güvenlik duvarı kuralı yoktur: komut geliştiricinin kendi
hesabıyla, AppContainer belirteciyle çalışır. Sonuçlar:
[AppContainer bulguları](../../docs/superpowers/bulgular/2026-09-25-kalfa-2c-appcontainer.md).

1. **Kurulum (yönetici).** Yönetici PowerShell'de, bu dizinde:

   ```powershell
   cargo build --offline
   powershell -ExecutionPolicy Bypass -File .\kap-kur.ps1
   ```

   Betik şunları yapar:
   - AppContainer profillerini oluşturur.
   - Araç zinciri dizinlerine bir yetenek SID'i için okuma izni verir.
   - `C:\` köküne o yetenek için listeleme izni verir. İzin mirasla yayılmaz;
     yalnızca kök değişir.
   - `C:\KalfaPrototip` altında deneme ağacını kurar.

   Ayrıntısı betiğin başındadır. Yönetici; kök izni, `C:\Python314` ve
   `C:\Program Files\nodejs` izinleri için gerekir. Betik yeniden
   çalıştırılabilir: ağacı sıfırdan kurar.
2. **Ölçüm (yönetici gerekmez):**

   ```powershell
   cargo run --offline -- kap olc
   ```

   Rapor `%LOCALAPPDATA%\KalfaPrototip\sonuc-kap.md`'ye yazılır. Ölçüm deneme
   ağacını, ölçüm boyunca bir sürücü harfine bağlar (`subst` gibi).
   `cargo run --offline -- kap dene <komut>` tek bir komutu A'nın kabında koşar.
3. **Geri alma (yönetici):**

   ```powershell
   powershell -ExecutionPolicy Bypass -File .\kap-geri-al.ps1
   ```

## Parçalar

| Dosya | İş |
| --- | --- |
| `src/olc.rs` | Senaryoları koşar, raporu yazar |
| `src/calistir.rs` | Yalıtım kullanıcısı olarak çalışır: kısıtlı belirteç, Job Object, komut |
| `src/win.rs` | SID, belirteç, Job Object, boru ve `CreateProcessWithLogonW` yardımcıları |
| `src/ui.rs` | P10: ileti, pano ve pencere bulma denemeleri |
| `src/kap.rs` | AppContainer profili ve AppContainer'da Job Object içinde başlatma |
| `src/kap_olc.rs` | AppContainer senaryoları ve raporu |
| `kap-kur.ps1`, `kap-geri-al.ps1` | AppContainer ölçümünün yönetici adımları |
| `agac.ps1` | Deneme ağacı ve izinleri |
| `kur.ps1`, `geri-al.ps1`, `users-grubu.ps1`, `ortak.ps1` | Yönetici adımları |
| `guvenlik-duvari.ps1` | Eski kurulumun güvenlik duvarı kuralını düzeltir (P5); `kur.ps1` kuralı artık doğru kurar |

## MCP 1.1a G1 ölçümü

Borulu AppContainer süreci, uzun yaşam ve Job Object kapanışı, sunucuya özel
yetenek SID'i ayrımı (1.1a tasarımı §4.1). Yönetici gerekmez; profiller,
ACE'ler ve test kökü sonunda silinir. Sonuçlar:
[G1 raporu](../../docs/superpowers/bulgular/2026-10-14-kalfa-11a-g1.md).

```powershell
cargo build --offline
.\target\debug\kalfa-prototip.exe g1 olc <boş bir kök dizini>
```

`KALFA_G1_SURE` uzun yaşam ölçümünün saniyesini değiştirir (varsayılan 60).
