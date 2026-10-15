# Triyaj etiketleri

Becerilerin beş triyaj rolü Jira `KALFA` projesindeki şu etiketlere eşlenir:

| Becerideki rol | Jira etiketi | Anlamı |
| --- | --- | --- |
| `needs-triage` | `needs-triage` | Bakım sorumlusunun değerlendirmesi gerekiyor. |
| `needs-info` | `needs-info` | Bildirenden ek bilgi bekleniyor. |
| `ready-for-agent` | `ready-for-agent` | Kapsam ve kabul ölçütleri ajan uygulamasına hazır. |
| `ready-for-human` | `ready-for-human` | İnsan uygulaması gerekiyor. |
| `wontfix` | `wontfix` | İşleme alınmayacak. |

Beceri bir rolün uygulanmasını istediğinde karşısındaki Jira etiketini
kullanın. Aynı triyaj değerlendirmesi için tek güncel rol bırakın;
eski triyaj rolünü kaldırırken diğer etiketleri ve `ajan-*` sahipliğini
koruyun. Jira iş akışı durumu [iş takibi](issue-tracker.md) kurallarına göre
ayrıca yönetilir.
