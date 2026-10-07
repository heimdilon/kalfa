// Onay sayfasının kararları (1c tasarımı §8.2). Seçilen çağrılar onaylanır;
// seçilmeyenler gerekçeyle reddedilir. Her karar çağrının mührünü taşır.
import { gorunmezMi } from '../bilesenler/gorunmez';
import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';

export type OnaySecimi = 'tumu' | 'secilenler' | 'hicbiri';

/**
 * `kisayollar`: kullanıcının "Kapsam" satırında kısayolunu seçtiği çağrılar
 * (aşama 3 tasarımı §9.1). Kısayol yalnızca seçilerek onaylanan ve teklifi
 * olan öğede gider; "Tümünü onayla" kısayol vermez.
 */
export function kararlariUret(
  ogeler: OnayOgesi[],
  secim: OnaySecimi,
  secilenler: ReadonlySet<CagriId>,
  gerekce: string,
  kisayollar: ReadonlySet<CagriId> = new Set(),
): OnayGirdisi[] {
  const temizGerekce = gerekce.trim() || null;
  return ogeler.map((o) => {
    const onay = secim === 'tumu' || (secim === 'secilenler' && secilenler.has(o.cagri));
    const kisayol =
      onay && secim === 'secilenler' && o.kisayol !== undefined && kisayollar.has(o.cagri);
    return {
      cagri: o.cagri,
      muhur: o.muhur,
      karar: onay ? { tip: 'Onayla' } : { tip: 'Reddet', gerekce: temizGerekce },
      kisayol,
    };
  });
}

function gorunmezVar(metin: string | null | undefined): boolean {
  return [...(metin ?? '').replace(/\r\n/g, '\n')].some(gorunmezMi);
}

/**
 * Toplu onaylanmayan, tek tek seçilmesi gereken öğe (güvenlik analizi
 * Bulgu 3, 7): ağlı komut, onaysız derleme dosyası değişikliğinden sonraki
 * komut ve görünmez karakter taşıyan işlem. Yerel onaylı öğe zaten yerel
 * pencerede ayrıca sorulur.
 */
export function riskliMi(o: OnayOgesi): boolean {
  if (o.kabuk?.ag) return true;
  if ((o.onaysiz_degisiklikler?.derleme_dosyalari.length ?? 0) > 0) return true;
  return [o.kabuk?.komut, o.kabuk?.dizin, o.diff, ...o.hedefler].some(gorunmezVar);
}

/** Dosya listesinde gösterilen kısa ad: hedefin son bileşeni. */
export function kisaAd(oge: OnayOgesi): string {
  if (oge.kabuk) return oge.kabuk.komut.split(/\s+/)[0] || oge.arac;
  const hedef = oge.hedefler[0];
  if (!hedef) return oge.arac;
  return hedef.split(/[\\/]/).filter(Boolean).at(-1) ?? hedef;
}
