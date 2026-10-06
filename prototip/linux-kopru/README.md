# Kalfa 3d: Linux ağ köprüsü prototipi

Aşama 3 tasarımının §8.5'indeki köprünün ve vekilin ölçüm prototipi (G5, G7).
Ürün kodu değildir; Kalfa'nın workspace'ine girmez. Sonuçlar:
[bulgu raporu](../../docs/superpowers/bulgular/2026-09-28-kalfa-3d-g5-linux-kopru.md).

## Ne ölçülür

- **Tanımlayıcı taşıma:** köprü, ağ ad alanının içinde `127.0.0.1`'de dinler.
  Kabul ettiği her bağlantının tanımlayıcısını devraldığı Unix soketinden
  (`SCM_RIGHTS`) vekile verir. Vekil soketi doğrudan okur ve yazar.
- **Ad alanı:** komut dışarıya, dış ad alanının geri döngüsüne ve DNS'e
  ulaşamaz.
- **Yaşam:** kanal komuta geçmez. Köprü komutun çıkış koduyla çıkar. Köprü
  çıkınca vekilin kanalı kapanır.
- **Filtre:** köprü, ağlı profilin soket kuralı altında da çalışır.
- **Araçların uyumu:** curl, wget, git, cargo, pip, uv, npm, node, python, go
  ve java `HTTPS_PROXY`'ye uyuyor mu, izinli hostta ClientHello'da SNI ve ECH
  ne?
- **Yapılandırma önceliği:** yapılandırma dosyasındaki bir vekil ayarı
  ortamdakini eziyor mu?
- **Maliyet:** köprü ve vekil üzerinden bağlantı kurma süresi.

## Yöntem

- Kalfa'nın bwrap'ı yerine `unshare --user --map-root-user --net` kullanılır.
  Ağ ad alanı aynıdır.
- bwrap `--unshare-net`'te geri döngüyü kendisi açar. `unshare` açmadığı için
  köprü `--lo-ac` ile açar.
- Komutun ortamı bwrap'taki `--clearenv` gibi boşaltılır.
- **Dışarıya hiçbir bağlantı kurulmaz.** Vekilin sahtesi izinli hostta `200`
  döner, aracın ClientHello'sunu okur ve bağlantıyı kapatır. Listede olmayan
  host `403` alır. Yalnızca `yerel.test`, ölçümün dış ad alanında açtığı
  yerel sunucuya aktarılır.
- Araçlar bu yüzden izinli hostta TLS hatasıyla biter. Ölçülen, vekile ne
  gönderdikleridir.

## Çalıştırma

Linux'ta; kullanıcı ad alanı açılabilmeli. Bağımlılık yalnızca `libc`'dir.

```sh
cargo build --offline --release
./target/release/kalfa-kopru-prototip olc --cikti sonuc.md
```

Önek verilirse yalnızca o gruplar koşar: `olc o1 o7`. Gruplar:

| Grup | Konu |
|---|---|
| o1 | Ağ ad alanı |
| o2 | Tanımlayıcı taşıma ve aktarım |
| o3 | Kanal komuta geçmez |
| o4 | Çıkış kodu ve yaşam süresi |
| o5 | Ağlı profilin soket kuralı altında köprü |
| o6 | `NO_PROXY` ve kabın içindeki sunucular |
| o7 | Araçların vekile uyumu (G5) ve ClientHello (G7) |
| o8 | Yapılandırma dosyasındaki vekil ortamı ezer mi |
| o9 | Bağlantı kurma maliyeti |

o7 ve o8, araçların kurulu olmasını ister; olmayan araç "başlatılamadı" diye
raporlanır.

## Bilinen farklar

Prototip `unshare` ile çalışır. Aşağıdakiler bwrap'la WSL'de, `kalfatest`
kullanıcısıyla yinelenecek:

- **Kanalın geçişi:** bwrap'ta kanalın bwrap'tan köprüye geçmesi.
- **Süreç ad alanı:** bwrap `--unshare-pid` ile köprü `pid 2`, komut `pid 3`
  olur; köprü çıkınca kabın bütün süreçleri ölür. Prototipte süreç ad alanı
  ayrı değildir.
- **Kullanıcı:** prototip ad alanında `root`'tur; bwrap kullanıcıyı kendi
  kimliğiyle eşler.
- **Filtre:** Kalfa'nın gerçek seccomp filtresi yerine, ağlı profilin soket
  kuralını taklit eden küçük bir filtre kullanılır.
