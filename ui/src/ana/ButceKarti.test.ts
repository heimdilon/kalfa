// Bütçe kararı kartı (5b tasarımı §7, §9): neden ve tüketim, eksik kullanım,
// hatalı girdi, DevamEt gövdesi (gözlenen değerler) ve durdur.
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { EkranOgesi } from '../durum/oturum';
import type { ButceGorunumu, ButceKarari } from '../ipc/tipler';
import ButceKarti from './ButceKarti.svelte';

afterEach(() => clearMocks());

type Kart = Extract<EkranOgesi, { tip: 'ButceKarari' }>;

function kart(
  ustune: Partial<ButceGorunumu> = {},
  durum: Kart['durum'] = 'Bekliyor',
): Kart {
  const gorunum: ButceGorunumu = {
    sinirlar: { istek: 200, token: 5_000_000 },
    baslatilan: 0,
    bilinen_token: 0,
    bilinmeyen: 0,
    onaylanan_bilinmeyen: 0,
    aktif: 0,
    engel: null,
    ...ustune,
  };
  return { tip: 'ButceKarari', anahtar: 'c-7', sira: 7, tur: null, gorunum, durum };
}

function ciz(oge: Kart, onkarar = vi.fn(async (_karar: ButceKarari) => {})) {
  render(ButceKarti, { props: { oge, onkarar } });
  return onkarar;
}

function girdi(etiket: string): HTMLInputElement {
  return screen.getByLabelText(etiket) as HTMLInputElement;
}

describe('ButceKarti: neden ve tüketim', () => {
  it('istek engelinde nedeni, tüketimi ve önerilen istek sınırını gösterir', () => {
    ciz(kart({ baslatilan: 200, sinirlar: { istek: 200, token: 5_000_000 }, engel: 'istek' }));
    expect(screen.getByText('İstek sınırına ulaşıldı. Yeni istek için sınırı yükseltin ya da turu durdurun.')).toBeTruthy();
    expect(screen.getByText('İstek: 200 / 200')).toBeTruthy();
    expect(girdi('İstek sınırı').value).toBe('400');
    expect(girdi('Token sınırı').value).toBe('5000000');
  });

  it('eksik kullanımda kullanımı bilinmeyen istek sayısını ve kabul metnini gösterir', () => {
    ciz(kart({ baslatilan: 2, bilinmeyen: 1, engel: 'bilinmeyen' }));
    expect(screen.getByText('Kullanımı bilinmeyen: 1 istek')).toBeTruthy();
    expect(screen.getByText('Kullanımı bilinmeyen 1 isteği kabul edersiniz; sayaçlar sıfırlanmaz.')).toBeTruthy();
    // Kesin toplam iddiası yoktur.
    expect(screen.queryByText(/kesin toplam/i)).toBeNull();
  });

  it('token engelinde bilinen tüketimi ve token sınırı önerisini gösterir', () => {
    ciz(kart({ bilinen_token: 5_000_000, engel: 'token' }));
    expect(screen.getByText('Bilinen token sınırına ulaşıldı. Yeni istek için sınırı yükseltin ya da turu durdurun.')).toBeTruthy();
    expect(girdi('Token sınırı').value).toBe('10000000');
  });

  it('yanıtlanmış kart karar düğmelerini göstermez', () => {
    ciz(kart({ bilinmeyen: 1, engel: 'bilinmeyen' }, 'Yanitlandi'));
    expect(screen.queryByRole('button', { name: 'Bütçeyle devam et' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Turu durdur' })).toBeNull();
    expect(screen.getByText('Yanıtlandı')).toBeTruthy();
  });
});

describe('ButceKarti: girdi doğrulama', () => {
  it('limit tüketimin altındaysa devam düğmesi kapanır ve hata metni görünür', async () => {
    const onkarar = ciz(kart({ baslatilan: 3, engel: 'istek' }));
    const istek = girdi('İstek sınırı');
    await fireEvent.input(istek, { target: { value: '3' } });
    const hata = screen.getByRole('alert');
    expect(hata.textContent).toBe('İstek sınırı 3 istekten büyük olmalı.');
    expect(istek.getAttribute('aria-invalid')).toBe('true');
    expect(istek.getAttribute('aria-describedby')).toContain(hata.id);
    expect(screen.getByRole('button', { name: 'Bütçeyle devam et' }).hasAttribute('disabled')).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Bütçeyle devam et' }));
    expect(onkarar).not.toHaveBeenCalled();
  });

  it('sınır rakam dışı ya da güvenli tamsayı üstündeyse biçim hatası verir', async () => {
    ciz(kart({ engel: 'istek' }));
    await fireEvent.input(girdi('Token sınırı'), { target: { value: '1e3' } });
    expect(screen.getByRole('alert').textContent).toBe(
      'Sınırları yalnızca rakamla, 1 ile 9007199254740991 arasında girin.',
    );
    expect(screen.getByRole('button', { name: 'Bütçeyle devam et' }).hasAttribute('disabled')).toBe(true);
  });
});

describe('ButceKarti: kararlar', () => {
  it('bütçeyle devam: gönderilen gövde yeni sınırları ve kartın gözlenen sayaçlarını taşır', async () => {
    const onkarar = ciz(kart({ baslatilan: 2, bilinmeyen: 1, sinirlar: { istek: 10, token: 5_000_000 }, engel: 'bilinmeyen' }));
    await fireEvent.input(girdi('İstek sınırı'), { target: { value: '20' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Bütçeyle devam et' }));
    expect(onkarar).toHaveBeenCalledTimes(1);
    expect(onkarar).toHaveBeenCalledWith({
      tip: 'DevamEt',
      sinirlar: { istek: 20, token: 5_000_000 },
      gozlenen_baslatilan: 2,
      gozlenen_bilinmeyen: 1,
    });
  });

  it('turu durdur: Durdur kararı gönderilir', async () => {
    const onkarar = ciz(kart({ bilinmeyen: 1, engel: 'bilinmeyen' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Turu durdur' }));
    expect(onkarar).toHaveBeenCalledWith({ tip: 'Durdur' });
  });
});
