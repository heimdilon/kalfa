# e2e ortam bulguları: AppContainer `node` hatası ve Linux gecikme eşiği

Tarih: 2026-10-19. Bağlam: KALFA-27 (PR #32) doğrulaması sırasında düşen
e2e testleri. İki sorun da PR'dan bağımsız; ikisinde de ürün kodu değişmez.

## 1. Windows: yalıtımlı `node` başlamıyor

**Belirti:** `05-kabuk` ("süren komut iptal edilir; torunuyla birlikte durur")
ve `09-ag-cikisi` (S13) düşüyor. Araç sonucu:
`[kalfa-kopru] Komut başlatılamadı: Erişim engellendi. (os error 5)`.
Aynı iki test `main` kodunda da aynı hatayla düştü.

**Kök neden:** Windows e2e derlemesi `KALFA_ARAC_ZINCIRI` vermediği için
`node`'u PATH'ten, yani `C:\Program Files\nodejs\node.exe`'den bulur. Bu dizin
mirassız bir DACL taşıyor ve `ALL APPLICATION PACKAGES`'a açık değil; bu
durum 2c ölçümünde de böyleydi (bulgu 2026-09-25, madde 4). Erişimi Kalfa'nın
yetenek SID'ine `kalfa-yalitim izin-ver` verir. 19 Ekim'de `icacls` iki
dizinde (`C:\Program Files\nodejs`, `C:\Python314`) hiçbir `S-1-15-3-…`
girdisi göstermedi. Kalfa 0.1.1'in kurulumu (18 Ekim, yükseltme) eski
sürümün araç zinciri izinlerini tasarım gereği geri alır
(`kalfa-uygulama/src/yalitim.rs`, `yalitimi_onar`). 14 Ekim'de e2e 41/41
geçerken izinler açıktı.

**Çözüm (kullanıcı, yönetici):** Kalfa'da Ayarlar > Gelişmiş > "Onar" ya da
`kalfa-yalitim izin-ver "C:\Program Files\nodejs"` ve
`kalfa-yalitim izin-ver C:\Python314`. Ürün davranışı doğru: yükseltme izinleri
geri alıyor, "Onar" yeniden açıyor.

## 2. Windows: artık AppContainer profilleri

Makinede dokuz `Kalfa.Alan.*` profili vardı.

- Kurulu Kalfa'nın ACE kaydında olan ikisi: `28af490f46d8dec3`
  (`C:\Users\kor_a\Kalfa Projeleri`, 28 Eylül) ve `bcf1546857a12ab7`
  (`C:\Users\kor_a\kalfa-deneme`, 18 Ekim). Bunlar meşrudur.
- Yedisi 17 Ekim 17:09–17:11 arasında oluşmuş ve hiçbir ACE kaydında yok.
  Bunlar o günkü bir test koşusunun artıklarıdır.
- 19 Ekim'deki iki tam Windows e2e koşusu, düşen kabuk testleri dahil, yeni
  profil bırakmadı. Sızıntı bugün yeniden üretilemedi.

Artıklar kullanıcının onayıyla silinir (`DeleteAppContainerProfile`, yönetici
gerekmez).

## 3. Linux (WSL): `04-kabul` gecikme eşiği

**Belirti:** Tam doğrulama sırasında p95 50–58,6 ms, eşik 50 ms. Hem taban
(`47f3abc`) hem PR başı aynı dağılımı verdi.

**Ölçüm** (aynı makine, Windows tarafında derleme ya da e2e yokken, her commit
üç koşu, p50 / p95 ms):

| Commit | düzenli | yığın | paralel |
|---|---|---|---|
| 14 Ekim `main` `04f16fc` | 28,5–28,7 / 37,4–39,3 | 27,9–29,5 / 37,1–38,3 | 30,0–30,7 / 38,8–40,6 |
| Taban `47f3abc` | 26,2–28,0 / 35,6–38,8 | 26,0–27,6 / 35,2–37,9 | 26,9–28,1 / 36,6–46,4 |
| PR başı `926b136` | 26,9–28,0 / 37,0–38,0 | 25,3–26,7 / 35,3–36,5 | 28,1–28,4 / 38,0–39,7 |

**Sonuç:** Gerileme yok. Yüksek değerler, Windows tarafında aynı anda süren
derleme ve e2e yükünden geliyor. Bu, 2b'deki (p95 63–108 ms) ve 3. aşamadaki
(p95 52–61 ms) gözlemlerle aynıdır. Eşik değişmez. Linux gecikme ölçümü
Windows'ta yük yokken koşturulur.
