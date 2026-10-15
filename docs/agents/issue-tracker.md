# İş takibi: Jira

İşler, görev sahipliği, engeller ve devirler [Jira KALFA
projesinde](https://heimdilon.atlassian.net/jira/software/projects/KALFA/boards/34)
tutulur. Kod, tasarım, plan ve doğrulama kanıtları depoda ve
[GitHub PR'larında](https://github.com/heimdilon/kalfa/pulls) kalır.

## Bağlantı ve başlangıç

- Site: `https://heimdilon.atlassian.net`; proje anahtarı: `KALFA`.
- Bağlı Atlassian araçlarını kullanın. Site kimliğini oturumda erişilebilir
  kaynaklardan doğrulayın; kimlik bilgilerini depo dosyalarına yazmayın.
- [KALFA-2](https://heimdilon.atlassian.net/browse/KALFA-2) ortak çalışma
  açıklaması ve devir geçmişidir. Güncel işin açıklamasını ve son yorumlarını
  ayrıca okuyun; sahipliği ve devam koşulunu bunlardan doğrulayın.
- Teknik gerçek için `AGENTS.md`, `docs/GELISTIRME_DURUMU.md`, ilgili plan
  ve `git log` esastır. Varsa `.yerel/IS_TAKIBI.md` yerel bağlantı ve devir
  bağlamını tamamlar.

## İşlemler

- **Oluşturma:** Önce KALFA'da aynı işi arayın; yoksa Türkçe başlık,
  kapsam, kabul ölçütü, bağımlılıklar ve ilgili plan/PR bağlantısıyla açın.
  Beceri “issue tracker'a yayımla” diyorsa burada bir Jira işi oluşturun.
- **Okuma:** `KALFA-<sayı>` anahtarıyla açıklamayı ve son yorumları okuyun.
  Beceri “ilgili bileti getir” diyorsa bu Jira kaydını kullanın.
- **Listeleme:** JQL'yi `project = KALFA` ile sınırlayın; ihtiyaç duyulan
  durum ve etiket filtrelerini ekleyin, sonuçların bütün sayfalarını okuyun.
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

GitHub PR'ı kod inceleme ve CI kanıtı taşır; ilgili Jira kaydına bağlantı
verin. GitHub'daki `#<sayı>` ile Jira'daki `KALFA-<sayı>` farklı kayıtlardır;
bağlantı veya açık proje anahtarıyla çözümleyin.
