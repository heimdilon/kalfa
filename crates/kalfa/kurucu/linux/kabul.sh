#!/bin/sh
# Kalfa .deb paketinin elle kabul denemesi (Linux paket tasarımı §8). Ubuntu
# 24.04 masaüstü VM'inde, sudo yetkili sıradan kullanıcıyla çalıştırın:
#   sh kabul.sh ./Kalfa_<sürüm>_amd64.deb
# Betik paketi kurar, sınar, yeniden kurar ve kaldırır. Yarıda kalırsa geri
# almak için: sudo apt-get remove -y kalfa; profil kaldıysa
# sudo apparmor_parser -R /etc/apparmor.d/kalfa-bwrap; sudo rm -f /etc/apparmor.d/kalfa-bwrap
set -u
paket=${1:?kullanım: sh kabul.sh ./Kalfa_<sürüm>_amd64.deb}
[ "$(id -u)" -ne 0 ] || { echo "Sıradan kullanıcıyla çalıştırın (sudo betik içinde istenir)." >&2; exit 2; }
kalan=0
sonuc() { # $1: 0 geçti; $2: açıklama
  if [ "$1" -eq 0 ]; then echo "GEÇTİ  $2"; else echo "DÜŞTÜ  $2"; kalan=$((kalan + 1)); fi
}
profil_yuklu() { sudo aa-status 2>/dev/null | grep -qx "   $1"; }
kopya=/usr/lib/kalfa/bwrap
kap() { "$kopya" --unshare-user --unshare-pid --unshare-net --ro-bind / / --proc /proc --dev /dev "$@"; }

echo "== Ön koşul"
grep -E '^(PRETTY_NAME)=' /etc/os-release
[ "$(cat /proc/sys/kernel/apparmor_restrict_unprivileged_userns 2>/dev/null)" = 1 ]
sonuc $? "AppArmor ayrıcalıksız ad alanını kısıtlıyor (sysctl = 1)"
if [ -x /usr/bin/bwrap ]; then
  ! /usr/bin/bwrap --unshare-user --ro-bind / / true 2>/dev/null
  sonuc $? "sistem bwrap'ı kurulumdan önce reddediliyor"
fi

echo "== Kurulum"
sudo apt-get install -y "$paket" >/dev/null
sonuc $? "apt kurulumu"
[ -f /etc/apparmor.d/kalfa-bwrap ]; sonuc $? "profil /etc/apparmor.d/kalfa-bwrap'ta"
profil_yuklu kalfa-bwrap; sonuc $? "kalfa-bwrap yüklü"
profil_yuklu kalfa-bwrap-cocuk; sonuc $? "kalfa-bwrap-cocuk yüklü"

echo "== Kalfa'nın bwrap kopyası"
kap true; sonuc $? "kopya ağsız ad alanı açıyor"
# Kopyayı Kalfa dışından çağıran biri kapta yetenek isteyebilir; çocuk profil
# yeteneğin kullanımını reddetmeli (yalnız unconfined bir profil izin verirdi).
! kap --uid 0 --cap-add ALL unshare -n true 2>/dev/null
sonuc $? "kopyayla açılan kapta yetenek (CAP_SYS_ADMIN) kullanılamıyor"
if [ -x /usr/bin/bwrap ]; then
  ! /usr/bin/bwrap --unshare-user --ro-bind / / true 2>/dev/null
  sonuc $? "sistem bwrap'ı hâlâ reddediliyor (profil yalnız kopyaya)"
fi

echo "== Elle: uygulama"
echo "Kalfa'yı uygulama menüsünden açın; Ayarlar'da yalıtım durumu 'Hazır' olmalı."
echo "Bir oturumda ağsız ve ağlı birer kabuk komutu çalıştırın."
printf "Yalıtım 'Yalıtımlı: bwrap …' göründü ve iki komut çalıştı mı? [e/h] "
read -r yanit
[ "$yanit" = e ]; sonuc $? "uygulamada yalıtım hazır, ağsız ve ağlı komut çalıştı (elle)"

echo "== Yeniden kurulum (yükseltme yolu)"
sudo apt-get install -y --reinstall "$paket" >/dev/null
sonuc $? "yeniden kurulum"
profil_yuklu kalfa-bwrap; sonuc $? "profil yeniden kurulumdan sonra yüklü"

echo "== Kaldırma"
# Kullanıcı verisi (kalfa-uygulama::dizin::veri_dizini) kaldırmada kalmalı.
veri="$HOME/.local/share/kalfa"
mkdir -p "$veri" && : > "$veri/.kabul-isaret"
sudo apt-get remove -y kalfa >/dev/null
sonuc $? "apt kaldırması"
[ ! -e /etc/apparmor.d/kalfa-bwrap ]; sonuc $? "profil dosyası kaldırıldı"
! profil_yuklu kalfa-bwrap; sonuc $? "profil çekirdekten kaldırıldı"
[ ! -e "$kopya" ]; sonuc $? "bwrap kopyası kaldırıldı"
[ -e "$veri/.kabul-isaret" ]; sonuc $? "kullanıcı verisi ($veri) kaldı"
rm -f "$veri/.kabul-isaret"

echo "== Sonuç: $kalan düşen"
[ "$kalan" -eq 0 ]
