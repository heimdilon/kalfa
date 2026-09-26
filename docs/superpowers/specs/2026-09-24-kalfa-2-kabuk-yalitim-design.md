# Kalfa 2. aşama — Kabuk aracı ve yalıtım: Tasarım

**Tarih:** 2026-09-24
**Durum:** Kapsam, yaklaşım ve tasarım özeti kullanıcıyla konuşulup onaylandı.
Belge bu özetin ayrıntısıdır; bölümler ayrıca tek tek onaya sunulmadı. Özetten
tek sapma §5.3'tedir: kabuk kısayolunun kuralı burada tanımlanır, uygulaması
aşama 3'e kalır.
2026-09-24 (2a planı): kod yazılırken netleşen ayrıntılar işlendi: çekirdek
arayüzü (§3.3), yerel onay yolu (§5.2), çıktı sınırları ve kaynak etiketi (§4.4),
başarısızlık sayacı (§4.5), terminal görünümünün sınırları (§10.1) ve yalıtım
durumunun taşınması (§10.3, §10.5).
2026-09-24 (2b planı): Linux arka ucu yazılıp WSL2'de ölçülürken netleşenler
işlendi: yalıtıcının yeni yöntemleri (§3.2), konuşmanın alta yapışması (§10.1), ortamın yalıtımlı çalıştırmadaki
değerleri (§6.1), süreç sayısı sınırı (§6.3, A1), araç zinciri dizinleri ve
profil özeti (§7.1), tarama (§7.2, A3), cargo registry'si (§7.4, A2), bwrap ve
seccomp ayrıntıları (§8.1, §8.2), algılama (§8.3), Ayarlar ve durumun kanalla
iletilmesi (§10.3–§10.5), testler (§12.2, §12.3, §12.5, A4) ve açık noktaların
cevapları (§14).
2026-09-25 (2c prototipi): Windows ölçümleri işlendi; sonuç Dal B ([2c bulguları](../bulgular/2026-09-25-kalfa-2c-windows-prototip.md)).
Kısıtlayıcı SID kümesi ve varsayılan DACL (§9.3), güvenlik duvarı kuralının
kullanıcı koşulu ve Users grubu (§9.2), izinlerin verilişi (§9.4), dal sonucu
(§9.5), yeni bilinen sınırlar (§9.6), A5–A7'nin cevapları ve 2d'den önce açık
A8–A10 (§14).
2026-09-25 (2c AppContainer ek ölçümü): aynı senaryolar AppContainer'da
ölçüldü; P1 geçti, A8–A10 çözülüyor ([AppContainer bulguları](../bulgular/2026-09-25-kalfa-2c-appcontainer.md)). Yeni açık noktalar A11–A13
(§14).
2026-09-25 (§9 yeniden yazıldı): Windows arka ucu AppContainer'a göre (§9); ayrı
kullanıcı ve kısıtlı belirteç yolu §9.7'de yedek. Ayrıca: §1.1, §2, §3.1, §6.1,
§7.4, §11, §12.2, §12.7, §13, §14 (A14, A15), §15; üst belge r3.5.
2026-09-26 (güvenlik düzeltmesi, 2d-1 planlanırken bulundu): yalıtımda `.git`
bütünüyle salt okunur; çalışma alanı dışındaki git dizinleri yalnızca
doğrulanırsa ve salt okunur açılır; Kalfa'nın çalışma alanına özel dizinlere
yazması bağlantı izlemez. §7.2, §7.3, §7.4, §8.1, §9.4, §12.1–§12.3, §13; üst
belge r3.6.
**Üst belge:** [Kalfa tasarım belgesi](2026-09-21-kalfa-design.md), özellikle
§7.2, §7.5–§7.7, §12.3, §13.2, §13.3 ve §14 (risk 7–9, 12, 13). Bu belgede `§`
işaretleri bu belgenin bölümlerini gösterir; üst belgeye "üst belge §x", 1c
tasarımına "1c §x" diye başvurulur.
**Temel aldığı iş:** 1c'nin tamamı (1c-1…1c-4, `kalfa-asama-1` dalında
`4c57b69`). [1c tasarımı](2026-09-22-kalfa-1c-design.md).

---

## 1. Amaç

Aşama 2, üst belge §12.3'teki teslim sırasının ikinci maddesidir. Aşama 2
bittiğinde ajan komut çalıştırabilir:

- Model `cargo test`, `git status` ya da `npm run build` gibi komutları bir
  kabuk aracıyla ister. Kullanıcı tam komutu, çalışma dizinini, ağ isteğini ve
  yalıtım durumunu görerek onaylar.
- Komut, işletim sistemi düzeyinde yalıtılmış çalışır:
  - Ev dizinindeki sırlar (`~/.ssh`, `~/.cargo/credentials.toml`) ve diğer
    çalışma alanları yalıtımın içinde görünmez.
  - Ağ kapalıdır.
  - Git'in kendiliğinden çalıştırdığı yollar salt okunurdur.
- Çıktı komut çalışırken arayüzde akar. Model, komut bitince kırpılmış çıktıyı
  ve çıkış kodunu alır.
- İptal ve zaman aşımı bütün süreç ağacını durdurur.
- Yalıtım kullanılamıyorsa Kalfa bunu gizlemez. Kabuk "yalıtımsız" durumuna
  geçer ve her çağrı her kipte yerel bir iletişim kutusunda onay ister.

### 1.1 Kapsam

1. **Kabuk aracı** (`kabuk`): iki biçim (program + argümanlar ya da betik),
   izin kararları, onay kartı, canlı çıktı (§4, §5).
2. **Süreç sınırları:** ortam değişkeni izin listesi, süre ve çıktı sınırı,
   süreç sayısı sınırı, süreç ağacının sonlandırılması (§6).
3. **Yalıtım profili:** işletim sisteminden bağımsız tanım (§7).
4. **Linux arka ucu:** `bubblewrap` + `seccomp` (§8).
5. **Windows arka ucu:** önce bir prototip (2c), sonra AppContainer + ACL.
   Kurulumu yapan yönetici yardımcısı `kalfa-yalitim.exe` (§9).
6. **Arayüz:** araç kartında terminal görünümü, kabuk onay kartı, durum
   çubuğunda yalıtım göstergesi, ayarlarda yalıtım bölümü (§10).

### 1.2 Kapsam dışı

| Konu | Ne zaman |
|---|---|
| "Bu oturumda izin ver" kısayolları (kabuk dahil) | Aşama 3; kabuk kısayolunun kuralı §5.3'te |
| Tur özeti: kabuğun değiştirdiği dosyaların listesi (üst belge §6.7) | Aşama 3 |
| Onay kartının tam hâli (veri çıkışı, geri alınabilirlik) | Aşama 3 |
| Linux dağıtım matrisi (Debian 12, Pardus, Fedora, AppArmor'lu Ubuntu masaüstü) | Dağıtım aşaması (`.deb`/`.rpm`, AppArmor profili) |
| MSI yükleyicisi | Dağıtım aşaması; yükleyici §9.2'deki yardımcıyı çağırır |
| Bellek ve CPU sınırı | Sonraki bir aşama; üst belge süreç sınırları arasında saymıyor |
| Etkileşimli komutlar (TTY, standart girdi) | v1 dışında; standart girdi her zaman kapalıdır |
| Aynı Windows makinesinde birden çok geliştirici hesabı | Sonraki bir aşama (§9.2) |
| Uzun süren arka plan süreçleri (geliştirme sunucusu, izleyici) | Sonraki bir aşama; komut bitince ya da süre dolunca ağaç sonlandırılır |

---

## 2. Başarı ölçütleri

1. Üst belge §13.2'nin `kalfa-sandbox` satırındaki bütün testler gerçek
   yalıtımla geçer: Linux'ta WSL2'de, Windows'ta bu makinede kurulumdan sonra.
   Windows'ta okuma sınırı AppContainer'la tam hedefi taşır (§9.5).
2. Üst belge §13.3'ün kabukla ilgili bütün senaryoları sınanır.
3. Yalıtım içinde `git`, `cargo`, `npm` ve `python`, bu depoda ve küçük örnek
   projelerde çalışır. Çalışmayan her durum §14'te gerekçesiyle yazılıdır.
4. Yalıtım kullanılamadığında kabuk sessizce yalıtımsız çalışmaz. Bu, sahte bir
   algılayıcıyla ve gerçek bir `bwrap` yokluğuyla sınanır.
5. Çıktı seli arayüzü kilitlemez ve turu iptal ettirmez (§4.4).
6. 1c-4'ün uçtan uca testlerine kabuk senaryoları eklenir; iki platformda
   geçer.

---

## 3. Mimari

### 3.1 Repo yerleşimi

```
crates/
  kalfa-sandbox/     yeni: profil, arka uçlar, süreç sınırları, çıktı yakalama
    src/
      profil.rs       işletim sisteminden bağımsız yalıtım profili (§7)
      surec.rs        başlatma, çıktı okuma, süre, iptal, süreç ağacı (§6)
      ortam.rs        ortam değişkeni izin listesi (§6.1)
      tarama.rs       gizli dosya ve korumalı yol taraması (§7.2)
      linux/          bwrap argümanları, seccomp filtresi, algılama (§8)
      windows/        AppContainer zinciri, ACL, kurulum durumu (§9)
      yalitimsiz.rs   yalıtımsız arka uç: yalnızca süreç sınırları (§6.5)
  kalfa-yalitim/     yeni, yalnızca Windows'ta işlev görür: kur, kaldir,
                     izin-ver, izin-geri-al (§9.1)
  kalfa-tools/       `kabuk` aracı eklenir (§4)
  kalfa-core/        protokol, izin kararı ve mühür eklemeleri (§3.3)
  kalfa-uygulama/    yalıtım durumu, yerel onay yolu, çalışma alanı kaydı (§5.2)
  kalfa/             yeni komutlar ve yetkileri (§10.5)
ui/                  terminal görünümü, kabuk onay kartı, durum çubuğu,
                     yalıtım ayarları (§10)
```

`kalfa-sandbox`, `kalfa-core`'a bağlı değildir. Profil ve istek tiplerini
kendisi tanımlar; `kalfa-tools` ikisini birleştirir. Böylece yalıtım
kendi testleriyle, oturum döngüsü olmadan sınanır.

### 3.2 Birimler

| Birim | Sorumluluk | Arayüz |
|---|---|---|
| `Yalitici` (trait, `kalfa-sandbox`) | Bir komutu bir profille başlatmak | `durum() -> YalitimDurumu`, `baslat(CalistirmaIstegi) -> Result<Surec, BaslatmaHatasi>` |
| `Surec` | Çıktıyı okumak, beklemek, süreç ağacını sonlandırmak | `cikti` akışı, `bekle() -> CikisDurumu`, `sonlandir()` |
| `LinuxYalitici` / `WindowsYalitici` / `Yalitimsiz` | Arka uçlar | `Yalitici` |
| `YalitimAlgilayici` | Arka ucun kullanılabilirliği ve nedeni | `algila() -> YalitimDurumu`; testte sahtesi |
| `KabukAraci` (`kalfa-tools`) | Şema, hazırlık, mühür girdisi, çıktı kırpma | `Arac` trait'i |
| `CalismaAlaniKaydi` (`kalfa-uygulama`) | Çalışma alanı kimliği, ilk kabuk onayı, Windows hazırlık ve ACE kaydı | dosya tabanlı, uygulama veri dizininde |

`YalitimDurumu`:

```
YalitimDurumu ::= Hazir { arka_uc: "bwrap" | "windows", surum: String }
                | Yok { neden: YalitimYoklukNedeni, talimat: String }
YalitimYoklukNedeni ::= BwrapYok | KullaniciAdAlaniKapali | KurulumGerekli
                      | KurulumBozuk | Desteklenmiyor
```

- `Yalitici`'ya 2b'de eklenenler: `profil_ozeti(ag)` (§7.1), `yeniden_algila()`
  (§8.3) ve `okunur_dizinleri_ayarla(dizinler)` (§7.1). Durum sonradan değişince
  yalıtıcı kurulurken verilen işlev çağrılır; uygulama bunu ana pencerenin
  kanalına yazar (§10.5).
- `CalistirmaIstegi.alan`: yalıtımlı çalıştırmanın çalışma alanı bilgisi
  (kanonik kök, gizli dosya taraması, korumalı yollar, dışarıdaki git
  dizinleri). Araç bunu onaylanan tarif yalıtımlıysa, her çalıştırmadan hemen
  önce üretir. Kip şimdiki durumdan değil onaylanan tariften alınır: mühür
  denetiminden sonra yeniden algılama durumu değiştirebilir. Verilmişse komut
  yalnızca yalıtımda, verilmemişse yalnızca yalıtımsız çalışır: istek
  onaylandığı kipte çalışır. Onaydan sonra durum değiştiyse komut başlamaz;
  yalıtımsız arka uç da `alan` verilmiş isteği reddeder. Mühür durumu zaten
  yakalar (§3.3); bu ikinci bir kilittir.

### 3.3 Çekirdekteki değişiklikler

- **Ara çıktı.** `Arac` trait'ine `calistir_akisli(argumanlar, hazirlik, iptal,
  cikti: CiktiGonderici)` eklenir. Varsayılanı `calistir`'ı çağırır; dosya
  araçları değişmez. Oturum araçları bununla çalıştırır.
- **`CiktiGonderici` kayıplıdır.** Arkasında sınırlı bir kanal vardır ve gönderim
  beklemez (`try_send`). Kanal doluysa parça atılır ve atılan bayt sayısı
  sonraki parçaya yazılır. Modele giden çıktı bundan etkilenmez; araç onu kendi
  arabelleğinde tutar (§4.4).
- **Oturum da parça atar.** Çekirdeğin olay kuyruğu olay ya da bayt bütçesinin
  yarısından fazla doluysa oturum parçayı arayüze iletmez, sayar. Çağrı
  bitince atılanların toplamı boş bir parçayla bildirilir. Kuyruğun öteki yarısı
  düşürülemeyen olaylara (araç sonucu, onay, tamamlanma) kalır. Araç tarafında
  atılanlar da bu toplama eklenir.
- **`AracHatasi::Basarisiz(AracCiktisi)`.** Araç çalıştı ama başarısız oldu;
  çıktı `[kaynak: …]` etiketiyle yine modele gider (§4.5).
- **`Hazirlik.kabuk: Option<KabukTarifi>`.** Tarif şunları taşır:
  - Tam komut metni ve biçimi (program ya da betik; betikse yorumlayıcı).
  - Çalışma dizini (çalışma alanına göre).
  - Ağ bayrağı.
  - Yalıtım durumu ve profilin özeti.

  Kabuk hazırlığı `tum_calisma_alani = true` verir; mevcut kilit yöneticisi
  bunu çalışma alanının özel kilidi olarak alır (1c §5.3).
- **Mühür** (üst belge §7.6) tarifin bütün alanlarını içerir. Yürütme anında
  yeniden hesaplanır. Yalıtım durumu arada değiştiyse (örneğin kurulum
  kaldırıldıysa) mühür tutmaz ve `AracHatasi::PolitikaDegisti` döner.
- **İzin kararı.** `izin::kabuk_karari(kip, &KabukTarifi)` §5.1'in tablosudur.
  `Karar::YerelOnayli { uyarilar }` eklenir: onay yalnızca yerel iletişim
  kutusundan gelebilir (§5.2). Birleştirmede sıra Yasak > Yerel onaylı > Onaylı
  > Serbest'tir.
- **Yerel onay komutu.** `Komut::YerelOnay { cagri, muhur }` eklenir. Yerel
  onaylı bir öğe `AracOnayi` ile onaylanamaz, yalnızca reddedilebilir;
  `YerelOnay` yalnızca yerel onaylı öğeyi onaylar.
- **Bekleyen onaylar.** `OturumTutamaci::bekleyen_onay(cagri)` onay bekleyen
  öğeyi döndürür. Yerel iletişim kutusu gösterdiği komutu buradan okur,
  webview'dan değil (§5.2).
- **Protokol:**

  ```
  Olay::AracCiktisiParcasi { cagri: CagriId, akis: Standart | Hata,
                             metin: String, atlanan_bayt: u64 }
  OnayOgesi += kabuk: Option<KabukTarifi>, yerel_onay: bool
  KabukTarifi ::= { komut: String, bicim: Program | Betik { yorumlayici: String },
                    dizin: String, ag: bool, yalitim: YalitimOzeti, ilk_cagri: bool }
  YalitimOzeti ::= Yalitimli { arka_uc, profil } | Yalitimsiz { talimat }
  ```

  - `AracCiktisiParcasi` birleştirilebilir olaylara eklenir (1c §4.2).
    Bitişik, aynı çağrıya ve aynı akışa ait parçalar tek parçada birleşir;
    atlanan bayt bildiren parça birleşmeyi keser.
  - `AracCiktisiParcasi` kayda yazılmaz. Kayıttan açılan oturumda terminal
    görünümü `AracSonucu`'ndaki son çıktıyı gösterir.

### 3.4 Veri akışı

```
model ─ tool_call(kabuk) ─▶ oturum döngüsü
  hazirla: argüman → KabukTarifi (yalıtım durumu dahil) → izin kararı
  onay: OnayGerekiyor ─▶ arayüz kartı ─▶ arac_onayi
        (yerel_onay ise kalfa-uygulama yerel iletişim kutusunu açar, komutu
         bekleyen_onay'dan okur ve Komut::YerelOnay gönderir)
  calistir: kilit → mühür yeniden hesaplanır → Yalitici::baslat
        stdout/stderr ─▶ CiktiGonderici (kayıplı) ─▶ AracCiktisiParcasi ─▶ arayüz
                      └▶ araç arabelleği (kayıpsız) ─▶ kırpma ─▶ AracSonucu ─▶ model
  iptal/süre: Surec::sonlandir → bütün ağaç
```

---

## 4. Kabuk aracı

### 4.1 Parametreler

```json
{
  "type": "object",
  "properties": {
    "program":        { "type": "string" },
    "argumanlar":     { "type": "array", "items": { "type": "string" } },
    "betik":          { "type": "string" },
    "dizin":          { "type": "string" },
    "ag":             { "type": "boolean", "default": false },
    "zaman_asimi_sn": { "type": "integer", "minimum": 1, "maximum": 1800, "default": 120 }
  },
  "oneOf": [ { "required": ["program"] }, { "required": ["betik"] } ],
  "additionalProperties": false
}
```

- **Program biçimi** doğrudan çalışır; kabuk yorumlayıcısı yoktur.
  - `program` ya çıplak bir addır ya da çalışma alanına göre bir yoldur.
  - Çıplak ad yalıtımın içindeki `PATH`'te aranır. Linux'ta arama bwrap'ın
    içinde yapılır.
  - Mutlak yol reddedilir. Böylece onay kartı `/usr/bin/…` ile yalıtım dışındaki
    bir aracın karışmasını göstermek zorunda kalmaz.
- **Betik biçimi:**
  - Linux'ta `/bin/sh -c <betik>`.
  - Windows'ta `powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command <betik>`.
    Başına çıktı kodlamasını UTF-8'e çeviren bir satır eklenir.
  - Onay kartı betiğin tam metnini ve yorumlayıcıyı gösterir.
- **`dizin`** çalışma alanına göre bir yoldur; varsayılan köktür. Mevcut yol
  doğrulamasından geçer (üst belge §7.3): `..`, mutlak yol, UNC ve aygıt yolu
  reddedilir. Dizin sembolik bağlantı olamaz.
- **Araç açıklaması** modele şunları söyler:
  - Program biçimini tercih etmesini.
  - Ağın varsayılan olarak kapalı olduğunu ve `ag: true` isteğinin her kipte
    onay gerektirdiğini.
  - Standart girdinin kapalı olduğunu.
  - Çıktının kırpılabileceğini.
- **İnceleme kipinde** araç modele hiç sunulmaz (`yazar_mi() = true`).

### 4.2 Çalışma alanında ilk kabuk çağrısı

Proje betiği çalıştırmak projeye kod olarak güvenmektir (üst belge §7.7).

- Bir çalışma alanındaki ilk kabuk çağrısı, kip ne olursa olsun (Serbest
  dahil) onay ister. Onay kartında şu uyarı bulunur: "Bu çalışma alanında ilk
  kez komut çalıştırılacak. Projenin betikleri (`build.rs`, `package.json`,
  `Makefile`) kod olarak çalışır."
- Onay, çalışma alanı kaydına yazılır (`CalismaAlaniKaydi`) ve sonraki
  oturumlarda sorulmaz. Kayıt uygulama veri dizinindedir; yalıtımın içinde
  görünmez.
- Windows'ta bu onay çalışma alanının hazırlığıyla birleşir ve yerel iletişim
  kutusunda sorulur (§9.4).

### 4.3 Standart girdi, terminal ve ortam

- Standart girdi kapalıdır (Linux'ta `/dev/null`, Windows'ta `NUL`). TTY
  yoktur.
- Etkileşim bekleyen araçlar için sabit değerler verilir: `TERM=dumb`,
  `GIT_TERMINAL_PROMPT=0`, `GIT_PAGER=cat`, `PAGER=cat`. Renk kapatılmaz; ANSI
  kaçışlarını terminal görünümü çizer (§10.1). Modele giden metinden ANSI
  kaçışları temizlenir.
- Ortam değişkenleri yalnızca izin listesiyle geçer (§6.1).

### 4.4 Çıktı

- **stdout ve stderr** geliş sırasıyla tek arabelleğe yazılır. Arayüze giden
  parçalar hangi akıştan geldiğini taşır.
- **Modele giden sonuç:**

  ```
  Çıkış kodu: 0            (ya da: "Zaman aşımı (120 sn); süreç ağacı durduruldu.")
  Süre: 3,4 sn
  --- çıktı ---
  <ilk 10 000 bayt>
  [... 1 204 311 bayt kırpıldı ...]
  <son 20 000 bayt>
  ```

  - Kırpma UTF-8 karakter sınırında yapılır.
  - Geçersiz UTF-8 baytları `U+FFFD` ile değiştirilir.
  - ANSI kaçışları temizlenir.
  - Kaynak etiketi `kabuk:<program>` olur (`kabuk:cargo`); betikte
    `kabuk:sh` ya da `kabuk:PowerShell`. Araç çağrının kimliğini bilmez.
- **Arayüze canlı akış:** en çok 1 MiB. Sınıra ulaşınca tek bir parça "Çıktının
  kalanı gösterilmiyor; model son 20000 baytı alacak." der. Bundan sonra
  yalnızca araç arabelleği dolmaya devam eder. Terminal görünümü zaten en çok
  200 000 karakter tuttuğu için (§10.1) daha fazlasını göndermek boşunadır.
- **Araç arabelleği:** baş için 10 000 bayt, son için 20 000 baytlık halka
  arabelleği. Toplam bayt sayısı ayrıca tutulur. Bellek, çıktının boyutundan
  bağımsızdır.
- **Neden kayıplı gönderici?** Çekirdek kuyruğu olay düşürmez ve dolduğunda
  turu iptal eder (1c §4.3). Hızla çıktı üreten bir komut arayüzü geride
  bırakırsa turun iptal edilmesi yanlış olur: komut sağlıklıdır, yalnızca
  ekran yetişemiyordur. Bu yüzden canlı parçalar kanal doluyken atılır. Atılan
  bayt sayısı bir sonraki parçada `atlanan_bayt` alanıyla bildirilir; terminal
  görünümü bunu "… N bayt gösterilmedi …" diye yazar. Modelin gördüğü çıktı
  kayıpsızdır (kırpma dışında).

### 4.5 Araç sonucu durumu

| Durum | `AracDurumu` |
|---|---|
| Çıkış kodu 0 | `Basarili` |
| Sıfırdan farklı çıkış kodu | `Basarisiz`; çıktı yine modele gider |
| Zaman aşımı | `Basarisiz`; sonuç metni zaman aşımını söyler |
| Tur iptali | `IptalEdildi` |
| Başlatılamadı (program yok, yalıtım başlatılamadı) | `Basarisiz`; metin nedeni söyler |

Sıfırdan farklı çıkış kodu ve zaman aşımı, aynı çağrının üst üste başarısızlık
sınırına (üç kez) sayılmaz: düzeltmeden sonra aynı testi yeniden koşmak
olağandır. Sayaç ne artar ne sıfırlanır. Başlatılamayan komut sayılır.

Kurtarmada (üst belge §6.6) başlamış ama sonucu yazılmamış bir kabuk çağrısı
yeniden çalıştırılmaz; `SonucuBilinmiyor` olur.

---

## 5. İzin kararları ve onay

### 5.1 Karar tablosu

| Kip | Yalıtımlı, ağsız | Yalıtımlı, ağlı | Yalıtımsız (ağ fark etmez) |
|---|---|---|---|
| İnceleme | Araç sunulmaz | Araç sunulmaz | Araç sunulmaz |
| Sıkı | Onay | Onay | Yerel onay |
| Proje | Onay | Onay | Yerel onay |
| Serbest | Otomatik | Onay | Yerel onay |

Her satıra eklenen kurallar:

- Çalışma alanındaki ilk kabuk çağrısı onay ister (§4.2).
- **Ağlı çalıştırmada uyarı:** "Komut ağa erişebilecek. Dosya yalıtımı sürer."
- **Yalıtımsız çalıştırmada uyarı:** "Yalıtımsız: komut senin bütün yetkilerinle
  çalışacak; ev dizinindeki dosyaları okuyabilir ve ağa erişebilir." Uyarı
  yalıtımın nasıl açılacağını da söyler (`YalitimDurumu::Yok.talimat`).
- **Yalıtımsız çalıştırmada ağ kapatılamaz.** Tarifteki `ag: false` yalnızca
  modelin niyetidir ve onay kartında öyle yazılır.

`yukseltme_aciklamasi` (`kalfa-uygulama/src/guvenlik.rs`) kabuğu anlatacak
biçimde güncellenir. Serbest kipin açıklaması: yalıtım varsa yalıtımlı ve ağsız
komutlar onay sormadan çalışır; ağlı ve yalıtımsız komutlar yine onay ister.

### 5.2 Yerel onay yolu

Üst belge §11.9: yüksek riskli onaylar webview'da değil, yerel iletişim
kutusunda verilir. Webview'a sızmış bir betik böylece kendi komutunu
onaylayamaz.

- Çekirdek `Karar::YerelOnayli` kararlarını `OnayOgesi.yerel_onay` olarak
  işaretler. Onay sayfası öğeyi ve "Yalıtımsız komutlar, onayınızdan sonra yerel
  bir pencerede bir kez daha sorulur." notunu gösterir. Onay düğmeleri aynıdır:
  webview'daki "Onayla" komutu çalıştırmaz, yalnızca yerel pencereyi açtırır.
- `arac_onayi` komutu (`kalfa-uygulama`), onaylanan her yerel öğe için
  bekleyen öğeyi çekirdekten okur (`bekleyen_onay`) ve yerel iletişim kutusunu
  açar. Kutuda tam komut, çalışma dizini, ağ niyeti, yalıtımsızlık uyarısı ve
  talimat yazar; düğmeler "Çalıştır" ve "Vazgeç"tir.
  - "Çalıştır": `Komut::YerelOnay` gider.
  - "Vazgeç": öğe "Kullanıcı yerel pencerede vazgeçti." gerekçesiyle reddedilir.
- Aynı onayda birden çok yerel öğe varsa tek kutu hepsini listeler. Yerel
  olmayan öğeler arayüzün kararıyla iletilir.
- Webview çekirdeğe doğrudan komut gönderemez; yalnızca `arac_onayi`'yı
  çağırabilir (1c §6). Çekirdek de `AracOnayi` ile gelen yerel öğe onayını
  reddeder; iki katman birbirinden bağımsız korur.

### 5.3 Kabuk kısayolu (aşama 3'te uygulanır)

"Bu oturumda izin ver" kısayolları aşama 3'te gelir (1c §1.2). Kabuk için
kural şimdiden sabittir, böylece aşama 3 yeniden karar vermez:

- Kısayol yalnızca program biçiminde, yalıtımlı ve ağsız çalıştırmaya verilir.
- Kapsamı program adı ve çalışma dizinidir: "Bu oturumda `cargo`'yu `.`
  dizininde yalıtımlı ve ağsız çalıştırabilir." Argümanlar serbesttir.
- Betik biçimi, ağlı ve yalıtımsız çalıştırma için kısayol sunulmaz.
- Kısayol bir kolaylıktır, güvenlik sınırı değildir (üst belge §7.2): `cargo`'ya
  izin vermek `build.rs`'e izin vermektir. Sınır yalıtımdır.

Aşama 2'de kabuk onayının kapsamı her zaman "yalnızca bu çağrı"dır.

---

## 6. Süreç sınırları

Bu bölüm yalıtımdan bağımsızdır; yalıtımsız çalıştırmada da geçerlidir.

### 6.1 Ortam değişkenleri

- Ortam boş başlar; yalnızca izin listesindekiler eklenir:
  - **Ortak:** `PATH`, `LANG`, `LC_*`, `TZ`.
  - **Linux:** `USER`, `LOGNAME`.
  - **Windows:** `SystemRoot`, `windir`, `ComSpec`, `PATHEXT`, `SystemDrive`,
    `ProgramFiles*`, `ProgramData`, `CommonProgramFiles*`,
    `NUMBER_OF_PROCESSORS`, `PROCESSOR_*`, `OS`.
- **Kalfa'nın verdikleri:**
  - Ev ve geçici dizin değişkenleri çalışma alanına özel dizinleri gösterir
    (§7.3): `HOME`, `TMPDIR`; Windows'ta `USERPROFILE`, `APPDATA`,
    `LOCALAPPDATA`, `TEMP`, `TMP`.
  - Araç yapılandırması: `CARGO_HOME`, `RUSTUP_HOME`, `GIT_CONFIG_GLOBAL`,
    `NPM_CONFIG_USERCONFIG`, `NPM_CONFIG_CACHE`, `PIP_CACHE_DIR`.
  - Windows'ta MSVC için `PATH` eki ve `LIB` (§9.3).
  - §4.3'teki sabit değerler.
- **Hiçbir koşulda aktarılmayanlar:** EVREN anahtarı, `SSH_AUTH_SOCK`,
  `GPG_AGENT_INFO`, `DBUS_SESSION_BUS_ADDRESS`, `DISPLAY`, `WAYLAND_DISPLAY`,
  `XDG_RUNTIME_DIR` ve adında `KEY`, `TOKEN`, `SECRET`, `PASSWORD` ya da
  `CREDENTIAL` geçen her değişken.
- İzin listesinin tam hâli `kalfa-sandbox/src/ortam.rs`'tedir. Kullanıcı
  ayarlardan değişken ekleyebilir; depo içindeki bir dosya ekleyemez (üst belge
  §7.2).
- Yalıtımsız çalıştırmada da aynı liste kullanılır. Yalıtımsız süreç ev dizinini
  okuyabildiği için bu tek başına bir sınır değildir, ama anahtarın ve ajan
  soketlerinin ortamdan sızmasını önler.
- **Kalfa'nın verdikleri yalnızca yalıtımlı çalıştırmada geçerlidir** (2b).
  Yalıtımsız süreç ev dizinini zaten okur; `HOME` ve `CARGO_HOME` gibi değerler
  kaynaktaki hâliyle geçer (2a'daki gibi). Yalıtımda `TEMP` ve `TMP` atılır,
  `TMPDIR` `/tmp`'dir.

### 6.2 Süre

- Varsayılan 120 sn, en çok 1800 sn; model `zaman_asimi_sn` ile ister.
- Süre dolunca süreç ağacı sonlandırılır (§6.4) ve sonuç zaman aşımını söyler
  (§4.5).
- Süre, komutun başlatıldığı andan ölçülür. Onay beklemesi ve Windows'taki
  hazırlık (§9.4) süreye dahil değildir.

### 6.3 Süreç sayısı

- Sınır 256 süreçtir.
- **Windows:** Job Object'in `ActiveProcessLimit` değeri.
- **Linux:** `RLIMIT_NPROC`, yalıtımın içinde konur (2b'de doğrulandı, A1):
  - Komut `/usr/bin/prlimit --nproc=256:256 -- <program>` ile başlar. prlimit,
    bwrap'ın açtığı kullanıcı ad alanının içinde çalışır.
  - Çekirdek 5.14'ten beri sınır ad alanı başına sayılır: yalnızca yalıtımın
    içindeki süreçler sayılır. Dışarıda 260 süreç varken içeride 100 süreç
    açılır, 300 açılamaz (`yalitim-testleri`).
  - Sınır bwrap'a dışarıda verilseydi ad alanının üst sınırı olurdu ve
    kullanıcının bütün süreçleri sayılırdı; komut hiç başlamazdı. Bu da
    denendi.
  - `prlimit` (util-linux) yoksa sınır uygulanmaz.
- **Yalıtımsız Linux:** `RLIMIT_NPROC` kullanıcının bütün süreçlerini sayar.
  Bu yüzden sınır uygulanmaz ve bu belgelenir.

### 6.4 İptal ve süreç ağacı

- **Linux, yalıtımlı:** bwrap'a `--unshare-pid` ve `--die-with-parent` verilir.
  Ad alanının ilk süreci ölünce çekirdek ad alanındaki her süreci öldürür.
  Sonlandırma sırası: bwrap'a `SIGTERM`, 2 sn sonra `SIGKILL`.
- **Linux, yalıtımsız:** komut kendi süreç grubunda başlar (`setsid`); iptalde
  gruba `SIGTERM`, 2 sn sonra `SIGKILL` gönderilir. Grubundan ayrılan
  (`setsid` çağıran) torunlar bu yolla yakalanamaz; bu sınır belgelenir.
- **Windows (iki kip):** komut bir Job Object içinde başlar
  (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`). İptal `TerminateJobObject` ile yapılır.
  Süreç askıda başlatılır, işe atanır, sonra sürdürülür. Böylece ilk komutun
  işe girmeden alt süreç doğurma yarışı olmaz.
- **Kalfa çökerse:** Linux'ta `--die-with-parent`, Windows'ta işin son tutamacının
  kapanması (§9.3) ağacı sonlandırır. Yalıtımsız Windows'ta da Job Object'in
  son tutamacı Kalfa'dadır. Yalıtımsız Linux'ta süreç grubunu çökme anında
  sonlandıran bir mekanizma yoktur; bu bilinen bir sınırdır ve 2b'de bwrap'la
  kapanır.
- Tur iptali, oturum kapanışı ve uygulama çıkışı (1c-4'teki `cikisi_ertele`)
  aynı sonlandırma yolunu kullanır.

### 6.5 Yalıtımsız arka uç

- `Yalitimsiz`, §6.1–§6.4'ü uygular ve başka hiçbir kısıt koymaz.
- **Ne zaman kullanılır:** yalnızca `YalitimAlgilayici` "yok" dediğinde.
  Kullanıcının yalıtımı kapatma seçeneği yoktur. Yalıtım varken yalıtımsız
  çalıştırma bilerek sunulmaz: Serbest kipin güvencesi yalıtımdır ve kullanıcı
  her komutu zaten yerel onayla çalıştırabilir.
- **Kip seçimi yalıtıma bağlı değildir.** Yalıtım yokken de her kip seçilebilir.
  Serbest kipte bile yalıtımsız her çağrı yerel onay ister (§5.1).

---

## 7. Yalıtım profili

### 7.1 Profil

İşletim sisteminden bağımsız tanım (`kalfa-sandbox/src/profil.rs`):

```
Profil {
  calisma_alani: Kok,                   // yazılabilir
  ozel_dizinler: { ev, gecici, onbellek }, // çalışma alanına özel, yazılabilir (§7.3)
  okunur: [Yol],                        // sistem yolları + izinli araç zinciri dizinleri
  gizli: [Yol],                         // okunamaz (§7.2)
  korumali: [Yol],                      // salt okunur (§7.2)
  git_dizinleri: [Yol],                 // çalışma alanı dışındaki gitdir/commondir (§7.4)
  ag: bool,
}
```

- **Okunur yollar.**
  - Varsayılanlar üst belge §7.7'deki listedir.
    - Linux: `/usr`, `/bin`, `/sbin`, `/lib*`, `/etc`.
    - Windows: sistem ve program dizinleri.
  - İzinli araç zinciri dizinleri:
    - Linux: `~/.cargo/bin` ve `~/.rustup`'ın tamamı. Rustup vekili her
      çalışmada `~/.rustup/update-hashes`'i ister; dizinde kimlik bilgisi
      yoktur (2b'de ölçüldü). `~/.cargo/registry` §7.4'teki kurala göre
      bağlanır.
    - Windows: bunların `%USERPROFILE%` karşılıkları.
  - Kullanıcı ayarlardan dizin ekleyebilir, örneğin `/opt/node` ya da
    `C:\Python314`.
  - Depo içindeki hiçbir dosya profili genişletemez (üst belge §7.2).
- **Profil özeti** (mühre giren) arka ucun, okunur yolların sıralı listesinin
  ve ağ bayrağının SHA-256'sıdır (ilk 16 onaltılık karakter). Kullanıcı bir
  dizin eklerse bekleyen onaylar geçersizleşir. Gizli ve korumalı yollar özete
  girmez: her çalıştırmadan hemen önce taranırlar ve onları belirleyen kurallar
  sabittir. Özete girselerdi, komutun kendi yazdığı bir dosya bekleyen onayları
  bozardı.

### 7.2 Gizli dosya ve korumalı yol taraması

Gizli ve korumalı yollar her çalıştırmadan hemen önce taranır.

- **Kapsam:**
  - Bütün çalışma alanı, `.gitignore` kapsamındakiler dahil. `.env` genellikle
    yok sayılan bir dosyadır.
  - Sembolik bağlantılar izlenmez.
  - Kalıplar üst belge §7.4'teki listedir; kullanıcının eklediği kalıplar
    dahildir (`Siniflandirici`).
- **Korumalı yollar** git koruma çözücüsünden (`git_koruma.rs`) gelir:
  `.git`'in kendisi ve `core.hooksPath`'in etkin değeri (`.git`'in dışındaysa).
  - `.git` (dizin ya da worktree'nin `.git` dosyası) yalıtımda bütünüyle salt
    okunurdur: yeniden adlandırılamaz, silinemez, içine yazılamaz. Kancalar ve
    yapılandırma onunla korunur. Yalnızca kanca ve yapılandırmayı korumak
    yetmiyordu: git'in yalıtım dışında kod çalıştırmasına yol açan başka
    dosyalar da `.git`'in içindedir (2026-09-26).
  - `git status`, `git diff` ve `git log` yalıtımda çalışır; `git add` ve
    `git commit` çalışmaz. Commit'i geliştirici yapar.
  - `.git` bir dosyaysa gösterdiği git dizini ve `commondir` ancak geri
    başvurusu tutarsa kullanılır: worktree'de ortak dizinin
    `worktrees/<ad>/gitdir` dosyası bu `.git`'i, alt modülde git dizininin
    `core.worktree`'si bu çalışma alanını gösterir. İkisi de çalışma alanında,
    yani yalıtımın yazabildiği yerdedir; doğrulanmasalardı yalıtımdaki komut
    başka bir dizini açtırabilirdi.
  - **Bilinen sınır:** `.git` sembolik bağlantıysa, çalışma alanı bir git
    deposu değilse ya da alt dizinlerde, yalıtımdaki komut yeni bir `.git`
    oluşturabilir. Git o dizinlerde bunu kullanır.
- **Linux'ta hard link.** Taramada bağlantı sayısı birden büyük olan normal
  dosyaların (aygıt, inode) çifti gizli dosyalarınkiyle karşılaştırılır.
  Eşleşen dosya da gizli sayılır. `a.txt`, `.env`'nin hard link'iyse o da
  okunamaz.
  - Windows'ta gerek yoktur: ACL dosya nesnesindedir ve hard link aynı nesneyi
    paylaşır.
  - Çalışma alanı dışındaki bir dosyaya hard link üst belge §7.3'teki bilinen
    sınırdır. Yalıtımda o dosya gizli kalıba uymuyorsa okunabilir.
- **Çalışma sırasında oluşan gizli dosya** (örneğin komutun kendisinin yazdığı
  bir `.env`) o çalıştırmada okunabilir. İçeriği komut zaten biliyordur. Sonraki
  çalıştırmanın taraması onu kapsar.
- **Maliyet:** tarama `ignore` crate'inin paralel yürüyücüsüyle yapılır. Hedef,
  100 000 dosyalık bir ağaçta 200 ms'nin altıdır. WSL2'de 60–63 ms ölçüldü
  (A3); önbelleğe gerek yok.
- **Gizli dizin** (örneğin kalıba uyan bir `credentials/`) boş ve salt okunur
  bir tmpfs'le örtülür. İçindeki dosyaların dışarıdaki hard link'leri de
  gizlidir.
- **Sembolik bağlantı** listelenmez: hedefi çalışma alanındaysa kendisi
  taranır, dışındaysa yalıtımda zaten görünmez.
- **Uzun liste:** Linux'ta bwrap seçenekleri her zaman `--args FD` ile, bir
  bellek dosyasından verilir; yol sayısı komut satırı sınırına takılmaz.
  Windows'ta her yol için bir ACE konur ve bu hazırlığın ilerlemesinde görünür.

### 7.3 Çalışma alanına özel dizinler

- Her çalışma alanı bir kimlik alır: kök dizinin dosya sistemi kimliği ve
  kanonik yolunun SHA-256'sından türetilen 16 onaltılık karakter. Kilit
  yöneticisinin kök kimliğiyle aynı kaynaktır (1c §5.3).
- Dizinlerin konumu:
  - Linux: `$XDG_DATA_HOME/kalfa/yalitim/<kimlik>/`, yani uygulamanın veri
    dizinindeki `yalitim/` (uçtan uca testlerde `KALFA_VERI_DIZINI`).
  - Windows: `%LOCALAPPDATA%\Kalfa\yalitim\<kimlik>\`.
- İçindeki dizinler:
  - `ev/`
  - `gecici/`
  - `onbellek/`, bunun altında `cargo/`, `npm/`, `pip/`.
- **Ortak yazılabilir durum yoktur** (üst belge §7.7 ilke 4). Ağlı çalıştırmada
  indirilen paketler bu önbelleğe yazılır.
- **Temizlik:** ayarlardaki yalıtım bölümü bir çalışma alanının dizinlerini
  silebilir (§10.4). Kalfa kendiliğinden silmez.
- **Bağlantı izlenmez.** Bu dizinlere yalıtımdaki komut yazabilir. Kalfa her
  çalıştırmadan önce dizinleri kurar ve `ev/.gitconfig` ile `ev/.npmrc`'yi
  yazar; bunu yaparken sembolik bağlantıları ve junction'ları izlemez. Yolda
  bağlantı varsa silinir, yerine dizin kurulur; dosya yeni bir geçici dosyaya
  yazılıp yerine taşınır. Böylece Kalfa'nın yazması yalıtım dışına ulaşmaz
  (2026-09-26).

### 7.4 Araç yapılandırması

Ev dizinindeki yapılandırma dosyaları yalıtıma açılmaz (üst belge §7.7 ilke 2).
Kalfa her çalıştırmadan önce çalışma alanına özel en küçük yapılandırmayı üretir.

- **Git:**
  - `ev/.gitconfig` yalnızca `user.name` ve `user.email`'i taşır. Değerler
    kullanıcının genel (`--global`) git yapılandırmasından okunur; depo
    yapılandırması yalıtımda zaten okunur. `GIT_CONFIG_GLOBAL` bu dosyayı
    gösterir.
  - Sistem yapılandırması (`/etc/gitconfig`) okunur yollar arasındadır ve
    olduğu gibi kalır.
  - Çalışma alanının git dizini çalışma alanı dışındaysa (worktree ya da
    submodule), doğrulanan `gitdir` ve `commondir` salt okunur açılır (§7.2).
    Worktree'de `git status` ve `git diff` çalışır; commit yalıtımda
    çalışmaz. Önceki tasarımda bu dizinler yazılabilirdi; 2026-09-26'da
    `.git`'in tamamıyla birlikte salt okunur oldu.
- **Cargo:**
  - `CARGO_HOME` çalışma alanına özeldir (`onbellek/cargo`).
  - Kullanıcının `~/.cargo/bin`'i salt okunur bağlanır ve `PATH`'e eklenir.
  - Kullanıcının `~/.cargo/registry`'si ağsız çalıştırmada salt okunur olarak
    `onbellek/cargo/registry` konumuna bağlanır. Daha önce indirilmiş ve
    açılmış crate'ler böylece ağsız derlemede kullanılır (A2: WSL2'de örnek
    crate `cargo test --offline` ile derlendi).
  - `credentials.toml` ve `config.toml` hiç görünmez.
  - Salt okunur registry'ye yeni crate indirilemez. Ağlı çalıştırmada
    kullanıcının registry'si bağlanmaz; cargo çalışma alanına özel, başta boş
    registry'ye indirir. O registry bir kez dolunca ağsız çalıştırmalar da onu
    kullanır. Kopyanın biçimi budur: kullanıcının registry'si kopyalanmaz
    (yüzlerce MB), çalışma alanının ihtiyacı ağla bir kez indirilir. bwrap
    0.9.0'da `--overlay` yoktur.
  - Windows'ta registry bağlanamaz: aynı dizini iki yerde göstermenin yolu
    yoktur. Çalışma alanına özel registry boş başlar ve ilk derleme ağlı bir
    çalıştırma ister (2c, P9). Geliştiricinin `CARGO_HOME`'u salt okunur
    verilince bağımlı crate ağsız derlendi ama 12 sn sürdü. Yol 2d planında
    seçilir (A15).
- **Rustup:** `RUSTUP_HOME` kullanıcının dizinini gösterir ve salt okunurdur.
  Eksik bir araç zincirini kurmaya çalışan rustup yalıtımda başarısız olur ve
  hatası modele gider.
- **npm:** `NPM_CONFIG_USERCONFIG` boş bir dosyayı gösterir. `~/.npmrc` token
  taşıyabilir ve açılmaz. Önbellek `onbellek/npm`'dedir.
- **pip:** önbellek `onbellek/pip`'tedir. Kullanıcı yapılandırması (`pip.conf`)
  açılmaz.
- Diğer araçlar için kural aynıdır: ev dizininde yapılandırma okunmaz.
  Gerekiyorsa en küçük kopya üretilir; bu, uyumluluk testlerinde karşılaşıldıkça
  eklenir (§12.3).

---

## 8. Linux arka ucu

### 8.1 bwrap çağrısı

- **`bwrap` yalnızca sistem yollarından kabul edilir:** `/usr/bin/bwrap` ya da
  `/usr/local/bin/bwrap`. `PATH`'ten aranmaz. Çalışma alanına konmuş sahte bir
  `bwrap` böylece seçilemez.
- **Argümanlar, sırasıyla:**

  ```
  --unshare-user --unshare-pid --unshare-ipc --unshare-uts --unshare-cgroup-try
  [--unshare-net]                            (ağsız profilde)
  --disable-userns --die-with-parent --new-session --clearenv
  --tmpfs /                                  boş kök
  --ro-bind <sistem yolu> <aynı yol> ...     /usr, /etc; bağlantılar --symlink
  --proc /proc --dev /dev --tmpfs /run
  [--ro-bind <resolv.conf hedefi> ...]       (ağlı profilde)
  --bind <ozel/gecici> /tmp
  --ro-bind <araç zinciri> <aynı yol> ...    ~/.cargo/bin, ~/.rustup, eklenenler
  --bind <çalışma alanı> <aynı yol>
  --bind <ozel/ev> <ozel/ev>                 HOME
  --bind <ozel/onbellek> <ozel/onbellek>
  [--ro-bind ~/.cargo/registry <ozel/onbellek/cargo/registry>]   (§7.4)
  --bind <gitdir> <aynı yol> ...             (§7.4)
  --ro-bind <korumalı yol> <aynı yol> ...    çalışma alanının üstüne
  --ro-bind /dev/null <gizli dosya> ...      gizli dosyalar
  --tmpfs <gizli dizin> --remount-ro <gizli dizin> ...
  --remount-ro /
  --setenv K V ...                           (§6.1)
  --chdir <dizin>
  --add-seccomp-fd <FD> --add-seccomp-fd <FD>   (§8.2)
  -- [/usr/bin/prlimit --nproc=256:256 --] <program> <argümanlar>
  ```

- **Yollar gerçek konumlarında bağlanır.** Çalışma alanı içeride de aynı
  yoldadır; hata mesajlarındaki yollar kullanıcının gördüğüyle eşleşir.
- **Ev dizininin geri kalanı yoktur.** `/home`, `/mnt`, `/srv`, `/media`, `/opt`,
  `/root` ve Kalfa'nın veri dizini bağlanmaz. `/opt` ya da benzeri bir yol ancak
  kullanıcı izinli araç zinciri dizini olarak eklerse görünür.
- **`/sys` bağlanmaz.** Uyumluluk testlerinde bir araç `/sys`'e ihtiyaç duyarsa
  yalnızca gereken alt yol salt okunur eklenir (§14).
- **`--disable-userns`** yalıtımın içinde yeni kullanıcı ad alanı açılmasını
  engeller. `--new-session`, `TIOCSTI` ile terminale karakter itmeyi önler.
- **2b'de netleşenler:**
  - Bağlama sırası: `/tmp` (çalışma alanına özel `gecici/`) sistem yollarından
    hemen sonra bağlanır. Altındaki yollar, örneğin geçici dizindeki bir
    çalışma alanı, onun üstüne gelir. Korumalı ve gizli yollar en sondadır;
    ardından boş kök `--remount-ro /` ile salt okunur olur. Böylece çalışma
    alanı dışına yazma denemesi sessizce bellekte kalmak yerine hata verir.
  - Birleşik `/usr`'da `/bin` ve `/lib` gibi bağlantılar `--symlink` ile
    kurulur.
  - Ağlı profilde `/etc/resolv.conf`'un gösterdiği dosya (`/run/...` ya da
    WSL'de `/mnt/wsl/...`) salt okunur bağlanır. `/run` boş olduğu için yoksa
    ad çözümü çalışmazdı.
  - Gizli dosya `/dev/null` ile örtülür. Kullanıcı ad alanındaki bağlama `nodev`
    olduğu için okuma "Permission denied" verir.
  - Eksik korumalı yol (örneğin `core.hooksPath`'in gösterdiği dizin) boş
    olarak oluşturulup salt okunur bağlanır; yoksa komut oraya kanca
    yazabilirdi. `.git` bağlama noktası olduğu için yeniden adlandırılamaz.
  - Program, yalıtımın içindeki `PATH`'te aranır. Arama dışarıda yapılır ama
    yalnızca içeride görünen yollara bakar; bulunan yol bwrap'a verilir.
    Bulunamazsa hata içerideki `PATH`'i söyler (§11). Çalışma alanındaki bir
    program, çalışma alanının kanonik yolundan çağrılır.
  - Seçenekler ve seccomp programları bellek dosyalarından (`memfd`) verilir;
    tanımlayıcılar yalnızca bwrap'a geçer.

### 8.2 seccomp

- Filtre `seccompiler` crate'iyle üretilir. Saf Rust'tır, C bağımlılığı yoktur;
  x86_64 ve aarch64'ü destekler. bwrap'a iki filtre verilir (`--add-seccomp-fd`,
  bellek dosyasından). Çekirdek ikisini de çalıştırır ve en kısıtlayıcı sonucu
  uygular:
  - **Engel filtresi** (`seccompiler`): aşağıdaki çağrılar `EPERM` döner.
    Mimari tutmazsa (örneğin x86_64'te 32 bit `int 0x80`) süreç öldürülür.
  - **`ENOSYS` filtresi** (elle yazılmış, birkaç komut): `clone3` ve x86_64'te
    x32 çağrıları (numarasında `0x40000000` biti olanlar). x32 çağrıları aynı
    mimari değeriyle gelir ve numara listesini atlatabilirdi; `seccompiler` bunu
    kapsamıyor.
- **İki profilde de engellenenler (`EPERM`):**
  - `io_uring_setup`, `io_uring_enter`, `io_uring_register`: io_uring soket
    işlemleri filtreyi atlatabilir.
  - `keyctl`, `add_key`, `request_key`: çekirdek anahtarlığı ad alanıyla
    ayrılmaz.
  - `bpf`, `perf_event_open`, `userfaultfd`, `ptrace`.
  - `unshare` ve `clone`'un ad alanı bayrakları; `--disable-userns`'e ek
    olarak.
  - `clone3` `ENOSYS` döner. Argümanları bir yapının içinde olduğu için
    bayrakları filtrelenemez; `ENOSYS` alan libc `clone`'a geri düşer ve o
    çağrının bayrakları denetlenir.
- **`socket` çağrısı adres ailesine göre izin listesiyle denetlenir;** listede
  olmayan aile `EPERM` döner.
  - Ağsız profilde yalnızca `AF_NETLINK` (`NETLINK_ROUTE` ile) açılabilir.
    `AF_INET`, `AF_INET6`, `AF_UNIX` ve `AF_PACKET` de dahil gerisi engellenir.
    Ağ ad alanı IP ağını zaten ayırır; filtre derinlemesine savunmadır. Asıl
    işi `docker.sock` ya da `ssh-agent` gibi yerel ayrıcalıklı soketlere
    ulaşmayı engellemektir. Bu soketler zaten bağlanmaz, ama soyut Unix
    soketleri dosya sisteminden bağımsızdır.
  - Neden izin listesi: ağ ad alanının kesmediği aileler var. `AF_VSOCK`
    sanal makineden ana makinedeki servislere ulaşır; WSL2 Windows'la bu yolla
    konuşur. Engel listesi bunları tek tek saymak zorunda kalırdı (PR
    incelemesinde bulundu).
  - `socketpair(AF_UNIX)` serbesttir.
- **Ağlı profilde:** ayrıca `AF_INET` ve `AF_INET6` açılabilir. `AF_UNIX` soket
  açmak yine engellenir; `socketpair` serbesttir. Ağ ad alanı ayrılmadığı için
  soyut Unix soketleri (X11, D-Bus) ancak böyle kapatılır.
- Kesin sistem çağrısı listesi `kalfa-sandbox/src/linux/seccomp.rs`'tedir. Her
  engelin bir testi vardır (§12.2).

### 8.3 Algılama

`YalitimAlgilayici` uygulama açılışında ve ayarlardan istenince şunları yapar:

1. `bwrap` sistem yollarında yoksa: `BwrapYok`. Talimat: "`bubblewrap`
   paketini kur (`sudo apt install bubblewrap`)."
2. Deneme çalıştırması (`bwrap --unshare-user --unshare-pid … /bin/true`)
   başarısızsa ve hata kullanıcı ad alanıyla ilgiliyse:
   `KullaniciAdAlaniKapali`. Talimat:
   - Ubuntu 24.04'te AppArmor kısıtını ve Kalfa'nın `.deb` paketinin bunu
     çözeceğini söyler.
   - Paket gelene kadar, bwrap için bir AppArmor profilinin nasıl
     ekleneceğini anlatır.
   - `kernel.apparmor_restrict_unprivileged_userns` sysctl'ini kapatmayı
     önermez: bu bütün sistemi etkiler.
3. Seccomp filtresi yüklenemiyorsa: `Desteklenmiyor`, nedeniyle birlikte.
4. Aksi hâlde: `Hazir { arka_uc: "bwrap", surum }`.

Algılama sonucu önbelleğe alınır. Bir başlatma yalıtım hatasıyla başarısız
olursa algılama yeniden çalışır ve durum güncellenir. Durum değişince bekleyen
onaylar mühür yoluyla geçersizleşir (§3.3).

2b'de netleşenler:

- Deneme çalıştırması gerçek profille, geçici bir çalışma alanında
  `/usr/bin/true`'yu çalıştırır; bir komutla aynı yoldan geçer.
- Hata metni "namespace", "uid map", "RTM_NEWADDR" ya da "Operation not
  permitted" içeriyorsa durum `KullaniciAdAlaniKapali`, değilse metniyle
  birlikte `Desteklenmiyor` olur.
- Yalıtımlı bir komut başlatılamadıysa ya da bwrap'ın kurulum hatasıyla (çıkış
  kodu 1, çıktı "bwrap: " ile başlıyor) bittiyse algılama yeniden çalışır.
- Talimatlar arayüzde gösterildiği için arayüzün diliyle ("siz") yazılır.
- Yalıtım yoksa Linux arka ucu komutu yalıtımsız çalıştırır ve durum bunu
  söyler; kabuk her çağrıda yerel onay ister (§5.1).

### 8.4 WSL2 ve test ortamı

- WSL2'deki Ubuntu 24.04'te (bwrap 0.9.0, çekirdek 6.18) ayrıcalıksız kullanıcı
  ad alanı açıktır ve AppArmor kısıtı yoktur. Bu durum 2026-09-24'te denendi.
- WSL'nin varsayılan kullanıcısı `root`'tur. Yalıtım testleri ayrıca sıradan bir
  kullanıcıyla koşmalıdır: root'un yalıtım dışında her şeyi okuyabilmesi bazı
  testleri anlamsız kılar. WSL'de bir test kullanıcısı oluşturmak 2b planının bir
  adımıdır ve kullanıcının iznini ister.
- AppArmor kısıtının kendisi WSL'de gözlenemez. Algılamanın o dalı sahte bir
  algılayıcıyla sınanır (§12.1). Gerçek davranış dağıtım aşamasının matrisinde
  doğrulanır.

---

## 9. Windows arka ucu

Komut, geliştiricinin kendi hesabıyla, çalışma alanına özel bir AppContainer'da
ve bir Job Object içinde çalışır. Bu, 2c'nin ölçtüğü Dal A'dır (§9.5).
Okunabilenler: sistem ve program dizinleri, izinli araç zinciri ve kendi
çalışma alanı. Ayrı yerel kullanıcı, parola, oturum açma hakkı ve güvenlik
duvarı kuralı yoktur. Önceki tasarım (ayrı kullanıcı ve kısıtlı belirteç)
yedek olarak §9.7'de özetlidir.

**Kavramlar:**

- **Paket SID'i** (`S-1-15-2-…`): çalışma alanının AppContainer profilinin
  kimliği. Çalışma alanı SID'inin yerini alır. Çalışma alanının ACL'i yazma
  iznini ona verir.
- **Araç zinciri yeteneği** (`S-1-15-3-1024-…`): `kalfaAracZinciri` adından
  `DeriveCapabilitySidsFromName` ile türetilir. Her çalışma alanının kabı bu
  yeteneği taşır. İzinli araç zinciri dizinleri ona okuma, `C:\` kökü de ona
  listeleme izni verir.
- **Erişim iki kez denetlenir.** Windows önce kullanıcıyı (geliştiriciyi),
  sonra kabı denetler. Kabın denetiminde paket SID'ine, yeteneklerine ya da
  `ALL APPLICATION PACKAGES`'a verilmiş bir izin gerekir. Sistem ve program
  dizinleri bu gruba zaten açıktır. Kullanıcı klasörleri ve profil açık
  değildir.

### 9.1 `kalfa-yalitim.exe`

Yönetici yetkisi isteyen işleri yapan yardımcı. Bütün alt komutları UAC ile
başlar:

| Alt komut | İş |
|---|---|
| `kur` | `C:\` köküne araç zinciri yeteneği için listeleme izni verir (§9.2), kurulum kaydını yazar |
| `kaldir` | Kurulum kaydına göre her şeyi geri alır |
| `izin-ver` / `izin-geri-al` | Geliştiricinin sahibi olmadığı bir araç zinciri dizinine yetenek için okuma izni verir ya da geri alır (§9.4) |

Komutu çalıştırmak için yardımcı gerekmez: Kalfa kabı kendisi başlatır (§9.3).
Durum denetimi de yükseltme istemez (§9.2).

**Hangi dosya çalıştırılır.** Kalfa, `kalfa-yalitim.exe`'yi yalıtımın
yazabileceği bir yerden hiçbir zaman başlatmaz. Başlatsaydı yalıtılmış bir komut
dosyayı değiştirip bir sonraki UAC yükseltmesinde yönetici yetkisi elde ederdi.

- Açılışta Kalfa, kendi çalıştırılabilir dosyasının yanındaki
  `kalfa-yalitim.exe`'yi `%LOCALAPPDATA%\Kalfa\bin\` dizinine kopyalar. Kopyanın
  SHA-256'sı kaynağınkiyle eşleşmelidir.
- Bütün çağrılar o kopyayı kullanır. Dizin geliştiricinin profilindedir ve hiçbir
  paket SID'ine izin vermez.
- Kopyalamadan önce kaynak dosyanın hazırlanmış bir çalışma alanının içinde
  olmadığı denetlenir. Geliştirme derlemesinde `target/` genellikle bu
  depodadır. Bu depo hazırlanmış bir çalışma alanıysa kopyalama reddedilir ve
  yalıtım durumu `KurulumBozuk` olur. Talimat `CARGO_TARGET_DIR`'i çalışma alanı
  dışına almayı söyler.
- Kurulu sürümde kaynak Program Files'tadır ve bu denetim kendiliğinden geçer.

### 9.2 Kurulum ve kaldırma

- **Kök izni.** `kur`, `C:\` köküne araç zinciri yeteneği için bir izin ekler:
  listeleme, öznitelik okuma ve geçiş (`0x1000A1`), miras yok.
  - **Neden gerekli:** git, cmd ve Node yolları üst dizinleri listeleyerek
    kanonikleştiriyor. İzin yokken git çalışma dizinini çözemiyor ("unable to
    get current working directory"). cmd'nin `dir` komutu düşüyor. npm de
    düşüyor: Node'un `realpath`'i `C:\`'nin özniteliklerini okuyamıyor (2c).
  - **Ne açar:** kap `C:\`'deki üst düzey adları görebilir; içlerini göremez.
  - **Nasıl yazılır:** yalnızca kökü değiştiren `SetFileSecurityW` ile,
    `SE_DACL_AUTO_INHERIT_REQ` bayrağı verilerek.
    - `SetNamedSecurityInfoW` (ve .NET'in `SetAccessControl`'ü) mirası bütün
      sürücüye yeniden yayar.
    - Bayrak verilmezse kök DACL'inin `SE_DACL_AUTO_INHERITED` bayrağı düşer
      (2c'de `D:PAI` → `D:P`).
  - Aynı desen başka yalıtım araçlarınca da kullanılıyor: ölçülen makinenin
    kökünde iki yetenek SID'i için aynı izin vardı.
  - Başka sürücülerdeki çalışma alanları o sürücünün köküne aynı izni ister;
    `kur` sabit disklerin hepsine uygular.
- **Kurulum kaydı:** `%ProgramData%\Kalfa\yalitim\kurulum.json`. Kök izni
  verilen sürücüleri ve `izin-ver` ile verilen izinleri listeler. Dosyayı
  yalnızca Administrators yazabilir.
- **Durum** yükseltme olmadan, uygulamanın içinde denetlenir:
  - Sabit disklerin kökünde yeteneğin izni var mı.
  - Deneme amaçlı bir `cmd /c exit 0` bir kapta başarılı mı. Bu deneme,
    `Kalfa.Deneme` adlı kendi profilinde koşar.

  Sonuç `Hazir`, `KurulumGerekli` ya da `KurulumBozuk`'tur. Bozuksa neyin eksik
  olduğu yazılır.
- **Kaldırma sırası:**
  1. Uygulama, yükseltme olmadan, her çalışma alanının izinlerini ACE kaydına
     göre geri alır ve AppContainer profillerini siler (§9.4).
  2. `kaldir` UAC ile başlar. Kök izinlerini ve `izin-ver` izinlerini kurulum
     kaydına göre geri alır, sonra kaydı siler.
  - Profiller yükseltilmemiş süreçten silinir. 2c'de yükseltilmiş geri alma
    betiğinden sonra profiller yerinde kalmıştı; yükseltilmemiş silme onları
    kaldırdı.
- **Birden çok geliştirici hesabı.** AppContainer profilleri ve çalışma
  alanlarının izinleri hesap başınadır. Kök izni ise ortaktır: bir hesabın
  `kaldir`'ı öteki hesabın yalıtımını bozar. Çok geliştiricili makineler kapsam
  dışıdır (§1.2); `kaldir`, kök iznini kaldırmadan önce bunu söyler.

### 9.3 Çalıştırma zinciri

```
Kalfa (geliştirici, orta bütünlük; açılışta SetErrorMode)
  │  profil: CreateAppContainerProfile("Kalfa.Alan.<kimlik>")  (yoksa)
  │  CreateProcessW(komut, EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED
  │                        | CREATE_NO_WINDOW | CREATE_UNICODE_ENVIRONMENT)
  │    SECURITY_CAPABILITIES: paket SID'i;
  │        yetenekler = [araç zinciri, (ağlıysa) internetClient]
  │    HANDLE_LIST: standart girdi (NUL), çıktı ve hata boruları
  │  Job Object: KILL_ON_JOB_CLOSE, ActiveProcessLimit=256, UI kısıtları
  │  işe ata → sürdür
  ▼
komut (geliştiricinin hesabı, AppContainer belirteci, düşük bütünlük)
```

- **Aracı süreç yoktur.** Kap doğrudan Kalfa'dan başlar. Başlatma çağrı başına
  22–29 ms sürer (2c, P6).
- **Kalfa çökerse** işin son tutamacı kapanır ve ağaç ölür (§6.4).
- **Paket SID'i** profil adından türetilir: `Kalfa.Alan.<kimlik>` (§7.3). Ad
  gizli değildir; güvenlik adın gizliliğine dayanmaz:
  - Kabın içinden başka bir paket SID'iyle süreç başlatılamıyor.
    `CreateProcessW` `ERROR_INVALID_PARAMETER` dönüyor (2c).
  - Kabın dışındaki geliştirici süreçleri çalışma alanına zaten erişebilir.
- **İki çalışma alanı** ayrı paket SID'leri taşır. A'nın kabı B'nin dizinlerini
  okuyamaz ve yazamaz. Kap geliştiricinin süreçlerini göremiyor (2c, P3).
- **Hata kutuları:** Kalfa açılışta hata kutularını kapatır
  (`SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX`).
  Kap bu kipi devralır. Yoksa yükleyici hataları (örneğin eksik DLL) UI
  kısıtlarına rağmen geliştiricinin masaüstünde iletişim kutusu açar ve komut
  kutu kapanana kadar bekler (2c, P10).
- **Ağ:**
  - Ağsız çalıştırmada kaba hiçbir ağ yeteneği verilmez: DNS dahil bağlantı
    kurulamaz.
  - Ağlı çalıştırmada `internetClient` verilir. Dış HTTP ve HTTPS açılır,
    Schannel dahil (A8).
  - Geri döngü (127.0.0.1) iki durumda da kapalıdır (A9). Kalfa geri döngü
    muafiyeti (`CheckNetIsolation LoopbackExempt`) vermez; yerel sunuculara
    erişim bilinen sınırdır (§9.6).
  - `privateNetworkClientServer` verilmez (A13).
- **Adlandırılmış borular:** geliştiricinin varsayılan izinli boruları kaptan
  açılamıyor (A10).
- **UI:** kap düşük bütünlükte çalışır. Geliştiricinin penceresine ileti
  gönderemez ve panoyu okuyamaz; Job Object'in UI kısıtları da verilir.
  `FindWindow` engellenmez (§9.6).
- **Çıktı:** Kalfa'nın açtığı borular doğrudan komuta geçer.
- **İptal:** `TerminateJobObject` (§6.4).
- **Ortam** (§6.1'e ek):
  - Windows, AppContainer süreçlerinde `LOCALAPPDATA`'yı
    `<verilen değer>\Packages\<profil adı>\AC` yapar (2c). Kalfa bu dizini
    çalışma alanına özel `ev` dizininde önceden oluşturur.
  - `TEMP` yazılabilir olmalıdır. Yoksa PowerShell ConstrainedLanguage kipinde
    açılır (2c).
  - **MSVC:** rustc, Visual Studio'nun `ProgramData`'daki kurulum kaydını
    okuyamadığı için `link.exe`'yi bulamaz. Kalfa, geliştirici tarafında
    `vswhere` ile son MSVC araç takımını ve Windows SDK'yı bulur:
    - `PATH`'e `…\VC\Tools\MSVC\<sürüm>\bin\Hostx64\x64` eklenir.
    - `LIB`, MSVC'nin `lib\x64`'ünü, SDK'nın `um\x64` ve `ucrt\x64`
      dizinlerini gösterir.
    - Bu dizinler Program Files (x86) altındadır ve kaba zaten açıktır.
    - C derleyen `build.rs` için `INCLUDE` ve `cl.exe` A12'dir.
- **LPAC kullanılmaz.** `ALL APPLICATION PACKAGES` dışındaki kapta PowerShell
  ve cargo çalışmıyor (2c).

### 9.4 Çalışma alanı hazırlığı ve ACE kaydı

- **Ne zaman:** çalışma alanındaki ilk kabuk çağrısında, onaydan sonra ve
  komuttan önce.
- **Onay:** çalışma alanının izinleri değişeceği için bu onay her kipte yerel
  iletişim kutusunda sorulur. Kutu §4.2'deki "projeye güven" uyarısıyla
  birleşir ve şunu da söyler: "Kalfa bu klasörün izinlerine yalıtım için
  giriş ekleyecek. Klasörü ayarlardan kaldırınca geri alınır."
- **Yapılanlar:**
  1. AppContainer profili oluşturulur, paket SID'i çalışma alanı kaydına
     yazılır.
  2. Kök dizine paket SID'i için devralınan (`OI|CI`) değiştirme izni eklenir;
     değişiklik ağaca yayılır. Devralmayı kapatmış (korumalı DACL'li) alt
     dizinler izni almaz ve yalıtımda erişilemez olur. Bu belgelenir.
  3. Çalışma alanına özel dizinler (§7.3) aynı izinle hazırlanır.
  4. **Gizli dosyalar:** mirası kesilir. Yeni DACL yalnızca geliştiriciye,
     SYSTEM'e ve Administrators'a izin verir; paket SID'ine izin yoktur.
  5. **Korumalı yollar** (§7.2):
     - `.git` (dizin ya da dosya) ve `core.hooksPath`'in hedefi: mirası
       kesilir, paket SID'ine yalnızca okuma (dizinde devralınan). Yazma,
       silme, üzerine taşıma ve yeniden adlandırma reddedilir.
     - Çalışma alanı dışındaki doğrulanmış git dizinine (§7.4) paket SID'i
       için yalnızca okuma verilir.
- **Ret izni kullanılmaz.** AppContainer, paket SID'ine verilen ret (deny)
  izinlerini uygulamıyor. 2c'de ret izni DACL'de ilk sırada olduğu hâlde `.env`
  okundu, `.git/config` değişti, `pre-commit` silindi. Korumalar yalnızca
  mirası kesip izin vermeyerek çizilir.
- **Bütünlük etiketi gerekmez:** kap, izinli ama etiketsiz (orta bütünlüklü)
  dizine yazabiliyor (2c).
- **Üst dizinler:** çalışma alanının `C:\` ile kendisi arasındaki dizinlerin
  listelenmesi gerekip gerekmediği A11'dir. Profil altındaki bir çalışma alanı
  için izin, profil kökündeki adları açar.
- **İlerleme:** büyük ağaçlarda yayılma zaman alır. İlerleme araç kartında
  canlı çıktı olarak görünür (`AracCiktisiParcasi`, "İzinler hazırlanıyor:
  12 400 / 80 000").
- **Her çalıştırmadan önce** §7.2'nin taraması yeni gizli dosyaları bulur ve
  onların da mirasını keser.
- **İzinler Win32 API'siyle verilir** (`SetEntriesInAclW`,
  `SetNamedSecurityInfoW`). `icacls`, hiçbir hesaba karşılık gelmeyen SID'leri
  reddediyor; paket ve yetenek SID'leri böyledir.
- **ACE kaydı:**
  - Konum: `%LOCALAPPDATA%\Kalfa\yalitim\ace-kaydi.jsonl`.
  - İçerik: uygulamanın eklediği her izin (yol, SID, maske, devralma). Mirası
    kesilen her nesne için de önceki DACL'i (SDDL).
  - Geri alma bu kayda göre yapılır, yeniden tarama yapılmaz. Mirası kesilen
    nesnenin önceki DACL'i geri yazılır.
  - Kayıt bozuksa geri alma, kökten başlayıp Kalfa'nın paket SID'lerini taşıyan
    bütün girişleri siler ve mirası kesilmiş nesnelerde mirası yeniden açar.
    Bu yavaştır ama eksiksizdir.
- **Araç zinciri dizinleri:**
  - Program Files ve Windows `ALL APPLICATION PACKAGES`'a zaten açıktır. Git,
    MSVC ve Windows SDK ayrıca izin istemez.
  - Geliştiricinin sahibi olduğu dizinlere (örneğin `%USERPROFILE%\.cargo\bin`
    ve `.rustup`) uygulama yükseltme olmadan yetenek için okuma izni verir.
  - Sahibi olmadığı dizinler `izin-ver` ile UAC'yle açılır. Örnekler:
    `C:\Python314`, ya da `ALL APPLICATION PACKAGES`'a kapalı
    `C:\Program Files\nodejs`. Bu izinler kurulum kaydına yazılır.
- **Kaldırma:** ayarlardaki yalıtım bölümünden bir çalışma alanı kaldırılınca:
  1. İzinleri ACE kaydına göre geri alınır.
  2. AppContainer profili silinir (`DeleteAppContainerProfile`) ve paket SID'i
     kayıttan çıkarılır.
  3. Çalışma alanına özel dizinler istenirse silinir.

### 9.5 Prototip ve sonuç

Windows arka ucu bir prototiple başladı (üst belge §12.3, risk 12). Prototip
ürün kodu değildir (`prototip/windows-yalitim/`):

- Senin makinende çalıştı. Yönetici gerektiren adımları sen çalıştırdın.
- Her kurulum adımının bir geri alma betiği var. Prototip bittikten sonra makine
  ilk hâline döndü.

**Ölçülenler:**

| # | Soru |
|---|---|
| P1 | Sistem ve program dizinleri okunuyor, `C:\projeler\` gibi kullanıcı klasörleri okunmuyor mu? |
| P2 | `cmd`, `powershell`, `git`, `cargo`, `npm`, `python` çalışıyor mu; bunun için ne gerekiyor? |
| P3 | A'nın yalıtımı B'yi okuyup yazamıyor mu? |
| P4 | `~\.ssh`, Credential Manager ve geliştiricinin profili okunamıyor mu? |
| P5 | Ağsız çalıştırmada giden trafik, geri döngü ve adlandırılmış borular |
| P6 | Başlatma gecikmesi, std tutamaç aktarımı, işe atama |
| P7 | Gizli dosya okunamıyor; korumalı yolda yazma, silme ve üzerine taşıma reddediliyor mu? |
| P8 | `Users` grubundan çıkarmanın etkisi (yalnızca ayrı kullanıcı yolunda) |
| P9 | Çalışma alanına özel `CARGO_HOME` ile derlemenin ağ ihtiyacı |
| P10 | Yalıtılmış sürecin geliştiricinin penceresine ileti gönderip gönderemediği, panoyu okuyup okuyamadığı |

**Sonuç (2026-09-25):**

- **Ayrı kullanıcı ve kısıtlı belirteç: Dal B** ([bulgular](../bulgular/2026-09-25-kalfa-2c-windows-prototip.md)).
  - S-1-15 SID'leri kısıtlayıcı olamıyor. Sistem dizinlerini okutan tek
    kullanılabilir SID Users, bu yüzden Users'ın okuyabildiği her klasör
    okunuyordu.
  - Schannel çalışmıyordu, geri döngü açıktı (§9.7).
- **AppContainer: Dal A** ([bulgular](../bulgular/2026-09-25-kalfa-2c-appcontainer.md)).
  - P1, P3, P4 ve P7 geçti; P2'de bütün araç zincirleri çalıştı.
  - Ağ, geri döngü ve borular için A8–A10 çözüldü.
  - Bedeli: kök izni, yalnızca izinle korumalar ve MSVC ortamı (§9.2–§9.4).

Bu bölüm AppContainer'a göre yazıldı. Başarı ölçütü 1'in okuma satırı Windows'ta
da tam hedefi taşır.

### 9.6 Bilinen sınırlar

- **Devralmayı kapatmış alt dizinler** yalıtımda erişilemez (§9.4). Git
  depolarında bu genellikle olmaz.
- **FAT32 ve exFAT'te ACL yoktur.** Böyle bir birimdeki çalışma alanı
  hazırlanamaz; kabuk bu çalışma alanında yalıtımsız kalır ve bu söylenir.
- **Ağ paylaşımı (SMB) üzerindeki çalışma alanı** kapsam dışıdır: ACL'ler sunucu
  tarafındadır. Hazırlık reddedilir.
- **`ALL APPLICATION PACKAGES`'a açık her yer okunur.** Program Files ve
  Windows'un yanında, başka uygulamaların bu gruba açtığı dizinler de okunur.
  2c'nin taramasında profilde böyle tek bir dizin bulundu: başka bir
  uygulamanın kendi yalıtımı için açtığı bir çalışma zamanı dizini.
- **`C:\`'deki üst düzey adlar** görünür, içerikleri görünmez (§9.2).
- **Yerel sunuculara erişilemez:** kap, 127.0.0.1'deki geliştirme
  sunucularına ve veritabanlarına bağlanamaz.
- **Pencere bulma** (`FindWindow`) engellenmez; bulunan pencereye ileti
  gönderilemez.
- **Tek geliştirici hesabı** (§9.2).

### 9.7 Yedek: ayrı kullanıcı ve kısıtlı belirteç

Bu bölümün ilk tasarımı ([2c bulguları](../bulgular/2026-09-25-kalfa-2c-windows-prototip.md)):

- **Kullanıcılar ve ağ:** iki gizli yerel kullanıcı vardı, ağsız ve ağlı. Ağsız
  kullanıcının giden trafiğini bir güvenlik duvarı kuralı engelliyordu.
- **Zincir:** `CreateProcessWithLogonW` → yardımcının `calistir` alt komutu →
  kısıtlı belirteçle `CreateProcessAsUserW`.
- **Kısıtlayıcı SID'ler:** çalışma alanı, ortak okuma, Everyone, Users ve
  RESTRICTED.

Ölçülen sınırları:

- **Okuma sınırı Dal B:** her yerel kullanıcının okuyabildiği dosyalar okunur.
- Schannel kısıtlı belirteçte `SEC_E_NO_CREDENTIALS` veriyor.
- Güvenlik duvarı geri döngüyü süzmüyor.
- Varsayılan izinli borular salt okunur açılıyor.
- Güvenlik duvarı kuralının kullanıcı koşulu, kısıtlı belirteçe ancak ortak
  okuma SID'ini de içerirse uygulanıyor.

AppContainer bir makinede kullanılamaz olursa bu yol yeniden değerlendirilir.
Örneğin kurum ilkesi AppContainer profili oluşturmayı engelleyebilir.

---

## 10. Arayüz

### 10.1 Araç kartı ve terminal görünümü

- Kabuk çağrısının kartı komutu tek satırda gösterir. Betikse ilk satır ve
  "…" gösterilir. Kartın özeti sonucun ilk satırıdır ("Çıkış kodu: 101").
- Durum simgesi: çalışıyor, başarılı, başarısız, iptal.
- **Terminal görünümü:**
  - Kartın altında, yüksekliği sınırlı ve kaydırılabilir bir kutudur. Kullanıcı
    en alttaysa yeni çıktıyla birlikte aşağı kayar.
  - stdout ve stderr aynı akışta çizilir; stderr satırları renkle ayrılır.
  - ANSI renk kaçışları güvenli bir alt kümeyle çizilir: yalnızca SGR (renk,
    kalın). İmleç hareketi ve başlık kaçışları atılır.
  - Çıktı metin düğümü olarak eklenir; HTML olarak yorumlanmaz.
  - `atlanan_bayt` görünürse "… N bayt gösterilmedi …" satırı eklenir.
- **Uzun çıktı:** görünüm en çok 5 000 satır ve 200 000 karakter tutar; fazlası
  baştan atılır ve bu söylenir. Tek satır 4 000 karakterde kesilir ve "…" ile
  gösterilir. Metin seçilip kopyalanabilir.
  - Neden karakter sınırı: çıktı seli ölçümünde Linux'taki webview (WebKitGTK,
    Xvfb, yazılım çizimi) terminaldeki metin 3 MB'a çıktığında saniyede yalnızca
    50–60 KB işleyebildi; tur 60 saniyede bitmedi. Sınırla aynı test 3,6
    saniyede bitti. Görünüm izlemek içindir; tam çıktı modele gider.
- **Kayıttan açılan oturumda** terminal görünümü `AracSonucu`'ndaki kırpılmış
  sonucu gösterir.
- **Konuşmanın alta yapışması** (2b'de bulunan hata): konuşma, kullanıcı yukarı
  kaydırmadıkça en altta kalır. Yapışma yalnızca kullanıcının kaydırma
  niyetinden (tekerlek, dokunarak sürükleme, kaydırma çubuğu, kaydırma
  tuşları) sonra bırakılır. Kendi kaydırmamızın geç gelen olayı, onay sayfası
  kapanınca odağın geri dönmesi ya da konuşmadaki bir düğmeye tıklamak
  bırakmaz. Hata 1c'den beri vardı; Linux'ta kabuk testleri yalıtımla koşunca
  görünür oldu (yeni onay kartı görünüm alanının altında kalıyordu).
  - Aynı hata 1c'nin Linux gecikme ölçümünü de etkiliyordu: konuşma akış
    başında en alttan kopuyor, akan metin görünüm alanının dışında kalıyordu.
    Düzeltmeden sonra metin görünür akar ve Xvfb'de p95 56–64 ms'ye çıktı.
    Neden arayüz değil birleştirici: WebKitGTK'nın GL birleştiricisi GPU'suz
    Xvfb'de yazılımla (llvmpipe) çalışıyor; görünür her değişiklikte bir
    sonraki kare ~25 ms gecikiyor (çizimin kendisi ~5 ms). Kaydırmayı 100
    ms'ye seyreltmek, `will-change` ya da `content-visibility`'yi kaldırmak
    değiştirmedi. Linux uçtan uca testleri bu yüzden
    `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile koşar (§12.5); ürün kodu
    değişmez.

### 10.2 Kabuk onay kartı

Onay sayfasında (1c §8.2) kabuk öğesi diff yerine şunları gösterir:

- Tam komut, eş aralıklı yazıyla. Betikse bütün metin ve yorumlayıcı.
- Çalışma dizini.
- Ağ: "Kapalı" ya da "Açık — komut internete erişebilecek".
- Yalıtım: "Yalıtımlı (bwrap)", "Yalıtımlı (Windows)" ya da "Yalıtımsız" ve
  nedeni.
- Uyarılar (§5.1, §4.2).
- `yerel_onay` öğesinde onayın yerel pencerede ayrıca sorulacağı notu (§5.2).

### 10.3 Durum çubuğu

- Yalıtım göstergesi eklenir (1c §8.3'te aşama 2'ye bırakılmıştı):
  - Yalıtımlı: yeşil.
  - Yalıtımsız: sarı.
  - Kurulum gerekli ya da bozuk: sarı.
- Üzerine gelince talimat (yalıtımlıysa arka uç) görünür. Tıklanınca Ayarlar
  açılır (§10.4).
- Gösterge uygulama düzeyindedir, oturuma bağlı değildir. Durum
  `uygulama_durumu`'nun `yalitim` alanıyla gelir. Ana pencere değişimi
  `yalitimi_dinle` ile verdiği kanaldan alır (§10.5).

### 10.4 Ayarlarda yalıtım bölümü

2b'de menüye "Ayarlar" eklenir; ekranın ilk bölümü yalıtımdır. 2b'de gelenler:
durum ve nedeni, talimat, "Yeniden dene" (Linux) ve izinli araç zinciri
dizinleri (Linux). Hazırlanmış çalışma alanları, önbellek silme, ek ortam
değişkenleri ve Windows düğmeleri 2d'dedir.

- Eklenen dizin ev dizinini ya da Kalfa'nın veri dizinini kapsayamaz, veri
  dizininin içinde de olamaz: sırlar ve oturum kayıtları yalıtıma açılırdı.
- Dizinler veri dizinindeki `yalitim.json`'dadır. Eklenince ya da çıkarılınca
  profil özeti değişir (§7.1).

- **Durum ve nedeni**, talimatıyla birlikte (`YalitimDurumu`).
- **Windows düğmeleri:** "Yalıtımı kur", "Yalıtımı kaldır". İkisi de önce yerel
  iletişim kutusunda ne yapılacağını söyler, sonra UAC'yi açar.
- **İzinli araç zinciri dizinleri:** liste, "Ekle" (yerel klasör seçici) ve
  "Kaldır". Windows'ta sahibi olunmayan dizin için UAC istenir.
- **Hazırlanmış çalışma alanları** (Windows) ve çalışma alanına özel dizinleri
  olanlar (iki platform). Her satırda:
  - Yol.
  - Hazırlık tarihi.
  - Gizli dosya sayısı.
  - "İzinleri geri al ve kaldır" düğmesi.
  - "Önbelleği sil" düğmesi.
- **Ek ortam değişkenleri:** izin listesine değişken ekleme (§6.1).

### 10.5 Komutlar ve yetkiler

Yeni komutlar yalnızca ana pencereye açılır (1c §6):

| Komut | İş |
|---|---|
| `uygulama_durumu` (mevcut) | `yalitim: YalitimDurumu` alanı eklenir (2a); `platform` (2b) |
| `yalitimi_dinle` | Ana pencere açılınca kanal verir; durum değişimi oraya yazılır (2b) |
| `yalitimi_algila` | Algılamayı yeniden çalıştırır, yeni durumu döndürür (2b) |
| `yalitimi_kur` / `yalitimi_kaldir` | Yerel onay → UAC → sonuç (Windows); Linux'ta talimat döndürür |
| `arac_zinciri_dizinleri` / `arac_zinciri_dizini_ekle` / `arac_zinciri_dizini_kaldir` | Ekleme yerel klasör seçiciyle yapılır; arayüz yol gönderemez |
| `yalitim_calisma_alanlari` / `yalitim_calisma_alanini_kaldir` | Kimlikle çalışır (1c'deki gibi arayüz yol göndermez) |

Durum değişimi, ana pencerenin `yalitimi_dinle` ile verdiği kanala yazılır
(`oturumu_dinle` gibi); kanal verilince şimdiki durum hemen yazılır. Tauri
olayı kullanılmaz: olay dinlemek, pencereye Tauri'nin çekirdek izinlerini
açmayı gerektirirdi (1c §6).
`arac_onayi`'nın davranışı §5.2'de.

### 10.6 Dil

Metinler 1c §9.4'teki kurallarla Türkçe yazılır. Yeni terimler:

| Terim | Anlamı |
|---|---|
| yalıtım / yalıtımlı / yalıtımsız | sandbox |
| çalışma alanını hazırlamak | Windows'ta ACL'leri kurmak |
| ağlı / ağsız | ağ erişimi açık / kapalı |
| yerel pencere | yerel iletişim kutusu |

"Sandbox" arayüzde kullanılmaz.

---

## 11. Hata durumları

| Durum | Davranış |
|---|---|
| Program bulunamadı | `Basarisiz`; "`xyz` bulunamadı. Yalıtımın içindeki PATH: …" modele gider |
| Yalıtım başlatılamadı (bwrap hatası, AppContainer başlatma hatası) | `Basarisiz`; algılama yeniden çalışır. Durum değiştiyse gösterge güncellenir. Komut yalıtımsız yeniden **denenmez** |
| Windows'ta kök izni yok ya da AppContainer profili oluşturulamıyor | Durum `KurulumGerekli` ya da `KurulumBozuk`; talimat "Yalıtımı kur" |
| Hazırlık yarıda kaldı (iptal, çökme) | ACE kaydı o ana kadar eklenenleri tutar. Sonraki çağrı hazırlığı baştan yapar; yinelenen girdi eklenmez |
| Gizli dosya taraması süreyi aşıyor | Tarama iptal edilmez. Süre ölçümü komutla başlar (§6.2); araç kartında "Gizli dosyalar taranıyor…" görünür |
| Çıktı sınırı aşıldı | Kırpılır; modele söylenir (§4.4) |
| Canlı akış yetişemiyor | Parçalar atılır, `atlanan_bayt` bildirilir; tur iptal edilmez (§4.4) |
| Süreç sayısı sınırı | Komutun kendi hatası olarak görünür (`fork: Resource temporarily unavailable`) |
| Onaydan sonra yalıtım durumu değişti | Mühür tutmaz; `PolitikaDegisti`, yeni onay (§3.3) |
| Kalfa kabuk çalışırken çöktü | Ağaç ölür (§6.4). Kurtarmada çağrı `SonucuBilinmiyor` olur (§4.5) |

---

## 12. Test stratejisi

Üst belge §13.2: "Test edilmeyen güvenlik sınırı yoktur."

### 12.1 Çekirdek ve araç (yalıtımsız, `cargo test`)

- **İzin tablosu (§5.1):** her kip × ağ × yalıtım hücresi. İlk çağrı kuralı
  ayrıca sınanır.
- **Mühür:**
  - Komut, dizin, ağ, yalıtım durumu ya da profil özeti değişirse onay
    geçersizleşir.
  - Yalıtım durumu hazırlıkla yürütme arasında değişirse `PolitikaDegisti`
    döner.
- **Yerel onay yolu:**
  - Webview'dan gelen onay yerel öğeyi onaylamaz.
  - Sahte iletişim kutusu "Vazgeç" derse öğe reddedilir.
  - Karışık onaylarda öğeler doğru ayrılır.
- **Çıktı:**
  - Baş ve son kırpma, UTF-8 sınırı, geçersiz bayt ve ANSI temizleme.
  - Kayıplı gönderici: yavaş tüketicide tur iptal edilmez, `atlanan_bayt`
    doğru sayılır, model çıktısı eksiksizdir.
- **Birleştirme:** `AracCiktisiParcasi` yalnızca aynı çağrı ve akışta birleşir;
  araya giren olay birleştirmeyi keser.
- **Kurtarma:** başlamış, sonucu yazılmamış kabuk çağrısı `SonucuBilinmiyor`
  olur ve yeniden çalışmaz.
- **Git yerleşimi (§7.2):** `.git` korumalıdır; geri başvurusu tutmayan
  `gitdir:` ya da `commondir` dış dizin açtırmaz; worktree ve alt modül
  doğrulanır.
- **Çalışma alanına özel dizinler (§7.3):** yalıtımın kurduğu bağlantılar
  (Unix'te sembolik bağlantı, Windows'ta junction) üzerinden Kalfa dışarıya
  yazmaz.
- **Algılama:** sahte algılayıcıyla her `YalitimYoklukNedeni` ve talimatı.
- **Ortam:** izin listesi dışındaki, özellikle adında `KEY` ya da `TOKEN` geçen
  değişkenler komuta ulaşmaz. Bu, yalıtımsız arka uçla gerçek süreçte sınanır.
- **Süreç ağacı:** yalıtımsız arka uçta süre ve iptal, torun süreçleri de
  durdurur (Windows'ta Job Object).

### 12.2 Yalıtım (`kalfa-sandbox`, gerçek yalıtımla)

Testler özellik bayrağının arkasındadır (`--features yalitim-testleri`);
yalıtım yoksa atlanmaz, başarısız olur. Linux'ta WSL2'de sıradan bir
kullanıcıyla, Windows'ta kurulumdan sonra koşar.

| Test | Linux | Windows |
|---|---|---|
| `~/.ssh/id_rsa` ve `~/.cargo/credentials.toml` okunamaz | ✓ | ✓ |
| Seçilmeyen yollar (`/srv`, `/mnt`, `C:\projeler`, başka bir çalışma alanı) görünmez ya da okunamaz | ✓ | ✓ |
| Aynı anda açık iki çalışma alanı birbirini okuyamaz ve yazamaz | ✓ | ✓ |
| Çalışma alanı dışına yazılamaz | ✓ | ✓ |
| `.git` (dizin ya da worktree dosyası) ve `core.hooksPath` hedefi yazılamaz, silinemez, yeniden adlandırılamaz; içlerine yeni dosya konamaz | ✓ | ✓ |
| Çalışma alanındaki `.env` ve onun hard link'i okunamaz | ✓ | ✓ |
| Ağsız profilde TCP bağlantısı ve Unix soketi (soyut dahil) açılamaz | ✓ | ✓ (TCP) |
| Ağlı profilde TCP açılır | ✓ | ✓ |
| Ağlı profilde Unix soketi açılamaz | ✓ | — |
| Seccomp engelleri (`io_uring_setup`, `keyctl`, `bpf`, `ptrace`, `unshare`) | ✓ | — |
| İptal süreç ağacını durdurur; torun süreç ve `setsid` kaçağı dahil | ✓ | ✓ |
| Kalfa'nın ölümü ağacı durdurur | ✓ | ✓ |
| Süreç sayısı sınırı | ✓ | ✓ |
| Yalıtılmış süreç panoyu okuyamaz, geliştiricinin penceresine ileti gönderemez | — | ✓ |
| Kaldırılan çalışma alanının ACE'leri ve mirası kesilen DACL'leri geri alınır, profili silinir | — | ✓ |
| `kalfa-yalitim.exe` yalıtımın yazabildiği bir yerden başlatılmaz | — | ✓ |
| Kabın içinden başka bir çalışma alanının paket SID'iyle süreç başlatılamaz | — | ✓ |
| Geri döngü (127.0.0.1) ağsız ve ağlı profilde açılamaz | — | ✓ |
| Sahte `bwrap` çalışma alanında olsa da seçilmez | ✓ | — |

2b'de Linux sütunu `crates/kalfa-sandbox/tests/yalitim.rs`'tedir; testler
WSL2'de sıradan kullanıcı `kalfatest` ile koşar. "Kalfa'nın ölümü" testi, test
ikilisini yardımcı süreç olarak başlatıp `SIGKILL` ile öldürür. Seccomp
engelleri `perl`'ün `syscall`'ıyla sınanır (`perl-base` Debian ve Ubuntu'da her
sistemde vardır).

### 12.3 Araç zinciri uyumluluğu

Yalıtım içinde, iki platformda, bu depoda ve `kalfa-sandbox/tests/ornekler/`
altındaki küçük projelerde:

- `git status`, `git diff`, `git log` (worktree'de dahil); `git add` ve
  `git commit` yalıtımda çalışmaz (§7.2).
- `cargo build`, `cargo test` (önceden indirilmiş crate'lerle, ağsız).
- `npm ci` (ağlı), `npm test` (ağsız).
- `python -m pytest` ya da `python -c` (ağsız).

Çalışmayan her durum §14'e gerekçesiyle yazılır. Profil yalnızca açık bir
kararla genişletilir.

2b (Linux, WSL2): `crates/kalfa-tools/tests/uyumluluk.rs` kabuk aracının bütün
yolundan (tarama, git yerleşimi, bwrap) geçer:

- git: worktree'de `status`, `diff`, `log` çalışır; `add` reddedilir ve ana
  deponun dizini değişmez (2026-09-26'dan beri; önce commit de çalışıyordu).
- cargo: örnek crate, önceden indirilmiş `hex` ile `cargo test --offline`.
- npm: `npm ci` (ağlı) ve `npm test` (ağsız).
- python: `python3 -c` (`json`, `sqlite3`, `ssl`).

Hiçbiri `/sys`'e ihtiyaç duymadı (A4). Rustup vekili
`~/.rustup/update-hashes`'i istedi; `~/.rustup`'ın tamamı açıldı (§7.1).

### 12.4 Arayüz (Vitest)

- Terminal görünümünün durumu: parça ekleme, satır başıyla üzerine yazma,
  5 000 satır ve 200 000 karakter sınırı, satır kesme, atlanan bayt satırı.
- ANSI alt kümesi: SGR dışındaki kaçışların atılması; HTML olarak
  yorumlanmama.
- Kabuk onay kartı: komut, dizin, ağ, yalıtım ve `yerel_onay` notu.
- Durum çubuğu göstergesi (sahte arka uçla); yalıtım ayarları bölümü 2b/2d'de.

### 12.5 Uçtan uca (1c-4 düzeni)

Sahte EVREN'e kabuk çağrısı yapan senaryolar eklenir. Komut gerçek yalıtımla
çalışır; `KALFA_E2E` ayarı yalıtımı zorlamaz.

- Onay kartı → onay → canlı çıktı → sonuç.
- Uzun süren komutun iptali; süreç ağacının durduğu süreç listesinden doğrulanır.
- Çıktı seli (`yes` ya da eşdeğeri, süre sınırıyla): arayüz yanıt vermeye devam
  eder, tur iptal edilmez.
- 2a'da yalıtım yoktur; her senaryo yalıtımsız çalışır ve sahte yerel iletişim
  kutusu (`onay: true`) onaylar. Komutlar `node` ile yazılır; iki platformda
  aynı çalışır.
- Linux'ta uygulama `WEBKIT_DISABLE_COMPOSITING_MODE=1` ile açılır: Xvfb'de
  GPU yoktur ve yazılım GL birleştiricisi gecikme ölçümünü arayüzün değil
  kendi kare hızının ölçümüne çevirir (§10.1).

### 12.6 Doğrulama komutları

1c §11.6'dakilere ek olarak:

```
cargo test -p kalfa-sandbox --features yalitim-testleri   # Linux: sıradan kullanıcıyla
cargo test -p kalfa-sandbox --features yalitim-testleri   # Windows: kurulumdan sonra
```

### 12.7 Elle kontrol

- Windows'ta UAC akışı: kur, kaldır, yeniden kur.
- Durum çubuğu ve ayarlar bölümünün tasarım sistemine uygunluğu.
- Kaldırmadan sonra makinede Kalfa'ya ait AppContainer profili, kök izni ya
  da ACE kalmadığı ve kök DACL'inin `AI` bayrağının korunduğu: durum ve bir
  ACL taraması.

---

## 13. Üst belgede değişenler

Bu belge commit'lenirken üst belgeye işlenenler:

- **§7.7 Windows arka ucu** (2c'den sonra, üst belge r3.5):
  - Arka uç AppContainer + ACL'dir; ayrı kullanıcı ve kısıtlı belirteç yolu
    bırakıldı (§9.5, §9.7).
  - Kurulumu MSI'dan önce `kalfa-yalitim.exe` yapar (kök izni, sahibi olunmayan
    araç zinciri dizinleri); uygulama onu UAC ile başlatır.
  - Komutu Kalfa doğrudan kapta başlatır (§9.3).
  - Yardımcı dosya yalıtımın yazamadığı bir yerden başlatılır (§9.1).
- **§7.7 Linux arka ucu:**
  - Ağlı profilde de `AF_UNIX` soket açmak engellenir (§8.2).
  - `.git` bütünüyle salt okunurdur; çalışma alanının git dizini dışarıdaysa
    doğrulanır ve salt okunur bağlanır (§7.2, §7.4; r3.6).
  - Hard link'li gizli dosyalar da örtülür (§7.2).
- **§7.7 Süreç sınırları:** değerler: 120/1800 sn, 30 000 bayt, 256 süreç.
  Canlı çıktı kayıplıdır (§4.4).
- **§12.3:** aşama 2, dört plana bölünür (§15).

---

## 14. Açık noktalar

Plan ya da prototip aşamasında cevaplanacaklar:

| # | Soru | Nerede ve sonuç |
|---|---|---|
| A1 | `RLIMIT_NPROC` kullanıcı ad alanında yalnızca içerideki süreçleri mi sayıyor? | 2b: evet, sınır içeride (`prlimit`) konunca (§6.3) |
| A2 | Salt okunur bağlanan cargo registry'siyle ağsız derleme çalışıyor mu; çalışmıyorsa kopyanın biçimi ne? | 2b: çalışıyor; yeni crate ağlı çalıştırmada çalışma alanının kendi registry'sine iner (§7.4) |
| A3 | Gizli dosya taraması 100 000 dosyada 200 ms'nin altında mı? | 2b: evet, 60–63 ms (§7.2) |
| A4 | Hangi araçlar `/sys`'e ihtiyaç duyuyor? | 2b: git, cargo, npm, node ve python duymadı (§12.3) |
| A5 | P1–P10 | 2c: kısıtlı belirteçle Dal B, AppContainer'la Dal A (§9.5) |
| A6 | Uzun ömürlü `calistir` aracısı gerekli mi (P6)? | 2c: hayır; AppContainer'da aracı süreç yok, çağrı başına 22–29 ms (§9.3) |
| A7 | Windows'ta çalışma alanı SID'inin biçimi | 2c: AppContainer profilinin paket SID'i, `Kalfa.Alan.<kimlik>` adından (§9.3) |
| A8 | Schannel kısıtlı belirteçte `SEC_E_NO_CREDENTIALS` veriyor; ağlı çalıştırmada cargo, rustup ve Schannel'li git bağlanamıyor. Yol ne (OpenSSL arka ucu, geliştiricinin önbelleği, AppContainer)? | 2c ek ölçümü: AppContainer'da Schannel çalışıyor (§9.3) |
| A9 | Geri döngü güvenlik duvarında süzülmüyor; ağsız kullanıcı 127.0.0.1'e ulaşıyor. WFP filtresi mümkün mü? | 2c ek ölçümü: AppContainer'da geri döngü kapalı (§9.3) |
| A10 | Varsayılan izinli borular Everyone üzerinden salt okunur açılıyor; kabul mü, önlem mi? | 2c ek ölçümü: AppContainer'da açılamıyor (§9.3) |
| A11 | AppContainer'da çalışma alanının üst dizinleri listelenebilmeli mi? Profil altındaki çalışma alanında üst dizinlere listeleme izni mi, sürücü harfi mi? | 2d planı |
| A12 | AppContainer'da C derleyen `build.rs` (`cl.exe`, `INCLUDE`) çalışıyor mu? | 2d planı |
| A13 | AppContainer'da `privateNetworkClientServer` ile LAN hedefi; `openssh-ssh-agent` ve `docker_engine` boruları | 2d planı |
| A14 | Kabın içinden başka bir paket SID'iyle süreç başlatılabiliyor mu? | 2c: hayır, `ERROR_INVALID_PARAMETER`; aynı paketle başlatılabiliyor (§9.3) |
| A15 | Windows'ta ağsız derlemede geliştiricinin cargo registry'si nasıl kullanılır? `CARGO_HOME`'u doğrudan vermek çalıştı ama 12 sn sürdü | 2d planı (§7.4) |

---

## 15. Uygulama planlarına geçiş

Planlar 1c'deki gibi PR ile yürütülür: plan ve PR'ı Claude yazar, kodu Codex
yazar, Claude inceler ve iki platformda doğrular.

1. **2a — Kabuk aracı ve süreç sınırları.** Yalıtımsız arka uç iki platformda.
   Bu planın sonunda kabuk kullanılabilir; her çağrı yerel onaylıdır.
   - `kalfa-sandbox` iskeleti: `Yalitici`, `Surec`, ortam, süre, iptal, süreç
     ağacı.
   - `kabuk` aracı ve `Arac::calistir` imza değişikliği.
   - Protokol eklemeleri, izin tablosu, yerel onay yolu.
   - Canlı çıktı ve terminal görünümü, kabuk onay kartı, durum çubuğu
     göstergesi (her zaman "Yalıtımsız"), uçtan uca testler.
2. **2b — Linux arka ucu.**
   - Profil, gizli dosya taraması, çalışma alanına özel dizinler, araç
     yapılandırması.
   - bwrap argümanları, seccomp, algılama.
   - §12.2'nin Linux sütunu ve §12.3 (Linux).
   - WSL'de test kullanıcısı (izinle).
3. **2c — Windows prototipi.** Ürün kodu yoktur. Prototip betikleri ve bulgu
   raporu hazırlanır, bu belge güncellenir (§9.5). Yönetici adımlarını
   kullanıcı çalıştırır.
4. **2d — Windows arka ucu.** AppContainer'a göre (§9):
   - `kalfa-yalitim.exe`: kök izni, araç zinciri izinleri, kaldırma.
   - `kalfa-sandbox`'ta AppContainer zinciri ve durum denetimi.
   - Çalışma alanı hazırlığı (profil, izinler, mirası kesilen korumalar) ve
     ACE kaydı.
   - Ayarlardaki yalıtım bölümü.
   - §12.2'nin Windows sütunu ve §12.3 (Windows).

2a ve 2b iki platformun ortak kodunu değiştirdiği için sırayla yürür. 2c, 2a ve
2b'yle paralel yürüyebilir: ürün koduna dokunmaz.
