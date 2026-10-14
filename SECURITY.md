# Güvenlik politikası

## Desteklenen sürümler

Kalfa henüz hazır derlenmiş bir sürüm yayımlamadı. Güvenlik düzeltmeleri
`main` dalına girer; sürümler yayımlandıktan sonra yalnızca en son sürüm
desteklenir. Sürümler imzasızdır: indirdiğiniz paketi sürümle birlikte
yayımlanan `SHA256SUMS` dosyasıyla doğrulayın.

## Açık bildirme

Bir güvenlik açığı bulduysanız lütfen herkese açık bir issue ya da PR
açmayın. GitHub'ın özel güvenlik bildirimini kullanın: depo sayfasında
**Security → Report a vulnerability**
(<https://github.com/heimdilon/kalfa/security/advisories/new>).

Bildiriminize şunları ekleyin:

- etkilenen sürüm ya da commit ve işletim sistemi,
- açığı yeniden üretme adımları; varsa küçük bir kavram kanıtı,
- açığın etkisi: hangi sınır aşılıyor, saldırganın neye eriştiği.

Bildirime gerçek bir EVREN anahtarı, kimlik bilgisi ya da kişisel veri
koymayın.

Bildirimi aldığımızı yanıtlar, açığı doğrular ve düzeltmeyi planlarız.
Düzeltme yayımlanınca GitHub güvenlik duyurusu yazılır; isterseniz adınız
duyuruda anılır. Düzeltme yayımlanana kadar açığı herkese açık
paylaşmamanızı rica ederiz.

## Kapsam

Kalfa'nın tehdit modeli [üst tasarım belgesi §7.1](docs/superpowers/specs/2026-09-21-kalfa-design.md)'dedir.
Kapsamdaki saldırgan kötü niyetli bir depo, kötü niyetli bir eklenti ya da
araç çıktısıyla gelen talimat enjeksiyonudur. Özellikle şu sınırların
aşılması kapsamdadır:

- izin kipleri ve bağlı onay: onaylanmamış ya da onaydan sonra değişmiş bir
  işlemin çalışması,
- çalışma alanı sınırı ve gizli dosya kuralı,
- kabuk yalıtımı (Linux'ta bubblewrap ve seccomp, Windows'ta AppContainer) ve
  ağ çıkışı sınırı,
- eklenti çalışma zamanının (wasmtime) kaynak ve yetki sınırları,
- arayüzün CSP'si ve pencere yetkileri; EVREN anahtarının saklanması,
- Windows kurucusu ve Linux paketlerinin kurduğu izinler.

Kapsam dışı:

- işletim sisteminin ya da kullanıcı hesabının ele geçirilmiş olması,
  yönetici yetkisine sahip bir saldırgan,
- EVREN hizmetinin kendisindeki açıklar (EVREN'e bildirin),
- Kalfa'yı etkilemeyen, üçüncü taraf bağımlılıklardaki açıklar (o projeye
  bildirin; Kalfa'yı etkiliyorsa bize de yazın).

Bağımlılıklar CI'da `cargo audit` ile RustSec duyurularına karşı denetlenir.
