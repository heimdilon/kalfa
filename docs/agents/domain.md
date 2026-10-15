# Alan belgeleri

Kalfa tek alan (single-context) düzenini kullanır: kökte `GLOSSARY.md`,
kararlar için `docs/adr/`.

## Araştırmaya başlarken

1. Varsa kökteki `GLOSSARY.md` dosyasını okuyun.
2. Varsa `docs/adr/` içindeki çalışılan konuyla ilgili kararları okuyun.
3. İlgili tasarım ve uygulama planlarını `AGENTS.md` başlangıç sırasıyla
   okuyun. Teknik durum için `docs/GELISTIRME_DURUMU.md` ve `git log` esastır.

Sözlük veya ADR dizini yoksa çalışmaya devam edin. Boş belge oluşturmayın;
`domain-modeling` becerisi terim ya da karar netleştiğinde ilgili belgeyi
oluşturur. Mevcut tasarım ve planlar kendi konularının kaynağı olarak kalır.

## Terimler ve kararlar

İş başlığı, öneri, hipotez, kod veya testte alan kavramı adlandırırken
sözlüğün tanımladığı Türkçe terimi kullanın. Eksik terim gerçek bir alan
boşluğuysa `domain-modeling` için not edin.

Bir öneri mevcut ADR ile çelişiyorsa ilgili ADR'yi gösterip çelişkiyi ve
yeniden değerlendirme gerekçesini açıkça yazın. Plan ile kod arasındaki
çelişkilerde `AGENTS.md` içindeki plana bağlılık kuralını izleyin.
