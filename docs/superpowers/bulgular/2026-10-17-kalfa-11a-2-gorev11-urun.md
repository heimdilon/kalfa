# MCP 1.1a-2 Görev 11 — Ayarlar, başlatma ve çağrı kartları

Durum: tamamlandı. Dal `codex/mcp-11a2-plan`, PR #20. Uygulayan: Claude.

## Davranış

**Ayarlar → MCP sunucuları** (`McpSunuculari.svelte`). Eklentiler
ekranından ayrı bir kart. Her sunucu için şunlar görünür:

- ad ve erişim rozeti (*Yalıtılmış süreç* / *Tam sistem erişimi*);
- rozet açıklaması: erişim sunucu başladığı andan kapanana kadar sürer;
  tam erişimde torun süreçlerin geride kalabileceği de yazar;
- etkinlik kutusu;
- çalışma alanı başına durum, araç sayısı, son hata ve başlatma izni;
- "İzni geri al" ve "Yeniden başlat" düğmeleri. Yeniden başlatma oturumla
  gider; alanda açık oturum yoksa düğme kapalıdır.

Ayrıca ayrıntı, düzenleme ve silme vardır. Görüntü `mcp_dinle`
kanalından gelir; bölüm açılınca `mcp_durumu` okunur. Kanal ve doğrudan
okuma aynı yere yazar ve eski revizyon yenisini ezmez.

**Form** (`McpFormu.svelte`):

- Yeni sunucu yalıtımlı başlar.
- Komut ya çıplak program adıdır ya da "Gözat" ile seçilir. Gözat yerel
  seçiciyi açar ve arka uçtan kimlik döner; arayüz yol göndermez.
- Argümanlar satır satır yazılır.
- Ortam değişkenleri satır satır girilir; gizli değer parola alanındadır.
- Kayıtlı gizli değer düzenlemede gösterilmez ve kimliğiyle korunur.
- Ağ izinleri host, port ve özel ağ alanlarıyla yalnız yalıtımlıda
  girilir. Tam erişimde ağ alanı gizlenir, "Sınırsız" ve kalıcı uyarı
  görünür.
- Kayıt ve ret yerel penceredir; ret "vazgeçildi" der, hiçbir şey yazılmaz.

**JSON içe aktarma** (`McpIceAktarma.svelte`). Metni arka uç ayrıştırır.
Metin önizlemeye gönderilir gönderilmez alandan silinir; önizleme
başarısız olsa da silinir. Önizleme gizli değeri göstermez; düz bir değer
"Gizli say" ile kasaya yönlendirilir. `url` içeren sunucular nedeniyle
listelenir ve atlanır.

**Ayrıntı** (`McpAyrinti.svelte`). Çalışma alanı başına denetlenen araçlar
ve tanım metni gösterilir. Tanım denetiminin durumu ayrı yazılır: kapalı,
denetlendi, şüpheli (uyarıyla sunuluyor ya da katı kipte sunulmuyor) veya
denetlenemedi. Kapatılan araçlar yalnız sıra numarasıyla, son stderr
maskeli gösterilir.

**Sohbette başlatma kartı** (`McpBaslatmaKarti.svelte`). Kart, aynı
çalışma alanındaki açık oturumlarda, model öğelerinden ayrı olarak
konuşmanın sonunda görünür. Salt okunur oturumda görünmez. Kartta şunlar
yazar:

- erişim türü, komut ve ağ;
- arka ucun uyarısı ("başladığı andan kapanana kadar çalışır");
- iznin süresi;
- kartı modelin açamayacağı ve yanıtlayamayacağı.

Tam erişimde "Başlat"tan sonra yerel pencerenin ayrıca sorduğu da yazar.
Kart sohbeti bekletmez: Composer açık kalır. Modelin araç çağrısı onayı
bu kartı yanıtlamaz; testte onay sayfası onaylandıktan sonra kart
yerinde kalır.

**Onay sayfası ve araç kartı:**

- MCP öğesinde "Kaynak" satırı sunucu, araç ve rozeti gösterir.
- "Erişim" satırı izin gerekçesini erişim türüyle söyler: yalıtımlıda
  izinli hostlar, tam erişimde sınırsız ağ ve yerel pencere.
- Tam erişimli MCP öğesinde kabuğun notu yerine MCP notu çıkar: kısayol
  yoktur.
- MCP kısayol cümlesi eklendi. Önceden `tip: 'Mcp'` teklifi "undefined"
  diye çizilecekti.
- Sohbetteki araç kartının başlığı "sunucu · araç" olur ve rozet görünür.

## Arka uca eklenenler

Bunlar arayüzün sözleşmesi için gereken en küçük alanlardır:

- `OnayOgesi.mcp: Option<McpOnayKarti>` (`kalfa-core::protokol`).
  `McpAracHazirligi`'ndan yalnız doğrulanmış görünen adlar, erişim ve
  hostlar alınır. Mühre girmez.
- `McpBaslatmaKarti.kip`: arayüz Türkçe etikete göre dallanmasın diye.
- `OturumAcilisi.calisma_alani_kimligi`: kart oturuma yolla değil
  kimlikle bağlanır.

## Kanıt

- `ui/src/ana/mcp.test.ts`, 19 test:
  - bölüm: rozet, açıklama, durum, sayı ve izin;
  - etkinlik, izni geri alma ve yeniden başlatma;
  - oturumsuz alanda yeniden başlatmanın kapalı olması;
  - eski revizyon, hem kanalda hem `mcp_durumu` okumasında;
  - silmede yerel ret;
  - form: yalıtımlı varsayılan; tam erişimde ağın gizlenmesi ve uyarı;
    Gözat ve kimlik; gizli değerin parola alanında olması ve kayıttan sonra
    DOM'da kalmaması; yerel ret; eksik alan; düzenlemede korunan gizli
    değer;
  - JSON: başarıda ve hatada metnin silinmesi, gizlinin gösterilmemesi,
    "gizli say" ve kayıt;
  - ayrıntı: denetim durumları, kapatılanlar ve stderr;
  - kart: alan eşleşmesi, salt okunur, "Başlat", tam erişimde ret ve
    model onayının kartı yanıtlamaması;
  - onay sayfası satırları ve MCP kısayolu;
  - araç kartı başlığı ve rozeti.
- Mutasyon denetimi: aşağıdaki 8 bozmanın her biri en az bir testi
  düşürdü.
  - alan süzgeci;
  - JSON metnini silme;
  - tam erişimde ağı gizleme;
  - gizli değerin parola alanı;
  - korunan gizlinin kimliği;
  - MCP kısayol cümlesi;
  - revizyon koruması;
  - salt okunur oturumda kart.

  İlk koşuda revizyon ve salt okunur bozmaları yakalanmadı. İki test
  eklendi, sonra ikisi de düştü.
- `crates/kalfa-core/tests/mcp_actor.rs`: dört kip × iki erişim matrisinde
  onay öğesi kart özetini taşır. Alan `None` yapılınca test düştü.
- `crates/kalfa-uygulama/tests/mcp_komutlari.rs`: kartın `kip`'i ve
  `alan_kimligi`'nin oturum açılışındaki `calisma_alani_kimligi` ile
  eşleşmesi.
- Önizleme (`onizleme.html`): kart ve Ayarlar bölümü tarayıcıda çizildi,
  konsolda hata yok; tam erişimde form uyarısı ve "Sınırsız" görüldü.

Arayüz: `npm run check` 0 hata/0 uyarı, **459/459** test, derleme ve paket
denetimi temiz. Worktree'de `node_modules` yok. Ana klasörün aynı kilit
dosyasından kurulumu OneDrive dışındaki geçici kopyaya alındı; indirme
yapılmadı (`.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g11-UI.log`).

Rust doğrulaması (`.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g11-{WINDOWS,WSL}.log`):

- Windows: workspace **1447 geçti, 19 ignore**; `e2e` özellikli Rust
  **267/267**; fmt, clippy ve `e2e` clippy temiz.
- WSL (`Ubuntu-24.04`, `kalfatest`): workspace **1402 geçti, 19 ignore**;
  `e2e` özellikli Rust **268/268**; fmt, clippy ve `e2e` clippy temiz.
- Sayılar Görev 10 ile aynıdır: Rust tarafında yeni test eklenmedi, var
  olan iki teste doğrulama eklendi.

## Kapsam notları

- Arayüz bölüm açılırken ve her kanal görüntüsünde tazelenir. Süreç
  çökmesi gibi yöneticinin kendi geçişleri Görev 10'daki gibi kanala ayrıca
  yazılmaz.
- Çalışma alanının yolu, açık bir oturumdan bilinir. Açık oturum yoksa
  kısa kimlik gösterilir; arka uç yol göndermez.
- Gerçek WebDriver e2e Görev 12'dedir.
