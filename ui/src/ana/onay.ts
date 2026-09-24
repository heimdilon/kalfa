// Onay sayfasının kararları (1c tasarımı §8.2). Seçilen çağrılar onaylanır;
// seçilmeyenler gerekçeyle reddedilir. Her karar çağrının mührünü taşır.
import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';

export type OnaySecimi = 'tumu' | 'secilenler' | 'hicbiri';

export function kararlariUret(
  ogeler: OnayOgesi[],
  secim: OnaySecimi,
  secilenler: ReadonlySet<CagriId>,
  gerekce: string,
): OnayGirdisi[] {
  const temizGerekce = gerekce.trim() || null;
  return ogeler.map((o) => {
    const onay = secim === 'tumu' || (secim === 'secilenler' && secilenler.has(o.cagri));
    return {
      cagri: o.cagri,
      muhur: o.muhur,
      karar: onay ? { tip: 'Onayla' } : { tip: 'Reddet', gerekce: temizGerekce },
    };
  });
}

/** Dosya listesinde gösterilen kısa ad: hedefin son bileşeni. */
export function kisaAd(oge: OnayOgesi): string {
  if (oge.kabuk) return oge.kabuk.komut.split(/\s+/)[0] || oge.arac;
  const hedef = oge.hedefler[0];
  if (!hedef) return oge.arac;
  return hedef.split(/[\\/]/).filter(Boolean).at(-1) ?? hedef;
}
