// Güvenlik denetçisinin arayüzü (aşama 3c, Görev 8): araç kartındaki rozetler,
// durum çubuğundaki kip ve katı denetimin kartı.
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';
import KonusmaOgesi from './KonusmaOgesi.svelte';
import OnizlemeKarti from './OnizlemeKarti.svelte';
import type { DenetimRozeti, EkranOgesi } from '../durum/oturum';

afterEach(() => clearMocks());

function aracKarti(denetim: DenetimRozeti | null) {
  return {
    tip: 'AracCagrisi' as const,
    anahtar: 'c-1',
    sira: 1,
    tur: 't1',
    cagri: 'c1',
    arac: 'dosya_oku',
    aciklama: 'a.txt',
    argumanlar: null,
    durum: 'Basarili' as const,
    sonuc: 'içerik',
    terminal: null,
    degisiklikler: [],
    denetim,
  };
}

function kartiCiz(denetim: DenetimRozeti | null) {
  render(KonusmaOgesi, { props: { oge: aracKarti(denetim), oturum: 'o1', durum: new AnaDurum() } });
}

describe('araç kartında dosya diff\'i (masaüstü arayüzü §4)', () => {
  function diffKarti(ek: Partial<Extract<EkranOgesi, { tip: 'AracCagrisi' }>>) {
    render(KonusmaOgesi, {
      props: { oge: { ...aracKarti(null), arac: 'dosya_duzenle', ...ek }, oturum: 'o1', durum: new AnaDurum() },
    });
  }
  const uzun = ['@@ -1,30 +1,30 @@', ...Array.from({ length: 30 }, (_, i) => `+satır ${i}`)].join('\n');

  it('dosya adı, sayılar ve diff satırları görünür; uzun diff katlanır', async () => {
    diffKarti({ degisiklikler: [{ yol: 'gorevler.py', islem: 'Degisti', eklenen: 30, silinen: 0, diff: uzun }] });
    expect(screen.getByText('gorevler.py')).toBeTruthy();
    expect(screen.getByText('+30')).toBeTruthy();
    expect(screen.getByText('satır 19')).toBeTruthy();
    expect(screen.queryByText('satır 20')).toBeNull();
    expect(screen.queryByText('içerik')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Tamamını göster (30 satır)' }));
    expect(screen.getByText('satır 29')).toBeTruthy();
  });

  it('başarısız çağrının değişikliği uygulanmadı; diff yoksa nedeni yazılır', () => {
    diffKarti({ durum: 'Basarisiz', degisiklikler: [{ yol: '.env', islem: 'Degisti', eklenen: 1, silinen: 1 }] });
    expect(screen.getByText('uygulanmadı')).toBeTruthy();
    expect(screen.getByText(/Diff gösterilmiyor/)).toBeTruthy();
  });
});

describe('araç kartında denetim rozeti', () => {
  it('denetleniyor', () => {
    kartiCiz({ tip: 'Suruyor' });
    expect(screen.getByText('Denetleniyor…')).toBeTruthy();
  });

  it('şüpheli, kategorinin okunur adıyla', () => {
    kartiCiz({ tip: 'Supheli', kategori: 'talimat_enjeksiyonu', gerekce: 'telkin' });
    expect(screen.getByText('Şüpheli: talimat enjeksiyonu')).toBeTruthy();
  });

  it('denetlenemedi ve denetlenmedi: gizli', () => {
    kartiCiz({ tip: 'Bilinmiyor', neden: 'bozuk JSON' });
    expect(screen.getByText('Denetlenemedi')).toBeTruthy();
    kartiCiz({ tip: 'Atlandi' });
    expect(screen.getByText('Denetlenmedi: gizli olabilir')).toBeTruthy();
  });

  it('güvenli sonuç ve eski kayıt rozet göstermez', () => {
    kartiCiz({ tip: 'Guvenli' });
    kartiCiz(null);
    expect(screen.queryByText(/Denetle|Şüpheli/)).toBeNull();
  });
});

describe('katı denetimin kartı', () => {
  it('şüpheli öğe işaretli gelir, güvenli öğe işaretsiz; tek düğme işaretliyi çıkarır', async () => {
    let karar: string[] | null = null;
    render(OnizlemeKarti, {
      props: {
        oge: {
          anahtar: 'c-2',
          sira: 2,
          tur: 't1',
          tip: 'GonderimOnizlemesi' as const,
          islem: 'i-1',
          neden: 'Denetim' as const,
          durum: 'Bekliyor' as const,
          ogeler: [
            { cagri: 'c1', kaynak: 'dosya:a.txt', bayt: 10, denetim: 'şüpheli: veri_sizdirma' },
            { cagri: 'c2', kaynak: 'dosya:b.txt', bayt: 20 },
          ],
        },
        onkarar: (_islem: string, g: string[]) => {
          karar = g;
        },
      },
    });
    const kutular = screen.getAllByRole<HTMLInputElement>('checkbox');
    expect(kutular.map((k) => k.checked)).toEqual([true, false]);
    expect(screen.getByText('Şüpheli: veri sızdırma')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Seçilenleri gönderme' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'İşaretlileri çıkarıp gönder' }));
    expect(karar).toEqual(['c1']);
  });

  it('kullanıcı işareti kaldırırsa düğme "Gönder" olur ve hepsi gider', async () => {
    let karar: string[] | null = null;
    render(OnizlemeKarti, {
      props: {
        oge: {
          anahtar: 'c-3',
          sira: 3,
          tur: 't1',
          tip: 'GonderimOnizlemesi' as const,
          islem: 'i-2',
          neden: 'Denetim' as const,
          durum: 'Bekliyor' as const,
          ogeler: [{ cagri: 'c1', kaynak: 'dosya:a.txt', bayt: 10, denetim: 'denetlenemedi' }],
        },
        onkarar: (_islem: string, g: string[]) => {
          karar = g;
        },
      },
    });
    expect(screen.getByText('Denetlenemedi')).toBeTruthy();
    await fireEvent.click(screen.getByRole('checkbox'));
    await fireEvent.click(screen.getByRole('button', { name: 'Gönder' }));
    expect(karar).toEqual([]);
  });
});

describe('durum çubuğunda denetçi kipi', () => {
  it('açılışta okunan kip gösterilir', async () => {
    const arka = new SahteArka().kur();
    arka.denetci = { kip: 'Kati' };
    const durum = new AnaDurum();
    render(Ana, { props: { durum } });
    expect(await screen.findByText('Denetçi: katı')).toBeTruthy();
  });

  it('ayar dosyası bozuksa kip kapalı görünür, ipucu nedeni söyler', async () => {
    const arka = new SahteArka().kur();
    arka.denetci = { kip: 'Kapali', ayar_hatasi: 'ayarlar.json bozuk' };
    render(Ana, { props: { durum: new AnaDurum() } });
    const oge = await screen.findByText('Denetçi: kapalı');
    expect(oge.closest('[title]')?.getAttribute('title')).toBe('ayarlar.json bozuk');
  });
});
