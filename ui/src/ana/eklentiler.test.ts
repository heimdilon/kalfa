// Eklentiler ekranı ve "Bu oturumdaki izinler" panelindeki eklentiler
// (aşama 4 tasarımı §13, §14).
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { KurulumSonucu, OturumEklentisi } from '../ipc/tipler';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const IZIN_MS = new Date(2026, 9, 20, 10, 5).getTime();
const ALAN_YOLU = 'C:\\projeler\\kalfa-api';

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('region', { name: 'Konuşma' });
  return { arka, durum };
}

function menu() {
  return screen.getByRole('navigation', { name: 'Menü' });
}

async function eklentileriAc() {
  await fireEvent.click(within(menu()).getByRole('button', { name: 'Eklentiler' }));
  return screen.findByRole('region', { name: 'Eklentiler' });
}

function kart(ekran: HTMLElement, ad: string) {
  return within(ekran).findByRole('group', { name: ad });
}

const kuruldu = (ek: Partial<Extract<KurulumSonucu, { tip: 'Kuruldu' }>> = {}): KurulumSonucu => ({
  tip: 'Kuruldu',
  ad: 'yeni-arac',
  surum: '1.0.0',
  guncelleme: false,
  izinler_silindi: false,
  ...ek,
});

describe('Eklentiler ekranı: liste', () => {
  it('yerleşikle aynı adlı eski kurulum listelenir ve yerleşik korunarak kaldırılır', async () => {
    const { arka } = await ciz((a) => {
      a.eklentiler[2].ad = 'kalfa-rag';
    });
    const ekran = await eklentileriAc();
    await waitFor(() =>
      expect(within(ekran).getAllByRole('group', { name: 'kalfa-rag' })).toHaveLength(2),
    );
    const [yerlesik, eski] = within(ekran).getAllByRole('group', { name: 'kalfa-rag' });
    expect(within(yerlesik).getByText('Yerleşik')).toBeTruthy();
    expect(within(eski).getByText('Kurulu')).toBeTruthy();
    expect(within(eski).getByText('Yüklenemedi')).toBeTruthy();

    await fireEvent.click(within(eski).getByRole('button', { name: 'Kaldır' }));
    await waitFor(() =>
      expect(within(ekran).getAllByRole('group', { name: 'kalfa-rag' })).toHaveLength(1),
    );
    expect(within(yerlesik).getByText('Yerleşik')).toBeTruthy();
    expect(arka.komutlar('eklenti_kaldir')).toEqual([{ ad: 'kalfa-rag' }]);
  });

  it('menüden açılır, listeyi okur; Oturumlar\'a dönülür', async () => {
    const { arka } = await ciz();
    expect(arka.komutlar('eklentileri_listele')).toHaveLength(0);
    const ekran = await eklentileriAc();
    expect(
      within(menu()).getByRole('button', { name: 'Eklentiler' }).getAttribute('aria-current'),
    ).toBe('page');
    expect(screen.queryByRole('region', { name: 'Konuşma' })).toBeNull();
    await kart(ekran, 'kalfa-rag');
    expect(arka.komutlar('eklentileri_listele')).toHaveLength(1);

    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    expect(await screen.findByRole('region', { name: 'Konuşma' })).toBeTruthy();
    expect(screen.queryByRole('region', { name: 'Eklentiler' })).toBeNull();
  });

  it('her satırda ad, sürüm, yerleşik/kurulu rozeti, durum ve araçlar var', async () => {
    await ciz();
    const ekran = await eklentileriAc();

    const rag = await kart(ekran, 'kalfa-rag');
    expect(within(rag).getByText('0.1.0')).toBeTruthy();
    expect(within(rag).getByText('Yerleşik')).toBeTruthy();
    expect(within(rag).getByText('Yüklendi')).toBeTruthy();
    expect(within(rag).getByText('kalfa_rag__anlamsal_ara')).toBeTruthy();

    const notlar = await kart(ekran, 'ornek-notlar');
    expect(within(notlar).getByText('0.2.0')).toBeTruthy();
    expect(within(notlar).getByText('Kurulu')).toBeTruthy();
    expect(within(notlar).getByText('Yüklendi')).toBeTruthy();
    expect(within(notlar).getByText('ornek_notlar__ozetle')).toBeTruthy();

    const bozuk = await kart(ekran, 'bozuk-eklenti');
    expect(within(bozuk).getByText('Kurulu')).toBeTruthy();
    expect(within(bozuk).getByText('Yüklenemedi')).toBeTruthy();
    expect(within(bozuk).getByText(/Neden: Eklentinin \.wasm dosyası kurulumdan sonra değişmiş/))
      .toBeTruthy();
    expect(within(bozuk).queryByText('Araçlar')).toBeNull();
  });

  it('devre dışı eklentinin durumu "Devre dışı"dır', async () => {
    await ciz((a) => {
      a.eklentiler[1].durum = { tip: 'DevreDisi' };
    });
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    expect(within(notlar).getByText('Devre dışı')).toBeTruthy();
    expect(within(notlar).getByRole('button', { name: 'Etkinleştir' })).toBeTruthy();
    expect(within(notlar).queryByRole('button', { name: 'Devre dışı bırak' })).toBeNull();
  });

  it('ayrıntı açılır bölümdedir: açıklama, yetenek cümlesi, SHA-256, kaynak, kurulum tarihi', async () => {
    await ciz();
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    const ayrinti = within(notlar).getByText('Ayrıntı').closest('details') as HTMLDetailsElement;
    expect(ayrinti.open).toBe(false);
    expect(ayrinti.textContent).toContain('Not dosyalarını özetler.');
    expect(ayrinti.textContent).toContain('çalışma alanındaki dosyaları okumayacak');
    expect(ayrinti.textContent).toContain(
      '1d0e8f7c6b5a49382716f5e4d3c2b1a0f9e8d7c6b5a4938271605f4e3d2c1b0a',
    );
    expect(ayrinti.textContent).toContain('C:\\Users\\kullanici\\eklentiler\\ornek-notlar');
    expect(ayrinti.textContent).toContain('18 Ekim 2026');
    // SHA-256 tek aralıklı yazıdır.
    expect(within(ayrinti).getByText(/^1d0e8f7c/).className).toContain('mono');
  });

  it('yetenek cümlesindeki ters tırnaklı kalıplar kod olarak çizilir; HTML yorumlanmaz', async () => {
    await ciz((a) => {
      a.eklentiler[1].yetenek = 'Okur: `**/*.md`, `<b>x</b>` ve `kapanmayan';
    });
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    const ayrinti = within(notlar).getByText('Ayrıntı').closest('details') as HTMLDetailsElement;
    const kodlar = [...ayrinti.querySelectorAll('code')].map((k) => k.textContent);
    expect(kodlar).toEqual(['**/*.md', '<b>x</b>']);
    expect(ayrinti.querySelector('b')).toBeNull();
    // Kapanmayan ters tırnak olduğu gibi kalır.
    expect(ayrinti.textContent).toContain('ve `kapanmayan');
  });

  it('eklentinin kendi yazdığı metindeki görünmez karakterler kaçışlanır', async () => {
    await ciz((a) => {
      a.eklentiler[1].aciklama = 'iyi\u202Ekötü';
      a.eklentiler[2].durum = { tip: 'Yuklenemedi', neden: 'bozuk\u200Bkalıp' };
    });
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    expect(notlar.textContent).toContain('iyi⟨U+202E⟩kötü');
    const bozuk = await kart(ekran, 'bozuk-eklenti');
    expect(bozuk.textContent).toContain('bozuk⟨U+200B⟩kalıp');
  });

  it('liste boşsa ve yüklenemezse', async () => {
    await ciz((a) => (a.eklentiler = []));
    const ekran = await eklentileriAc();
    expect(await within(ekran).findByText(/Kurulu eklenti yok/)).toBeTruthy();
  });

  it('listeyi okuyamazsa hata şeritte gösterilir', async () => {
    await ciz((a) =>
      a.hatalar.set('eklentileri_listele', { kod: 'Ic', mesaj: 'Eklenti kaydı okunamadı.' }),
    );
    const ekran = await eklentileriAc();
    expect((await within(ekran).findByRole('alert')).textContent).toContain(
      'Eklenti kaydı okunamadı.',
    );
  });
});

describe('Eklentiler ekranı: satır eylemleri', () => {
  it('yerleşik eklentide Güncelle ve Kaldır yok; devre dışı bırakılabilir', async () => {
    await ciz();
    const ekran = await eklentileriAc();
    const rag = await kart(ekran, 'kalfa-rag');
    expect(within(rag).queryByRole('button', { name: 'Güncelle' })).toBeNull();
    expect(within(rag).queryByRole('button', { name: 'Kaldır' })).toBeNull();
    expect(within(rag).getByRole('button', { name: 'Devre dışı bırak' })).toBeTruthy();
  });

  it('kurulu eklentide Devre dışı bırak, Güncelle ve Kaldır var', async () => {
    await ciz();
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    expect(within(notlar).getByRole('button', { name: 'Devre dışı bırak' })).toBeTruthy();
    expect(within(notlar).getByRole('button', { name: 'Güncelle' })).toBeTruthy();
    expect(within(notlar).getByRole('button', { name: 'Kaldır' })).toBeTruthy();
  });

  it('yüklenemeyen eklentide devre dışı bırakma yok; Güncelle ve Kaldır var', async () => {
    await ciz();
    const ekran = await eklentileriAc();
    const bozuk = await kart(ekran, 'bozuk-eklenti');
    expect(within(bozuk).queryByRole('button', { name: 'Devre dışı bırak' })).toBeNull();
    expect(within(bozuk).queryByRole('button', { name: 'Etkinleştir' })).toBeNull();
    expect(within(bozuk).getByRole('button', { name: 'Güncelle' })).toBeTruthy();
    expect(within(bozuk).getByRole('button', { name: 'Kaldır' })).toBeTruthy();
  });

  it('Devre dışı bırak ve Etkinleştir doğru komutu çağırır; liste yenilenir', async () => {
    const { arka } = await ciz();
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');

    await fireEvent.click(within(notlar).getByRole('button', { name: 'Devre dışı bırak' }));
    expect(await within(notlar).findByText('Devre dışı')).toBeTruthy();
    expect(arka.komutlar('eklenti_etkin_ayarla')).toEqual([{ ad: 'ornek-notlar', etkin: false }]);
    expect(arka.komutlar('eklentileri_listele')).toHaveLength(2);

    await fireEvent.click(await within(notlar).findByRole('button', { name: 'Etkinleştir' }));
    await waitFor(() => expect(within(notlar).queryByText('Devre dışı')).toBeNull());
    expect(arka.komutlar('eklenti_etkin_ayarla')).toEqual([
      { ad: 'ornek-notlar', etkin: false },
      { ad: 'ornek-notlar', etkin: true },
    ]);
    expect(await within(notlar).findByRole('button', { name: 'Devre dışı bırak' })).toBeTruthy();
  });

  it('Kaldır yalnızca adı gönderir; kaldırılan satır listeden çıkar', async () => {
    const { arka } = await ciz();
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    await fireEvent.click(within(notlar).getByRole('button', { name: 'Kaldır' }));
    await waitFor(() => expect(within(ekran).queryByRole('group', { name: 'ornek-notlar' })).toBeNull());
    expect(arka.komutlar('eklenti_kaldir')).toEqual([{ ad: 'ornek-notlar' }]);
    expect(within(ekran).getByRole('group', { name: 'kalfa-rag' })).toBeTruthy();
  });

  it('yerel pencerede vazgeçilirse Kaldır hiçbir şeyi değiştirmez', async () => {
    const { arka } = await ciz((a) => (a.eklentiOnayi = false));
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    await fireEvent.click(within(notlar).getByRole('button', { name: 'Kaldır' }));
    await waitFor(() => expect(arka.komutlar('eklenti_kaldir')).toHaveLength(1));
    await waitFor(() => expect(arka.komutlar('eklentileri_listele')).toHaveLength(2));
    expect(within(ekran).getByRole('group', { name: 'ornek-notlar' })).toBeTruthy();
    expect(within(ekran).queryByRole('alert')).toBeNull();
  });

  it('komut hatası eklentinin satırında gösterilir; öteki satırlar etkilenmez', async () => {
    await ciz((a) =>
      a.hatalar.set('eklenti_etkin_ayarla', {
        kod: 'GecersizGirdi',
        mesaj: 'Eklenti kaydı yazılamadı.',
      }),
    );
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    await fireEvent.click(within(notlar).getByRole('button', { name: 'Devre dışı bırak' }));
    expect((await within(notlar).findByRole('alert')).textContent).toBe(
      'Eklenti kaydı yazılamadı.',
    );
    const rag = within(ekran).getByRole('group', { name: 'kalfa-rag' });
    expect(within(rag).queryByRole('alert')).toBeNull();
    // Liste hâlâ eskiyi gösterir; eylem sürerken düğmeler kapalıydı, şimdi açık.
    await waitFor(() =>
      expect(
        (within(notlar).getByRole('button', { name: 'Devre dışı bırak' }) as HTMLButtonElement)
          .disabled,
      ).toBe(false),
    );
  });
});

describe('Eklentiler ekranı: klasörden kurma ve güncelleme (§13.2)', () => {
  it('"Klasörden kur" yol göndermeden komutu çağırır; kurulan eklenti listeye girer ve duyurulur', async () => {
    const { arka, durum } = await ciz((a) => (a.kurulumSonucu = kuruldu()));
    const ekran = await eklentileriAc();
    await kart(ekran, 'kalfa-rag');
    await fireEvent.click(within(ekran).getByRole('button', { name: 'Klasörden kur' }));
    expect(await within(ekran).findByText('yeni-arac 1.0.0 kuruldu.')).toBeTruthy();
    expect(arka.komutlar('eklenti_klasorden_kur')).toEqual([{}]);
    expect(await kart(ekran, 'yeni-arac')).toBeTruthy();
    expect(durum.duyuru).toBe('yeni-arac 1.0.0 kuruldu.');
    expect(document.querySelector('[aria-live="polite"]')?.textContent).toBe(
      'yeni-arac 1.0.0 kuruldu.',
    );
  });

  it('Güncelle aynı akıştır; güncellendi iletisi gelir, sürüm değişir', async () => {
    const { arka } = await ciz(
      (a) => (a.kurulumSonucu = kuruldu({ ad: 'ornek-notlar', surum: '0.3.0', guncelleme: true })),
    );
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    await fireEvent.click(within(notlar).getByRole('button', { name: 'Güncelle' }));
    expect(await within(ekran).findByText('ornek-notlar 0.3.0 güncellendi.')).toBeTruthy();
    expect(arka.komutlar('eklenti_klasorden_kur')).toEqual([{}]);
    expect(await within(notlar).findByText('0.3.0')).toBeTruthy();
    expect(within(ekran).queryByText(/gönderim izinleri silindi/)).toBeNull();
  });

  it('güncelleme gönderim izinlerini sildiyse ayrıca not düşülür', async () => {
    const { durum } = await ciz(
      (a) =>
        (a.kurulumSonucu = kuruldu({
          ad: 'kalfa-rag',
          surum: '0.2.0',
          guncelleme: true,
          izinler_silindi: true,
        })),
    );
    const ekran = await eklentileriAc();
    // Yerleşikte Güncelle yok; aynı akış "Klasörden kur"dan da yürür.
    await kart(ekran, 'kalfa-rag');
    await fireEvent.click(within(ekran).getByRole('button', { name: 'Klasörden kur' }));
    expect(await within(ekran).findByText('kalfa-rag 0.2.0 güncellendi.')).toBeTruthy();
    expect(
      within(ekran).getByText(/gönderim izinleri silindi. İlk gönderim yeniden sorulacak/),
    ).toBeTruthy();
    expect(durum.duyuru).toContain('kalfa-rag 0.2.0 güncellendi.');
    expect(durum.duyuru).toContain('İlk gönderim yeniden sorulacak');
    const alan = within(await kart(ekran, 'kalfa-rag')).getByRole('group', { name: ALAN_YOLU });
    expect(await within(alan).findByText('İzin yok')).toBeTruthy();
  });

  it('başarısız kurulumun iletisi görünür ve duyurulur', async () => {
    const { durum } = await ciz(
      (a) => (a.kurulumSonucu = { tip: 'Basarisiz', mesaj: 'Klasörde kalfa.toml yok.' }),
    );
    const ekran = await eklentileriAc();
    await kart(ekran, 'kalfa-rag');
    await fireEvent.click(within(ekran).getByRole('button', { name: 'Klasörden kur' }));
    const ileti = await within(ekran).findByText('Eklenti kurulamadı: Klasörde kalfa.toml yok.');
    expect(ileti.closest('div')?.className).toContain('hata');
    expect(durum.duyuru).toBe('Eklenti kurulamadı: Klasörde kalfa.toml yok.');
  });

  it('vazgeçilirse sessiz kalınır; önceki ileti de silinir', async () => {
    const { arka, durum } = await ciz((a) => (a.kurulumSonucu = kuruldu()));
    const ekran = await eklentileriAc();
    await kart(ekran, 'kalfa-rag');
    await fireEvent.click(within(ekran).getByRole('button', { name: 'Klasörden kur' }));
    await within(ekran).findByText('yeni-arac 1.0.0 kuruldu.');

    arka.kurulumSonucu = { tip: 'Vazgecildi' };
    durum.duyuru = '';
    const kur = within(ekran).getByRole('button', { name: 'Klasörden kur' }) as HTMLButtonElement;
    // Liste yenilenirken eylemler art arda çalışır; düğme açılınca ikinci kez basılır.
    await waitFor(() => expect(kur.disabled).toBe(false));
    await fireEvent.click(kur);
    await waitFor(() => expect(arka.komutlar('eklenti_klasorden_kur')).toHaveLength(2));
    await waitFor(() => expect(within(ekran).queryByText(/kuruldu\./)).toBeNull());
    expect(within(ekran).queryByRole('alert')).toBeNull();
    expect(durum.duyuru).toBe('');
  });

  it('kurulum komutu hata verirse ekranın üstündeki şeritte gösterilir', async () => {
    await ciz((a) =>
      a.hatalar.set('eklenti_klasorden_kur', { kod: 'Ic', mesaj: 'Klasör seçici açılamadı.' }),
    );
    const ekran = await eklentileriAc();
    await kart(ekran, 'kalfa-rag');
    await fireEvent.click(within(ekran).getByRole('button', { name: 'Klasörden kur' }));
    expect((await within(ekran).findByRole('alert')).textContent).toContain(
      'Klasör seçici açılamadı.',
    );
  });
});

describe('Eklentiler ekranı: çalışma alanı satırları (§13.3, §8.4)', () => {
  async function ragAlani() {
    const arka = await ciz();
    const ekran = await eklentileriAc();
    const rag = await kart(ekran, 'kalfa-rag');
    const alan = within(rag).getByRole('group', { name: ALAN_YOLU });
    return { ...arka, ekran, rag, alan };
  }

  it('yol, açık anahtarı, gönderim izni ve depo boyutu görünür', async () => {
    const { alan } = await ragAlani();
    expect(within(alan).getByText(ALAN_YOLU)).toBeTruthy();
    const kutu = within(alan).getByRole('checkbox', {
      name: 'Bu çalışma alanında açık',
    }) as HTMLInputElement;
    expect(kutu.checked).toBe(true);
    expect(within(alan).getByText('EVREN\'e gönderim')).toBeTruthy();
    expect(within(alan).getByText('İzin var · 20 Ekim')).toBeTruthy();
    expect(within(alan).getByRole('button', { name: 'İzni kaldır' })).toBeTruthy();
    expect(within(alan).getByText('6 MiB')).toBeTruthy();
    expect(within(alan).getByRole('button', { name: 'Veriyi sil' })).toBeTruthy();
  });

  it('anahtar kimlikle çalışma alanını açar ve kapatır; kapatınca izin de gider', async () => {
    const { arka, alan } = await ragAlani();
    const kutu = within(alan).getByRole('checkbox', {
      name: 'Bu çalışma alanında açık',
    }) as HTMLInputElement;
    await fireEvent.click(kutu);
    await waitFor(() => expect(kutu.checked).toBe(false));
    expect(arka.komutlar('eklenti_alani_ayarla')).toEqual([
      { ad: 'kalfa-rag', hedef: { tip: 'Kimlik', kimlik: 'alan-1' }, acik: false },
    ]);
    expect(arka.komutlar('eklentileri_listele')).toHaveLength(2);
    expect(await within(alan).findByText('İzin yok')).toBeTruthy();
    expect(within(alan).queryByRole('button', { name: 'İzni kaldır' })).toBeNull();

    await fireEvent.click(kutu);
    await waitFor(() => expect(kutu.checked).toBe(true));
    expect(arka.komutlar('eklenti_alani_ayarla')[1]).toEqual({
      ad: 'kalfa-rag',
      hedef: { tip: 'Kimlik', kimlik: 'alan-1' },
      acik: true,
    });
  });

  it('"İzni kaldır" adı ve kimliği gönderir; satır "İzin yok" olur', async () => {
    const { arka, alan } = await ragAlani();
    await fireEvent.click(within(alan).getByRole('button', { name: 'İzni kaldır' }));
    expect(await within(alan).findByText('İzin yok')).toBeTruthy();
    expect(arka.komutlar('gonderim_iznini_kaldir')).toEqual([
      { ad: 'kalfa-rag', kimlik: 'alan-1' },
    ]);
    expect(within(alan).queryByRole('button', { name: 'İzni kaldır' })).toBeNull();
    // Kapatmaz: anahtar açık kalır.
    expect(
      (within(alan).getByRole('checkbox', { name: 'Bu çalışma alanında açık' }) as HTMLInputElement)
        .checked,
    ).toBe(true);
  });

  it('"Veriyi sil" adı ve kimliği gönderir; vazgeçilirse boyut değişmez', async () => {
    const { arka, alan } = await ragAlani();
    arka.eklentiOnayi = false;
    await fireEvent.click(within(alan).getByRole('button', { name: 'Veriyi sil' }));
    await waitFor(() => expect(arka.komutlar('eklenti_verisini_sil')).toHaveLength(1));
    await waitFor(() => expect(arka.komutlar('eklentileri_listele')).toHaveLength(2));
    expect(within(alan).getByText('6 MiB')).toBeTruthy();

    arka.eklentiOnayi = true;
    await fireEvent.click(within(alan).getByRole('button', { name: 'Veriyi sil' }));
    expect(await within(alan).findByText('0 bayt')).toBeTruthy();
    expect(arka.komutlar('eklenti_verisini_sil')).toEqual([
      { ad: 'kalfa-rag', kimlik: 'alan-1' },
      { ad: 'kalfa-rag', kimlik: 'alan-1' },
    ]);
  });

  it('çalışma alanı yoksa oturum panelini işaret eden not çıkar', async () => {
    await ciz();
    const ekran = await eklentileriAc();
    const notlar = await kart(ekran, 'ornek-notlar');
    expect(within(notlar).getByText(/"Bu oturumdaki izinler" panelinden/)).toBeTruthy();
    expect(within(notlar).queryByRole('checkbox')).toBeNull();
  });

  it('alan eyleminin hatası eklentinin satırında gösterilir', async () => {
    const { arka, rag, alan } = await ragAlani();
    arka.hatalar.set('gonderim_iznini_kaldir', {
      kod: 'GecersizGirdi',
      mesaj: 'Çalışma alanı kayıtta yok.',
    });
    await fireEvent.click(within(alan).getByRole('button', { name: 'İzni kaldır' }));
    expect((await within(rag).findByRole('alert')).textContent).toBe('Çalışma alanı kayıtta yok.');
    expect(within(alan).getByText('İzin var · 20 Ekim')).toBeTruthy();
  });
});

describe('Bu oturumdaki izinler paneli: eklentiler (§14)', () => {
  const SATIRLAR: OturumEklentisi[] = [
    { ad: 'kalfa-rag', yerlesik: true, acik: true, kullanilamaz: false, gonderim_izni_ms: IZIN_MS },
    { ad: 'ornek-notlar', yerlesik: false, acik: false, kullanilamaz: false, gonderim_izni_ms: null },
    { ad: 'bozuk-eklenti', yerlesik: false, acik: false, kullanilamaz: true, gonderim_izni_ms: null },
  ];

  async function oturumluCiz(ayar: (a: SahteArka) => void = () => {}) {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Panel', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';
    arka.oturumEklentileri = structuredClone(SATIRLAR);
    ayar(arka);
    const durum = new AnaDurum();
    render(Ana, { props: { durum } });
    await screen.findByRole('heading', { name: 'Panel' });
    return { arka, durum };
  }

  it('yerleşikle aynı adlı kullanılamayan eski kayıt paneli bozmaz', async () => {
    await oturumluCiz((a) => {
      a.oturumEklentileri[2].ad = 'kalfa-rag';
    });
    await screen.findByText('Bu oturumdaki izinler (1)');
    const liste = screen.getByRole('list', { name: 'Eklentiler' });
    const satirlar = within(liste).getAllByRole('group', { name: 'kalfa-rag' });
    expect(satirlar).toHaveLength(2);
    const acik = within(satirlar[0]).getByRole('checkbox') as HTMLInputElement;
    const eski = within(satirlar[1]).getByRole('checkbox') as HTMLInputElement;
    expect(acik.checked).toBe(true);
    expect(acik.disabled).toBe(false);
    expect(eski.checked).toBe(false);
    expect(eski.disabled).toBe(true);
  });

  it('eklenti ve kısayol yoksa panel görünmez', async () => {
    const { arka } = await oturumluCiz((a) => (a.oturumEklentileri = []));
    await waitFor(() => expect(arka.komutlar('oturum_eklentileri')).toHaveLength(1));
    expect(screen.queryByText(/Bu oturumdaki izinler/)).toBeNull();
  });

  it('oturum açılınca eklentiler okunur; panel her eklentiyi listeler, başlık yalnızca açık olanı sayar', async () => {
    const { arka } = await oturumluCiz();
    expect(await screen.findByText('Bu oturumdaki izinler (1)')).toBeTruthy();
    expect(arka.komutlar('oturum_eklentileri')).toEqual([{ oturum: 'o1' }]);

    const liste = screen.getByRole('list', { name: 'Eklentiler' });
    const rag = within(liste).getByRole('group', { name: 'kalfa-rag' });
    const ragKutu = within(rag).getByRole('checkbox', {
      name: 'Bu çalışma alanında açık',
    }) as HTMLInputElement;
    expect(ragKutu.checked).toBe(true);
    expect(ragKutu.disabled).toBe(false);
    expect(within(rag).getByText('İzin var · 20 Ekim')).toBeTruthy();

    const notlar = within(liste).getByRole('group', { name: 'ornek-notlar' });
    expect(
      (within(notlar).getByRole('checkbox') as HTMLInputElement).checked,
    ).toBe(false);
    expect(within(notlar).getByText('İzin yok')).toBeTruthy();
    expect(within(notlar).queryByText('Devre dışı ya da yüklenemedi')).toBeNull();
  });

  it('kullanılamayan eklentinin anahtarı kapalıdır ve nedeni yazar', async () => {
    await oturumluCiz();
    const bozuk = await screen.findByRole('group', { name: 'bozuk-eklenti' });
    expect((within(bozuk).getByRole('checkbox') as HTMLInputElement).disabled).toBe(true);
    expect(within(bozuk).getByText('Devre dışı ya da yüklenemedi')).toBeTruthy();
  });

  it('anahtar oturumun çalışma alanını hedefler; liste yeniden okunur', async () => {
    const { arka } = await oturumluCiz();
    const notlar = await screen.findByRole('group', { name: 'ornek-notlar' });
    const kutu = within(notlar).getByRole('checkbox') as HTMLInputElement;
    await fireEvent.click(kutu);
    await waitFor(() => expect(kutu.checked).toBe(true));
    expect(arka.komutlar('eklenti_alani_ayarla')).toEqual([
      { ad: 'ornek-notlar', hedef: { tip: 'Oturum', oturum: 'o1' }, acik: true },
    ]);
    expect(arka.komutlar('oturum_eklentileri')).toHaveLength(2);
  });

  it('panel açılınca liste yeniden okunur', async () => {
    const { arka } = await oturumluCiz();
    const ozet = (await screen.findByText('Bu oturumdaki izinler (1)')).closest('details')!;
    expect(ozet.open).toBe(false);
    expect(arka.komutlar('oturum_eklentileri')).toHaveLength(1);
    // Eklentiler ekranında yapılan bir değişiklik panel açılınca görünür.
    arka.oturumEklentileri[1].acik = true;
    ozet.open = true;
    await waitFor(() => expect(arka.komutlar('oturum_eklentileri')).toHaveLength(2));
    const notlar = await screen.findByRole('group', { name: 'ornek-notlar' });
    await waitFor(() =>
      expect((within(notlar).getByRole('checkbox') as HTMLInputElement).checked).toBe(true),
    );
  });

  it('kısayollarla birlikte sayılır; ikisi de kendi bölümünde durur', async () => {
    const { arka } = await oturumluCiz();
    await screen.findByText('Bu oturumdaki izinler (1)');
    arka.olaylar('o1', [
      {
        tip: 'Kisayollar',
        liste: [{ kimlik: 7, teklif: { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' } }],
      },
    ]);
    expect(await screen.findByText('Bu oturumdaki izinler (2)')).toBeTruthy();
    expect(
      screen.getByText('Bu oturumda src/ altındaki dosyaları dosya_duzenle ile onaysız düzenleyebilir.'),
    ).toBeTruthy();
    expect(screen.getByRole('list', { name: 'Eklentiler' })).toBeTruthy();
  });

  it('yalnızca kısayol varsa eskisi gibi görünür', async () => {
    const { arka } = await oturumluCiz((a) => (a.oturumEklentileri = []));
    await waitFor(() => expect(arka.komutlar('oturum_eklentileri')).toHaveLength(1));
    arka.olaylar('o1', [
      {
        tip: 'Kisayollar',
        liste: [{ kimlik: 7, teklif: { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' } }],
      },
    ]);
    expect(await screen.findByText('Bu oturumdaki izinler (1)')).toBeTruthy();
    expect(screen.queryByRole('list', { name: 'Eklentiler' })).toBeNull();
  });

  it('başka oturum seçilince onun eklentileri okunur', async () => {
    const { arka } = await oturumluCiz((a) => {
      a.oturumEkle({ oturum: 'o2' }, { baslik: 'İkinci', durum: { tip: 'Tamamlandi' } });
    });
    await screen.findByText('Bu oturumdaki izinler (1)');
    await fireEvent.click(await screen.findByText('İkinci', { selector: '.baslik' }));
    await waitFor(() =>
      expect(arka.komutlar('oturum_eklentileri')).toEqual([{ oturum: 'o1' }, { oturum: 'o2' }]),
    );
  });

  it('geç gelen eski oturum yanıtı yeni oturumun panelini bozmaz', async () => {
    let birinciYanit: (liste: OturumEklentisi[]) => void = () => {};
    await oturumluCiz((a) => {
      a.oturumEkle({ oturum: 'o2' }, { baslik: 'İkinci', durum: { tip: 'Tamamlandi' } });
      a.ozel.set('oturum_eklentileri', (arg) =>
        arg.oturum === 'o1'
          ? new Promise<OturumEklentisi[]>((coz) => (birinciYanit = coz))
          : [{ ad: 'yalniz-o2', yerlesik: false, acik: true, kullanilamaz: false, gonderim_izni_ms: null }],
      );
    });
    await fireEvent.click(await screen.findByText('İkinci', { selector: '.baslik' }));
    expect(await screen.findByRole('group', { name: 'yalniz-o2' })).toBeTruthy();

    birinciYanit(structuredClone(SATIRLAR));
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByRole('group', { name: 'kalfa-rag' })).toBeNull();
    expect(screen.getByRole('group', { name: 'yalniz-o2' })).toBeTruthy();
    expect(screen.getByText('Bu oturumdaki izinler (1)')).toBeTruthy();
  });

  it('Eklentiler ekranından dönünce liste yeniden okunur', async () => {
    const { arka } = await oturumluCiz();
    await screen.findByText('Bu oturumdaki izinler (1)');
    await eklentileriAc();
    await fireEvent.click(within(menu()).getByRole('button', { name: 'Oturumlar' }));
    await screen.findByRole('heading', { name: 'Panel' });
    await waitFor(() => expect(arka.komutlar('oturum_eklentileri')).toHaveLength(2));
  });

  it('salt okunur oturumda eklentiler sorulmaz', async () => {
    const { arka } = await oturumluCiz((a) =>
      a.oturumEkle(
        { oturum: 'o1', salt_okunur: { tip: 'CalismaAlaniKayitsiz' } },
        { baslik: 'Panel', durum: { tip: 'Tamamlandi' } },
      ),
    );
    expect(arka.komutlar('oturum_eklentileri')).toHaveLength(0);
    expect(screen.queryByText(/Bu oturumdaki izinler/)).toBeNull();
  });

  it('okuma hatası konuşmanın üstündeki şeritte gösterilir', async () => {
    await oturumluCiz((a) =>
      a.hatalar.set('oturum_eklentileri', { kod: 'OturumYok', mesaj: 'Bu oturum açık değil.' }),
    );
    expect((await screen.findByRole('alert')).textContent).toContain('Bu oturum açık değil.');
    expect(screen.queryByText(/Bu oturumdaki izinler/)).toBeNull();
  });
});
