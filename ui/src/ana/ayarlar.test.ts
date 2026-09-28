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
  ayrinti: 'denetim=bwrap-yolu; aranan=/usr/bin/bwrap',
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
  it('menüden açılır; sade açıklama ve teknik ayrıntı görünür; Oturumlar\'a dönülür', async () => {
    await ciz();
    const ayarlar = await ayarlariAc();
    expect(within(menu()).getByRole('button', { name: 'Ayarlar' }).getAttribute('aria-current')).toBe(
      'page',
    );
    // Anahtar kapalıyken yalıtım yoksa komutlar kapalıdır (§9.8).
    expect(within(ayarlar).getByText('Komutlar kapalı: bubblewrap bulunamadı')).toBeTruthy();
    expect(within(ayarlar).getByText(/bubblewrap bileşeni bu bilgisayarda kurulu değil/)).toBeTruthy();
    expect(within(ayarlar).getByText(/BT departmanınızla/)).toBeTruthy();
    // Talimat ve ayrıntı yalnızca "Teknik ayrıntılar"dadır.
    const teknik = within(ayarlar).getByText('Teknik ayrıntılar').closest('details')!;
    expect(teknik.open).toBe(false);
    expect(teknik.textContent).toContain('denetim=bwrap-yolu');
    expect(teknik.textContent).toContain(BWRAP_YOK.talimat);
    expect(screen.queryByRole('region', { name: 'Konuşma' })).toBeNull();

    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    expect(await screen.findByRole('region', { name: 'Konuşma' })).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Ayarlar' })).toBeNull();
  });

  it('durum çubuğundaki gösterge Ayarlar\'ı açar', async () => {
    await ciz();
    await fireEvent.click(within(durumCubugu()).getByRole('button', { name: 'Komutlar kapalı' }));
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

  it('Linux\'ta araç zinciri kartı Gelişmiş\'in dışında, kendi yerindedir', async () => {
    await ciz((a) => {
      a.aracZinciri = ['/opt/araclar'];
    });
    const ayarlar = await ayarlariAc();
    const liste = await within(ayarlar).findByRole('list', { name: 'Araç zinciri dizinleri' });
    const gelismis = within(ayarlar).getByText('Gelişmiş').closest('section');
    expect(gelismis?.contains(liste)).toBe(false);
  });

  it('Windows\'ta araç zinciri dizinleri Gelişmiş\'tedir; "Yeniden dene" var', async () => {
    const { arka } = await ciz((a) => {
      a.uygulama.platform = 'windows';
      a.aracZinciri = ['C:\\Program Files\\nodejs'];
      a.uygulama.yalitim = {
        tip: 'Yok',
        neden: 'Desteklenmiyor',
        talimat: 'Windows yalıtımı hazır değil.',
        ayrinti: 'denetim=appcontainer-deneme; erişim engellendi',
      };
    });
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Komutlar kapalı: bu sistemde kullanılamıyor')).toBeTruthy();
    expect(within(ayarlar).queryByText('Araç zinciri dizinleri')).toBeNull();
    const liste = await within(ayarlar).findByRole('list', {
      name: 'İzinli araç zinciri dizinleri',
    });
    expect(within(liste).getByText('C:\\Program Files\\nodejs')).toBeTruthy();
    const gelismis = within(ayarlar).getByText('Gelişmiş').closest('section');
    expect(gelismis?.contains(liste)).toBe(true);
    expect(
      within(gelismis as HTMLElement).getByText(/Kalfa kendi Python ve Node'unu getirir/),
    ).toBeTruthy();
    await fireEvent.click(
      within(gelismis as HTMLElement).getByRole('button', { name: 'Dizin ekle' }),
    );
    expect(arka.komutlar('arac_zinciri_dizini_ekle')).toEqual([{}]);
    // BT izin verdikten sonra kullanıcı yeniden dener.
    expect(within(ayarlar).getByRole('button', { name: 'Yeniden dene' })).toBeTruthy();
  });

  it('Windows\'ta yükseltmeden sonra okunamayan dizin için Gelişmiş\'te Onar çıkar', async () => {
    const { arka } = await ciz((a) => {
      a.uygulama.platform = 'windows';
      a.uygulama.yalitim = { tip: 'Hazir', arka_uc: 'windows', surum: 'AppContainer' };
      a.aracZinciri = ['C:\\Program Files\\nodejs'];
      a.aracOnarimGerekli = true;
    });
    const ayarlar = await ayarlariAc();
    const gelismis = (await within(ayarlar).findByText(/artık Kalfa'nın komutlarınca okunamıyor/))
      .closest('section') as HTMLElement;
    expect(gelismis.textContent).toContain('Gelişmiş');
    await fireEvent.click(within(gelismis).getByRole('button', { name: 'Onar' }));
    expect(arka.komutlar('yalitimi_onar')).toHaveLength(1);
    await waitFor(() =>
      expect(within(ayarlar).queryByText(/artık Kalfa'nın komutlarınca okunamıyor/)).toBeNull(),
    );
  });
});

describe('Ayarlar: BT için teknik ayrıntı (aşama 2 tasarımı §9.8)', () => {
  it('"Teknik ayrıntıları kopyala" raporu panoya yazar', async () => {
    const yazilan: string[] = [];
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: async (m: string) => void yazilan.push(m) },
    });
    const { arka } = await ciz();
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Teknik ayrıntıları kopyala' }));
    expect(await within(ayarlar).findByText('Teknik ayrıntılar panoya kopyalandı.')).toBeTruthy();
    expect(arka.komutlar('yalitim_teknik_raporu')).toHaveLength(1);
    expect(yazilan).toEqual(['Kalfa yalıtım raporu\nDurum: Yok\n']);
  });

  it('pano kullanılamazsa rapor açık gösterilir', async () => {
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined });
    await ciz();
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Teknik ayrıntıları kopyala' }));
    expect(await within(ayarlar).findByText(/Panoya kopyalanamadı/)).toBeTruthy();
    const teknik = within(ayarlar).getByText('Teknik ayrıntılar').closest('details')!;
    expect(teknik.open).toBe(true);
    expect(teknik.textContent).toContain('Kalfa yalıtım raporu');
  });
});

describe('Ayarlar: Onar (aşama 2 tasarımı §9.8)', () => {
  const kurulumBozuk = (a: SahteArka) => {
    a.uygulama.platform = 'windows';
    a.uygulama.yalitim = {
      tip: 'Yok',
      neden: 'KurulumBozuk',
      talimat: 'Windows yalıtımı hazır değil.',
      ayrinti: 'denetim=kok-izni; C:\\ sürücüsünün kök izni eksik.',
    };
  };

  it('Linux\'ta Onar yok', async () => {
    await ciz();
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).queryByRole('button', { name: 'Onar' })).toBeNull();
  });

  it('Windows\'ta Onar kurulumu yineler; durum hazır olur ve bildirilir', async () => {
    const { arka } = await ciz(kurulumBozuk);
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText(/Önce "Onar"ı deneyin/)).toBeTruthy();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Onar' }));
    expect(await within(ayarlar).findByText('Yalıtımlı: windows AppContainer')).toBeTruthy();
    expect(within(ayarlar).getByText(/Yalıtım onarıldı/)).toBeTruthy();
    expect(arka.komutlar('yalitimi_onar')).toHaveLength(1);
    const gosterge = within(durumCubugu()).getByText('Yalıtımlı', { selector: '.durum-ogesi' });
    expect(gosterge).toBeTruthy();
  });

  it('başarısız onarım BT\'ye yönlendirir', async () => {
    await ciz((a) => {
      kurulumBozuk(a);
      a.onarimSonucu = { tip: 'Basarisiz', mesaj: 'kalfa-yalitim kur 1 koduyla bitti.' };
    });
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Onar' }));
    const ileti = await within(ayarlar).findByText(/Onarım tamamlanamadı/);
    expect(ileti.textContent).toContain('1 koduyla bitti');
    expect(ileti.textContent).toContain('BT departmanınızla');
  });

  it('kurulu sürüm değilse talimat gösterilir', async () => {
    await ciz((a) => {
      kurulumBozuk(a);
      a.onarimSonucu = { tip: 'KuruluDegil', talimat: 'kalfa-yalitim.exe kur komutunu çalıştırın.' };
    });
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Onar' }));
    expect(await within(ayarlar).findByText('kalfa-yalitim.exe kur komutunu çalıştırın.')).toBeTruthy();
  });
});

describe('Ayarlar: yalıtımsız çalıştırma (aşama 2 tasarımı §9.8)', () => {
  it('anahtar varsayılan kapalıdır; açınca komut çağrılır ve durum güncellenir', async () => {
    const { arka, durum } = await ciz();
    const ayarlar = await ayarlariAc();
    const kutu = within(ayarlar).getByRole('checkbox', {
      name: 'Yalıtım yokken komutların yalıtımsız çalışmasına izin ver',
    }) as HTMLInputElement;
    expect(kutu.checked).toBe(false);

    await fireEvent.click(kutu);
    await waitFor(() => expect(durum.uygulama?.yalitimsiz_izinli).toBe(true));
    expect(arka.komutlar('yalitimsizi_ayarla')).toEqual([{ acik: true }]);
    expect(kutu.checked).toBe(true);
  });

  it('anahtar açıkken durum çubuğu ve Ayarlar "Yalıtımsız" der', async () => {
    await ciz((a) => (a.uygulama.yalitimsiz_izinli = true));
    const oge = within(durumCubugu()).getByText('Yalıtımsız', { selector: '.durum-ogesi' });
    expect(oge.getAttribute('title')).toContain('onayınızla yalıtımsız');
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('Yalıtımsız: bubblewrap bulunamadı')).toBeTruthy();
    expect(within(ayarlar).getByText(/Yalıtımsız çalıştırma açık/)).toBeTruthy();
  });

  it('yerel pencerede vazgeçilirse kapalı kalır', async () => {
    const { arka, durum } = await ciz((a) => (a.yalitimsizOnayi = false));
    const ayarlar = await ayarlariAc();
    const kutu = within(ayarlar).getByRole('checkbox', {
      name: 'Yalıtım yokken komutların yalıtımsız çalışmasına izin ver',
    }) as HTMLInputElement;
    await fireEvent.click(kutu);
    await waitFor(() => expect(arka.komutlar('yalitimsizi_ayarla')).toHaveLength(1));
    expect(durum.uygulama?.yalitimsiz_izinli).toBe(false);
    await waitFor(() => expect(kutu.checked).toBe(false));
  });
});

describe('Ayarlar: hazırlanmış çalışma alanları (aşama 2 tasarımı §9.8)', () => {
  it('Windows çalışma alanlarını listeler ve yalnızca kimlikle kaldırır', async () => {
    const { arka } = await ciz((a) => {
      a.uygulama.platform = 'windows';
      a.hazirAlanlar = [
        { kimlik: 'Kalfa.Alan.bir', yol: 'C:\\projeler\\bir' },
        { kimlik: 'Kalfa.Alan.iki', yol: 'C:\\projeler\\iki' },
      ];
    });
    const ayarlar = await ayarlariAc();
    const liste = await within(ayarlar).findByRole('list', { name: 'Hazırlanmış çalışma alanları' });
    expect(within(liste).getByText('C:\\projeler\\bir')).toBeTruthy();
    expect(within(liste).getByText('C:\\projeler\\iki')).toBeTruthy();
    expect(arka.komutlar('yalitim_calisma_alanlari')).toEqual([{}]);

    await fireEvent.click(
      within(liste).getByRole('button', {
        name: 'C:\\projeler\\bir izinlerini geri al ve kaldır',
      }),
    );
    await waitFor(() => expect(within(liste).queryByText('C:\\projeler\\bir')).toBeNull());
    expect(within(liste).getByText('C:\\projeler\\iki')).toBeTruthy();
    expect(arka.komutlar('yalitim_calisma_alanini_kaldir')).toEqual([
      { kimlik: 'Kalfa.Alan.bir' },
    ]);
    expect(arka.komutlar('yalitim_calisma_alanlari')).toEqual([{}, {}]);

    arka.yalitimKaldirmaOnayi = false;
    await fireEvent.click(
      within(liste).getByRole('button', {
        name: 'C:\\projeler\\iki izinlerini geri al ve kaldır',
      }),
    );
    await waitFor(() =>
      expect(arka.komutlar('yalitim_calisma_alanini_kaldir')).toHaveLength(2),
    );
    expect(within(liste).getByText('C:\\projeler\\iki')).toBeTruthy();
    expect(arka.komutlar('yalitim_calisma_alanlari')).toEqual([{}, {}]);
  });

  it('Linux\'ta çalışma alanı bölümü görünmez; kaldırma hatası listeyi korur', async () => {
    const { arka, durum } = await ciz();
    const linuxAyarlari = await ayarlariAc();
    expect(within(linuxAyarlari).queryByText('Hazırlanmış çalışma alanları')).toBeNull();
    expect(arka.komutlar('yalitim_calisma_alanlari')).toHaveLength(0);

    arka.uygulama.platform = 'windows';
    durum.uygulama!.platform = 'windows';
    arka.hazirAlanlar = [{ kimlik: 'Kalfa.Alan.bir', yol: 'C:\\projeler\\bir' }];
    arka.hatalar.set('yalitim_calisma_alanini_kaldir', {
      kod: 'GecersizGirdi',
      mesaj: 'Bu çalışma alanının açık bir oturumu var.',
    });
    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    const ayarlar = await ayarlariAc();
    const liste = await within(ayarlar).findByRole('list', { name: 'Hazırlanmış çalışma alanları' });
    await fireEvent.click(within(liste).getByRole('button', { name: /izinlerini geri al ve kaldır/ }));
    expect((await within(ayarlar).findByRole('alert')).textContent).toContain('açık bir oturumu');
    expect(within(liste).getByText('C:\\projeler\\bir')).toBeTruthy();
  });
});
