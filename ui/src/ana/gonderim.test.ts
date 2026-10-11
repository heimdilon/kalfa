import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { EklentiIstekOzeti, GunlukSayfasi, OnizlemeOgesi } from '../ipc/tipler';
import { SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';
import SourceTag from '../bilesenler/SourceTag.svelte';
import LogTable from '../bilesenler/LogTable.svelte';
import OnizlemeKarti from './OnizlemeKarti.svelte';
import GonderimGunlugu from './GonderimGunlugu.svelte';

afterEach(() => clearMocks());

const ORNEK_GUNLUK: GunlukSayfasi = {
  satirlar: [
    {
      sira: 10,
      tur: 't1',
      zaman: Date.parse('2026-10-05T10:00:00Z'),
      amac: 'ana',
      model: 'deepseek-v4-flash',
      yonlendirilen_model: 'deepseek-v4-flash',
      istek_id: 'istek-123',
      token: { istem: 120, tamamlama: 40, dusunce: 10 },
      sonuc: { tip: 'Basarili' },
      yeni: [
        { kaynak: 'mesaj:kullanıcı', sira: 1, bayt: 25 },
        { kaynak: 'dosya:src/main.rs:1-40', sira: 2, bayt: 1024 },
        { kaynak: 'kabuk:cargo test', sira: 3, bayt: 2048 },
        { kaynak: 'kalfa:sistem', sira: null, bayt: 500 },
        { kaynak: 'kalfa:araçlar', sira: null, bayt: 800 },
      ],
      onceki: 0,
    },
    {
      sira: 15,
      tur: 't1',
      zaman: Date.parse('2026-10-05T10:05:00Z'),
      amac: 'denetci',
      model: 'deepseek-v4-flash',
      yonlendirilen_model: 'deepseek-v4-flash',
      istek_id: 'istek-456',
      token: { istem: 300, tamamlama: 80, dusunce: 0 },
      sonuc: { tip: 'Basarili' },
      yeni: [
        { kaynak: 'arama:TODO', sira: 4, bayt: 512 },
      ],
      onceki: 5,
    },
    {
      sira: 20,
      tur: 't1',
      zaman: Date.parse('2026-10-05T10:10:00Z'),
      amac: 'ana',
      model: 'deepseek-v4-flash',
      yonlendirilen_model: 'deepseek-v4-flash',
      istek_id: 'istek-789',
      token: null,
      sonuc: { tip: 'Hata', asama: 'Yanit', mesaj: '429 Çok Fazla İstek' },
      yeni: [],
      onceki: 0,
    },
  ],
  daha_eski_var: false,
};

describe('gönderim günlüğü ve önizleme kartı (aşama 3b)', () => {
  it('SourceTag farklı kaynakları ve bayt boyutunu gösterir', async () => {
    const { container } = render(SourceTag, {
      props: {
        kaynak: 'dosya:src/main.rs:1-40',
        bayt: 1536,
      },
    });
    expect(screen.getByText(/src\/main\.rs:1-40/)).toBeTruthy();
    expect(screen.getByText(/1,5 KiB/)).toBeTruthy();
  });

  it('LogTable satırları listeler ve satır seçtirir', async () => {
    let secilen: any = null;
    render(LogTable, {
      props: {
        satirlar: ORNEK_GUNLUK.satirlar,
        seciliSira: 10,
        onsec: (s: any) => {
          secilen = s;
        },
      },
    });

    expect(screen.getAllByText('Ana model').length).toBe(2);
    expect(screen.getByText('Denetçi')).toBeTruthy();
    expect(screen.getByText('istek-123')).toBeTruthy();
    expect(screen.getByText('160')).toBeTruthy(); // 120 + 40
    expect(screen.getAllByText('Başarılı').length).toBe(2);
    expect(screen.getByText(/Hata/)).toBeTruthy();

    const ikinciSatir = screen.getByText('istek-456').closest('tr');
    expect(ikinciSatir).toBeTruthy();
    await fireEvent.click(ikinciSatir!);
    expect(secilen?.sira).toBe(15);
  });

  it('GonderimGunlugu satırları, SourceTag ve +N önceki mesajı gösterir; öğeye tıklanınca tam metin açılır', async () => {
    const arka = new SahteArka().kur();
    arka.gonderimGunlukleri.set('o1', ORNEK_GUNLUK);
    arka.gonderimOgeleri.set('o1:2', 'fn main() { println!("merhaba"); }');

    render(GonderimGunlugu, {
      props: {
        oturum: 'o1',
        onkapat: () => {},
      },
    });

    await waitFor(() => expect(screen.getByText('istek-123')).toBeTruthy());
    expect(screen.getByText(/src\/main\.rs:1-40/)).toBeTruthy();

    // Satır 15'i seçince +5 önceki mesaj görünmeli
    const satir2 = screen.getByText('istek-456').closest('tr');
    await fireEvent.click(satir2!);
    expect(screen.getByText('+5 önceki mesaj')).toBeTruthy();

    // Satır 10'a dönüp öğeye tıklayınca tam metin açılır
    const satir1 = screen.getByText('istek-123').closest('tr');
    await fireEvent.click(satir1!);
    const etiket = screen.getByText(/src\/main\.rs:1-40/);
    await fireEvent.click(etiket);

    await waitFor(() => expect(screen.getByText(/println!\("merhaba"\)/)).toBeTruthy());
  });

  it('GonderimGunlugu eski kayıtta "ayrıntı yok" gösterir', async () => {
    const arka = new SahteArka().kur();
    arka.gonderimGunlukleri.set('o1', {
      satirlar: [
        {
          sira: 5,
          amac: 'ana',
          sonuc: { tip: 'Basarili' },
          yeni: [],
          onceki: 0,
        },
      ],
      daha_eski_var: false,
    });

    render(GonderimGunlugu, {
      props: {
        oturum: 'o1',
        onkapat: () => {},
      },
    });

    await waitFor(() => expect(screen.getByText('Ayrıntı yok')).toBeTruthy());
  });

  it('OnizlemeKarti öğeleri listeler, Gönder tümünü yollar, işaretliyle düğme işaretliyi çıkarır', async () => {
    let sonKarar: { islem: string; gonderilmeyecekler: string[] } | null = null;
    const oge = {
      anahtar: 'c-1',
      sira: 1,
      tur: 't1' as const,
      tip: 'GonderimOnizlemesi' as const,
      islem: 'islem-1',
      neden: 'Onizleme' as const,
      durum: 'Bekliyor' as const,
      ogeler: [
        { cagri: 'c1', kaynak: 'dosya:src/a.rs:1-20', bayt: 500 },
        { cagri: 'c2', kaynak: 'dosya:src/b.rs:1-30', bayt: 700 },
      ],
    };

    const { rerender } = render(OnizlemeKarti, {
      props: {
        oge,
        onkarar: (islem: string, gonderilmeyecekler: string[]) => {
          sonKarar = { islem, gonderilmeyecekler };
        },
      },
    });

    expect(screen.getByText(/Gönderim önizlemesi/)).toBeTruthy();
    expect(screen.getByText(/src\/a\.rs:1-20/)).toBeTruthy();
    expect(screen.getByText(/src\/b\.rs:1-30/)).toBeTruthy();

    // Hiçbir öğe seçili değilken "Gönder" tümünü yollar
    const gonderDugmesi = screen.getByRole('button', { name: 'Gönder' });
    await fireEvent.click(gonderDugmesi);
    expect(sonKarar).toEqual({ islem: 'islem-1', gonderilmeyecekler: [] });

    // Birini işaretle
    const kutular = screen.getAllByRole('checkbox');
    expect(kutular.length).toBe(2);
    await fireEvent.click(kutular[0]);

    // İşaretli öğe varken tek düğme işaretlileri çıkarıp kalanları yollar (3c Görev 8)
    const cikarmaDugmesi = screen.getByRole('button', { name: 'İşaretlileri çıkarıp gönder' });
    await fireEvent.click(cikarmaDugmesi);
    expect(sonKarar).toEqual({ islem: 'islem-1', gonderilmeyecekler: ['c1'] });
    expect(screen.queryByRole('button', { name: 'Seçilenleri gönderme' })).toBeNull();
  });

  it('OnizlemeKarti neden: Denetim olduğunda işaretli öğe "gönderme" diye işaretli gelir', async () => {
    const oge = {
      anahtar: 'c-2',
      sira: 2,
      tur: 't1' as const,
      tip: 'GonderimOnizlemesi' as const,
      islem: 'islem-2',
      neden: 'Denetim' as const,
      durum: 'Bekliyor' as const,
      ogeler: [
        { cagri: 'c3', kaynak: 'dosya:gizli.txt:1-10', bayt: 200, denetim: 'Şüpheli içerik' },
      ],
    };

    render(OnizlemeKarti, {
      props: {
        oge,
        onkarar: () => {},
      },
    });

    // İşaretin anlamı "gönderme"dir; denetçinin işaretlediği öğe gönderilmemek üzere gelir.
    const kutu = screen.getByRole<HTMLInputElement>('checkbox');
    expect(kutu.checked).toBe(true);
  });

  it('oturumda "Göndermeden önce göster" anahtarı komutu çağırır', async () => {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Oturum 1', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';

    const durum = new AnaDurum();
    render(Ana, { props: { durum } });
    await screen.findByRole('heading', { name: 'Oturum 1' });

    const anahtar = screen.getByRole('checkbox', { name: 'Göndermeden önce göster' });
    expect(anahtar).toBeTruthy();
    await fireEvent.click(anahtar);

    await waitFor(() => {
      expect(arka.komutlar('onizleme_ayarla')).toEqual([{ oturum: 'o1', acik: true }]);
    });
  });

  it('menüden ve TurOzeti EVREN çağrısına tıklanınca gönderim günlüğü açılır', async () => {
    const arka = new SahteArka().kur();
    arka.gonderimGunlukleri.set('o1', ORNEK_GUNLUK);
    arka.oturumEkle(
      {
        oturum: 'o1',
        gecmis: {
          ogeler: [
            {
              tip: 'TurOzeti',
              sira: 1,
              tur: 't1',
              ozet: {
                tur: 't1',
                dosyalar: [
                  {
                    yol: 'src/main.rs',
                    eklenen: 1,
                    silinen: 0,
                    islem: 'Olusturuldu',
                    sonucu_bilinmiyor: false,
                    geri_alinabilir: { tip: 'Evet' },
                  },
                ],
                komutlar: [],
                istekler: { sayi: 2, istem: 100, tamamlama: 50, dusunce: 0 },
                guvenlik_uyarisi: 0,
              },
            },
          ],
          daha_eski_var: false,
        },
      },
      { baslik: 'Oturum 1', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';

    const durum = new AnaDurum();
    render(Ana, { props: { durum } });
    await screen.findByRole('heading', { name: 'Oturum 1' });

    // Tur özeti içindeki EVREN çağrısı satırına tıkla
    const istekSatiri = screen.getByRole('button', { name: /2 EVREN isteği, 150 token/ });
    await fireEvent.click(istekSatiri);

    const dialog = await screen.findByRole('dialog', { name: 'Gönderim günlüğü' });
    expect(dialog).toBeTruthy();

    // Kapat
    await fireEvent.click(within(dialog).getByRole('button', { name: 'Kapat' }));
    expect(screen.queryByRole('dialog')).toBeNull();

    // Menüdeki "Gönderim günlüğü"ne tıkla
    const menu = screen.getByRole('navigation', { name: 'Menü' });
    const menuGunluk = within(menu).getByRole('button', { name: 'Gönderim günlüğü' });
    await fireEvent.click(menuGunluk);

    expect(await screen.findByRole('dialog', { name: 'Gönderim günlüğü' })).toBeTruthy();
  });
});

describe('eklenti gönderimi (aşama 4b)', () => {
  const EKLENTI_OZETI: EklentiIstekOzeti = {
    eklenti: 'kalfa-rag',
    uc: 'Gomme',
    model: 'qwen3-embedding',
    metin_sayisi: 12,
    bayt: 2048,
    ozet: 'f3a9c2',
    dosyalar: ['src/a.rs', 'src/b.rs'],
    dosya_fazlasi: 12,
  };

  const eklentiOgesi = (cagri: string): OnizlemeOgesi => ({
    cagri,
    kaynak: 'eklenti:kalfa-rag:gomme',
    bayt: 2048,
    eklenti: EKLENTI_OZETI,
  });

  it('önizleme kartı eklenti isteğini ayrıntısıyla gösterir ve üç kararı yollar', async () => {
    const kararlar: { islem: string; gonderilmeyecekler: string[]; hep: string[] }[] = [];
    const oge = {
      anahtar: 'c-3',
      sira: 3,
      tur: 't1' as const,
      tip: 'GonderimOnizlemesi' as const,
      islem: 'islem-3',
      neden: 'Eklenti' as const,
      durum: 'Bekliyor' as const,
      ogeler: [eklentiOgesi('c9')],
    };

    render(OnizlemeKarti, {
      props: {
        oge,
        onkarar: (islem: string, gonderilmeyecekler: string[], hep: string[]) => {
          kararlar.push({ islem, gonderilmeyecekler, hep });
        },
      },
    });

    expect(screen.getByText("Eklenti EVREN'e veri göndermek istiyor")).toBeTruthy();
    expect(screen.getByText('Eklenti · kalfa-rag')).toBeTruthy();
    expect(screen.getByText('Gömme')).toBeTruthy();
    expect(screen.getByText('qwen3-embedding')).toBeTruthy();
    expect(screen.getByText('2 KiB')).toBeTruthy();
    expect(screen.getByText('src/b.rs')).toBeTruthy();
    expect(screen.getByText('ve 12 dosya daha')).toBeTruthy();
    expect(screen.queryByRole('checkbox')).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Gönder' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Gönderme' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Bu araç çağrısında hep gönder' }));

    expect(kararlar).toEqual([
      { islem: 'islem-3', gonderilmeyecekler: [], hep: [] },
      { islem: 'islem-3', gonderilmeyecekler: ['c9'], hep: [] },
      { islem: 'islem-3', gonderilmeyecekler: [], hep: ['c9'] },
    ]);
  });

  it('"hep gönder" düğmesi yalnızca eklenti isteğinde gelir', () => {
    const oge = {
      anahtar: 'c-4',
      sira: 4,
      tur: 't1' as const,
      tip: 'GonderimOnizlemesi' as const,
      islem: 'islem-4',
      neden: 'Onizleme' as const,
      durum: 'Bekliyor' as const,
      ogeler: [{ cagri: 'c5', kaynak: 'dosya:src/c.rs:1-5', bayt: 300 }],
    };

    render(OnizlemeKarti, { props: { oge, onkarar: () => {} } });

    expect(screen.getByRole('button', { name: 'Gönder' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Gönderme' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Bu araç çağrısında hep gönder' })).toBeNull();
  });

  it('"hep gönder" kararı gonderim_karari komutuna hep_gonderilecekler olarak gider', async () => {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Oturum 1', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';

    render(Ana, { props: { durum: new AnaDurum() } });
    await screen.findByRole('heading', { name: 'Oturum 1' });

    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'GonderimOnizlemesi', islem: 'islem-5', neden: 'Eklenti', ogeler: [eklentiOgesi('c7')] },
    ]);
    await fireEvent.click(
      await screen.findByRole('button', { name: 'Bu araç çağrısında hep gönder' }),
    );

    await waitFor(() =>
      expect(arka.komutlar('gonderim_karari')).toEqual([
        { oturum: 'o1', islem: 'islem-5', gonderilmeyecekler: [], hep_gonderilecekler: ['c7'] },
      ]),
    );
  });

  it('gönderim günlüğü eklenti satırını amacı, "Gönderilmedi" sonucu ve ayrıntısıyla gösterir', async () => {
    const arka = new SahteArka().kur();
    const gunluk: GunlukSayfasi = {
      satirlar: [
        {
          sira: 30,
          tur: 't1',
          zaman: Date.parse('2026-10-08T09:00:00Z'),
          amac: 'eklenti:kalfa-rag',
          model: 'qwen3-embedding',
          istek_id: 'istek-eklenti-1',
          sonuc: { tip: 'Basarili' },
          yeni: [{ kaynak: 'eklenti:kalfa-rag:gomme', bayt: 2048 }],
          onceki: 0,
          eklenti: EKLENTI_OZETI,
        },
        {
          sira: 31,
          tur: 't1',
          amac: 'eklenti:kalfa-rag',
          model: 'rerank-qwen3',
          sonuc: { tip: 'Gonderilmedi', neden: 'kullanıcı göndermedi' },
          yeni: [{ kaynak: 'eklenti:kalfa-rag:siralama', bayt: 512 }],
          onceki: 0,
          eklenti: { ...EKLENTI_OZETI, uc: 'Siralama', dosyalar: [], dosya_fazlasi: 0 },
        },
      ],
      daha_eski_var: false,
    };
    arka.gonderimGunlukleri.set('o1', gunluk);

    render(GonderimGunlugu, { props: { oturum: 'o1', onkapat: () => {} } });

    await waitFor(() => expect(screen.getByText('istek-eklenti-1')).toBeTruthy());
    expect(screen.getAllByText('Eklenti · kalfa-rag').length).toBe(2);
    expect(screen.getByText('Gönderilmedi: kullanıcı göndermedi')).toBeTruthy();

    // İlk satırın ayrıntısı: uç, okunan dosyalar, fazlası, özet ve metnin kaydedilmediği notu
    const ilkSatir = screen.getByText('istek-eklenti-1').closest('tr');
    await fireEvent.click(ilkSatir!);
    expect(screen.getByText('Gömme')).toBeTruthy();
    expect(screen.getByText('src/b.rs')).toBeTruthy();
    expect(screen.getByText('ve 12 dosya daha')).toBeTruthy();
    expect(screen.getByText('f3a9c2')).toBeTruthy();
    expect(screen.getByText(/Metin kayda yazılmadı\./)).toBeTruthy();

    // İkinci satır: okunan dosya yok ve yeniden sıralama ucu
    const ikinciSatir = screen.getByText('Gönderilmedi: kullanıcı göndermedi').closest('tr');
    await fireEvent.click(ikinciSatir!);
    expect(screen.getByText('Yeniden sıralama')).toBeTruthy();
    expect(screen.getByText('Bu çağrıda okunan dosya yok.')).toBeTruthy();
  });
});
