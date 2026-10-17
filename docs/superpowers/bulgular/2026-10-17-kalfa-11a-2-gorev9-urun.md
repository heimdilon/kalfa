# MCP 1.1a-2 Görev 9 — yaşam döngüsünün uygulamaya bağlanması ve kurtarma

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20. Uygulayan: Claude.

## Davranış

**Keşif zinciri (yeni).** Görev 8'in alan kaynağı artık uygulamada bir
planlayıcıyla çalışır. `oturumlar.rs` her oturum bağlamında alanın
`McpAlanAracKaynagi`'na bir kez `McpAlanBaglantisi` planlayıcısını bağlar;
`Uygulama` tek bir `McpTanimDenetleyicisi` tutar. Core her model turunun
başında denetimleri sahiplenirken planlayıcı ayarlardaki etkin sunucuları
okur ve her biri için şu işi kurar:

1. `baslat_veya_dogrula`: izin yoksa, alan kaldırılıyorsa, yalnız İnceleme
   oturumu kaldıysa ya da çökme geri çekilmesi sürüyorsa süreç başlamaz,
   o sunucunun araçları yayımdan düşer ve sonraki başlatma listeyi yeniden
   denetler. Turu bekletmez (§5.1, §7.1).
2. İlk keşifte ya da çalışma tarifi `R` değişince
   `sunucu_araclarini_kesfet_ve_denetle`; aksi hâlde yalnız
   `gerekiyorsa_yenile` (liste değişimi ya da 30 sn yedek yoklama). Denetçi
   bütçesi turu sahiplenen oturumun bütçesidir.
3. Denetimden geçen her araç için `arac_olustur` ile `McpArac`, oradan
   `KaynakAraci`; yayım bir sonraki model turunun görüntüsüne girer.

Ayarlarda kapatılan ya da silinen sunucunun yayımı ve bekleyen işi bir
sonraki sahiplenmede düşer. Sürmekte olan iş bitmeden aynı sunucu için
ikinci iş açılmaz.

**Başlatma kartı birikmesi (düzeltme).** Otomatik başlatma artık her turda
denendiği için izinsiz sunucu her tur yeni bir bekleyen kart üretiyordu
(`bekleyen_kartlar` sınırsız büyürdü). `karti_bekletmeye_al` aynı
koşullarla (alan, sunucu, erişim, T, Π, kod güveni, W) bekleyen kartın
kimliğini yeniden verir; koşul değiştiyse aynı (alan, sunucu) için eski
kartı düşürüp yenisini ekler.

**Mevcut bağlantılar (Görev 5–6).** Oturum aç/kapat/kip olayları
(`oturum_kipi_guncelle`), politika ve ayar değişimi (`politika_degisti`),
alan kaldırma rezervi (`alan_kapanisini_baslat_ve_bekle`), alan başına
yazma kuşağı ve son koruma denetimi (`AlanYazmaBildirimi`) zaten bağlıydı;
bu görev onları yeni zincirle kabul testinde birleştirdi.

**Kurtarma.** Core kurtarması araçtan bağımsızdır: yarım kalan çağrı
"sonucu bilinmiyor" olarak kapanır, hiçbir araç yeniden çalıştırılmaz.
Kaynaktan gelen (MCP) aracın da yeniden çalıştırılmadığı testle sabitlendi.
`AracTanimlari` günlüğü görüntüdeki denetlenmiş tanımları yazar; stderr ve
sırlar kayda girmez (G7 maskeleri).

## Kapsam notları

- Başlatma kartının arayüze ve native onaya taşınması Görev 10–11'dir;
  bu görevde kart yöneticide bekler, araç gelmez.
- Planlayıcı her turda etkin her sunucu için `baslat_veya_dogrula` çağırır;
  çalışan süreçte bu, mevcut koruma doğrulamasıdır.
- Ağ hatası gibi geçici nedenle `Bilinmiyor` denetlenen araç, Katı kipte
  liste ya da `R` değişene kadar sunulmaz (kalıcı önbelleğe yazılmaz).
- Tasarım §7.1'deki "sohbette tek satırlık not" Görev 11 arayüzündedir.

## Kanıt

Yeni testler:

- `mcp::yonetici::tests::alan_baglantisi` (5 test, sahte süreçle):
  izinli sunucu turda başlar, keşfedilir, yayımlanır; değişiklik yoksa
  yayım ve süreç aynı kalır; kapatılan sunucu düşer. İzinsiz sunucu
  başlamaz, turu bekletmez, kartlar birikmez ve kimlik korunur. Yalnız
  İnceleme oturumu kalınca başlatılmaz. İzin geri alınınca ve alan
  kaldırılırken araçlar düşer, yeni başlatma olmaz. İki alan ayrı izin ve
  ayrı yayım tutar.
- `kalfa-core/tests/oturum_kurtarma.rs`: yarım MCP çağrısı yeniden
  çalıştırılmaz.

Mutasyon denetimi: kapatılan sunucunun yayımı düşürülmeyince, ilk keşif
atlanınca, başlatma hatasında eski yayım korununca ve kart yeniden
kullanılmayınca ilgili testler düştü; hepsi geri alındı.

Doğrulama (yerel `.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g9-{WINDOWS,WSL}.log`):

- Windows: workspace **1445 geçti, 19 ignore**; `e2e` özellikli Rust **265/265**;
  fmt, clippy ve `e2e` clippy temiz.
- WSL (`Ubuntu-24.04`, `kalfatest`): workspace **1400 geçti, 19 ignore**;
  `e2e` özellikli Rust **266/266**; fmt, clippy ve `e2e` clippy temiz.
- Görev 8 sayılarına (`59ddc94`) göre workspace +7 (G9 için +6, `c8a8a39`
  Unicode testi +1), e2e +5, iki platformda da.
  Yalıtım/uyumluluk ve gerçek WebDriver e2e bu görevde koşulmadı
  (sandbox/tools ve arayüz değişmedi); tam kabul Görev 12.
