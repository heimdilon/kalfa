// "Bu oturumda izin ver" kısayolları ve Ayarlar'ın aşama 3 bölümleri
// (aşama 3 tasarımı §9.1, §9.2, §11.4; 3e planı Görev 5).
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { OnayGirdisi } from '../ipc/tipler';
import { onayOgesi, SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';
import OnaySayfasi from './OnaySayfasi.svelte';

afterEach(() => clearMocks());

const TEKLIF = { tip: 'Dosya' as const, arac: 'dosya_duzenle', dizin: 'src' };
const CUMLE = 'Bu oturumda src/ altındaki dosyaları dosya_duzenle ile onaysız düzenleyebilir.';

function sayfa(ogeler = [onayOgesi({ cagri: 'c1', muhur: 'm1', kisayol: TEKLIF })]) {
  let kararlar: OnayGirdisi[] = [];
  render(OnaySayfasi, {
    props: {
      ogeler,
      onkarar: async (k: OnayGirdisi[]) => {
        kararlar = k;
      },
      onkapat: () => {},
    },
  });
  return () => kararlar;
}

describe('onay sayfasında kapsam (§9.1)', () => {
  it('kapsam satırı yalnızca teklif varsa görünür', () => {
    sayfa([onayOgesi({ cagri: 'c1' })]);
    expect(screen.queryByText('Kapsam')).toBeNull();
  });

  it('varsayılan "Yalnızca bu çağrı"; seçilen kısayol "Seçilenleri onayla" ile gider', async () => {
    const kararlar = sayfa();
    expect(screen.getByText('Kapsam')).toBeTruthy();
    expect(screen.getByText('Yalnızca bu çağrı')).toBeTruthy();
    await fireEvent.click(screen.getByRole('checkbox', { name: CUMLE }));
    expect(screen.queryByText('Yalnızca bu çağrı')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Seçilenleri onayla' }));
    expect(kararlar()).toEqual([
      { cagri: 'c1', muhur: 'm1', karar: { tip: 'Onayla' }, kisayol: true },
    ]);
  });

  it('"Tümünü onayla" kısayol göndermez', async () => {
    const kararlar = sayfa();
    await fireEvent.click(screen.getByRole('checkbox', { name: CUMLE }));
    await fireEvent.click(screen.getByRole('button', { name: 'Tümünü onayla' }));
    expect(kararlar()[0].kisayol).toBe(false);
  });

  it('kabuk kısayolunun cümlesi', () => {
    sayfa([
      onayOgesi({
        cagri: 'c1',
        kisayol: { tip: 'Kabuk', program: 'cargo', dizin: '.' },
      }),
    ]);
    expect(
      screen.getByRole('checkbox', {
        name: 'Bu oturumda cargo programını . dizininde yalıtımlı ve ağsız çalıştırabilir.',
      }),
    ).toBeTruthy();
  });
});

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle(
    { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
    { baslik: 'Kısayollar', durum: { tip: 'Tamamlandi' } },
  );
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('heading', { name: 'Kısayollar' });
  return { arka, durum };
}

describe('konuşmanın üstünde kısayol listesi', () => {
  it('liste çekirdekten gelir; kaldırmak kisayolu_kaldir çağırır', async () => {
    const { arka } = await ciz();
    expect(screen.queryByText(/Bu oturumdaki izinler/)).toBeNull();
    arka.olaylar('o1', [{ tip: 'Kisayollar', liste: [{ kimlik: 7, teklif: TEKLIF }] }]);
    expect(await screen.findByText('Bu oturumdaki izinler (1)')).toBeTruthy();
    expect(screen.getByText(CUMLE)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Bu izni kaldır' }));
    await waitFor(() =>
      expect(arka.komutlar('kisayolu_kaldir')).toEqual([{ oturum: 'o1', kimlik: 7 }]),
    );
    arka.olaylar('o1', [{ tip: 'Kisayollar', liste: [] }]);
    await waitFor(() => expect(screen.queryByText(/Bu oturumdaki izinler/)).toBeNull());
  });
});

async function ayarlariAc(ayar: (a: SahteArka) => void = () => {}) {
  const sonuc = await ciz(ayar);
  const menu = screen.getByRole('navigation', { name: 'Menü' });
  await fireEvent.click(within(menu).getByRole('button', { name: 'Ayarlar' }));
  await screen.findByRole('region', { name: 'Ayarlar' });
  return sonuc;
}

describe('Ayarlar: gizli dosyalar, ağ ve denetçi', () => {
  it('varsayılanlar salt okunur; ek kalıp eklemek onaysız komutu çağırır', async () => {
    const { arka } = await ayarlariAc();
    expect(await screen.findByText(/Varsayılan kalıplar \(4\); kapatılamaz/)).toBeTruthy();
    expect(
      screen.getByText('Yalnızca adı eşleşen dosyalar gizli sayılır; içerik taranmaz.'),
    ).toBeTruthy();
    const girdi = screen.getByRole('textbox', { name: 'Ek gizli kalıplar' });
    await fireEvent.input(girdi, { target: { value: '*.sifre' } });
    await fireEvent.submit(girdi.closest('form')!);
    await waitFor(() =>
      expect(arka.komutlar('ek_gizli_kalip_ekle')).toEqual([{ kalip: '*.sifre' }]),
    );
    expect(await screen.findByText('*.sifre')).toBeTruthy();
  });

  it("istisna eklemek Rust'un yerel onay komutunu çağırır; vazgeçilirse liste değişmez", async () => {
    const { arka } = await ayarlariAc((a) => (a.gizliOnayi = false));
    const girdi = await screen.findByRole('textbox', { name: 'Gizli sayılmayanlar' });
    await fireEvent.input(girdi, { target: { value: 'src/credentials.rs' } });
    await fireEvent.submit(girdi.closest('form')!);
    await waitFor(() =>
      expect(arka.komutlar('gizli_sayilmayan_ekle')).toEqual([{ kalip: 'src/credentials.rs' }]),
    );
    expect(screen.getByText('Gizli sayılmayan kalıp yok.')).toBeTruthy();
  });

  it('ağ listesinde özel ağ işareti görünür; kaldırmak komutu çağırır', async () => {
    const { arka } = await ayarlariAc((a) => {
      a.agListesi = [{ host: 'pkg.sirket.local', port: 8443, ozel_ag: true }];
    });
    expect(await screen.findByText('pkg.sirket.local:8443')).toBeTruthy();
    expect(screen.getByText('(özel ağ)')).toBeTruthy();
    await fireEvent.click(
      screen.getByRole('button', { name: 'pkg.sirket.local:8443 öğesini kaldır' }),
    );
    await waitFor(() =>
      expect(arka.komutlar('ag_hostu_kaldir')).toEqual([{ host: 'pkg.sirket.local:8443' }]),
    );
    expect(await screen.findByText('Eklenmiş adres yok.')).toBeTruthy();
  });

  it('komut hatası şeritte gösterilir', async () => {
    const { arka } = await ayarlariAc((a) => {
      a.hatalar.set('ek_gizli_kalip_ekle', {
        kod: 'GecersizGirdi',
        mesaj: '`/x`: kalıp çalışma alanına göre yazılır; mutlak yol olamaz.',
      });
    });
    const girdi = await screen.findByRole('textbox', { name: 'Ek gizli kalıplar' });
    await fireEvent.input(girdi, { target: { value: '/x' } });
    await fireEvent.submit(girdi.closest('form')!);
    expect(await screen.findByText(/mutlak yol olamaz/)).toBeTruthy();
    expect(arka.komutlar('ek_gizli_kalip_ekle').length).toBe(1);
  });

  it('denetçi kipi seçilir', async () => {
    const { arka } = await ayarlariAc();
    expect(await screen.findByText('Güvenlik denetçisi')).toBeTruthy();
    expect(screen.getByRole('combobox', { name: 'Denetçi kipi' })).toBeTruthy();
    expect(arka.denetci.kip).toBe('Kapali');
  });
});
