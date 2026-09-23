import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { onayOgesi, SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const DIFF = ['@@ -1,2 +1,2 @@', ' fn main() {', '-    eski();', '+    yeni();'].join('\n');

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle(
    {
      oturum: 'o1',
      gecmis: {
        ogeler: [{ tip: 'KullaniciMesaji', sira: 1, tur: 't1', metin: 'Dosyaları incele' }],
        daha_eski_var: false,
      },
    },
    { baslik: 'Dosyaları incele', durum: { tip: 'Tamamlandi' } },
  );
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('heading', { name: 'Dosyaları incele' });
  return { arka, durum };
}

function onayIstegi(arka: SahteArka) {
  arka.olaylar('o1', [
    { tip: 'TurBasladi' },
    {
      tip: 'OnayGerekiyor',
      islem: 'i1',
      ogeler: [
        onayOgesi({
          cagri: 'c1',
          aciklama: 'main.rs dosyasını düzenler',
          hedefler: ['C:\\projeler\\kalfa\\src\\main.rs'],
          diff: DIFF,
          uyarilar: ['Dosya çalışma alanının kökünde'],
          muhur: 'm1',
        }),
        onayOgesi({ cagri: 'c2', hedefler: ['C:\\projeler\\kalfa\\src\\lib.rs'], muhur: 'm2' }),
      ],
    },
  ]);
}

describe('ana pencere (1c tasarımı §8.1)', () => {
  it('menü, liste, konuşma ve durum çubuğu', async () => {
    await ciz();
    expect(screen.getByRole('navigation', { name: 'Menü' })).toBeTruthy();
    const liste = screen.getByRole('complementary', { name: 'Oturumlar' });
    expect(within(liste).getByText('Bugün')).toBeTruthy();
    expect(within(liste).getByRole('button', { current: true }).textContent).toContain(
      'Dosyaları incele',
    );
    expect(screen.getAllByText('Dosyaları incele').length).toBeGreaterThan(1);
    expect(screen.getByText('Hazır')).toBeTruthy();
    expect(screen.getByText('Sıkı', { selector: '.durum-ogesi' })).toBeTruthy();
  });

  it('oturum yokken karşılama ekranı yeni oturum başlatır', async () => {
    const arka = new SahteArka().kur();
    render(Ana, { props: { durum: new AnaDurum() } });
    const konusma = await screen.findByRole('region', { name: 'Konuşma' });
    const dugme = within(konusma).getByRole('button', { name: 'Yeni oturum' });
    await fireEvent.click(dugme);
    await waitFor(() => expect(arka.komutlar('oturum_baslat')).toHaveLength(1));
  });

  it('bağlantı yok ve anahtar bellekte uyarıları', async () => {
    await ciz((a) => {
      a.uygulama.baglanti = 'Yok';
      a.uygulama.anahtar_deposu = 'Bellekte';
    });
    expect(screen.getByText(/EVREN bağlantısı yok/)).toBeTruthy();
    expect(screen.getByText('Bağlantı yok')).toBeTruthy();
    expect(screen.getByText('Anahtar bellekte')).toBeTruthy();
  });

  it('salt okunur oturumda mesaj alanı kapalı ve nedeni yazılı (§10)', async () => {
    await ciz((a) =>
      a.oturumEkle(
        { oturum: 'o1', salt_okunur: { tip: 'CalismaAlaniYok', yol: 'C:\\silindi' } },
        { baslik: 'Dosyaları incele' },
      ),
    );
    const neden = 'Çalışma alanı bulunamadı: C:\\silindi';
    expect(screen.getAllByText(neden).length).toBeGreaterThan(0);
    expect(screen.getByRole<HTMLTextAreaElement>('textbox', { name: neden }).disabled).toBe(true);
  });

  it('canlı yanıt, araç kartı ve tur sonu çizilir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'TokenParcasi', metin: 'Okuyorum.' },
      { tip: 'AracCagrisiBasladi', cagri: 'c9', arac: 'dosya_oku', aciklama: 'src/main.rs' },
      { tip: 'AracSonucu', cagri: 'c9', arac: 'dosya_oku', durum: 'Basarili', icerik: 'fn main() {}' },
      { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
    ]);
    expect(await screen.findByText('Okuyorum.')).toBeTruthy();
    expect(screen.getByText('Dosya okundu')).toBeTruthy();
    expect(screen.getByText('src/main.rs')).toBeTruthy();
    expect(screen.getAllByText('Tamamlandı').length).toBeGreaterThan(0);
  });

  it('oturum ortasında anahtar geçersizleşirse hata kartında "Anahtarı değiştir" (§10)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      {
        tip: 'Hata',
        mesaj:
          'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
      },
    ]);
    const kart = (await screen.findByText('Tur hatayla bitti')).closest('section');
    await fireEvent.click(within(kart as HTMLElement).getByRole('button', { name: /Anahtarı/ }));
    expect(arka.komutlar('anahtar_penceresini_ac')).toHaveLength(1);
  });

  it('aktarım durunca kart oturumu yeniden açtırır', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [{ tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' }]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Yeniden aç' }));
    await waitFor(() => expect(arka.komutlar('oturum_ac')).toHaveLength(2));
  });
});

describe('onay isteği ve onay sayfası (1c tasarımı §8.2, §9.5)', () => {
  it('kart "İncele" ile sayfayı açar; sayfada diff, ne, nerede ve uyarılar', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    expect(await screen.findByText('Onay isteği: 2 değişiklik')).toBeTruthy();
    expect(screen.getByText('Onay bekliyor', { selector: '.durum-ogesi' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'İncele' }));
    const sayfa = screen.getByRole('dialog', { name: 'Kalfa 2 değişiklik yapmak istiyor.' });
    expect(within(sayfa).getByText('yeni();')).toBeTruthy();
    expect(within(sayfa).getByText('main.rs dosyasını düzenler')).toBeTruthy();
    expect(within(sayfa).getByText('Dosya çalışma alanının kökünde')).toBeTruthy();
    expect(within(sayfa).queryByText(/geri alınabilir/)).toBeNull();
    expect(document.activeElement).toBe(sayfa);
  });

  it('seçilmeyen değişiklik gerekçeyle reddedilir; kart yanıtlandı olur', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = screen.getByRole('dialog');
    await fireEvent.click(within(sayfa).getByRole('checkbox', { name: /lib\.rs/ }));
    await fireEvent.input(within(sayfa).getByRole('textbox'), {
      target: { value: 'lib.rs kalsın' },
    });
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Seçilenleri onayla' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(arka.komutlar('arac_onayi')).toEqual([
      {
        oturum: 'o1',
        kararlar: [
          { cagri: 'c1', muhur: 'm1', karar: { tip: 'Onayla' } },
          { cagri: 'c2', muhur: 'm2', karar: { tip: 'Reddet', gerekce: 'lib.rs kalsın' } },
        ],
      },
    ]);
    expect(screen.getByText('Yanıtlandı')).toBeTruthy();
  });

  it('Escape karar vermeden kapatır; istek kartta bekler', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(arka.komutlar('arac_onayi')).toEqual([]);
    expect(screen.getByRole('button', { name: 'İncele' })).toBeTruthy();
  });

  it('odak sayfanın içinde döner', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const reddet = screen.getByRole('button', { name: 'Reddet' });
    reddet.focus();
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Kapat' }));
  });
});
