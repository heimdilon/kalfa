# İş takibi: Jira

İşler, görev sahipliği, engeller ve devirler bağlı Jira projesinde tutulur.
Kod, tasarım, plan ve doğrulama kanıtları depoda ve
[GitHub PR'larında](https://github.com/heimdilon/kalfa/pulls) kalır.

## Bağlantı ve başlangıç

- Site, proje anahtarı, pano ve ortak giriş kaydı yerel
  `.yerel/IS_TAKIBI.md` kılavuzundan okunur. Bu dosya depoya girmez;
  kişisel Jira adreslerini veya belirli iş bağlantılarını açık belgelere
  ve PR açıklamalarına kopyalamayın.
- Bağlı Atlassian araçlarını kullanın. Site kimliğini oturumda erişilebilir
  kaynaklardan doğrulayın; kimlik bilgilerini depo dosyalarına yazmayın.
- Yerel kılavuzdaki ortak çalışma açıklamasını ve güncel işin açıklamasıyla
  son yorumlarını okuyun; sahipliği ve devam koşulunu bunlardan doğrulayın.
  Yerel bağlantı bağlamı yoksa proje sahibinden alın; site veya iş tahmin etmeyin.
- Teknik gerçek için `AGENTS.md`, `docs/GELISTIRME_DURUMU.md`, ilgili plan
  ve `git log` esastır. Yerel kılavuz bağlantı ve devir bağlamını tamamlar.

## İşlemler

- **Oluşturma:** Önce bağlı projede aynı işi arayın; yoksa Türkçe başlık,
  kapsam, kabul ölçütü, bağımlılıklar ve ilgili plan/PR bağlantısıyla açın.
  Beceri “issue tracker'a yayımla” diyorsa burada bir Jira işi oluşturun.
- **Okuma:** `<PROJE>-<sayı>` anahtarıyla açıklamayı ve son yorumları okuyun.
  Beceri “ilgili bileti getir” diyorsa bu Jira kaydını kullanın.
- **Listeleme:** JQL'yi yerel kılavuzda doğrulanan `project = <PROJE>` ile
  sınırlayın; ihtiyaç duyulan durum ve etiket filtrelerini ekleyin,
  sonuçların bütün sayfalarını okuyun.
- **Sahiplenme:** Aktif sahibi ve bağımlılıkları doğrulayın. Sahipsiz işe
  ajan adı, kapsam, dal/worktree, HEAD ve zaman içeren başlangıç yorumu
  bırakın; kendi `ajan-*` etiketinizi ekleyip Çalışılıyor durumuna geçirin.
  Yazma sonrası kaydı yeniden okuyup sahipliğin çakışmadığını doğrulayın.
- **Etiketleme:** Mevcut etiketleri koruyun. Triyaj rollerini
  [triyaj etiketleri](triage-labels.md) ile eşleyin. Jira etiketi gerektiğinde
  işe eklenir; GitHub'da ikinci bir etiket veya iş kuyruğu oluşturulmaz.
- **Bağımlılık:** Bloklayan işleri Jira ilişkisinde belirtin; bir işin
  başlaması için gerekli onay ve koşulları açıklamada veya devirde yazın.
- **Yorum ve devir:** Kendi ajan adınızla yeni yorum ekleyin. Kanıtı,
  test sonucunu, kalan engeli ve tek somut sonraki adımı kaydedin.
- **Durum:** Hazır → Çalışılıyor → İnceleme → Bitti. Kota, bağımlılık veya
  kullanıcı kararı bekleyen iş Beklemede olur. Geçişi ilgili işin mevcut
  geçişlerinden seçin; Bitti için kabul ölçütlerini doğrulayın.

Triyaj etiketi Jira iş akışı durumundan ayrıdır. Etiket değişikliği işi
kendiliğinden sahiplenmez, uygulama veya birleştirme onayı vermez.
Kurulum, indirme, canlı deney ve merge için `AGENTS.md` kuralları geçerlidir.

## PR'ların rolü

**PRs as a request surface: no.**

GitHub PR'ı kod inceleme ve CI kanıtı taşır; PR bağlantısını ilgili Jira
kaydına yazın. GitHub'daki `#<sayı>` ile Jira'daki `<PROJE>-<sayı>` farklı
kayıtlardır; bağlantı veya açık proje anahtarıyla çözümleyin.
