// Arayüzün Türkçe metin kataloğu (1c tasarımı §9.4). Kurallar: "siz" dili;
// ünlem ve emoji yok; biten iş "Tamamlandı" ile özetlenir; hata metni ne
// olduğunu ve ne yapılacağını söyler. Değişkenler `{ad}` biçimindedir.
import type {
  AktarimDurmaNedeni,
  AracDurumu,
  DenetimKipi,
  Kip,
  ModelProfili,
  YalitimYoklukNedeni,
} from '../ipc/tipler';

export const tr = {
  'uygulama.adi': 'Kalfa',

  'mesaj.kullanici': 'Siz',
  'mesaj.kalfa': 'Kalfa',

  'dusunce.baslik': 'Düşünce',
  'dusunce.ozet': 'Modelin bu yanıt için düşündükleri',

  'composer.yer_tutucu': 'Bir şey sorun ya da bir görev tanımlayın…',
  'composer.gonder': 'Gönder',
  'composer.durdur': 'Turu durdur',
  'composer.model': 'Model',
  'composer.kip': 'İzin kipi',

  'kip.Inceleme': 'İnceleme',
  'kip.Siki': 'Sıkı',
  'kip.Proje': 'Proje',
  'kip.Serbest': 'Serbest',

  'model.Hizli': 'Hızlı',
  'model.Dengeli': 'Dengeli',
  'model.Derin': 'Derin',
  'model.Otomatik': 'Otomatik',

  'arac.durum.Suruyor': 'Çalışıyor',
  'arac.durum.Basarili': 'Tamamlandı',
  'arac.durum.Basarisiz': 'Başarısız',
  'arac.durum.Reddedildi': 'Reddedildi',
  'arac.durum.IptalEdildi': 'İptal edildi',
  'arac.durum.SonucuBilinmiyor': 'Sonucu bilinmiyor',
  'arac.durum.Baslamadi': 'Çalıştırılmadı',

  'tur.bitti.Normal': 'Tamamlandı',
  'tur.bitti.Iptal': 'İptal edildi',
  'tur.bitti.Sinir': 'Sınıra ulaşıldı: {aciklama}',
  'tur.bitti.Hata': 'Hatayla bitti',

  'aktarim.TuketiciYetismedi':
    'Arayüz uzun süre yanıt vermediği için oturum durduruldu. Oturumu yeniden açabilirsiniz.',
  'aktarim.OlayCokBuyuk':
    'Bu adımın çıktısı gösterilemeyecek kadar büyük olduğu için oturum durduruldu. Oturumu yeniden açabilirsiniz.',
  'aktarim.Diger': 'Oturumun olay aktarımı durdu. Oturumu yeniden açabilirsiniz.',

  'hata.anahtar_gecersiz':
    'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
  'hata.evren_ulasilamiyor': "EVREN'e ulaşılamıyor. Bağlantınızı kontrol edip yeniden deneyin.",
  'hata.anahtar_bellekte':
    'Anahtar bu oturum boyunca bellekte tutulacak. Kalfa kapanınca yeniden girmeniz gerekecek.',
  'hata.kayit_mesgul': 'Bu oturum başka bir Kalfa sürecinde açık.',
  'hata.calisma_alani_yok': 'Çalışma alanı bulunamadı: {yol}',
  'hata.calisma_alani_kayitsiz':
    'Bu kayıt çalışma alanını yazmıyor; yalnızca geçmişi gösterilebilir.',
  'hata.sart_guncellendi': 'Şart metni bu sırada güncellendi.',
  'hata.baglanti_yok':
    'EVREN bağlantısı yok. Geçmiş oturumları okuyabilirsiniz; mesajları bağlantı gelince gönderin.',

  'markdown.gorsel': 'görsel',
  'kod.kirpildi': 'Çıktının devamı gösterilmiyor',
  'uygulama.slogan': 'Daha iyi yazılım, daha aydınlık yarınlar.',
  'uygulama.alt_not': 'Küçük adımlar, büyük işler.',
  'genel.kapat': 'Kapat',
  'zaman.dun': 'Dün',

  'menu.ad': 'Menü',
  'menu.oturumlar': 'Oturumlar',
  'menu.ayarlar': 'Ayarlar',
  'menu.anahtar': 'Anahtarı değiştir',
  'menu.gunluk': 'Gönderim günlüğü',

  'liste.baslik': 'Oturumlar',
  'liste.yeni': 'Yeni oturum',
  'liste.ara': 'Oturumlarda ara…',
  'liste.bos': 'Henüz oturum yok. Yeni bir oturumla başlayın.',
  'liste.sonuc_yok': 'Aramaya uyan oturum yok.',
  'liste.sil': '{baslik} oturumunu sil',
  'liste.grup.bugun': 'Bugün',
  'liste.grup.dun': 'Dün',
  'liste.grup.hafta': 'Bu hafta',
  'liste.grup.eski': 'Daha eski',

  'oturum.yeni': 'Yeni oturum',
  'oturum.durum.calisiyor': 'Çalışıyor',
  'oturum.durum.onay': 'Onay bekliyor',
  'oturum.durum.sart': 'Şart onayı bekliyor',
  'oturum.durum.tamamlandi': 'Tamamlandı',
  'oturum.durum.iptal': 'İptal edildi',
  'oturum.durum.hata': 'Hatayla bitti',
  'oturum.durum.yarim': 'Yarım kaldı',
  'oturum.durum.okunamadi': 'Okunamadı',
  'oturum.durum.bos': 'Boş',

  'konusma.ad': 'Konuşma',
  'konusma.kapat': 'Oturumu kapat',
  'konusma.yeniden_ac': 'Yeniden aç',
  'konusma.eskiler': 'Daha eski mesajları göster',
  'konusma.bos': 'Bu oturumda henüz mesaj yok.',
  'konusma.kapandi': 'Oturum kapandı. Devam etmek için yeniden açın.',
  'konusma.gunluk': 'Gönderim günlüğü',
  'konusma.onizleme_ayari': 'Göndermeden önce göster',
  'konusma.karsilama.baslik': 'Bir oturum seçin ya da yeni bir oturum başlatın.',
  'konusma.karsilama.metin':
    'Kalfa seçtiğiniz klasörde çalışır. Dosya yazmadan önce onayınızı ister.',

  'arac.dosya_oku.suruyor': 'Dosya okunuyor',
  'arac.dosya_oku.bitti': 'Dosya okundu',
  'arac.dosya_oku.ad': 'Dosya okuma',
  'arac.dosya_listele.suruyor': 'Dizin listeleniyor',
  'arac.dosya_listele.bitti': 'Dizin listelendi',
  'arac.dosya_listele.ad': 'Dizin listeleme',
  'arac.icerik_ara.suruyor': 'İçerik aranıyor',
  'arac.icerik_ara.bitti': 'İçerik arandı',
  'arac.icerik_ara.ad': 'İçerik arama',
  'arac.dosya_yaz.suruyor': 'Dosya yazılıyor',
  'arac.dosya_yaz.bitti': 'Dosya yazıldı',
  'arac.dosya_yaz.ad': 'Dosya yazma',
  'arac.dosya_duzenle.suruyor': 'Dosya düzenleniyor',
  'arac.dosya_duzenle.bitti': 'Dosya düzenlendi',
  'arac.dosya_duzenle.ad': 'Dosya düzenleme',
  'arac.kabuk.suruyor': 'Komut çalışıyor',
  'arac.kabuk.bitti': 'Komut çalıştı',
  'arac.kabuk.ad': 'Komut',

  'terminal.etiket': 'Komut çıktısı',
  'terminal.atilan': 'En eski {sayi} satır gösterilmiyor.',
  'terminal.kesik': 'Satırın devamı gösterilmiyor; model çıktının tamamını alır.',

  'onay.kart.baslik': 'Onay isteği: {sayi} değişiklik',
  'onay.kart.incele': 'İncele',
  'onay.kart.yanitlandi': 'Yanıtlandı',
  'onay.kart.gecersiz': 'Tur bitti; karar gerekmiyor',
  'onay.kart.uyari': '{sayi} uyarı',
  'onay.sayfa.baslik': 'Kalfa {sayi} değişiklik yapmak istiyor.',
  'onay.sayfa.alt':
    'Devam etmeden önce değişiklikleri inceleyin. Seçmediğiniz değişiklikler gerekçenizle reddedilir.',
  'onay.sayfa.tumunu_sec': 'Tümünü seç',
  'onay.sayfa.secili': '{secili} / {toplam} seçili',
  'onay.sayfa.goster': '{ad} değişikliğini göster',
  'onay.sayfa.diff_yok': 'Bu değişiklik için önizleme yok.',
  'onay.sayfa.ne': 'Ne',
  'onay.sayfa.nerede': 'Nerede',
  'onay.sayfa.veri_cikisi': 'Veri çıkışı',
  'onay.veri.DosyaIcerigi': 'Dosya içeriği',
  'onay.veri.KomutCiktisi': 'Komut çıktısı',
  'onay.veri.AramaSonuclari': 'Arama sonuçları',
  'onay.veri.DosyaAdlari': 'Dosya adları',
  'onay.veri.YalnizcaYazildi': 'Yalnızca "yazıldı" bilgisi',
  'onay.veri.Bilinmiyor': 'Araç sonucu',
  'onay.sayfa.uyarilar': 'Uyarılar',
  'onay.sayfa.geri_alinabilir': 'Geri alınabilir mi',
  'onay.sayfa.hepsi_geri_alinabilir': 'Bu değişiklikler geri alınabilir.',
  'onay.geri.Evet': 'Evet: turu geri alarak',
  'onay.geri.Buyuk': "Hayır: dosya 8 MiB'tan büyük",
  'onay.geri.Kayitsiz': 'Hayır: kayıtsız oturum',
  'onay.geri.Kabuk': 'Hayır: komutların etkileri geri alınamaz',
  'onay.geri.VeriSilindi': 'Hayır: önceki hâl silindi',
  'onay.sayfa.gerekce': 'Reddetme nedeni (isteğe bağlı)',
  'onay.sayfa.gerekce_yer': 'Reddetme nedeninizi buraya yazabilirsiniz…',
  'onay.sayfa.tumunu_onayla': 'Tümünü onayla',
  'onay.sayfa.secilenleri_onayla': 'Seçilenleri onayla',
  'onay.sayfa.reddet': 'Reddet',
  'onay.sayfa.riskli':
    'Ağa çıkan, derleme dosyası değişmiş ya da görünmeyen karakter taşıyan işlemler tek tek seçilerek onaylanır.',
  'onay.kabuk.onaysiz': 'Bu oturumda onayınız olmadan değişen dosyalar',
  'onay.kabuk.onaysiz_yok': 'Yok',
  'onay.kabuk.onaysiz_komut': 'Onaysız bir komut da çalıştı; değiştirdiği dosyalar bilinmiyor.',
  'onay.kabuk.derleme':
    'Derleme ya da betik dosyası değişti; komut bu kodu çalıştırabilir: {dosyalar}',
  'onay.kabuk.komut': 'Komut',
  'onay.kabuk.betik': 'Betik ({yorumlayici})',
  'onay.kabuk.dizin': 'Dizin',
  'onay.kabuk.ag': 'Ağ',
  'onay.kabuk.ag_kapali': 'Kapalı',
  'onay.kabuk.ag_acik': 'Açık: komut internete erişebilecek',
  'onay.kabuk.ag_yalitimsiz': 'Sınırsız (yalıtımsız)',
  'onay.kabuk.ag_varsayilan':
    'Açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri',
  'onay.kabuk.ag_ek_adresler':
    'Açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri ve şu adresler:',
  'onay.kabuk.yalitim': 'Yalıtım',
  'onay.kabuk.yalitimli': 'Yalıtımlı ({arka_uc})',
  'onay.kabuk.yalitimsiz': 'Yalıtımsız',
  'onay.kabuk.yerel':
    'Yalıtımsız komutlar ve çalışma alanının yalıtım için hazırlığı, onayınızdan sonra yerel bir pencerede bir kez daha sorulur.',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
  'sart.kart.bekliyor':
    'EVREN, şartlar kabul edilene kadar isteği yanıtlamıyor. Tur, şart penceresindeki kararınızı bekliyor.',
  'sart.kart.cozuldu': 'Şart kararı verildi.',
  'sart.kart.ac': 'Şart penceresini aç',

  'hata.kart.baslik': 'Tur hatayla bitti',
  'guvenlik.kart.baslik': 'Güvenlik uyarısı',
  'geri_alma.yapildi': 'Tur geri alındı: {sayi} dosya önceki hâline döndü.',
  'geri_alma.dokunulmadi': '{sayi} dosyaya dokunulmadı.',
  'geri_alma.yarim': 'Geri alma yarım kaldı; turu yeniden geri alabilirsiniz.',
  'geri_alma.not': 'Modele bildirildi: {metin}',
  'geri_alma.sayfa.baslik': 'Turu geri al',
  'geri_alma.sayfa.alt':
    'Kalfa bu turun dosya değişikliklerini geri alacak. Sonradan değişen dosyalara dokunulmaz.',
  'geri_alma.sayfa.tur': 'Tur: {baslik}',
  'geri_alma.sayfa.goster': '{ad} için ne olacağını göster',
  'geri_alma.sayfa.diff_yok': 'Bu dosya için önizleme yok.',
  'geri_alma.sayfa.yapilacak_yok': 'Geri alınacak bir değişiklik yok.',
  'geri_alma.sayfa.veri_silindi':
    'Bu turun bazı önceki hâlleri depo sınırı yüzünden silindi; o dosyalar geri alınamaz.',
  'geri_alma.sayfa.komutlar': 'Kabukla yapılan değişiklikler',
  'geri_alma.sayfa.komut_dosyalari': '{sayi} dosya değişti',
  'geri_alma.sayfa.geri_al': 'Geri al',
  'geri_alma.sayfa.vazgec': 'Vazgeç',
  'geri_alma.eylem.GeriYukle': 'Önceki hâline döner',
  'geri_alma.eylem.Sil': 'Silinir; bu turda oluşturulmuştu',
  'geri_alma.eylem.ZatenOnceki': 'Zaten önceki hâlinde',
  'geri_alma.eylem.Cakisma': 'Çakışma; dosyaya dokunulmaz',
  'geri_alma.eylem.GeriAlinamaz': 'Geri alınamaz',
  'geri_alma.cakisma.SonrakiTur': 'Bu dosya sonraki bir turda da değişti; önce o turu geri alın.',
  'geri_alma.cakisma.Komut': 'Bir komut bu dosyayı değiştirdi.',
  'geri_alma.cakisma.BasarisizCagri':
    'Dosya, başarısız bir çağrının yazmak istediği hâlde; bu hâli Kalfa mı yazdı, siz mi, bilinmiyor.',
  'geri_alma.cakisma.Elle': "Dosya Kalfa'dan sonra değişti.",
  'geri_alma.cakisma.HardLink':
    'Dosyanın başka bir adı var (hard link); geri yazmak öteki adı da değiştirirdi.',
  'geri_alma.cakisma.Okunamadi': 'Dosyanın şimdiki hâli okunamadı: {hata}',
  'geri_alma.neden.Buyuk': "Dosya 8 MiB'tan büyüktü; önceki hâli saklanmadı.",
  'geri_alma.neden.VeriSilindi': 'Önceki hâli depo sınırı yüzünden silindi.',
  'geri_alma.neden.Kayitsiz': 'Kayıtsız oturumda önceki hâller saklanmaz.',
  'geri_alma.neden.Kabuk': 'Komutların etkileri geri alınamaz.',
  'tur_ozeti.baslik': 'Tur özeti',
  'tur_ozeti.geri_al': 'Turu geri al…',
  'tur_ozeti.dosya': '{yol} +{eklenen} −{silinen}',
  'tur_ozeti.yeni': 'yeni',
  'tur_ozeti.bilinmiyor': 'yazmanın sonucu bilinmiyor',
  'tur_ozeti.geri_alinamaz': 'geri alınamaz',
  'tur_ozeti.komut': '{komut}: {sayi} dosya değişti',
  'tur_ozeti.komut_degismedi': '{komut}: dosya değişmedi',
  'tur_ozeti.istekler': '{sayi} EVREN isteği, {token} token',
  'tur_ozeti.guvenlik': '{sayi} güvenlik uyarısı',
  'tur_ozeti.not_kabuk': 'Kabukla yapılan değişiklikler geri alınamaz.',
  'tur_ozeti.not_eksik': 'Liste eksik olabilir.',
  'tur_ozeti.not_geri_alindi': 'Bu tur geri alındı.',
  'tur_ozeti.ag_host': 'Ağ ({host}:{port}): {sayi} bağlantı, {bayt}',
  'tur_ozeti.ag_host_ret': 'Ağ ({host}:{port}): {sayi} bağlantı reddedildi',
  'tur_ozeti.ag_host_kismi_ret': 'Ağ ({host}:{port}): {sayi} bağlantı ({ret} reddedildi), {bayt}',
  'aktarim.baslik': 'Aktarım durdu',

  'onizleme.kart.baslik': 'Gönderim önizlemesi ({sayi} öğe)',
  'onizleme.kart.denetim_baslik': 'Gönderim önizlemesi (Güvenlik denetimi, {sayi} öğe)',
  'onizleme.kart.gonder': 'Gönder',
  'onizleme.kart.isaretlileri_cikar': 'İşaretlileri çıkarıp gönder',
  'onizleme.kart.isaret_ipucu': 'İşaretli öğeler gönderilmez; yerine "kullanıcı bu içeriği göndermedi" gider.',
  'denetim.rozet.suruyor': 'Denetleniyor…',
  'denetim.rozet.supheli': 'Şüpheli: {kategori}',
  'denetim.rozet.bilinmiyor': 'Denetlenemedi',
  'denetim.rozet.atlandi': 'Denetlenmedi: gizli olabilir',
  'denetim.kategori.talimat_enjeksiyonu': 'talimat enjeksiyonu',
  'denetim.kategori.veri_sizdirma': 'veri sızdırma',
  'denetim.kategori.yetki_yukseltme': 'yetki yükseltme',
  'denetim.kategori.diger': 'diğer',
  'denetim.kategori.yok': 'belirtilmedi',
  'durum.denetci': 'Denetçi: {kip}',
  'durum.denetci_ipucu': 'Güvenlik denetçisinin kipi. Dış kaynaklı araç sonuçları modele gitmeden önce bir modelle denetlenir.',
  'durum.denetci_ayar_hatasi': '{hata}',
  'denetci.kip.Kapali': 'kapalı',
  'denetci.kip.Uyar': 'uyar',
  'denetci.kip.Kati': 'katı',
  'onizleme.kart.yanitlandi': 'Yanıtlandı',
  'onizleme.kart.gecersiz': 'Tur bitti; karar gerekmiyor',

  'gunluk.baslik': 'Gönderim günlüğü',
  'gunluk.daha_eski': 'Daha eski istekler',
  'gunluk.ayrinti_yok': 'Ayrıntı yok',
  'gunluk.onceki_mesaj': '+{sayi} önceki mesaj',
  'gunluk.kolon.zaman': 'Zaman',
  'gunluk.kolon.amac': 'Amaç',
  'gunluk.kolon.model': 'Model',
  'gunluk.kolon.token': 'Token',
  'gunluk.kolon.istek_id': 'İstek kimliği',
  'gunluk.kolon.sonuc': 'Sonuç',
  'gunluk.amac.ana': 'Ana model',
  'gunluk.amac.denetci': 'Denetçi',
  'gunluk.sonuc.basarili': 'Başarılı',
  'gunluk.sonuc.hata': 'Hata: {mesaj}',
  'gunluk.metin_yukleniyor': 'Yükleniyor…',
  'gunluk.metin_baslik': 'Gönderilen içerik',

  'durum.ad': 'Durum çubuğu',
  'durum.hazir': 'Hazır',
  'durum.calisiyor': 'Çalışıyor',
  'durum.baglanti_yok': 'Bağlantı yok',
  'durum.anahtar_bellekte': 'Anahtar bellekte',
  'durum.bakiye': '{kalan} kredi · {saat}',
  'durum.bakiye_ipucu': 'Son gözlenen bakiye ve gözlem zamanı',
  'durum.token': '{sayi} token',
  'durum.token_ipucu': 'İstem {istem} · Tamamlama {tamamlama} · Düşünce {dusunce}',
  'durum.model_ipucu': 'Yönlendirilen model',
  'durum.yalitimli': 'Yalıtımlı',
  'durum.yalitimsiz': 'Yalıtımsız',
  'durum.komutlar_kapali': 'Komutlar kapalı',
  'durum.komutlar_kapali_ipucu':
    'Bu bilgisayarda komutlar güvenle çalıştırılamıyor. Ayrıntı için tıklayın.',
  'durum.yalitimsiz_ipucu':
    'Yalıtım yok; komutlar onayınızla yalıtımsız çalışıyor. Ayrıntı için tıklayın.',
  'durum.yalitimli_ipucu': 'Komutlar yalıtımda çalışıyor ({arka_uc})',

  'ayarlar.baslik': 'Ayarlar',
  'ayarlar.yalitim': 'Yalıtım',
  'ayarlar.yalitim.hazir': 'Yalıtımlı: {arka_uc} {surum}',
  'ayarlar.yalitim.hazir_aciklama':
    'Komutlar yalıtımda çalışır. Ev dizininiz ve diğer çalışma alanları görünmez, çalışma alanı dışına yazılamaz, ağ kapalıdır; ağ isteyen komut ayrıca onayınıza sunulur.',
  'ayarlar.yalitim.yok': 'Yalıtımsız: {neden}',
  'ayarlar.yalitim.kapali': 'Komutlar kapalı: {neden}',
  'ayarlar.yalitim.sade.BwrapYok':
    'Komutları güvenle çalıştırmak için gereken bubblewrap bileşeni bu bilgisayarda kurulu değil.',
  'ayarlar.yalitim.sade.KullaniciAdAlaniKapali':
    "Bu bilgisayarın güvenlik ayarları, Kalfa'nın komutları güvenle çalıştırmasını engelliyor.",
  'ayarlar.yalitim.sade.KurulumGerekli':
    "Kalfa'nın komutları güvenle çalıştırması için gereken kurulum tamamlanmamış.",
  'ayarlar.yalitim.sade.KurulumBozuk':
    "Bu bilgisayarda Kalfa'nın komutları güvenle çalıştırması engellenmiş ya da kurulumu bozulmuş.",
  'ayarlar.yalitim.sade.Desteklenmiyor': 'Bu sistemde Kalfa komutları güvenle çalıştıramıyor.',
  'ayarlar.yalitim.sade.kapali':
    'Kalfa sohbet edebilir ve dosyaları düzenleyebilir, ama komut çalıştıramaz. BT departmanınızla ya da bilgisayarınızı yöneten kişiyle görüşün ve teknik ayrıntıları iletin.',
  'ayarlar.yalitim.sade.acik':
    'Yalıtımsız çalıştırma açık: her komut yerel pencerede onayınızı ister ve sizin bütün yetkilerinizle çalışır.',
  'ayarlar.yalitim.kopyala': 'Teknik ayrıntıları kopyala',
  'ayarlar.yalitim.kopyalandi': 'Teknik ayrıntılar panoya kopyalandı.',
  'ayarlar.yalitim.kopyalanamadi':
    'Panoya kopyalanamadı. Aşağıdaki teknik ayrıntıları seçip kopyalayın.',
  'ayarlar.yalitim.teknik': 'Teknik ayrıntılar',
  'ayarlar.yalitim.onar': 'Onar',
  'ayarlar.yalitim.onar_ipucu':
    'Önce "Onar"ı deneyin: Kalfa kurulumu yineler, Windows sizden yönetici izni ister.',
  'ayarlar.yalitim.onarildi': 'Yalıtım onarıldı; komutlar yeniden güvenle çalışabilir.',
  'ayarlar.yalitim.onarim_yetmedi':
    'Kurulum yinelendi ama yalıtım hâlâ kullanılamıyor. BT departmanınızla görüşün ve teknik ayrıntıları iletin.',
  'ayarlar.yalitim.onarim_basarisiz':
    'Onarım tamamlanamadı: {mesaj} BT departmanınızla görüşün ve teknik ayrıntıları iletin.',
  'ayarlar.yalitim.neden.BwrapYok': 'bubblewrap bulunamadı',
  'ayarlar.yalitim.neden.KullaniciAdAlaniKapali': 'sistem kullanıcı ad alanlarını kısıtlıyor',
  'ayarlar.yalitim.neden.KurulumGerekli': 'yalıtımın kurulması gerekiyor',
  'ayarlar.yalitim.neden.KurulumBozuk': 'yalıtımın kurulumu bozuk',
  'ayarlar.yalitim.neden.Desteklenmiyor': 'bu sistemde kullanılamıyor',
  'ayarlar.yalitim.yeniden_dene': 'Yeniden dene',
  'ayarlar.geri_alma': 'Geri alma verisi',
  'ayarlar.geri_alma.aciklama':
    'Turları geri alabilmek için dosyaların önceki hâlleri bu bilgisayarda saklanır. Bir oturumu sildiğinizde onun verisi de silinir.',
  'ayarlar.geri_alma.boyut': 'Toplam: {boyut}',
  'ayarlar.arac_zinciri': 'Araç zinciri dizinleri',
  'ayarlar.arac_zinciri.aciklama':
    "Yalıtımda salt okunur görünen dizinler. Cargo ve rustup araç zincirleri kendiliğinden görünür; başka bir yere kurulan araçlar (örneğin /opt/node) için dizini buradan ekleyin. Ev dizininiz ve Kalfa'nın veri dizini eklenemez.",
  'ayarlar.arac_zinciri.ekle': 'Dizin ekle',
  'ayarlar.arac_zinciri.kaldir': '{yol} dizinini kaldır',
  'ayarlar.arac_zinciri.bos': 'Eklenmiş dizin yok.',
  'ayarlar.arac_zinciri.windows': 'İzinli araç zinciri dizinleri',
  'ayarlar.arac_zinciri.onarim_gerekli':
    "Listedeki bir dizin artık Kalfa'nın komutlarınca okunamıyor; bu genellikle Kalfa güncellenince olur. Onar izinleri yeniden açar ve Windows yönetici onayı ister.",
  'ayarlar.arac_zinciri.windows_aciklama':
    "Bilgisayarınızda kurulu araçlar (Python, Node, Rust, Visual Studio). Kalfa kendi Python ve Node'unu getirir; bunlara yalnızca başka bir sürüm gerekirse ihtiyaç vardır. Yöneticiye ait bir dizin için Windows yönetici onayı ister.",
  'ayarlar.yalitim.calisma_alanlari': 'Hazırlanmış çalışma alanları',
  'ayarlar.yalitim.calisma_alanlari.aciklama':
    'Yalıtım için hazırlanmış çalışma alanlarının izinlerini buradan geri alabilirsiniz.',
  'ayarlar.yalitim.calisma_alanlari.kaldir':
    '{yol} izinlerini geri al ve kaldır',
  'ayarlar.yalitim.calisma_alanlari.bos': 'Hazırlanmış çalışma alanı yok.',
  'ayarlar.yalitim.calisma_alanlari.yukleniyor': 'Çalışma alanları yükleniyor…',
  'ayarlar.yalitim.calisma_alanlari.yuklenemedi': 'Çalışma alanları yüklenemedi.',
  'ayarlar.gelismis': 'Gelişmiş',
  'ayarlar.yalitimsiz': 'Yalıtım yokken komutların yalıtımsız çalışmasına izin ver',
  'ayarlar.yalitimsiz.aciklama':
    'Kapalıyken, yalıtım kullanılamadığında Kalfa komut çalıştırmaz; sohbet ve dosya düzenleme sürer. Açıkken her komut yerel pencerede onayınızı ister ve sizin bütün yetkilerinizle çalışır.',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
  'duyuru.sart': 'EVREN şartlarının onayı bekleniyor.',
  'duyuru.tur_sonu': 'Tur bitti.',

  'anahtar.baslik': 'EVREN API anahtarı',
  'anahtar.aciklama.Yok':
    "Kalfa EVREN'e sizin anahtarınızla bağlanır. Anahtarınızı EVREN portalından alabilirsiniz.",
  'anahtar.aciklama.KayitliGecersiz':
    'Kayıtlı anahtar artık geçerli değil. EVREN portalından yeni anahtarınızı alıp girin.',
  'anahtar.aciklama.Degistir': 'Yeni anahtarınızı girin. Açık oturumlar yeni anahtarı kullanır.',
  'anahtar.etiket': 'Anahtar',
  'anahtar.not':
    'Anahtar işletim sisteminin kimlik deposunda saklanır. Kalfa onu arayüze geri göndermez.',
  'anahtar.kaydet': 'Kaydet',

  'sart.baslik': 'EVREN kullanım şartları',
  'sart.surum': 'Sürüm {surum}',
  'sart.esasli': 'Esaslı değişiklik',
  'sart.esasli_degil': 'Küçük değişiklik',
  'sart.aciklama':
    "Kalfa'yı kullanmak için EVREN'in şartlarını kabul etmeniz gerekir. Metin EVREN'den geldiği gibi gösterilir.",
  'sart.kabul': 'Kabul ediyorum',
  'sart.reddet': 'Kabul etmiyorum',

  'kilitli.baslik': 'Şartlar kabul edilmedi',
  'kilitli.metin':
    "EVREN, şartlar kabul edilmeden hiçbir isteği yanıtlamıyor. Bekleyen turlar iptal edildi; oturumlarınız kayıtlı. Anahtarınız saklı kalır; Kalfa'yı bir sonraki açışınızda şartlar yeniden gösterilir.",
  'kilitli.goster': 'Metni yeniden göster',

  'sartlar.tamam': 'Tamamlandı. Kalfa açılıyor.',
  'sartlar.alt_not': 'Kalfa dikkatli çalışır. Kontrol her zaman sizdedir.',
} as const;

export type MetinAnahtari = keyof typeof tr;

/** Katalogdaki metni verir; `{ad}` yer tutucularını `degerler` ile doldurur. */
export function m(anahtar: MetinAnahtari, degerler: Record<string, string | number> = {}): string {
  return tr[anahtar].replace(/\{(\w+)\}/g, (butun, ad: string) =>
    ad in degerler ? String(degerler[ad]) : butun,
  );
}

// Çekirdekten gelen değerlerin adları. `Record` her değerin karşılığı olduğunu
// derleme sırasında denetler.
export const DENETCI_KIPI_ADI: Record<DenetimKipi, MetinAnahtari> = {
  Kapali: 'denetci.kip.Kapali',
  Uyar: 'denetci.kip.Uyar',
  Kati: 'denetci.kip.Kati',
};

const DENETIM_KATEGORISI: Record<string, MetinAnahtari> = {
  talimat_enjeksiyonu: 'denetim.kategori.talimat_enjeksiyonu',
  veri_sizdirma: 'denetim.kategori.veri_sizdirma',
  yetki_yukseltme: 'denetim.kategori.yetki_yukseltme',
  diger: 'denetim.kategori.diger',
  yok: 'denetim.kategori.yok',
};

/** Denetçinin kategorisinin okunur adı; bilinmeyen kategori olduğu gibi. */
export function denetimKategorisi(kategori: string): string {
  const anahtar = DENETIM_KATEGORISI[kategori];
  return anahtar ? m(anahtar) : kategori;
}

export const KIP_ADI: Record<Kip, MetinAnahtari> = {
  Inceleme: 'kip.Inceleme',
  Siki: 'kip.Siki',
  Proje: 'kip.Proje',
  Serbest: 'kip.Serbest',
};

export const MODEL_PROFILI_ADI: Record<ModelProfili, MetinAnahtari> = {
  Hizli: 'model.Hizli',
  Dengeli: 'model.Dengeli',
  Derin: 'model.Derin',
  Otomatik: 'model.Otomatik',
};

export const ARAC_DURUMU_ADI: Record<AracDurumu | 'Suruyor' | 'Baslamadi', MetinAnahtari> = {
  Suruyor: 'arac.durum.Suruyor',
  Basarili: 'arac.durum.Basarili',
  Basarisiz: 'arac.durum.Basarisiz',
  Reddedildi: 'arac.durum.Reddedildi',
  IptalEdildi: 'arac.durum.IptalEdildi',
  SonucuBilinmiyor: 'arac.durum.SonucuBilinmiyor',
  Baslamadi: 'arac.durum.Baslamadi',
};

export const AKTARIM_DURMA_METNI: Record<AktarimDurmaNedeni, MetinAnahtari> = {
  TuketiciYetismedi: 'aktarim.TuketiciYetismedi',
  OlayCokBuyuk: 'aktarim.OlayCokBuyuk',
  Diger: 'aktarim.Diger',
};

export const YALITIM_NEDENI: Record<YalitimYoklukNedeni, MetinAnahtari> = {
  BwrapYok: 'ayarlar.yalitim.neden.BwrapYok',
  KullaniciAdAlaniKapali: 'ayarlar.yalitim.neden.KullaniciAdAlaniKapali',
  KurulumGerekli: 'ayarlar.yalitim.neden.KurulumGerekli',
  KurulumBozuk: 'ayarlar.yalitim.neden.KurulumBozuk',
  Desteklenmiyor: 'ayarlar.yalitim.neden.Desteklenmiyor',
};

/** Yalıtım yokken kullanıcıya giden sade açıklama (aşama 2 tasarımı §9.8). */
export const YALITIM_SADE: Record<YalitimYoklukNedeni, MetinAnahtari> = {
  BwrapYok: 'ayarlar.yalitim.sade.BwrapYok',
  KullaniciAdAlaniKapali: 'ayarlar.yalitim.sade.KullaniciAdAlaniKapali',
  KurulumGerekli: 'ayarlar.yalitim.sade.KurulumGerekli',
  KurulumBozuk: 'ayarlar.yalitim.sade.KurulumBozuk',
  Desteklenmiyor: 'ayarlar.yalitim.sade.Desteklenmiyor',
};
