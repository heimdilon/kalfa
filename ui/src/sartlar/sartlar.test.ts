import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { SartPenceresiDurumu } from '../ipc/tipler';
import { GECERSIZ_ANAHTAR, SahteArka } from '../sahte/arka';
import Sartlar from './Sartlar.svelte';

afterEach(() => clearMocks());

async function ciz(sart: SartPenceresiDurumu, ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.sart = sart;
  ayar(arka);
  render(Sartlar);
  await waitFor(() => expect(arka.komutlar('sart_penceresi_durumu')).toHaveLength(1));
  return arka;
}

async function anahtarGir(anahtar: string) {
  const alan = await screen.findByLabelText<HTMLInputElement>('Anahtar');
  await fireEvent.input(alan, { target: { value: anahtar } });
  await fireEvent.click(screen.getByRole('button', { name: 'Kaydet' }));
  return alan;
}

describe('anahtar formu (1c tasarımı §7.2)', () => {
  it('geçersiz anahtar saklanmaz; neden söylenir, alan boşalır', async () => {
    const arka = await ciz({ tip: 'AnahtarGerekli', neden: 'Yok' });
    const alan = await anahtarGir(GECERSIZ_ANAHTAR);
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
    );
    expect(alan.value).toBe('');
    expect(arka.komutlar('anahtari_kaydet')).toEqual([{ anahtar: GECERSIZ_ANAHTAR }]);
  });

  it('kayıtlı anahtar geçersizleştiyse bunu söyler', async () => {
    await ciz({ tip: 'AnahtarGerekli', neden: 'KayitliGecersiz' });
    expect(await screen.findByText(/Kayıtlı anahtar artık geçerli değil/)).toBeTruthy();
  });

  it('geçerli anahtardan sonra şart metni gelir; kabul kendiliğinden gitmez', async () => {
    const arka = await ciz({ tip: 'AnahtarGerekli', neden: 'Yok' }, (a) => {
      a.uygulama.anahtar_deposu = 'Bellekte';
    });
    await anahtarGir('  evren_llm_dogru  ');
    expect(await screen.findByRole('heading', { name: 'EVREN kullanım şartları' })).toBeTruthy();
    expect(arka.komutlar('anahtari_kaydet')).toEqual([{ anahtar: 'evren_llm_dogru' }]);
    expect(screen.getByText(/bellekte tutulacak/)).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });
});

describe('şart metni (1c tasarımı §7.3, §7.4)', () => {
  const METIN = { surum: 3, esasli: true, metin: '## Madde 1\n\n![rozet](https://img.shields.io/x)' };

  it('sürüm ve esaslılık yazılır; görsel yüklenmez; onay kutusu yok', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN });
    expect(await screen.findByText('Sürüm 3')).toBeTruthy();
    expect(screen.getByText('Esaslı değişiklik')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Madde 1' })).toBeTruthy();
    expect(document.querySelector('img')).toBeNull();
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });

  it('"Kabul ediyorum" kabulü bir kez, gösterilen sürümle gönderir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => (a.sartMetni = METIN));
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByText('Tamamlandı. Kalfa açılıyor.')).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([{ surum: 3 }]);
  });

  it('sürüm okurken değiştiyse yeni metin notla gösterilir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => {
      a.sartMetni = { surum: 4, esasli: false, metin: 'Yeni metin' };
    });
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByText('Şart metni bu sırada güncellendi.')).toBeTruthy();
    expect(screen.getByText('Sürüm 4')).toBeTruthy();
    expect(screen.getByText('Küçük değişiklik')).toBeTruthy();
    expect(arka.sart.tip).toBe('SartGerekli');
  });

  it('"Kabul etmiyorum" kilitli görünüme geçer; metin yeniden gösterilebilir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => (a.sartMetni = METIN));
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul etmiyorum' }));
    expect(await screen.findByRole('heading', { name: 'Şartlar kabul edilmedi' })).toBeTruthy();
    expect(screen.getByText(/Anahtarınız saklı kalır/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Metni yeniden göster' }));
    expect(await screen.findByText('Sürüm 3')).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });

  it('ret ile yarışan kabul tıklaması kilitli görünümde kalır', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN });
    arka.sart = { tip: 'Kilitli' };
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByRole('heading', { name: 'Şartlar kabul edilmedi' })).toBeTruthy();
  });
});
