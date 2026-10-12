// Ayarlar > "Yeni oturumların bütçesi" (5b tasarımı §9): varsayılanın okunması,
// girdi doğrulaması ve doğrulanmış kayıt.
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { SahteArka } from '../sahte/arka';
import { AnaDurum } from './anaDurum.svelte';
import ButceAyari from './ButceAyari.svelte';

afterEach(() => clearMocks());

async function ciz() {
  const arka = new SahteArka().kur();
  const durum = new AnaDurum();
  await durum.ayarlariAc();
  render(ButceAyari, { props: { durum } });
  await waitFor(() => expect(screen.getByLabelText('İstek sınırı')).toBeTruthy());
  return { arka, durum };
}

describe('Ayarlar: yeni oturumların bütçesi (5b tasarımı §9)', () => {
  it('varsayılanı okur ve açık oturumların etkilenmediğini söyler', async () => {
    await ciz();
    expect((screen.getByLabelText('İstek sınırı') as HTMLInputElement).value).toBe('200');
    expect((screen.getByLabelText('Token sınırı') as HTMLInputElement).value).toBe('5000000');
    expect(
      screen.getByText(
        'Yeni oturumlar bu istek ve token sınırlarıyla başlar. Açık ve kayıttan açılan oturumların sınırları bu ayardan etkilenmez.',
      ),
    ).toBeTruthy();
  });

  it('sıfır ya da geçersiz girdide kaydetmez ve hata metni gösterir', async () => {
    const { arka } = await ciz();
    await fireEvent.input(screen.getByLabelText('İstek sınırı'), { target: { value: '0' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Kaydet' }));
    expect(screen.getByRole('alert').textContent).toBe(
      'Sınırlar 1 ile 9007199254740991 arasında tam sayı olmalı. Girdiğiniz değeri kontrol edin.',
    );
    expect(screen.getByLabelText('İstek sınırı').getAttribute('aria-invalid')).toBe('true');
    expect(arka.komutlar('oturum_butcesini_ayarla')).toEqual([]);
  });

  it('geçerli sınırı Rust komutuyla kaydeder ve onay durumunu gösterir', async () => {
    const { arka, durum } = await ciz();
    await fireEvent.input(screen.getByLabelText('İstek sınırı'), { target: { value: '3' } });
    await fireEvent.input(screen.getByLabelText('Token sınırı'), { target: { value: '1000' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Kaydet' }));
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toBe(
        'Kaydedildi. Yeni oturumlar bu sınırlarla başlar.',
      ),
    );
    expect(arka.komutlar('oturum_butcesini_ayarla')).toEqual([{ istek: 3, token: 1000 }]);
    expect(durum.butceVarsayilani).toEqual({ istek: 3, token: 1000 });
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
