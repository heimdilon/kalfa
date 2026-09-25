import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { YalitimDurumu } from '../ipc/tipler';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const HAZIR: YalitimDurumu = { tip: 'Hazir', arka_uc: 'bwrap', surum: '0.9.0' };
const BWRAP_YOK: YalitimDurumu = {
  tip: 'Yok',
  neden: 'BwrapYok',
  talimat: 'Komutları yalıtmak için bubblewrap paketini kurun.',
};

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.uygulama.yalitim = BWRAP_YOK;
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('region', { name: 'Konuşma' });
  return { arka, durum };
}

function menu() {
  return screen.getByRole('navigation', { name: 'Menü' });
}

async function ayarlariAc() {
  await fireEvent.click(within(menu()).getByRole('button', { name: 'Ayarlar' }));
  return screen.findByRole('region', { name: 'Ayarlar' });
}

function durumCubugu() {
  return screen.getByRole('contentinfo', { name: 'Durum çubuğu' });
}

describe('Ayarlar: yalıtım (aşama 2 tasarımı §10.3, §10.4)', () => {
  it('menüden açılır; durum, neden ve talimat görünür; Oturumlar\'a dönülür', async () => {
    await ciz();
    const ayarlar = await ayarlariAc();
    expect(within(menu()).getByRole('button', { name: 'Ayarlar' }).getAttribute('aria-current')).toBe(
      'page',
    );
    expect(within(ayarlar).getByText('Yalıtımsız: bubblewrap bulunamadı')).toBeTruthy();
    expect(within(ayarlar).getByText(BWRAP_YOK.talimat)).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Konuşma' })).toBeNull();

    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    expect(await screen.findByRole('region', { name: 'Konuşma' })).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Ayarlar' })).toBeNull();
  });

  it('durum çubuğundaki gösterge Ayarlar\'ı açar', async () => {
    await ciz();
    await fireEvent.click(within(durumCubugu()).getByRole('button', { name: 'Yalıtımsız' }));
    expect(await screen.findByRole('region', { name: 'Ayarlar' })).toBeTruthy();
  });

  it('"Yeniden dene" algılamayı çalıştırır; ekran ve durum çubuğu güncellenir', async () => {
    const { arka } = await ciz((a) => {
      a.sonrakiYalitim = HAZIR;
    });
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Yeniden dene' }));
    expect(await within(ayarlar).findByText('Yalıtımlı: bwrap 0.9.0')).toBeTruthy();
    expect(arka.komutlar('yalitimi_algila')).toHaveLength(1);
    const gosterge = within(durumCubugu()).getByText('Yalıtımlı', { selector: '.durum-ogesi' });
    expect(gosterge.getAttribute('title')).toBe('Komutlar yalıtımda çalışıyor (bwrap)');
  });

  it('kanaldan gelen durum değişimi durum çubuğunu günceller', async () => {
    const { arka } = await ciz();
    await waitFor(() => expect(arka.komutlar('yalitimi_dinle')).toHaveLength(1));
    arka.yalitimDurumu(HAZIR);
    expect(
      await within(durumCubugu()).findByText('Yalıtımlı', { selector: '.durum-ogesi' }),
    ).toBeTruthy();
  });

  it('araç zinciri dizini klasör seçiciyle eklenir ve kaldırılır', async () => {
    const { arka } = await ciz((a) => {
      a.aracZinciri = ['/opt/araclar'];
    });
    const ayarlar = await ayarlariAc();
    const liste = await within(ayarlar).findByRole('list', { name: 'Araç zinciri dizinleri' });
    expect(within(liste).getByText('/opt/araclar')).toBeTruthy();

    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Dizin ekle' }));
    expect(await within(liste).findByText('/opt/node')).toBeTruthy();
    // Arayüz yol göndermez; klasörü Rust seçtirir.
    expect(arka.komutlar('arac_zinciri_dizini_ekle')).toEqual([{}]);

    await fireEvent.click(
      within(liste).getByRole('button', { name: '/opt/araclar dizinini kaldır' }),
    );
    await waitFor(() => expect(within(liste).queryByText('/opt/araclar')).toBeNull());
    expect(arka.komutlar('arac_zinciri_dizini_kaldir')).toEqual([{ yol: '/opt/araclar' }]);
  });

  it('eklenemeyen dizinin hatası Ayarlar\'da gösterilir', async () => {
    await ciz((a) => {
      a.hatalar.set('arac_zinciri_dizini_ekle', {
        kod: 'GecersizGirdi',
        mesaj: 'Bu dizin ev dizinini ya da Kalfa\'nın veri dizinini kapsıyor; yalıtıma açılamaz.',
      });
    });
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Eklenmiş dizin yok.')).toBeTruthy();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Dizin ekle' }));
    expect((await within(ayarlar).findByRole('alert')).textContent).toContain(
      'yalıtıma açılamaz',
    );
  });

  it('Windows\'ta araç zinciri bölümü ve "Yeniden dene" yok', async () => {
    const { arka } = await ciz((a) => {
      a.uygulama.platform = 'windows';
      a.uygulama.yalitim = {
        tip: 'Yok',
        neden: 'Desteklenmiyor',
        talimat: 'Bu sürümde komutlar yalıtımsız çalışıyor.',
      };
    });
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Yalıtımsız: bu sistemde kullanılamıyor')).toBeTruthy();
    expect(within(ayarlar).queryByText('Araç zinciri dizinleri')).toBeNull();
    expect(within(ayarlar).queryByRole('button', { name: 'Yeniden dene' })).toBeNull();
    expect(arka.komutlar('arac_zinciri_dizinleri')).toHaveLength(0);
  });
});
