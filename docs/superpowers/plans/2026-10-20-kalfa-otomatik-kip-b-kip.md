# Kalfa otomatik kip, PR B: Kip, hakem kapısı ve arayüz (uygulama planı)

**Durum:** Uygulandı (20 Ekim). PR #35 `5260ed3` olarak birleşti.
**Uygulayıcılar:** Görev 1–8'i Antigravity uyguladı.

- **Claude:** İnceledi; tek bulgu bir açıklamaydı ve düzeltildi. Windows ile
  WSL'de baştan doğruladı.
- **Canlı deneme:** Kullanıcının kararıyla arayüzsüz, çekirdek düzeyinde
  yapıldı (`tests/otomatik_kip_canli.rs`). Arayüzlü deneme, yerel Windows
  pencereleri yüzünden otomatikleştirilemedi.
**Taban:** `main`, `4041a39` (PR A, #34 birleşti).
**Tasarım:** [Otomatik kip](../specs/2026-10-19-kalfa-otomatik-kip-design.md):
§3 kurallar, §4 hakem, §5 ret ve geri düşme, §6 kayıt, §7 yaşam döngüsü,
§9 güvenlik testleri.
**Önceki iş:** [PR A planı](2026-10-19-kalfa-otomatik-kip-a-hakem.md).
Hakem (`kalfa_core::hakem::degerlendir`) ve eşikleri hazırdır:
izin 0,80, ret 0,40 ([ölçüm](../bulgular/2026-10-19-kalfa-otomatik-kip-olcum.md)).

**Hedef:** Kullanıcı kip menüsünden Otomatik'i seçebilir. Bu kipte hakeme
uygun onay öğeleri önce hakeme gider:

- **İzin:** işlem onay sorulmadan çalışır.
- **Ret:** modele ret sonucu döner.
- **Belirsiz ya da hata:** onay kartı kullanıcıya gelir.

Hakeme uygun olmayan her öğe bugünkü gibi kullanıcıya sorulur.

## Global Constraints

- **Bağımlılık:** Yeni crate ya da npm paketi eklenmez. `unsafe` yoktur.
  `npx prettier` kullanılmaz.
- **Kod kuralları:** Üretimde `unwrap`/`expect` yoktur. Adlar ve metinler
  Türkçedir; kullanıcıya görünen metinler "siz" diliyle yazılır.
- **Commit önekleri:** Çekirdek için `çekirdek:`, uygulama için
  `uygulama:`, arayüz için `ui:`, belgeler için `docs:`.
- **Hakem yalnız onaycıyı değiştirir:** Kararın kendisi değişmez. Taban
  karar (`Oturum::karar`), mühür, politika sürümü ve `tek_calistir`'daki
  yürütme anı denetimi **değişmeden** kalır. Hakemin izin verdiği plan,
  kullanıcının onayladığı planla aynı yoldan yürür.
- **İzin listesi:** Hakeme uygunluk bir izin listesidir (tasarım §3.2).
  Listede olmayan her uyarı, yeni eklenen uyarılar dahil, öğeyi
  kullanıcıya bırakır. Yerel onaylı öğe (`yerel == true`) hakeme **asla**
  gitmez.
- **Kapalı düşme:** Hakemin `Belirsiz` sonucu (hata, zaman aşımı, bütçe
  reddi, bozuk yanıt dahil) öğeyi kullanıcıya gönderir. Hiçbir hata yolu
  işlemi çalıştırmaz.
- **Gönderim önizlemesi:** "Göndermeden önce göster" açıkken
  (`onizleme_acik`) hakem çağrılmaz.
- **Hakeme gitmeyenler:** Araç çıktıları, model metni ve gizli hedefin
  diff'i hakeme gitmez. Kullanıcı isteği yalnız kullanıcının yazdığı
  mesajlardan gelir. Sohbet geçmişinden alınmaz, çünkü compact özetleri
  ve Kalfa notları da kullanıcı rolündedir.
- **Kip yaşam döngüsü:** Otomatik kip oturumluktur. Kurtarılan oturum Sıkı
  açılır. Yeni oturumun varsayılan kipi Otomatik olamaz.
- **Testsiz sınır yok:** Tasarım §9'daki her sınır testiyle gelir
  (AGENTS.md).
- **Canlı istek yok:** Testler sahte sağlayıcıyla koşar. Bu PR'da canlı
  EVREN isteği gerekmez.

## Dosya ve sözleşme haritası

| Dosya | Değişiklik |
|---|---|
| `kalfa-core/src/izin.rs` | `Kip::Otomatik`, taban kurallar, `HAKEME_UYGUN_UYARILAR`, `hakeme_uygun`, otomatik kip uyarı metinleri |
| `kalfa-core/src/oturum/arac_asamasi.rs` | `MCP_TAZE_ONAY_UYARISI` sabitinin `izin.rs`'e taşınması, hakem kapısı, bağlı grup, ret ve geri düşme, `Planlanan.hakem_onayi` |
| `kalfa-core/src/oturum/mod.rs` | Ret sayaçları, son kullanıcı istekleri, kip değişince sıfırlama |
| `kalfa-core/src/hakem.rs` | `ARDISIK_RET_SINIRI`, `TOPLAM_RET_SINIRI`, `risk_aciklamasi` |
| `kalfa-core/src/kayit.rs`, `protokol.rs`, `gecmis/mod.rs`, `kurtarma.rs` | `OtomatikKarar` kaydı, olayı ve görünümü; Otomatik → Sıkı |
| `kalfa-core/tests/otomatik_kip.rs` (yeni) | Oturum düzeyinde sınır testleri |
| `kalfa-uygulama/src/guvenlik.rs` | Kip sırası, adı, yükseltme metni |
| `ui/src/ipc/tipler.ts`, `dil/tr.ts`, `durum/oturum.ts`, `ana/KonusmaOgesi.svelte` | Kip, olay, görünüm öğesi, satır notu |
| `docs/…` | Üst tasarım §7.5, otomatik kip tasarımı ve `GELISTIRME_DURUMU.md` |

Bu tabloda crate yolları `crates/` altında kısaltılmıştır.

## Görev 1: İzin motoru (`izin.rs`)

1. **Testler önce yazılır** (`izin.rs`'in test modülü ya da `tests/izin.rs`):
   - **Taban kurallar:** `kabuk_karari`, `mcp_karari` ve `karar_ver`,
     `Kip::Otomatik` için bütün girdilerde `Kip::Serbest` ile aynı kararı
     verir. Testte iki kip yan yana karşılaştırılır.
   - **Uygun uyarılar:** `hakeme_uygun(&[u], false)` şu sabitlerin her biri
     için tek tek `true` döner:
     - `ILK_KABUK_UYARISI`,
     - `AGLI_KABUK_UYARISI`,
     - `AYNI_ADIM_UYARISI`,
     - `MCP_ILK_CAGRI_UYARISI`,
     - `MCP_AG_UYARISI`,
     - `MCP_TAZE_ONAY_UYARISI`.

     Boş liste de `true` döner.
   - **Uygun olmayan uyarılar:** Aynı çağrı şunların her biri için tek tek
     `false` döner:
     - `GIZLI_UYARISI`,
     - `GERI_ALINAMAZ_UYARISI`,
     - `KORUMALI_UYARISI`,
     - `BAGLANTI_UYARISI`,
     - `YALITIMSIZ_UYARISI`,
     - `ALAN_HAZIRLIGI_UYARISI`,
     - `MCP_TAM_ERISIM_UYARISI`,
     - `YAPILANDIRMA_UYARISI`,
     - `GORUNMEZ_UYARISI` (`gorunum.rs`),
     - bilinmeyen bir dizgi.

     Uygun bir uyarıyla uygun olmayan birinin karışımı da `false` döner.
   - **Yerel onay:** `hakeme_uygun(&[], true)` `false` döner.
2. **Uygulama:**
   - `Kip`'e `Otomatik` eklenir. Serde adı `"Otomatik"`'tir ve değişken en
     sona konur.
   - `kabuk_karari`'daki `kip == Kip::Serbest` koşulu
     `matches!(kip, Kip::Serbest | Kip::Otomatik)` olur.
   - `mcp_karari`'nın `Serbest` kolu `Kip::Serbest | Kip::Otomatik` olur.
   - `karar_ver` zaten yalnız `Siki` ve `Inceleme` için ayrışır; değişmez.
     Testi bunu kanıtlar.
   - `MCP_TAZE_ONAY_UYARISI`, `arac_asamasi.rs`'ten `izin.rs`'e `pub` olarak
     taşınır. Metni aynen kalır.
   - Şunlar eklenir:

     ```rust
     pub const HAKEME_UYGUN_UYARILAR: &[&str] = &[ILK_KABUK_UYARISI,
         AGLI_KABUK_UYARISI, AYNI_ADIM_UYARISI, MCP_ILK_CAGRI_UYARISI,
         MCP_AG_UYARISI, MCP_TAZE_ONAY_UYARISI];
     pub fn hakeme_uygun(uyarilar: &[String], yerel: bool) -> bool
     pub const OTOMATIK_EMIN_DEGIL_UYARISI: &str =
         "Otomatik kip bu işlemden emin olamadı; karar sizin.";
     pub fn otomatik_riskli_uyarisi(risk_aciklamasi: &str) -> String
         // "Otomatik kip bu işlemi riskli buldu: {açıklama}. Karar sizin."
     ```

   - `exhaustive match` hatası veren her yere (`guvenlik.rs` dahil)
     `Otomatik` eklenir. Başka bir davranış değişmez.

Doğrulama:

- `cargo test -p kalfa-core --locked`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`

## Görev 2: Oturum durumu ve hakem yardımcıları

1. **`hakem.rs`'e eklenenler:**
   - `pub const ARDISIK_RET_SINIRI: u32 = 3;`
   - `pub const TOPLAM_RET_SINIRI: u32 = 20;`
   - `pub fn risk_aciklamasi(risk: &str) -> &'static str`, şu eşlemeyle:

     | Risk | Açıklama |
     |---|---|
     | `yikici` | "yıkıcı bir işlem" |
     | `veri_sizdirma` | "veri sızdırma riski" |
     | `kapsam_disi` | "isteğin kapsamı dışında" |
     | `guvenlik_zayiflatma` | "güvenliği zayıflatıyor" |
     | `supheli_kaynak` | "doğrulanmamış kaynaktan kod çalıştırıyor" |
     | diğer | "belirsiz risk" |

     Tablo testiyle gelir.
2. **`Oturum`'a eklenen alanlar** (kayda girmez):
   - `hakem_ret_ardisik: u32`
   - `hakem_ret_toplam: u32`
   - `son_kullanici_istekleri: VecDeque<String>`: en çok 3 öğe, en yeni
     sonda.
3. **Doldurma ve sıfırlama:**
   - `tur_calistir`'ın `TurIsi::Mesaj(metin)` kolunda, `mesaj_ekle`'den önce
     `metin` kuyruğa eklenir. Kuyruk 3'ü aşarsa en eskisi atılır.
     Kurtarmada kuyruk boş başlar.
   - `Komut::IzinKipiDegistir` iki sayacı sıfırlar.

## Görev 3: Hakem kapısı (`arac_asamasi.rs`)

**Yer:** `arac_asamasi` döngüsünde, aynı adım kuralından ve eski planların
kapatılmasından sonra, `if !onayli.is_empty()`'den önce. Kapı yalnız şu
üç koşul birlikte sağlanınca çalışır:

- `self.ayarlar.kip == Kip::Otomatik`,
- `!self.onizleme_acik`,
- `onayli` boş değil.

1. **Aday seçimi:**
   - **Uygun öğeler:** `onayli` öğelerinden `hakeme_uygun(uyarilar, yerel)`
     olanlar adaydır. Burada kartın uyarı listesi kullanılır, planın
     kararındaki liste değil: aynı adım ve taze MCP uyarıları yalnız
     kartta durur.
   - **Bağlı grup:** Bağlı grubun üyeleri onay isteyen kabuk planları ile
     kart uyarısında `AYNI_ADIM_UYARISI` taşıyan yazmalardır.
   - **Gruptaki uygunsuz öğe:** Gruptan bir öğe uygun değilse grubun
     hiçbir üyesi aday olmaz.
   - **Öğe sınırı:** Adaylar `AZAMI_HAKEM_OGESI`'ni aşarsa fazlası aday
     olmaz.
2. **Girdi:**
   - **Kimlikler:** Her aday için bir `HakemOgesi` kurulur. Kimlikler
     `i1`, `i2`… diye verilir; eşleme yerel bir `HashMap`'te tutulur.
   - **Alanlar:**
     - `arac`, `aciklama`, `uyarilar`: kartın değerleri.
     - `hedefler`: mevcut `hedefler(&hazirlik)`.
     - `diff`: `hazirlik.diff`.
     - `gizli_hedef`: yazılan ya da okunan hedeflerden biri
       `siniflandirici.gizli_mi` ise `true`.
     - Kabukta `komut`, `dizin`, `yalitim` ve `ag`: `KabukTarifi`'nden.
       `yalitim` için `"yalıtımlı"` ya da `"yalıtımsız"` yazılır.
     - MCP'de `mcp`: `McpOnayKarti::yeni(…)`'nin `serde_json::to_value`'su.
       Hata olursa `None` olur.
   - **Girdinin kendisi:**
     - `kullanici_istegi`: `son_kullanici_istekleri`.
     - `calisma_alani`: çalışma alanı kökünün **son bileşeni**; tam yol
       değil.
3. **Çağrı:**

   ```rust
   self.iptal_edilebilir(hakem::degerlendir(saglayici.as_ref(), &butce, &girdi))
   ```

   - **Bağımlılıklar:** `saglayici = self.baglam.saglayici.clone()`,
     `butce = self.butce_talepci()`. Bekleme komutları ve bütçe
     taleplerini işler.
   - **İptal:** İptalde bugünkü iptal yolu izlenir: bütün planlar
     `IptalEdildi` olur, sonuç `Bitir(Iptal)`'dir.
4. **Kayıt:**
   - **İstekler:** Her `HakemYaniti.denemeler` için bir `Istek` kaydı
     yazılır. `denetim_sonucu_isle`'deki kalıp izlenir: amaç
     `HAKEM_BUTCE_AMACI`, model `HAKEM_MODELI`, `yeni` için tek
     `GonderilenOge { kaynak: "otomatik kip", sira: None, bayt:
     gonderilen_bayt }`.
   - **Kip değişikliği:** Bekleme sırasında politika değiştiyse (`self.politika`,
     adımın planlarındaki `politika`'dan farklıysa) `onay_topla`'nın
     `politika_degisti` yolu izlenir. Hata iletisi verilir, adımdaki bütün
     planlar `[çalıştırılmadı: izin kipi değişti; gerekirse yeniden dene]`
     ile kapanır ve `AsamaSonucu::Devam` döner.
5. **Sonuç eşleme:**
   - **Bağlı grup:** En kötü sonucu alır. Sıra Kullanıcıya > Ret > İzin'dir.
   - **`Izin`:**
     - Plan `onayli`'den çıkar ve `serbest`'e girer;
       `plan.hakem_onayi = true` olur.
     - `hakem_ret_ardisik = 0` olur.
   - **`Ret`, geri düşme eşiğine gelinmemişse**
     (`ardisik < ARDISIK_RET_SINIRI && toplam < TOPLAM_RET_SINIRI`):
     - Plan `onayli`'den çıkar.
     - `CagriSonucu::plandan(&p, AracDurumu::Reddedildi, icerik)` ile
       `sonuc_isle` çağrılır. İçerik şudur:
       `"[otomatik kip reddetti] {risk_aciklamasi}. Bu işlemi yapmayın; başka bir yol deneyin ya da gerekiyorsa kullanıcıdan isteyin."`
     - İki sayaç birer artar.
   - **`Ret`, geri düşme eşiği aşılmışsa:** Öğe `onayli`'de kalır;
     uyarılarına `otomatik_riskli_uyarisi(açıklama)` eklenir.
   - **`Belirsiz`:** Öğe `onayli`'de kalır; uyarılarına
     `OTOMATIK_EMIN_DEGIL_UYARISI` eklenir.
   - **Kartta biten öğeler:** Kartta kalan öğeler "Kullaniciya" olarak
     kaydedilir.
6. **Onaysız iz:**
   - `Planlanan`'a `hakem_onayi: bool` eklenir; varsayılanı `false`'tur.
   - `onaysiz_isle`'nin erken dönüşü `p.karar != Karar::Serbest &&
     !p.hakem_onayi` olur. Hakemin izin verdiği yazma ve komutlar,
     sonraki kullanıcı kartında "onaysız değişiklikler" listesine girer.
   - `kabuk_onaylandi` hakem izninde **çağrılmaz**.
7. **Kullanıcı onayı:** `onay_topla`'da `OnayKarari::Onayla` ve
   `Komut::YerelOnay` başarıyla onaylandığında `hakem_ret_ardisik = 0` olur.
8. **Kayıt ve olay:**
   - **Ne zaman:** Her hakem sonucu için, öğe yürütülmeden, reddedilmeden
     ya da kart gösterilmeden önce yazılır.
   - **Nasıl:** `KayitOgesi::OtomatikKarar` kaydedilir ve
     `Olay::OtomatikKarar` yayılır (Görev 4).

## Görev 4: Kayıt, olay, kurtarma ve geçmiş

1. **Tip:**

   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
   pub enum OtomatikSonuc { Izin, Ret, Kullaniciya }
   // KayitOgesi ve Olay'a aynı alanlarla:
   OtomatikKarar {
       cagri: CagriId,
       arac: String,
       aciklama: String,
       sonuc: OtomatikSonuc,
       #[serde(default, skip_serializing_if = "Option::is_none")]
       risk: Option<String>,
       #[serde(default, skip_serializing_if = "Option::is_none")]
       olasilik: Option<f64>,
   }
   ```

   - **Konum:** `OtomatikSonuc` `protokol.rs`'te durur. `KayitOgesi` ve
     `Olay` yalnız `PartialEq` türetir, bu yüzden `f64` sorun çıkarmaz.
   - **`aciklama`:** Hazırlığın açıklamasıdır. Satır, araç satırından
     bağımsız okunabilmelidir; reddedilen öğenin araç satırı olmayabilir.
2. **Kurtarma:**
   - `kurtarma.rs`'te `Kip::Serbest` kuralı `matches!(kip, Kip::Serbest |
     Kip::Otomatik)` olur; kurtarılan oturum Sıkı açılır.
   - `OtomatikKarar` kaydı sohbete katılmaz (mevcut `_ => {}` kolu).
   - **Testler** (`tests/oturum_kurtarma.rs`):
     - Otomatik → Sıkı.
     - Kayıttaki bir `OtomatikKarar`, kurtarılan mesajları değiştirmez.
3. **Geçmiş:**
   - `gecmis/mod.rs`'e `GorunumOgesi::OtomatikKarar` eklenir;
     `GuvenlikUyarisi`'nin `sira` ve `tur` alanlarıyla aynı kalıptadır.
   - Geçmiş testine bir örnek eklenir.

## Görev 5: Oturum düzeyinde sınır testleri (`tests/otomatik_kip.rs`)

`SahteSaglayici`'nın karar kuyruğu ve `SahteArac`/`SahteArac::kabuk`
kullanılır. Kalıp `tests/oturum_kabuk.rs`, `tests/onaysiz_degisiklik.rs` ve
`tests/mcp_actor.rs`'tekidir. Tasarım §9'daki her satır ayrı bir testtir.

| Test | Beklenen |
|---|---|
| Hakem izin | `OnayGerekiyor` yok, araç çalışır. Kayıtta `OtomatikKarar { Izin }` ve amacı `otomatik kip` olan `Istek` vardır. |
| Hakem ret | Araç çalışmaz. `AracSonucu` `Reddedildi`'dir, içerik `[otomatik kip reddetti]` ile başlar. |
| Hakem belirsiz | `OnayGerekiyor` gelir; kartın uyarıları `OTOMATIK_EMIN_DEGIL_UYARISI`'nı içerir. |
| Hakem hatası (`Err`) ve bütçe reddi | `OnayGerekiyor` gelir, araç çalışmaz. |
| Uygun olmayan uyarı (gizli dosya okuma) | `karar_istekleri()` boştur, kart gelir. |
| Yalıtımsız kabuk | Kuyrukta "izin" olsa bile `karar_istekleri()` boştur; öğe `yerel_onay: true`'dur. |
| Bağlı grup 1 | Kabuk izin, aynı adım yazması belirsiz → ikisi de kartta. |
| Bağlı grup 2 | Kabuk ret, yazma izin → ikisi de reddedilir, yazma yapılmaz. |
| Bağlı grup 3 | Yalıtımsız kabuk ve aynı adım yazması → `karar_istekleri()` boştur, ikisi de kartta. |
| Önizleme açık | `karar_istekleri()` boştur, kart gelir. |
| Geri düşme | 3 ardışık ret modele gider; 4. ret karta `otomatik_riskli_uyarisi` ile düşer. Toplam sınır için sayaç doğrudan 19'a getirilebiliyorsa 20. ret de sınanır; getirilemiyorsa bu not edilir. |
| Sayaç sıfırlama | Kullanıcı onayı ardışık sayacı sıfırlar; kip değişikliği iki sayacı da sıfırlar. |
| Kip değişikliği | Hakem beklenirken (`karar_gecikme`) kip değişir → hiçbir araç çalışmaz. |
| Araç çıktısı gitmez | Önceki adımın araç çıktısındaki işaret dizgesi `karar_istekleri()` gövdesinde yoktur. |
| Kullanıcı isteği | `state.kullanici_istegi` yalnız yazılan mesajlardır; en çok 3 tanedir. |
| Gizli diff gitmez | Gizli yola aynı adımda yazılırken diff işaret dizgesi gövdede yoktur. |
| Onaysız iz | Hakemin izin verdiği yazma, sonraki kullanıcı kartının `onaysiz_degisiklikler` listesinde görünür. |
| Mühür | Hakem izninden sonra hedef değişirse işlem çalışmaz. `SahteArac` hazırlığı değiştirmeye izin vermiyorsa bu test atlanır ve PR'a yazılır; aynı yol `tests/arac_muhur.rs`'te sınanıyor. |

Doğrulama:

- `cargo test -p kalfa-core --locked`
- workspace clippy ve fmt.

## Görev 6: Uygulama katmanı (`guvenlik.rs`)

1. **Kip sırası:** `sira`'da `Otomatik => 4` olur; Serbest'ten yükseltme
   yerel pencerede onaylanır.
2. **Kip adı:** `kip_adi` "Otomatik" döner.
3. **Yükseltme metni:** `yukseltme_aciklamasi(Otomatik)` tasarım §7'deki
   metni döndürür. Sonuna şu cümle eklenir: "Göndermeden önce göster
   açıkken otomatik kip çalışmaz; her işlem size sorulur."
4. **Testler:**
   - `yukseltme_mi(Serbest, Otomatik)` `true`'dur.
   - `yukseltme_mi(Otomatik, Serbest)` `false`'tur.
   - Metin boş değildir.

Doğrulama:

- `cargo test -p kalfa-uygulama -p kalfa --features e2e --locked`
- `cargo clippy -p kalfa -p kalfa-uygulama --features e2e --all-targets --locked -- -D warnings`

## Görev 7: Arayüz

1. **`ipc/tipler.ts`:**
   - `Kip`'e `'Otomatik'` eklenir. `KIPLER` dizisinin sonuna da eklenir;
     sıra yükseltme sırasıdır.
   - `Olay`'a `OtomatikKarar` eklenir.
   - Geçmiş öğelerine `OtomatikKarar` eklenir.
2. **`dil/tr.ts`:**
   - Kip adı: `'kip.Otomatik': 'Otomatik'` ve `KIP_ADI`.
   - Satır metinleri: "Otomatik kip onayladı: {açıklama}", "Otomatik kip
     reddetti: {açıklama} ({risk})" ve "Otomatik kip size bıraktı:
     {açıklama}".
   - Risk adları için Görev 2'deki açıklamalar kullanılır.
3. **`durum/oturum.ts`:** Canlı olay ve geçmiş öğesi
   `GuvenlikUyarisi`'deki gibi konuşma öğesine dönüşür.
4. **`ana/KonusmaOgesi.svelte`:**
   - Tek satırlık not gösterilir; mevcut güvenlik uyarısı satırının
     biçimi kullanılır.
   - Olasılık yüzde olarak ipucunda gösterilir.
   - CSP kuralı: sabit `style="…"` kullanılmaz, ham HTML basılmaz.
   - Tasarım §6'daki "rozet" bu satırla karşılanır (Claude kararı:
     reddedilen öğenin araç satırı olmayabilir).
5. **Vitest:** Kip menüsünde beş kip görünür. Üç sonuç için de olay ve
   geçmiş öğesi doğru metni üretir. `sahte/arka.ts` yükseltme sırası
   Otomatik'i en üst sayar.
6. **Doğrulama:**

   ```sh
   npm run check && npm test && npm run build
   ```

   `check` 0 hata ve 0 uyarı vermelidir.

## Görev 8: Belgeler

- **Üst tasarım §7.5:** Kip tablosuna Otomatik satırı eklenir; otomatik
  kip tasarımına bağlantı verilir.
- **Otomatik kip tasarımı:** "Durum" alanı güncellenir. §6'ya "rozet
  yerine satır notu" kararı yazılır.
- **`docs/GELISTIRME_DURUMU.md`:** Kısa bir giriş eklenir. Commit'ten önce
  `git diff` ile başkasının düzenlemesi var mı bakılır.

## Bitirme doğrulaması

AGENTS.md'deki varsayılan üç komut, `kalfa-uygulama`/`kalfa` için e2e
özellikli test ve clippy ile arayüz komutları Windows'ta ve WSL'de koşar.

- **Uçtan uca (önerilir):** Kip menüsü değiştiği için `e2e/` takımı
  önerilir. Yerelde koşar; CI kapsamında değildir. Koşturulamazsa bu
  PR'a yazılır.
- **Canlı deneme (isteğe bağlı):** Uygulama gerçek anahtarla açılır, bir
  deneme projesinde Otomatik kip seçilir ve `npm install` gibi bir komut
  istenir. Bunu Claude yapar. Kredi harcamaz (`clef` 1 Kasım'a kadar
  ücretsiz), ancak ana model isteği kredi harcar. Kullanıcı onayıyla
  yapılır.
