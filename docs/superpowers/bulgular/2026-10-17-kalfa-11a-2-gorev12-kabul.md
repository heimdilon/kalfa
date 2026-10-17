# MCP 1.1a-2 Görev 12 — İki platformda kabul

Durum: tamam; kabul testleri, ürün hatasının düzeltmesi ve iki platform doğrulaması bitti. Birleştirme
kararı kullanıcıda. Dal `codex/mcp-11a2-plan`, PR #20. Uygulayan: Claude.

## 1. Gerçek yalıtımla kabul (plan 1)

Yeni hedef: `cargo test -p kalfa-mcp --features yalitim-testleri --test yalitim`.
Bu hedef ürünün kendi yolunu kullanır:

1. `McpTanim::hazirla`
2. `CozulmusTanim::etkilesimli_istek`
3. `Yalitici::baslat_etkilesimli`
4. Sınırlı taşıma ve MCP istemcisi

Sunucu depodaki test sunucusunun yeni `sinama` kipidir (`dene` aracı). Dosya
ve ağ kararını işletim sistemi verir. Üç test vardır:

- `agsiz_sunucu_alan_disina_ve_aga_cikamaz_kapanista_iz_birakmaz`:
  - alanı okur ve yazar;
  - evdeki sırrı okuyamaz;
  - alan dışına yazamaz;
  - doğrudan TCP bağlantısı kuramaz.

  Kapanıştan sonra izli süreç kalmaz. Windows'ta çalışma alanında
  `S-1-15-2` ACE'si kalmaz.
- `agli_sunucu_izinli_hedefe_ulasir_yasakli_403_alir`:
  - izinli hosta vekilden ulaşır;
  - listede olmayan host 403 alır;
  - vekili atlayan doğrudan bağlantı kurulamaz.
- `sonradan_gelen_gizli_dosya_eski_surecte_gorunur_yeni_surecte_gorunmez`:
  aşağıda §3.

Sonuç: Windows (AppContainer) **3/3**, Ubuntu 24.04 (bwrap, `kalfatest`)
**3/3**.

Mutasyon denetimi: aynı testler tanım tam erişime çevrilince iki
platformda da **0/3** geçti.

Linux'ta `/tmp` sunucunun özel geçici dizinidir. Alan dışına yazma oraya
düşer ve ana makineye çıkmaz. Test, sınırı ana makinedeki dosyanın
yokluğuyla ölçer.

## 2. §11 matrisi (plan 2)

Kanıt adları test işlevleridir. "Sahte" sahte saat, koruma, başlatıcı
veya iletişimle yapılan testtir. "OS" gerçek yalıtımla yapılan testtir.

| §11 satırı | Kanıt |
|---|---|
| Başlatma: yalnız İnceleme, izin reddi ve izin yokken süreç başlamaz | `yonetici::{baslatma_izni_yok_veya_reddedildi_ise_isaret_dosyasi_olusmaz, yalniz_inceleme_oturumunda_surec_baslamaz_ve_son_oturum_incelemeye_gecince_kapanir, yalniz_inceleme_oturumu_kalinca_sunucu_baslatilmaz}` (sahte; işaret dosyası) |
| Tam erişim izni yeniden açılışta sorulur | `tam_erisim_izni_diske_yazilmaz_yalitimli_kalici_izin_yeniden_kullanilir` |
| Koruma: `.env`, gizli kalıp, okunur dizin, ağ daraltma | Sahte: `koruma_degisimi_ve_dogrulanamadi_durumu_sureci_durdurur_ve_yeniden_baslatmayi_engeller`, `koruma::politika_ozeti_daralma_ve_genislemede_degisir`. Uygulama: `mcp_komutlari::sonradan_gelen_gizli_icerik_sonuc_ve_tanim_yoluyla_saglayiciya_ulasir` (çağrı öncesi `.env` çağrıyı keser, kuşak artar). OS: `yalitim::sonradan_gelen_gizli_dosya_…` |
| `.git` ve yapılandırma karantinası | `kalfa-tools::koruma::son_denetim_tehlikeli_git_ve_yeni_yapilandirmayi_kabuk_gibi_raporlar` |
| Boşta kapanma, mutlak yaş, doğrulanamayan tarama, görüntü sınırı | `yonetici::{birakilan_cagri_bosta_kapanmayi_engellemez, aktif_cagri_bosta_suresini_sifirlarken_mutlak_koruma_yasi_dolunca_denetim_calisir, etkin_cagri_kesif_veya_onay_beklerken_bosta_kapanmaz_ama_mutlak_koruma_yasi_calisir, mutlak_denetim_kendiliginden_calisir, taramada_durdurma_baslatma_rezervasyonunu_gecersiz_kilar}`; `kalfa-tools::koruma::ayni_k_gibi_gorunen_kismi_tarama_reddedilir_goruntu_eksik_koruma_tam_kabul_edilir` (sahte saat) |
| Stdout üzerinden çıkış | §3 |
| Sınırlar: 4 MiB + 1, 5 MiB görsel, dev stderr, 11 sayfa, 129 araç, sel | `kalfa-mcp::sinirlar::*`, `protokol::gercek_istemci_sayfalar_ve_kismi_limit_asimini_yayimlamaz`, `istemci::arac_sayisi_yuz_yirmi_sekiz_sinirinda_kalir`, `maske::stderr_halkasi_dev_satiri_sinirli_tuketir_ve_tasan_sirri_gizler` |
| İptal ve zaman aşımı, üç sunucu davranışı | `gercek_surec_testleri::{gercek_g5_iptal_ve_deadline_uc_server_davranisinda_kilidi_son_denetimden_once_cozmez, gecikmeli_torun_surec_kapanisi_sonrasinda_dosya_yazamaz, eski_kusak_cleanup_yeni_kusagi_durdurmaz_ve_eski_guard_serbest_kalir}`, `protokol::iptalin_uc_sunucu_davranisinda_istemci_kapanir` (gerçek süreç) |
| Mühür ve kısayol, Π | `cagri::r_d_ve_pi_son_kapi_yarisinda_rpc_sayaci_sifir_kalir`, `arac_muhur::mcp_muhru_r_d_ve_argumanlari_baglar`, `kisayol::mcp_kisayolu_t_d_ve_politikayi_baglar_n_ve_k_degisimi_kapsami_korur`, `mcp_actor::*` |
| Kod kökleri | OS: sandbox `etkilesimli_sunucu_ozel_koku_kabuktan_ve_obur_sunucudan_ayri` (Linux), `etkilesimli_sunucu_koku_yetenek_sidiyle_ayrilir_ve_izinler_geri_alinir` (Windows), `etkilesimli_ortam_ozel_koke_…`. Tanım: `tarif::w_proje_ve_kabuk_koklerini_kapsar_sunucunun_ozel_koku_disarida_kalir`, `tanim::{node_path_icindeki_ikinci_w_koku_guven_denetimine_girer, kod_arama_listesinde_bos_ve_eksik_girdi_guvenilir_sayilmaz}` |
| Sunucu kodu | `tarif::ayni_boyutlu_ve_bosluklu_kod_dosyasi_degisiminde_t_degisir`, `yonetici::{baslatma_sirasinda_yazma_kusagi_degisirse_wde_kodlu_sunucu_eski_izinle_baslamaz, coken_wde_kodlu_sunucu_w_veya_k_degisince_otomatik_yeniden_baslatilmaz}`, `tanim::goreli_path_programi_sunucunun_ozel_evinde_bulur`, `tanimlar::onay_sonrasi_dosya_ozeti_degisen_kodu_yakalar` |
| Tanım denetimi ve tanımda sır | `denetim::{ic_ice_input_schema_metinleri_denetciye_gider_ve_kati_uyar_kapali_matrisi_uygulanir, onbellek_isabeti_…, tanimda_sirin_dort_konumu_denetci_model_cache_ui_ve_gunlukten_uzak_tutulur}`, `tanim_denetimi::*` |
| Protokol ve dönemler | `kalfa-mcp::protokol::*` (gerçek istemci ve sunucu) |
| İçerik, sırlar, çıkış şeması | `sonuc::*`, `maske::*`, `denetim::sonuc_donusturucu_ve_uzak_rpc_hatasi_sirlari_maskeler_cikis_semasini_zorlar` |
| Windows: ortak yaşam, profil ve ACE kalmaz | `yonetici::ayni_calisma_alaninda_iki_sunucu_ve_kabuk_birbirini_bozmaz`; OS: `yalitim::*` (`S-1-15-2` yok), e2e `13-mcp` (profil sayısı aynı) |
| Çalışma kipi ve ağ | `etkilesimli::kip_uyusmasi_dort_hucre`, sandbox `etkilesimli_tam_erisim_yalitim_hazirken_yalitimsiz_baslar`, `tanim::ad_komut_timeout_ve_tam_erisim_ag_kurali_dogrulanir`, OS `yalitim::agli_…` |
| Ortam ve kasa | `ortam::*`, `ayarlar::*`, `kasa::*`, `maske::*` |
| Araç kaynağı | `arac_goruntusu::*`, `yonetici::izinli_sunucu_turda_baslar_kesfedilir_ve_yayimlanir`, `mcp_komutlari::sonradan_gelen_…` (araç sonraki turlarda kalır; liste değişimi `tools`'a ulaşır) |
| Arayüz sınırı, `mcp_karari`, Vitest | `mcp_komutlari` (Görev 10), `kalfa::izinler`, `izin::mcp_izin_matrisi_dort_kip_iki_erisim`, `ui/src/ana/mcp.test.ts` |
| E2e | `e2e/testler/13-mcp.test.mjs` (aşağıda) |

**Kapsanmayan ya da kısmi kalanlar:**

- Başlatma testleri işaret dosyasını ölçer; "yerel test ucuna bağlantı
  gelmez" ayrıca ölçülmedi.
- `npx -y paket@1.2.3` karşı örneği gerçek paketle koşulmadı: dış paket
  indirilmez. Yerine şunlar ölçüldü:
  - özel kökün kabuktan ve öbür sunucudan ayrılması (iki OS);
  - npm önbelleğinin özel köke yönelmesi.
- `node_modules/.bin` altındaki `npx <ad>` için yalnız çalışma dizininin
  özel ev olması ölçüldü.
- Sınır ihlalinde uygulama düzeyinde süreç ağacının temizlendiği ayrıca
  ölçülmedi. Taşıma ve istemci düzeyi ölçüldü.
- Canlı EVREN denemesi (`#[ignore]`) ayrı onay ister; koşulmadı.

## 3. Stdout ve tanım yolu riski (plan 3)

Sınama kasadaki bilinen sırrı değil, sunucu başladıktan sonra çalışma
alanına gelen gizli dosyanın içeriğini kullanır. Ölçülenler:

- **Sonuç yolu (kabul edilen kalan risk, §4.3):** Dosya çağrının içinde
  gelirse, yani çağrı öncesi koruma taramasından sonra, içerik araç
  sonucuyla sahte sağlayıcıya ulaşır. Kanıt:
  `mcp_komutlari::sonradan_gelen_gizli_icerik_sonuc_ve_tanim_yoluyla_saglayiciya_ulasir`.
- **Tanım yolu, denetim örneği:** Gizli olmayan bir dosyanın içeriği araç
  açıklamasına konunca liste değişir. İçerik sonraki turların `tools`
  alanına ulaşır; tesisat çalışır.
- **Tanım yolu, gizli dosya:** Dosya iki çağrı arasında gelirse sonraki
  turun başlatma doğrulaması korumayı yeniden tarar ve süreci yeniden
  başlatır (kuşak artar). Eski sürecin açıklaması modele ulaşmaz.
- **Yeni süreç:** Koruma yenilenip yeni süreç başlayınca aynı dosya
  okunamaz ve açıklamaya konamaz. İki OS'ta ölçüldü:
  `yalitim::sonradan_gelen_gizli_dosya_eski_surecte_gorunur_yeni_surecte_gorunmez`.
  Bu testin ilk yarısı aynı dosyanın eski süreçte görünür olduğunu ve
  içeriğin açıklama ile tanım denetiminden geçtiğini de gösterir.
- Mutlak yaş dolunca sürecin durması sahte saatle ayrıca ölçülür (§2).

## 4. Bulunan ve düzeltilen ürün hatası

**Belirti.** Sunucu bir kez keşfedildikten sonra, sonraki turda araçları
modelden kayboluyordu ("adlı bir araç yok"). Çağrı da sık sık "Dosya
onaydan sonra değişti" ile reddediliyordu.

**Kök neden.** `sunucu_korumasini_denetle` aynı kuşakta süren bir tarama
varsa `try_lock` ile hemen `BaslatmaYarisi` dönüyordu. İki çağıran bunu
yanlış yorumluyordu:

- Başlatma doğrulaması bunu "korumalar değişti" sanıp yeni kuşak
  başlatmaya çalışıyor, eski süreç dururken yine `BaslatmaYarisi` alıyordu.
  Keşif işi de bunu "başlamadı" sayıp boş araç listesi yayımlıyordu.
- Çağrı hazırlığı (`guncel_kart`) aynı yarışı `OnayEskidi` sayıyordu.

Aynı turdaki keşif ile çağrı hazırlığı ya da kuşağın zamanlayıcısı
birbirini düşürüyordu. Gerçek süreçli kabul testi bunu yakaladı. Sahte
başlatıcılı birim testleri bu eşzamanlılığı üretmiyordu.

**Düzeltme.** `sunucu_korumasini_bekleyerek_denetle` eklendi. Süren
taramanın bitişini bekler ve kilidi bırakmadan kendi taramasını yapar.
Başlatma doğrulaması ve çağrı hazırlığı bunu kullanır. Zamanlayıcının
davranışı değişmedi: süren tarama `t_K`'yı zaten yenilediği için o yine
atlar.

İlk denemede bekleyen iki çağıran kilidi alıp bıraktıktan sonra
`try_lock`'ta birbirini düşürerek sonsuz döngüye girdi. Bu yüzden
bekleyen sürüm taramayı kilidi tutarken yapar.

**Kanıt:**

- `yonetici::suren_koruma_taramasi_baslatma_dogrulamasini_dusurmez`
  düzeltmeden önce `BaslatmaYarisi` ile düştü, sonra geçti.
- Bekleme kapatılınca bu test ve kabul testi yine düştü.
- MCP birim testlerinin hepsi (75) geçiyor.

## 5. E2e (plan 4)

`e2e/testler/13-mcp.test.mjs` iki akışı test eder:

1. **Yalıtımlı:**
   - sunucu Ayarlar'daki formla eklenir; Gözat, seçili dosyayı döner;
   - kart model dışı görünür ve "Başlat" ile izin verilir;
   - keşiften sonra sahte model aracı çağırır;
   - onay sayfasında MCP kaynağı ve rozet görünür;
   - onaylanınca sonuç gelir ve araç kartında "sunucu · araç" görünür.
2. **Tam erişim:**
   - formda kalıcı uyarı ve "Sınırsız" görünür;
   - kart yerel onayı söyler;
   - onay sayfasında MCP'nin yerel onay notu görünür.

Her akışın sonunda alan hazırlığı kaldırılır. Alandan başlamış süreç
kalmaz ve Kalfa AppContainer profil sayısı başlangıçtakiyle aynıdır.

`calistir.mjs` test sunucusunu da derler. Test sunucusu kurucuya girmez.
`Kalfa.ac` ayarları alanın yoluna göre işlevle de alır.

Sonuçlar (bütün e2e takımı, gerçek WebDriver):

- Windows: **43/43**. WebView2 155.0.4283.60, `msedgedriver`
  155.0.4283.45; MCP yalıtımlı akışı gerçek AppContainer'da koştu.
- Ubuntu 24.04: **43/43**. `xvfb-run`, WebKitWebDriver, bwrap.

Ortam: worktree'de `node_modules` yoktu. Hiçbir şey indirilmedi:

- Arayüzün `dist-e2e` paketi ana klasörün kurulumuyla geçici kopyada
  derlendi ve iki platforma kopyalandı.
- `e2e/node_modules` ana klasörden alındı: Windows'ta bağlantı (junction),
  WSL'de kopya. Kilit dosyaları aynıdır.

Loglar: `.yerel/mcp-tasarim-inceleme/11A2-CLAUDE-g12-E2E-{WINDOWS,WSL}.log`.

## 6. Doğrulama komutları (plan 4)

Aynı ağaç (`07376d4`) iki platformda:

| Komut | Windows | Ubuntu 24.04 (WSL2, `kalfatest`) |
|---|---|---|
| `cargo fmt --all -- --check` | temiz | temiz |
| `cargo clippy --workspace --all-targets` | temiz | temiz |
| `cargo clippy -p kalfa -p kalfa-uygulama --features e2e` | temiz | temiz |
| `cargo test --workspace` | **1450** geçti, 19 ignore | **1405** geçti, 19 ignore |
| `cargo test -p kalfa-uygulama -p kalfa --features e2e` | **269** | **270** |
| `kalfa-sandbox --features yalitim-testleri --test yalitim` | 50 | 32 |
| `kalfa-tools --features yalitim-testleri --test uyumluluk` | 24 | 16 |
| `kalfa-mcp --features yalitim-testleri --test yalitim` | 3 | 3 |
| G2 (`prototip/mcp-tasima`) test/clippy/fmt | 7, temiz | 7, temiz |
| Arayüz `check`/`test`/`build` | 0/0, 459/459 (Görev 12 arayüze dokunmaz) | — |
| Gerçek WebDriver e2e | 43/43 | 43/43 |

İlk Linux workspace koşusunda `kalfa-plugin`
`bellek_siniri_cagriyi_durdurur_sonraki_calisir` düştü. O sırada Windows
takımı ve e2e aynı makinede koşuyordu. Dal `kalfa-plugin`'e dokunmaz; test
tek başına 3/3 geçti ve yük yokken workspace koşusu yukarıdadır.

## 7. PR incelemesi (Codex, `162a4e6`)

Yedi bulgunun yedisi de kodda doğrulandı ve düzeltildi. Her biri, düzeltme
geri alınınca düşen bir testle gelir.

| Bulgu | Düzeltme | Test |
|---|---|---|
| İzin dosyası yazılamazsa geri alınan izin yeniden açılışta dönüyordu | Yazma düşerse eski dosya silinir (izinler yeniden sorulur); silme de düşerse geri alma hata bildirir | `yonetici::izin_dosyasi_yazilamazsa_geri_alinan_izin_yeniden_acilista_donmez` |
| Yeniden açılıştan sonra yuvası olmayan kalıcı izin Ayarlar'da görünmüyordu | Satırlar yuvalarla kayıtlı izinlerin birleşimidir | `mcp_komutlari::yeniden_acilista_uyuyan_izin_gorunur_ve_geri_alinir` |
| Silmede sunucunun özel kökü (Windows'ta ACE kaydı) kalıyordu | Silme, alan başına `sunucu_kokunu_temizle` çağırır | `mcp_komutlari::silinen_sunucunun_ozel_koku_kaldirilir` |
| Yeniden adlandırmada eski adlı süreç ve izin kalıyordu | Kayıt sonucu eski adı taşır; eski ad silinmiş gibi kaldırılır | `mcp_komutlari::yeniden_adlandirmada_eski_adin_izni_ve_koku_kaldirilir` |
| Ayarlar yazılıp kasa temizliği düşünce silme/kayıt hata dönüp süreçleri durdurmuyordu | `TemizlikGerekiyor` tamamlanmış kayıt sayılır; kalanlar sonraki kayıtta silinir | `tanimlar::kasa_temizligi_kalan_duzenleme_tamamlanmis_sayilir_ve_eski_adi_doner` (Linux) |
| İçe aktarmada gizli başlayan kısa değer düze çevrilemiyordu; kayıt hep düşüyordu | "Gizli say" iki yönde çalışır (tasarım §8: "düz değere çevirebilir") | `mcp.test.ts` içe aktarma |
| Formla düzenleme kapalı sunucuyu yeniden açıyordu | Düzenleme mevcut `etkin` değerini korur | `mcp.test.ts` düzenleme |

Kalan sınır: veri dizini bütünüyle salt okunur olursa (dosya ne yazılır ne
silinir) yazma kuşağı artışı ve politika değişikliği diske geçmez. Π aynı
dizindeki ayarlarda saklandığı için o da diske geçmemiştir; açık geri alma
bu durumda hata bildirir.

Düzeltmelerden sonra Windows/WSL: workspace **1454/1410**, e2e özellikli
Rust **273/275**; arayüz 0/0 ve 459/459, build temiz; fmt/clippy temiz.
