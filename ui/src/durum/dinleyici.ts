import type { KanalMesaji } from '../ipc/tipler';
import { uygula, type OturumEkrani } from './oturum';

/**
 * Kanal dinleyicisi: mesajı duruma uygular ve "işledim"i hemen bildirir.
 * Bildirim çizimi beklemez (1c tasarımı §4.3): küçültülmüş pencere çizim
 * yapmasa da paketleri işlemeyi sürdürür, tur iptal edilmez. Boşluk görülen
 * ya da kapanan abonelikte bildirim gönderilmez.
 */
export function kanalIsleyicisi(
  oku: () => OturumEkrani,
  yaz: (durum: OturumEkrani) => void,
  isledim: (no: number) => void,
): (mesaj: KanalMesaji) => void {
  return (mesaj) => {
    const yeni = uygula(oku(), { tip: 'Kanal', mesaj });
    yaz(yeni);
    if (mesaj.tip === 'Olaylar' && !yeni.bosluk) isledim(mesaj.no);
  };
}
