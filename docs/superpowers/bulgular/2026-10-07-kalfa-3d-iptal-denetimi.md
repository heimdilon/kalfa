# Kalfa 3d — Windows'ta kapta sertifika iptal denetimi

**Tarih:** 2026-10-07
**Tasarım:** [aşama 3 tasarımı](../specs/2026-09-28-kalfa-3-gorunurluk-design.md) §8.5, §8.6
**İlgili:** [G6 bulguları](2026-10-05-kalfa-3d-g6-windows-kopru.md)

**Sonuç:** Ağlı yalıtımlı komutta Schannel kullanan araçlar (cargo, curl),
OCSP stapling yapmayan hostlara bağlanamıyordu. Kalfa artık cargo'ya
`CARGO_HTTP_CHECK_REVOKE=false`, curl'e kabın evindeki `.curlrc` ile
`ssl-revoke-best-effort` verir.

## Belirti

`cargo test -p kalfa-tools --features yalitim-testleri --test uyumluluk
cargo_ozel_registry` ana dalda (`f1c0e32`) düşüyordu. cargo A15'e ulaşmadan,
`Updating crates.io index` adımında şu hatayı alıyordu:

```text
[35] SSL connect error (schannel: next InitializeSecurityContext failed:
CRYPT_E_REVOCATION_OFFLINE (0x80092013))
```

## Ölçümler

Ölçümler Kalfa'nın gerçek AppContainer yalıtımında (`WindowsYalitici`), vekil
ve köprüyle, normal kullanıcıyla yapıldı.

1. **Kabın CryptNet önbelleği kabın kendisine ait ve boş.** Kapta
   `USERPROFILE` ve `LOCALAPPDATA`, çalışma alanına özel sahte eve
   (`<yalitim>\<alan>\ev`) yönlenir. Kapta `certutil -urlcache` GlobalSign
   kaydı göstermiyor. Geliştiricinin `LocalLow\Microsoft\CryptnetUrlCache`'i
   kaptan görünmüyor. "Makinedeki önbelleğin süresi doldu" açıklaması bu
   yüzden geçerli değil.
2. **CryptNet kaptan ağa çıkmayı denemiyor.** Kapta
   `certutil -verify -urlfetch`, `index.crates.io` zincirinin bütün
   adreslerinde hemen `0x800708ca (ERROR_NOT_CONNECTED)` verdi; vekile
   bağlantı gelmedi. CryptNet `HTTPS_PROXY`'ye uymuyor. Vekil ayarını
   WinHTTP'den ya da kullanıcının Internet Settings'inden alıyor; ikisi de
   süreç başına değil. İstenen uçlar (hepsi HTTP, port 80):

   | Sertifika | Tür | Adres |
   |---|---|---|
   | index.crates.io | OCSP | `http://ocsp.globalsign.com/ca/gsatlasr3dvtlsca2025q4` |
   | index.crates.io | CRL | `http://crl.globalsign.com/ca/gsatlasr3dvtlsca2025q4.crl` |
   | Atlas R3 DV TLS CA 2025 Q4 | OCSP | `http://ocsp2.globalsign.com/rootr3` |
   | Atlas R3 DV TLS CA 2025 Q4 | CRL | `http://crl.globalsign.com/root-r3.crl` |

3. **Ayırıcı olan OCSP stapling.** Sunucu iptal yanıtını el sıkışmada
   gönderirse kap dışarı çıkmadan denetim tamamlanıyor. Varsayılan izin
   listesinde, kapta `curl.exe` ile (ana makinede `openssl s_client -status`):

   | Host | Stapling | curl, sıkı | curl, best-effort |
   |---|---|---|---|
   | index.crates.io | yok | ✖ | ✔ 200 |
   | static.crates.io | var | ✔ | ✔ |
   | registry.npmjs.org | yok | ✖ | ✔ 200 |
   | pypi.org | var | ✔ 200 | ✔ 200 |
   | files.pythonhosted.org | var | ✔ | ✔ |
   | codeload.github.com | yok | ✖ | ✔ 301 |
   | objects.githubusercontent.com | yok | ✖ | ✔ 404 |
   | raw.githubusercontent.com | yok | ✖ | ✔ 301 |

4. **2026-10-06'da neden geçiyordu.** Geriye dönük ölçülemedi. En olası
   açıklama, `index.crates.io`'nun (Fastly) o gün stapling yapmasıdır. Fastly
   aynı IP'de `static.crates.io` için stapling yapıyor, `index.crates.io` için
   yapmıyor; bugün altı denemenin hiçbirinde gelmedi.
5. **Etkilenenler:** Yalnızca Schannel kullanan araçlar etkileniyor: cargo
   (`http.check-revoke` yalnızca Windows'ta varsayılan olarak açık) ve curl.
   Git for Windows (OpenSSL arka ucu), npm/Node ve pip/Python çevrimiçi iptal
   denetimi yapmıyor.
6. **`agli_curl_git_npm_node_pip_vekillle_connect_gonderir` TLS'i sınamıyor.**
   Sahte sunucu CONNECT'e "200 OK" deyip TLS konuşmuyor, git
   `http.sslVerify=false` ile çalışıyor. Test yalnızca vekile CONNECT
   gittiğini ölçer.
7. **`cargo_fetch_a15_siniri_beklenir` yanlış nedenle yeşildi.** İddiası
   `error: failed`'ı da kabul ettiği için iptal hatasıyla da geçiyordu.

## Değerlendirilen yollar

| Yol | Karar |
|---|---|
| cargo'ya `CARGO_HTTP_CHECK_REVOKE=false` | **Seçildi.** cargo Linux ve macOS'taki gibi davranır. Zincir ve host adı doğrulaması sürer. |
| curl'e `.curlrc` ile `ssl-revoke-best-effort` | **Seçildi.** İptal bilgisi bulunduğunda (stapling) iptal edilmiş sertifika yine reddedilir. Kapta curl `$HOME\.curlrc`'yi okuyor (ölçüldü). |
| CRL/OCSP hostlarını izin listesine almak, CryptNet'i vekile yönlendirmek | Uygulanamaz. Yönlendirme makine ya da kullanıcı genelinde ayar ister; her CA'nın uçları farklıdır. |
| İptal bilgisini ana makinede önceden alıp depoya koymak | Önerilmez. Kullanıcının sertifika deposuna yazar, OCSP yanıtı depoya konamaz, CDN uçları farklı sertifikalar sunuyor. |
| Bilinen sınır olarak belgelemek | Seçilmedi. Kapta cargo, indekse erişip erişemeyeceği CDN'e bağlı kalırdı. |

Bu iki ayar Kalfa'nın güvenlik sınırını değiştirmez. Sınır, vekilin izin
listesi ve adres denetimidir. Kaptaki komut iptal denetimini zaten kendisi
kapatabilir (`--ssl-no-revoke`, `CARGO_HTTP_CHECK_REVOKE`). İptal denetimi
aracın kendini korumasıdır, Kalfa'yı kaptan korumaz.

## Uygulama

- `vekil_ortami` (`kalfa-sandbox/src/vekil/ortam.rs`): Windows'ta ağlı
  komutun ortamına `CARGO_HTTP_CHECK_REVOKE=false` eklenir; değişkeni kaptaki
  köprü verir.
- `OzelDizinler::hazirla` (`kalfa-sandbox/src/profil.rs`): Windows'ta kabın
  `ev`'ine her çalıştırmadan önce `.curlrc` (`ssl-revoke-best-effort`)
  yazılır. Yazma, `.gitconfig` ve `.npmrc` gibi bağlantı izlemez.
- Testler: `curl_kapta_iptal_bilgisi_olmadan_baglanir` gerçek TLS ile
  `index.crates.io`'ya bağlanır. `cargo_fetch_a15_siniri_beklenir` artık
  indirmeyi ve A15'i birlikte ister. `cargo_ozel_registry_kapta_doldurulamaz`
  yeniden A15'e ulaşıyor.
