// Sağ panel (masaüstü arayüzü tasarımı §6): bölüm düğmeleri, Ortam,
// Değişiklikler'e geçiş, Dosyalar'ın kimlikle çalışan ağacı ve hatırlanan
// bölüm/genişlik.
import { clearMocks } from '@tauri-apps/api/mocks';
import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => {
  clearMocks();
  localStorage.clear();
});

const DIFF_B = ['@@ -1 +1 @@', '-eski b', '+yeni b'].join('\n');

function turOzeti(tur: string, dosyalar: { yol: string; eklenen: number; silinen: number; diff?: string }[]) {
  return {
    tip: 'TurOzeti' as const,
    ozet: {
      tur,
      dosyalar: dosyalar.map((d) => ({
        ...d,
        islem: 'Degisti' as const,
        sonucu_bilinmiyor: false,
        geri_alinabilir: { tip: 'Evet' as const },
      })),
      komutlar: [],
      istekler: { sayi: 1, istem: 10, tamamlama: 2, dusunce: 0 },
      guvenlik_uyarisi: 0,
    },
  };
}

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle(
    {
      oturum: 'o1',
      gecmis: {
        ogeler: [{ tip: 'KullaniciMesaji', sira: 1, tur: 't1', metin: 'Görevleri düzelt' }],
        daha_eski_var: false,
      },
    },
    { baslik: 'Görevleri düzelt', durum: { tip: 'Tamamlandi' } },
  );
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('heading', { name: 'Görevleri düzelt' });
  return { arka, durum };
}

const panel = (ad: string) => screen.queryByRole('complementary', { name: ad });
const dugme = (ad: string) => screen.getByRole('button', { name: ad });

describe('sağ panel (masaüstü arayüzü tasarımı §6)', () => {
  it("Linux'ta düğmeler konuşma başlığında; açık bölümün düğmesi paneli kapatır", async () => {
    await ciz();
    expect(panel('Ortam')).toBeNull();
    await fireEvent.click(dugme('Ortam'));
    expect(panel('Ortam')).toBeTruthy();
    await fireEvent.click(dugme('Dosyalar'));
    expect(panel('Ortam')).toBeNull();
    expect(panel('Dosyalar')).toBeTruthy();
    await fireEvent.click(dugme('Dosyalar'));
    expect(panel('Dosyalar')).toBeNull();
  });

  it("Windows'ta üç bölüm düğmesi başlık çubuğunda", async () => {
    await ciz((a) => (a.uygulama.platform = 'windows'));
    const cubuk = document.querySelector<HTMLElement>('.baslik-cubugu')!;
    for (const ad of ['Ortam', 'Değişiklikler', 'Dosyalar']) {
      expect(within(cubuk).getByRole('button', { name: ad })).toBeTruthy();
    }
    await fireEvent.click(within(cubuk).getByRole('button', { name: 'Değişiklikler' }));
    expect(panel('Değişiklikler')).toBeTruthy();
  });

  it('Ortam çalışma alanını, git dalını, kipi ve bağlam kullanımını gösterir', async () => {
    const { arka } = await ciz((a) => {
      a.alanOzeti = { kisa_yol: '~\\kalfa-deneme', git: { tip: 'Dal', ad: 'ozellik/sag-panel' } };
    });
    await fireEvent.click(dugme('Ortam'));
    const ortam = panel('Ortam')!;
    expect(await within(ortam).findByText('ozellik/sag-panel')).toBeTruthy();
    expect(within(ortam).getByText('~\\kalfa-deneme')).toBeTruthy();
    expect(within(ortam).getByText('Bu açılışta henüz istek yapılmadı.')).toBeTruthy();

    arka.olaylar('o1', [{ tip: 'TokenKullanimi', istem: 32000, tamamlama: 10, dusunce: 0 }]);
    expect(await within(ortam).findByText(/32\.000 \/ 64\.000 token \(%50\)/)).toBeTruthy();
  });

  it("Ortam'daki çıktı Değişiklikler'de o dosyayı açar", async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      turOzeti('t1', [
        { yol: 'a.py', eklenen: 1, silinen: 0 },
        { yol: 'b.py', eklenen: 1, silinen: 1, diff: DIFF_B },
      ]),
    ]);
    await fireEvent.click(dugme('Ortam'));
    const ortam = panel('Ortam')!;
    await fireEvent.click(await within(ortam).findByRole('button', { name: /b\.py/ }));
    const degisiklikler = panel('Değişiklikler')!;
    expect(degisiklikler).toBeTruthy();
    expect(within(degisiklikler).getByText('yeni b')).toBeTruthy();
  });

  it('Dosyalar kimlikle açılır; gizli dosyanın içeriği yok, bağlantı açılmaz', async () => {
    const { arka } = await ciz((a) => {
      a.dosyaAgaci.set(null, {
        girdiler: [
          { kimlik: 1, ad: 'src', tur: 'Dizin', boyut: null },
          { kimlik: 2, ad: '.env', tur: 'Dosya', boyut: 12 },
          { kimlik: 3, ad: 'dis', tur: 'Baglanti', boyut: null },
        ],
        kirpildi: false,
      });
      a.dosyaAgaci.set(1, {
        girdiler: [{ kimlik: 4, ad: 'gorevler.py', tur: 'Dosya', boyut: 20 }],
        kirpildi: false,
      });
      a.dosyaIcerikleri.set(2, { tip: 'Gizli' });
      a.dosyaIcerikleri.set(4, { tip: 'Metin', metin: 'def tamamla():\n    pass\n', kirpildi: false });
    });
    await fireEvent.click(dugme('Dosyalar'));
    const dosyalar = panel('Dosyalar')!;
    const agac = await within(dosyalar).findByRole('tree');

    await fireEvent.click(within(agac).getByRole('button', { name: 'src' }));
    await fireEvent.click(await within(agac).findByRole('button', { name: 'gorevler.py' }));
    expect(await within(dosyalar).findByText(/def tamamla/)).toBeTruthy();
    expect(arka.komutlar('calisma_alani_dizini').map((k) => k.dizin ?? null)).toEqual([null, 1]);
    expect(arka.komutlar('calisma_alani_dosyasi')).toEqual([{ oturum: 'o1', kimlik: 4 }]);

    await fireEvent.click(within(agac).getByRole('button', { name: '.env' }));
    expect(await within(dosyalar).findByText('Gizli dosya; içerik gösterilmez.')).toBeTruthy();
    expect(within(agac).queryByRole('button', { name: 'dis' })).toBeNull();
    expect(within(agac).getByText('bağlantı; izlenmez')).toBeTruthy();
  });

  it('açık bölüm ve genişlik sonraki açılışta hatırlanır', async () => {
    await ciz();
    await fireEvent.click(dugme('Dosyalar'));
    const tutamac = dugme('Bölme genişliğini ayarla');
    await fireEvent.keyDown(tutamac, { key: 'ArrowLeft' });
    expect(document.querySelector('.sag-oran-40')).toBeTruthy();

    cleanup();
    clearMocks();
    await ciz();
    expect(panel('Dosyalar')).toBeTruthy();
    expect(document.querySelector('.sag-oran-40')).toBeTruthy();
  });
});
