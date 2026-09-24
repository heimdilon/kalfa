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
5. **Windows arka ucu:** önce bir prototip, sonra ayrı yerel kullanıcı +
   kısıtlı belirteç + ACL + güvenlik duvarı. Kurulumu yapan yönetici yardımcısı
   `kalfa-yalitim.exe` (§9).
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
   Windows'un okuma sınırı satırı prototipin sonucuna bağlıdır (§9.5).
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
      windows/        çalıştırma zinciri, ACL, kurulum durumu (§9)
      yalitimsiz.rs   yalıtımsız arka uç: yalnızca süreç sınırları (§6.5)
  kalfa-yalitim/     yeni, yalnızca Windows'ta işlev görür: kur, kaldir,
                     durum, izin-ver, calistir (§9.1)
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

### 6.2 Süre

- Varsayılan 120 sn, en çok 1800 sn; model `zaman_asimi_sn` ile ister.
- Süre dolunca süreç ağacı sonlandırılır (§6.4) ve sonuç zaman aşımını söyler
  (§4.5).
- Süre, komutun başlatıldığı andan ölçülür. Onay beklemesi ve Windows'taki
  hazırlık (§9.4) süreye dahil değildir.

### 6.3 Süreç sayısı

- Sınır 256 süreçtir.
- **Windows:** Job Object'in `ActiveProcessLimit` değeri.
- **Linux:** `RLIMIT_NPROC`. Çekirdek 5.14'ten beri bu sınır kullanıcı ad alanı
  başına sayılır. Bu yüzden yalıtımın içindeki süreçleri sınırlar ve
  kullanıcının diğer süreçlerini saymaz. 2b planı bunu bir testle doğrular.
  Doğrulanamazsa `systemd-run --user --scope -p TasksMax=256` kullanılır;
  systemd kullanıcı oturumu yoksa sınır belgeye ve arayüze "uygulanamadı" diye
  yazılır.
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
    - Linux: `~/.cargo/bin`, `~/.cargo/registry`, `~/.rustup/toolchains`,
      `~/.rustup/settings.toml`.
    - Windows: bunların `%USERPROFILE%` karşılıkları.
  - Kullanıcı ayarlardan dizin ekleyebilir, örneğin `/opt/node` ya da
    `C:\Python314`.
  - Depo içindeki hiçbir dosya profili genişletemez (üst belge §7.2).
- **Profil özeti** (mühre giren) okunur yolların, gizli ve korumalı yolların ve
  ağ bayrağının sıralı listesinin SHA-256'sıdır. Kullanıcı bir dizin eklerse
  bekleyen onaylar geçersizleşir.

### 7.2 Gizli dosya ve korumalı yol taraması

Gizli ve korumalı yollar her çalıştırmadan hemen önce taranır.

- **Kapsam:**
  - Bütün çalışma alanı, `.gitignore` kapsamındakiler dahil. `.env` genellikle
    yok sayılan bir dosyadır.
  - Sembolik bağlantılar izlenmez.
  - Kalıplar üst belge §7.4'teki listedir; kullanıcının eklediği kalıplar
    dahildir (`Siniflandirici`).
- **Korumalı yollar** git koruma çözücüsünden (`git_koruma.rs`) gelir:
  `.git/hooks`, `.git/config` ve `core.hooksPath`'in etkin değeri. `.git` bir
  dosyaysa gerçek git dizini çözülür.
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
  100 000 dosyalık bir ağaçta 200 ms'nin altıdır; 2b planı bunu ölçer. Hedef
  tutmazsa dizin değişiklik zamanına dayalı bir önbellek eklenir.
- **Liste çok uzunsa** (2 000'den fazla yol) Linux'ta bwrap argümanları
  `--args FD` ile verilir. Windows'ta her yol için bir ACE konur ve bu
  hazırlığın ilerlemesinde görünür.

### 7.3 Çalışma alanına özel dizinler

- Her çalışma alanı bir kimlik alır: kök dizinin dosya sistemi kimliği ve
  kanonik yolunun SHA-256'sından türetilen 16 onaltılık karakter. Kilit
  yöneticisinin kök kimliğiyle aynı kaynaktır (1c §5.3).
- Dizinlerin konumu:
  - Linux: `$XDG_DATA_HOME/kalfa/yalitim/<kimlik>/`.
  - Windows: `%LOCALAPPDATA%\Kalfa\yalitim\<kimlik>\`.
- İçindeki dizinler:
  - `ev/`
  - `gecici/`
  - `onbellek/`, bunun altında `cargo/`, `npm/`, `pip/`.
- **Ortak yazılabilir durum yoktur** (üst belge §7.7 ilke 4). Ağlı çalıştırmada
  indirilen paketler bu önbelleğe yazılır.
- **Temizlik:** ayarlardaki yalıtım bölümü bir çalışma alanının dizinlerini
  silebilir (§10.4). Kalfa kendiliğinden silmez.

### 7.4 Araç yapılandırması

Ev dizinindeki yapılandırma dosyaları yalıtıma açılmaz (üst belge §7.7 ilke 2).
Kalfa her çalıştırmadan önce çalışma alanına özel en küçük yapılandırmayı üretir.

- **Git:**
  - `ev/.gitconfig` yalnızca `user.name` ve `user.email`'i taşır. Değerler
    kullanıcının etkin git yapılandırmasından okunur; `GIT_CONFIG_GLOBAL` bu
    dosyayı gösterir.
  - Sistem yapılandırması (`/etc/gitconfig`) okunur yollar arasındadır ve
    olduğu gibi kalır.
  - Çalışma alanının git dizini çalışma alanı dışındaysa (worktree ya da
    submodule), çözülen `gitdir` ve `commondir` yazılabilir bağlanır.
    Korumaları aynen uygulanır. Böylece bu depodaki `.worktrees/` gibi
    worktree'lerde `git commit` çalışır. Bu, "çalışma alanı dışına yazılamaz"
    kuralının tek ve bilinçli istisnasıdır: yazılan yer çalışma alanının kendi
    deposudur.
- **Cargo:**
  - `CARGO_HOME` çalışma alanına özeldir (`onbellek/cargo`).
  - Kullanıcının `~/.cargo/bin`'i salt okunur bağlanır ve `PATH`'e eklenir.
  - Kullanıcının `~/.cargo/registry`'si salt okunur olarak
    `onbellek/cargo/registry` konumuna bağlanır. Böylece daha önce indirilmiş
    crate'ler ağsız derlemede kullanılır.
  - `credentials.toml` ve `config.toml` hiç görünmez.
  - Bilinen risk: registry salt okunur olduğu için cargo yeni bir crate'i
    açamaz. Böyle bir derleme, ağlı ve onaylı bir çalıştırma ister. Bu durumda
    registry'nin çalışma alanına özel bir kopyası kullanılır. bwrap 0.9.0'da
    `--overlay` yoktur (WSL'deki Ubuntu 24.04'te doğrulandı); kopyanın biçimi
    2b planında ölçülür.
  - Windows'ta registry bağlanamaz: aynı dizini iki yerde göstermenin yolu
    yoktur. Çalışma alanına özel registry boş başlar ve ilk derleme ağlı bir
    çalıştırma ister. Prototip bunu ölçer (§9.5).
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
  --ro-bind <okunur yol> <aynı yol> ...      sistem ve araç zinciri
  --proc /proc --dev /dev --tmpfs /run
  --bind <çalışma alanı> <aynı yol>
  --bind <ozel/ev> <ozel/ev>                 HOME
  --bind <ozel/gecici> /tmp
  --bind <ozel/onbellek> <ozel/onbellek>
  --bind <gitdir> <aynı yol> ...             (§7.4)
  --ro-bind <korumalı yol> <aynı yol> ...    çalışma alanının üstüne
  --ro-bind /dev/null <gizli dosya> ...      gizli dosyalar
  --tmpfs <gizli dizin> ...                  gizli kalıba uyan dizinler
  --setenv K V ...                           (§6.1)
  --chdir <dizin>
  --seccomp <FD>
  -- <program> <argümanlar>
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

### 8.2 seccomp

- Filtre `seccompiler` crate'iyle üretilir. Saf Rust'tır, C bağımlılığı yoktur;
  x86_64 ve aarch64'ü destekler. Derlenmiş BPF bir boru üzerinden bwrap'a
  (`--seccomp FD`) verilir.
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
- **Ağsız profilde:** `socket` çağrısı adres ailesine göre engellenir.
  - Engellenen aileler: `AF_INET`, `AF_INET6`, `AF_UNIX`, `AF_NETLINK`
    (`NETLINK_ROUTE` hariç), `AF_PACKET`. Ağ ad alanı zaten ayrıdır; filtre
    derinlemesine savunmadır. Asıl işi `docker.sock` ya da `ssh-agent` gibi yerel
    ayrıcalıklı soketlere ulaşmayı engellemektir. Bu soketler zaten bağlanmaz,
    ama soyut Unix soketleri dosya sisteminden bağımsızdır.
  - `socketpair(AF_UNIX)` serbesttir.
- **Ağlı profilde:** `AF_INET` ve `AF_INET6` serbesttir. `AF_UNIX` soket açmak
  yine engellenir; `socketpair` serbesttir. Ağ ad alanı ayrılmadığı için soyut
  Unix soketleri (X11, D-Bus) ancak böyle kapatılır.
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

### 9.1 `kalfa-yalitim.exe`

Tek bir çalıştırılabilir dosya, beş alt komut:

| Alt komut | Kim çalıştırır | İş |
|---|---|---|
| `kur` | Yönetici (UAC) | Yalıtım grubunu ve iki kullanıcıyı oluşturur, parolaları ayarlar, güvenlik duvarı kuralını ekler, kurulum kaydını yazar |
| `kaldir` | Yönetici (UAC) | Kurulum kaydına göre her şeyi geri alır |
| `izin-ver` / `izin-geri-al` | Yönetici (UAC) | Geliştiricinin sahibi olmadığı bir araç zinciri dizinine okuma izni verir ya da geri alır (§9.4) |
| `durum` | Geliştirici | Kurulumun eksiksiz olup olmadığını makine tarafından okunur biçimde bildirir |
| `calistir` | Yalıtım kullanıcısı | Kısıtlı belirteci türetir, komutu Job Object içinde başlatır (§9.3) |

**Hangi dosya çalıştırılır.** Kalfa, `kalfa-yalitim.exe`'yi yalıtımın
yazabileceği bir yerden hiçbir zaman başlatmaz. Başlatsaydı yalıtılmış bir komut
dosyayı değiştirip bir sonraki UAC yükseltmesinde yönetici yetkisi, bir sonraki
`calistir`'da kısıtsız belirteç elde ederdi.

- Açılışta Kalfa, kendi çalıştırılabilir dosyasının yanındaki
  `kalfa-yalitim.exe`'yi `%LOCALAPPDATA%\Kalfa\bin\` dizinine kopyalar. Kopyanın
  SHA-256'sı kaynağınkiyle eşleşmelidir.
- Bütün çağrılar o kopyayı kullanır.
- Yalıtım kullanıcıları bu dizine yazamaz; dizin geliştiricinin profilindedir.
- Kopyalamadan önce kaynak dosyanın hazırlanmış bir çalışma alanının içinde
  olmadığı denetlenir. Geliştirme derlemesinde `target/` genellikle bu
  depodadır. Bu depo hazırlanmış bir çalışma alanıysa kopyalama reddedilir ve
  yalıtım durumu `KurulumBozuk` olur. Talimat `CARGO_TARGET_DIR`'i çalışma alanı
  dışına almayı söyler.
- Kurulu sürümde kaynak Program Files'tadır ve bu denetim kendiliğinden geçer.

### 9.2 Kurulum ve kaldırma

- **Neler oluşturulur:**
  - Yerel grup `KalfaYalitim`.
  - İki yerel kullanıcı:
    - `KalfaAgsiz`: bütün giden trafiği engellenen kullanıcı.
    - `KalfaAgli`: ağlı çalıştırmalar için.

  İki kullanıcı da yalnızca `KalfaYalitim` grubundadır, `Users` grubundan
  çıkarılır. Giriş ekranında gizlenir (`SpecialAccounts\UserList`); ağdan ve
  uzak masaüstünden oturum açma hakları reddedilir.
  - `Users` grubundan çıkarmanın sistem dizinlerini okumayı bozup bozmadığını
    prototip ölçer (§9.5). Bozuyorsa grup üyeliği korunur. Okuma sınırı zaten
    kısıtlayıcı SID'lerle çizilir.
- **Güvenlik duvarı:** `KalfaAgsiz`'ın SID'i için bütün programları kapsayan bir
  giden engelleme kuralı (`LocalUserAuthorizedList`, Windows 8'den beri).
  Kuralın geri döngü (loopback) trafiğini engelleyip engellemediği prototipte
  ölçülür (§9.5).
- **Parolalar:**
  - Uygulama (yükseltilmemiş) iki rastgele parola üretir: 32 bayt, base64.
  - Parolaları geliştiricinin DPAPI'siyle (`CryptProtectData`, kullanıcı
    kapsamı) şifreleyip `%LOCALAPPDATA%\Kalfa\yalitim\parolalar.bin`'e yazar.
  - Yükseltilmiş `kur` parolaları uygulamadan adı rastgele bir adlandırılmış
    boruyla alır. Borunun DACL'i geliştiriciye ve Administrators'a izin verir.
  - Parolalar komut satırında taşınmaz.
  - Yönetici hesabı geliştiriciden farklı olabilir ("başka bir yöneticinin
    parolasıyla yükseltme"). Parolaları uygulamanın üretip saklaması bu yüzden
    gerekir; yönetici tarafında saklanan bir DPAPI kaydını geliştirici
    çözemezdi.
- **Kurulum kaydı:** `%ProgramData%\Kalfa\yalitim\kurulum.json`. Oluşturulan
  kullanıcıları, grubu, kuralı ve `izin-ver` ile verilen ACE'leri listeler.
  Dosyayı yalnızca Administrators yazabilir.
- **Kaldırma sırası:**
  1. Uygulama, geliştiricinin kendi ACE kaydındaki bütün izinleri
     yükseltilmeden geri alır (§9.4).
  2. `kaldir` UAC ile başlar. Kurulum kaydına göre ACE'leri, kuralı,
     kullanıcıları (profilleriyle), grubu ve kaydı siler.
  3. Uygulama parola dosyasını siler.
- **Tek geliştirici varsayımı.** Aynı makinede ikinci bir geliştirici hesabı
  `kur`'u yeniden çalıştırırsa parolalar değişir ve ilk hesabın parola dosyası
  geçersiz kalır. Durum `KurulumBozuk` olur ve talimat bunu söyler. Çok
  geliştiricili makineler kapsam dışıdır (§1.2).
- **`durum`**, yükseltme olmadan şunları denetler:
  - Grup ve kullanıcılar var mı.
  - Güvenlik duvarı kuralı var mı.
  - Parola dosyası çözülebiliyor mu.
  - Deneme amaçlı bir `calistir` (`cmd /c exit 0`) başarılı mı.

  Sonuç `Hazir`, `KurulumGerekli` ya da `KurulumBozuk`'tur; bozuksa neyin eksik
  olduğu yazılır.

### 9.3 Çalıştırma zinciri

```
Kalfa (geliştirici, orta bütünlük)
  │  CreateProcessWithLogonW(KalfaAgsiz | KalfaAgli, parola,
  │      "…\Kalfa\bin\kalfa-yalitim.exe calistir --boru <ad>")
  │  std tutamaçları: Kalfa'nın açtığı borular (STARTF_USESTDHANDLES)
  ▼
kalfa-yalitim calistir (yalıtım kullanıcısı)
  │  istek boru üzerinden gelir: program/argv, dizin, ortam, kısıtlayıcı SID'ler, sınırlar
  │  CreateRestrictedToken(kendi birincil belirteci,
  │      DISABLE_MAX_PRIVILEGE, kısıtlayıcı SID'ler = [çalışma alanı SID'i,
  │      ortak okuma SID'i, (varsayım) ALL APPLICATION PACKAGES, …])
  │  Job Object: KILL_ON_JOB_CLOSE, ActiveProcessLimit=256, UI kısıtları
  │  CreateProcessAsUserW(kısıtlı belirteç, CREATE_SUSPENDED) → işe ata → sürdür
  ▼
komut (yalıtım kullanıcısı, kısıtlı belirteç)
```

- **Ayrıcalık gerekmez.** `CreateProcessAsUserW`, belirteç çağıranın birincil
  belirtecinin kısıtlı bir türeviyse `SE_ASSIGNPRIMARYTOKEN` ayrıcalığı istemez.
  Chromium'un sandbox'ı aynı yolu kullanır. Tek yönetici işi kurulumdur.
- **Kısıtlayıcı SID'ler:**
  - Çalışma alanı SID'i: çalışma alanına özel, rastgele üretilir. Biçimi
    prototipte kesinleşir.
  - Ortak okuma SID'i: izinli araç zinciri dizinlerine okuma iznini taşır.
  - Sistem dizinlerini okumak için gereken SID'ler.

  Kesin küme prototipin çıktısıdır (§9.5).
- **Erişim iki kez denetlenir.** Kısıtlı belirteçte Windows hem kullanıcı
  SID'lerini hem kısıtlayıcı SID'leri ayrı ayrı denetler; ikisinin de izni
  gerekir (üst belge §7.7). Çalışma alanının ACL'i bu yüzden iki izin taşır:
  - `KalfaYalitim` grubuna okuma-yazma.
  - O çalışma alanının SID'ine okuma-yazma.

  A için çalışan bir süreç B'nin SID'ini taşımadığı için B'ye erişemez.
- **UI kısıtları:** Job Object'e bütün `JOB_OBJECT_UILIMIT_*` bayrakları
  verilir: pano, genel atomlar, masaüstü, sistem parametreleri, çıkış ve
  kullanıcı nesnesi tutamaçları. Yalıtılmış süreç geliştiricinin pencerelerine
  ileti gönderemez ve panoyu okuyamaz. Ayrı bir masaüstünün gerekip gerekmediği
  prototipte ölçülür.
- **Çıktı:** Kalfa'nın açtığı borular `calistir`'a, oradan komuta geçer.
  `calistir` çıktıya dokunmaz.
- **İptal:**
  - Kalfa istek borusuna "sonlandır" yazar; `calistir` işi sonlandırır.
  - Boru kapanırsa (Kalfa çöktüyse) `calistir` işi yine sonlandırır ve çıkar.
  - Ayrıca Kalfa, `calistir` sürecini kendi Job Object'ine atar
    (`KILL_ON_JOB_CLOSE`); Kalfa'nın son tutamacı kapanınca zincir ölür.
    `CreateProcessWithLogonW`'nin döndürdüğü tutamaçla bunun yapılıp
    yapılamadığı prototipte doğrulanır. Yapılamazsa boru yolu tek başına
    yeterli sayılır ve bu belgelenir.
- **Başlatma maliyeti.**
  - `CreateProcessWithLogonW` her çağrıda bir oturum açar. İlk çağrıda profil
    oluşturulduğu için yavaştır.
  - Ölçülen gecikme 500 ms'den büyükse, çalışma alanı başına uzun ömürlü bir
    `calistir` aracısı değerlendirilir. Aracı oturum boyunca açık kalır ve her
    komut için yalnızca `CreateProcessAsUserW` yapar. Karar prototipin
    ölçümüyle verilir.

### 9.4 Çalışma alanı hazırlığı ve ACE kaydı

- **Ne zaman:** çalışma alanındaki ilk kabuk çağrısında, onaydan sonra ve
  komuttan önce.
- **Onay:** çalışma alanının izinleri değişeceği için bu onay her kipte yerel
  iletişim kutusunda sorulur. Kutu §4.2'deki "projeye güven" uyarısıyla
  birleşir ve şunu da söyler: "Kalfa bu klasörün izinlerine yalıtım kullanıcısı
  için giriş ekleyecek. Klasörü ayarlardan kaldırınca geri alınır."
- **Yapılanlar:**
  1. Çalışma alanı SID'i üretilir ve çalışma alanı kaydına yazılır.
  2. Kök dizine devralınan (`OI|CI`) iki izin eklenir: `KalfaYalitim` ve
     çalışma alanı SID'i için okuma-yazma. Değişiklik ağaca yayılır. Devralmayı
     kapatmış (korumalı DACL'li) alt dizinler izni almaz ve yalıtımda
     erişilemez olur. Bu belgelenir.
  3. Çalışma alanına özel dizinler (§7.3) aynı izinlerle hazırlanır.
  4. Gizli dosyalara `KalfaYalitim` için okuma reddi (deny) konur.
  5. Korumalı yollara `KalfaYalitim` için yazma reddi konur. Silme ve üzerine
     taşıma da engellenir:
     - Dosyada `DELETE` reddedilir.
     - Üst dizinde `FILE_DELETE_CHILD` reddedilir.
     - `.git/hooks` dizininde alt öğe oluşturma reddedilir.
- **İlerleme:** büyük ağaçlarda yayılma zaman alır. İlerleme araç kartında
  canlı çıktı olarak görünür (`AracCiktisiParcasi`, "İzinler hazırlanıyor:
  12 400 / 80 000").
- **Her çalıştırmadan önce** §7.2'nin taraması yeni gizli dosyaları bulur ve
  onlara da ret izni ekler.
- **ACE kaydı:**
  - Konum: `%LOCALAPPDATA%\Kalfa\yalitim\ace-kaydi.jsonl`.
  - İçerik: uygulamanın eklediği her izin (yol, SID, maske, tür, devralma).
  - Geri alma bu kayda göre yapılır, yeniden tarama yapılmaz.
  - Kayıt bozuksa geri alma, kökten başlayıp Kalfa SID'lerini taşıyan bütün
    girişleri silerek yapılır. Bu yavaştır ama eksiksizdir.
- **Araç zinciri dizinleri:**
  - Geliştiricinin sahibi olduğu dizinlere (örneğin `%USERPROFILE%\.cargo`)
    uygulama yükseltmeden okuma izni verir. İzin grup ve ortak okuma SID'i
    içindir.
  - Sahibi olmadığı dizinler (örneğin `C:\Python314`) `izin-ver` ile UAC'yle
    açılır. Bu izinler kurulum kaydına yazılır.
- **Kaldırma:** ayarlardaki yalıtım bölümünden bir çalışma alanı kaldırılınca:
  1. İzinleri ACE kaydına göre geri alınır.
  2. Çalışma alanı SID'i kayıttan silinir.
  3. Çalışma alanına özel dizinler istenirse silinir.

### 9.5 Prototip ve iki dal

Windows arka ucu bir prototiple başlar (üst belge §12.3, risk 12). Prototip ürün
kodu değildir:

- Bir bulgu raporu üretir ve bu belgeyi günceller.
- Senin makinende çalışır. Yönetici gerektiren adımları (kullanıcı, grup,
  güvenlik duvarı kuralı, yönetici ACE'leri) sen çalıştırırsın.
- Her kurulum adımının bir geri alma betiği vardır; prototip bitince makine ilk
  hâline döner.

**Ölçülecekler:**

| # | Soru | Nasıl |
|---|---|---|
| P1 | Kısıtlayıcı SID olarak `ALL APPLICATION PACKAGES` ile sistem ve program dizinleri okunuyor, `C:\projeler\` gibi kullanıcı klasörleri okunmuyor mu? | Kısıtlı belirteçle dizin listeleme ve okuma denemeleri |
| P2 | Kısıtlayıcı SID kümesinin en küçüğü ne? Masaüstü ve pencere istasyonu erişimi, `cmd`, `powershell`, `git`, `cargo`, `npm`, `python` için ne gerekiyor? | Kümeyi daraltarak deneme |
| P3 | A'nın SID'iyle çalışan süreç B'yi okuyup yazamıyor mu? | Aynı yalıtım kullanıcısıyla iki çalışma alanı |
| P4 | `~\.ssh`, Credential Manager ve geliştiricinin profili okunamıyor mu? | Doğrudan okuma denemeleri |
| P5 | Güvenlik duvarı kuralı `KalfaAgsiz`'ın giden trafiğini kesiyor mu? Geri döngü trafiğini ve adlandırılmış boruları (`docker_engine`, `openssh-ssh-agent`) ne yapıyor? | Bağlantı denemeleri |
| P6 | `CreateProcessWithLogonW` gecikmesi (ilk ve sonraki çağrılar), std tutamaç aktarımı, dönen tutamaçla işe atama | Ölçüm |
| P7 | Gizli dosyaya ret, korumalı yolda silme ve üzerine taşıma reddi | Deneme |
| P8 | `Users` grubundan çıkarmanın etkisi | P1–P2'yi iki üyelikle tekrarlama |
| P9 | Çalışma alanına özel `CARGO_HOME` ile ilk derlemenin ağ ihtiyacı | `cargo build` ağlı ve ağsız |
| P10 | Aynı masaüstündeki yalıtılmış sürecin geliştiricinin penceresine ileti gönderip gönderemediği, panoyu okuyup okuyamadığı | UI kısıtlarıyla ve kısıtsız deneme |

**Dallar:**

- **Dal A: P1, P3 ve P4 geçer.** Okuma sınırı üst belgedeki hedefi taşır:
  sistem, izinli araç zinciri ve kendi çalışma alanı. §9.1–§9.4 olduğu gibi
  uygulanır.
- **Dal B: P3 ve P4 geçer, P1 geçmez.** Kısıtlayıcı SID'ler kullanıcı
  klasörlerini ayıramıyor demektir.
  - Çalışma alanlarını birbirinden ayırmak yazmada korunur: yazma izni yalnızca
    o çalışma alanının SID'inde.
  - Okuma sınırı "bilgisayardaki her yerel kullanıcının okuyabildiği dosyalar"
    olur.
  - Bu durum arayüzde ve belgede yazılır.
  - Geniş izinli bir klasördeki (Authenticated Users, Users ya da Everyone'a
    okuma izni veren) çalışma alanı hazırlanırken uyarı gösterilir (üst belge
    §7.7).
  - Başarı ölçütü 1'in okuma satırı bu dalda "belgelenmiş sınır" olarak
    kapanır.
- **Dal C: P3 ya da P4 geçmez, ya da zincir araç zincirleriyle çalışmaz (P2).**
  - Windows'ta kabuk yalıtımsız kalır: her çağrı yerel onaylıdır.
  - Belge ve üst belge §14 güncellenir.
  - Kullanıcıyla yeni bir yaklaşım konuşulur, örneğin AppContainer ya da ayrı
    bir sanal makine.

### 9.6 Bilinen sınırlar

- **Devralmayı kapatmış alt dizinler** yalıtımda erişilemez (§9.4). Git
  depolarında bu genellikle olmaz.
- **FAT32 ve exFAT'te ACL yoktur.** Böyle bir birimdeki çalışma alanı
  hazırlanamaz; kabuk bu çalışma alanında yalıtımsız kalır ve bu söylenir.
- **Ağ paylaşımı (SMB) üzerindeki çalışma alanı** kapsam dışıdır: ACL'ler sunucu
  tarafındadır. Hazırlık reddedilir.
- **Tek geliştirici hesabı** (§9.2).

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
- Üzerine gelince talimat görünür. Ayarlardaki yalıtım bölümü (§10.4) gelince
  tıklanınca o bölüm açılır.
- Gösterge uygulama düzeyindedir, oturuma bağlı değildir. Durum
  `uygulama_durumu`'nun `yalitim` alanıyla gelir. 2a'da durum değişmez
  (her zaman "Yalıtımsız"); değişim olayı algılamanın geldiği 2b'de eklenir.

### 10.4 Ayarlarda yalıtım bölümü

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
| `uygulama_durumu` (mevcut) | `yalitim: YalitimDurumu` alanı eklenir (2a) |
| `yalitimi_kur` / `yalitimi_kaldir` | Yerel onay → UAC → sonuç (Windows); Linux'ta talimat döndürür |
| `arac_zinciri_dizinleri` / `arac_zinciri_dizini_ekle` / `arac_zinciri_dizini_kaldir` | Ekleme yerel klasör seçiciyle yapılır; arayüz yol gönderemez |
| `yalitim_calisma_alanlari` / `yalitim_calisma_alanini_kaldir` | Kimlikle çalışır (1c'deki gibi arayüz yol göndermez) |

Durum değişiminde ana pencereye `yalitim-durumu` olayı yayılır (2b).
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
| Yalıtım başlatılamadı (bwrap hatası, oturum açma hatası) | `Basarisiz`; algılama yeniden çalışır. Durum değiştiyse gösterge güncellenir. Komut yalıtımsız yeniden **denenmez** |
| Windows parola dosyası çözülemiyor | Durum `KurulumBozuk`; talimat "Yalıtımı yeniden kur" |
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
| Seçilmeyen yollar (`/srv`, `/mnt`, başka bir çalışma alanı) görünmez ya da okunamaz | ✓ | ✓ (dal A) |
| Aynı anda açık iki çalışma alanı birbirini okuyamaz ve yazamaz | ✓ | ✓ |
| Çalışma alanı dışına yazılamaz | ✓ | ✓ |
| `.git/hooks/pre-commit`, `.git/config`, `core.hooksPath` hedefi yazılamaz, silinemez, üzerine taşınamaz | ✓ | ✓ |
| Çalışma alanındaki `.env` ve onun hard link'i okunamaz | ✓ | ✓ |
| Ağsız profilde TCP bağlantısı ve Unix soketi (soyut dahil) açılamaz | ✓ | ✓ (TCP) |
| Ağlı profilde TCP açılır | ✓ | ✓ |
| Ağlı profilde Unix soketi açılamaz | ✓ | — |
| Seccomp engelleri (`io_uring_setup`, `keyctl`, `bpf`, `ptrace`, `unshare`) | ✓ | — |
| İptal süreç ağacını durdurur; torun süreç ve `setsid` kaçağı dahil | ✓ | ✓ |
| Kalfa'nın ölümü ağacı durdurur | ✓ | ✓ |
| Süreç sayısı sınırı | ✓ | ✓ |
| Yalıtılmış süreç panoyu okuyamaz, geliştiricinin penceresine ileti gönderemez | — | ✓ |
| Kaldırılan çalışma alanının ACE'leri geri alınır | — | ✓ |
| `kalfa-yalitim.exe` yalıtımın yazabildiği bir yerden başlatılmaz | — | ✓ |
| Sahte `bwrap` çalışma alanında olsa da seçilmez | ✓ | — |

### 12.3 Araç zinciri uyumluluğu

Yalıtım içinde, iki platformda, bu depoda ve `kalfa-sandbox/tests/ornekler/`
altındaki küçük projelerde:

- `git status`, `git diff`, `git add`, `git commit` (worktree'de dahil).
- `cargo build`, `cargo test` (önceden indirilmiş crate'lerle, ağsız).
- `npm ci` (ağlı), `npm test` (ağsız).
- `python -m pytest` ya da `python -c` (ağsız).

Çalışmayan her durum §14'e gerekçesiyle yazılır. Profil yalnızca açık bir
kararla genişletilir.

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

### 12.6 Doğrulama komutları

1c §11.6'dakilere ek olarak:

```
cargo test -p kalfa-sandbox --features yalitim-testleri   # Linux: sıradan kullanıcıyla
cargo test -p kalfa-sandbox --features yalitim-testleri   # Windows: kurulumdan sonra
```

### 12.7 Elle kontrol

- Windows'ta UAC akışı: kur, kaldır, yeniden kur.
- Durum çubuğu ve ayarlar bölümünün tasarım sistemine uygunluğu.
- Kaldırmadan sonra makinede Kalfa'ya ait kullanıcı, grup, kural ya da ACE
  kalmadığı: `kalfa-yalitim durum` ve bir ACL taraması.

---

## 13. Üst belgede değişenler

Bu belge commit'lenirken üst belgeye işlenenler:

- **§7.7 Windows arka ucu:**
  - Kurulumu MSI'dan önce `kalfa-yalitim.exe` yapar; uygulama onu UAC ile
    başlatır.
  - Parolaları uygulama üretir ve saklar.
  - Çalıştırma zinciri `CreateProcessWithLogonW` → `calistir` →
    `CreateProcessAsUserW` biçimindedir (§9.3).
  - Yardımcı dosya yalıtımın yazamadığı bir yerden başlatılır (§9.1).
- **§7.7 Linux arka ucu:**
  - Ağlı profilde de `AF_UNIX` soket açmak engellenir (§8.2).
  - Çalışma alanının git dizini dışarıdaysa yazılabilir bağlanır (§7.4).
  - Hard link'li gizli dosyalar da örtülür (§7.2).
- **§7.7 Süreç sınırları:** değerler: 120/1800 sn, 30 000 bayt, 256 süreç.
  Canlı çıktı kayıplıdır (§4.4).
- **§12.3:** aşama 2, dört plana bölünür (§15).

---

## 14. Açık noktalar

Plan ya da prototip aşamasında cevaplanacaklar:

| # | Soru | Nerede |
|---|---|---|
| A1 | `RLIMIT_NPROC` kullanıcı ad alanında yalnızca içerideki süreçleri mi sayıyor? | 2b |
| A2 | Salt okunur bağlanan cargo registry'siyle ağsız derleme çalışıyor mu; çalışmıyorsa kopyanın biçimi ne? | 2b |
| A3 | Gizli dosya taraması 100 000 dosyada 200 ms'nin altında mı? | 2b |
| A4 | Hangi araçlar `/sys`'e ihtiyaç duyuyor? | 2b (§12.3) |
| A5 | P1–P10 | 2c (§9.5) |
| A6 | Uzun ömürlü `calistir` aracısı gerekli mi (P6)? | 2c |
| A7 | Windows'ta çalışma alanı SID'inin biçimi | 2c |

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
4. **2d — Windows arka ucu.** 2c'nin dalına göre:
   - `kalfa-yalitim.exe`: kurulum, kaldırma, durum, çalıştırma zinciri.
   - Çalışma alanı hazırlığı ve ACE kaydı.
   - Ayarlardaki yalıtım bölümü.
   - §12.2'nin Windows sütunu ve §12.3 (Windows).

2a ve 2b iki platformun ortak kodunu değiştirdiği için sırayla yürür. 2c, 2a ve
2b'yle paralel yürüyebilir: ürün koduna dokunmaz.
