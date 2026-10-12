// Eklentinin onay ve önizleme kartları ile durum çubuğundaki günlük sınır
// (aşama 4 tasarımı §8.4, §9.4, §9.5, §14).
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { EkranOgesi } from '../durum/oturum';
import type { EklentiIstekOzeti, EklentiTarifi, OnayGirdisi, OnayOgesi } from '../ipc/tipler';
import { onayOgesi, SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';
import IlkGonderimKarti from './IlkGonderimKarti.svelte';
import OnaySayfasi from './OnaySayfasi.svelte';
import OnizlemeKarti from './OnizlemeKarti.svelte';

afterEach(() => clearMocks());

const ACIKLAMA_ILK = [
  'kalfa-rag eklentisinin `anlamsal_ara` aracını çalıştırmak istiyor.',
  "**kalfa-rag** bu çalışma alanında ilk kez EVREN'e veri gönderecek.",
  "Okuyabileceği dosyalar: `**/*.md`, `**/*.rs` ile eşleşen **412 dosya (6,0 MB)**. Bu dosyalardan seçtiği parçalar gömme ve yeniden sıralama için EVREN'e gidecek. Gizli dosyalar gönderilmez.",
  'İzin verirseniz bu çalışma alanında bir daha sorulmaz. İzni Eklentiler ekranından geri alabilirsiniz.',
].join('\n\n');

const TARIF: EklentiTarifi = {
  ad: 'kalfa-rag',
  surum: '0.1.0',
  ozet: 'abc123',
  globlar: ['**/*.md', '**/*.rs'],
  evren_uclari: ['embeddings', 'rerank'],
  ilk_gonderim: true,
  dosya_sayisi: 412,
  bayt: 6_000_000,
};

function ilkGonderimOgesi(ek: Partial<OnayOgesi> = {}): OnayOgesi {
  return onayOgesi({
    cagri: 'e1',
    arac: 'kalfa_rag__anlamsal_ara',
    aciklama: ACIKLAMA_ILK,
    muhur: 'muhur-e1',
    yerel_onay: true,
    veri_cikisi: 'EklentiCiktisi',
    eklenti: TARIF,
    ...ek,
  });
}

describe('ilk gönderim kartı (§8.4, §14)', () => {
  it('eklenti adı ve sürümü, EVREN uçları, dosya sayısı ve boyut', () => {
    render(IlkGonderimKarti, { props: { oge: ilkGonderimOgesi() } });
    const kart = screen.getByRole('region', { name: "EVREN'e ilk gönderim: kalfa-rag" });
    expect(within(kart).getByText('kalfa-rag 0.1.0')).toBeTruthy();
    expect(within(kart).getByText('Gömme, Yeniden sıralama')).toBeTruthy();
    expect(within(kart).getByText('412 dosya (6,0 MB)')).toBeTruthy();
  });

  it('"Neden izin gerekiyor" altında açıklamanın tamamı, paragrafları korunarak', () => {
    render(IlkGonderimKarti, { props: { oge: ilkGonderimOgesi() } });
    expect(screen.getByRole('heading', { name: 'Neden izin gerekiyor' })).toBeTruthy();
    const neden = screen.getByText(/ilk kez EVREN'e veri gönderecek/).closest('p') as HTMLElement;
    expect(neden.textContent).toBe(ACIKLAMA_ILK);
  });

  it('görünmez karakterler kaçışlanır; açıklama HTML olarak yorumlanmaz', () => {
    render(IlkGonderimKarti, {
      props: { oge: ilkGonderimOgesi({ aciklama: 'iyi‮kötü <b>kalın</b>' }) },
    });
    const neden = screen.getByText(/iyi/).closest('p') as HTMLElement;
    expect(neden.textContent).toBe('iyi⟨U+202E⟩kötü <b>kalın</b>');
    expect(neden.querySelector('b')).toBeNull();
  });

  it('onay yerel pencerede ayrıca sorulacaksa not düşülür', () => {
    const { unmount } = render(IlkGonderimKarti, { props: { oge: ilkGonderimOgesi() } });
    expect(
      screen.getByText('Bu izin kalıcı olduğu için onayınız yerel bir pencerede ayrıca sorulur.'),
    ).toBeTruthy();
    unmount();
    render(IlkGonderimKarti, { props: { oge: ilkGonderimOgesi({ yerel_onay: false }) } });
    expect(screen.queryByText(/yerel bir pencerede/)).toBeNull();
  });

  it('tarifi olmayan öğede hiçbir şey çizmez', () => {
    const { container } = render(IlkGonderimKarti, {
      props: { oge: ilkGonderimOgesi({ eklenti: undefined }) },
    });
    expect(container.querySelector('section')).toBeNull();
  });
});

describe('onay sayfasında eklenti', () => {
  function sayfa(ogeler: OnayOgesi[]) {
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

  it('ilk gönderimde diff yerine kart gelir; açıklama iki kez basılmaz', () => {
    sayfa([ilkGonderimOgesi()]);
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByRole('region', { name: "EVREN'e ilk gönderim: kalfa-rag" })).toBeTruthy();
    expect(within(dialog).getByRole('heading', { name: 'Neden izin gerekiyor' })).toBeTruthy();
    expect(within(dialog).queryByText('Bu değişiklik için önizleme yok.')).toBeNull();
    // "Ne" satırında yalnızca aracı adlandıran ilk paragraf, tamamı kartta.
    expect(within(dialog).getAllByText(/ilk kez EVREN'e veri gönderecek/)).toHaveLength(1);
    expect(
      within(dialog).getByText('kalfa-rag eklentisinin `anlamsal_ara` aracını çalıştırmak istiyor.'),
    ).toBeTruthy();
    // Kaynak satırı ve veri çıkışı.
    expect(within(dialog).getByText('Eklenti kalfa-rag 0.1.0')).toBeTruthy();
    expect(within(dialog).getByText('Eklentinin ürettiği metin modele gidecek.')).toBeTruthy();
  });

  it('kabuğa özgü yerel onay notu eklentide çıkmaz; yerel onay kartta anlatılır', () => {
    sayfa([ilkGonderimOgesi()]);
    expect(screen.queryByText(/Yalıtımsız komutlar/)).toBeNull();
    expect(screen.getByText(/onayınız yerel bir pencerede ayrıca sorulur/)).toBeTruthy();
  });

  it('ilk gönderim değilse kart yok; Kaynak satırı var', () => {
    sayfa([ilkGonderimOgesi({ eklenti: { ...TARIF, ilk_gonderim: false }, aciklama: 'Tek paragraf.' })]);
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).queryByRole('region', { name: /ilk gönderim/ })).toBeNull();
    expect(within(dialog).getByText('Bu değişiklik için önizleme yok.')).toBeTruthy();
    expect(within(dialog).getByText('Eklenti kalfa-rag 0.1.0')).toBeTruthy();
    expect(within(dialog).getByText('Tek paragraf.')).toBeTruthy();
  });

  it('eklentisiz öğede Kaynak satırı ve kart yok; kabuk notu eskisi gibi', () => {
    sayfa([onayOgesi({ cagri: 'k1', arac: 'kabuk', yerel_onay: true, aciklama: 'x' })]);
    expect(screen.queryByText('Kaynak')).toBeNull();
    expect(screen.getByText(/Yalıtımsız komutlar/)).toBeTruthy();
  });

  it('karar akışı değişmez: mühür ve onay aynen gider', async () => {
    const kararlar = sayfa([ilkGonderimOgesi()]);
    await fireEvent.click(screen.getByRole('button', { name: 'Tümünü onayla' }));
    expect(kararlar()).toEqual([
      { cagri: 'e1', muhur: 'muhur-e1', karar: { tip: 'Onayla' }, kisayol: false },
    ]);
  });

  it('konuşmadaki onay isteğinden açılan sayfada da kart görünür', async () => {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Eklenti', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';
    render(Ana, { props: { durum: new AnaDurum() } });
    await screen.findByRole('heading', { name: 'Eklenti' });
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'OnayGerekiyor', islem: 'i1', ogeler: [ilkGonderimOgesi()] },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const dialog = await screen.findByRole('dialog');
    expect(within(dialog).getByRole('heading', { name: 'Neden izin gerekiyor' })).toBeTruthy();
  });
});

describe('önizleme kartında "Metni göster" (§9.4)', () => {
  const OZET: EklentiIstekOzeti = {
    eklenti: 'kalfa-rag',
    uc: 'Gomme',
    model: 'qwen3-embedding-8b',
    metin_sayisi: 12,
    bayt: 2048,
    ozet: 'f3a9c2',
    dosyalar: ['src/a.rs'],
    dosya_fazlasi: 0,
  };

  function onizleme(ek: Partial<Extract<EkranOgesi, { tip: 'GonderimOnizlemesi' }>> = {}) {
    return {
      anahtar: 'c-1',
      sira: 1,
      tur: 't1',
      tip: 'GonderimOnizlemesi' as const,
      islem: 'islem-7',
      neden: 'Eklenti' as const,
      durum: 'Bekliyor' as const,
      ogeler: [{ cagri: 'c9', kaynak: 'eklenti:kalfa-rag:gomme', bayt: 2048, eklenti: OZET }],
      ...ek,
    };
  }

  function ciz(
    metin: (islem: string) => Promise<string>,
    ek: Partial<Extract<EkranOgesi, { tip: 'GonderimOnizlemesi' }>> = {},
  ) {
    const istenen: string[] = [];
    render(OnizlemeKarti, {
      props: {
        oge: onizleme(ek),
        onkarar: () => {},
        onmetin: (islem: string) => {
          istenen.push(islem);
          return metin(islem);
        },
      },
    });
    return istenen;
  }

  it('tıklanınca işlem kimliğiyle metni ister, kaydırılabilir eş aralıklı alanda gösterir', async () => {
    const istenen = ciz(async () => '{"input":["fn main() {}"]}');
    expect(screen.queryByRole('region', { name: "EVREN'e gidecek metin" })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    const alan = await screen.findByRole('region', { name: "EVREN'e gidecek metin" });
    expect(alan.textContent).toBe('{"input":["fn main() {}"]}');
    expect(alan.tagName).toBe('PRE');
    expect(alan.getAttribute('tabindex')).toBe('0');
    expect(istenen).toEqual(['islem-7']);
    expect(screen.getByRole('button', { name: 'Metni gizle' }).getAttribute('aria-expanded')).toBe(
      'true',
    );
  });

  it('ikinci tık gizler; yeniden göstermek metni bir daha istemez', async () => {
    const istenen = ciz(async () => 'gövde');
    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    await screen.findByRole('region', { name: "EVREN'e gidecek metin" });
    await fireEvent.click(screen.getByRole('button', { name: 'Metni gizle' }));
    expect(screen.queryByRole('region', { name: "EVREN'e gidecek metin" })).toBeNull();
    expect(screen.getByRole('button', { name: 'Metni göster' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    expect(await screen.findByRole('region', { name: "EVREN'e gidecek metin" })).toBeTruthy();
    expect(istenen).toHaveLength(1);
  });

  it('hata satırda gösterilir; metin açılmaz; yeniden denenebilir', async () => {
    let sira = 0;
    const istenen = ciz(async () => {
      sira += 1;
      if (sira === 1) throw { kod: 'GecersizGirdi', mesaj: 'Bu istek artık beklemiyor.' };
      return 'ikinci denemede geldi';
    });
    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    expect((await screen.findByRole('alert')).textContent).toBe('Bu istek artık beklemiyor.');
    expect(screen.queryByRole('region', { name: "EVREN'e gidecek metin" })).toBeNull();
    expect(screen.getByRole('button', { name: 'Metni göster' })).toBeTruthy();

    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    expect((await screen.findByRole('region', { name: "EVREN'e gidecek metin" })).textContent).toBe(
      'ikinci denemede geldi',
    );
    expect(screen.queryByRole('alert')).toBeNull();
    expect(istenen).toHaveLength(2);
  });

  it('görünmez karakterler kaçışlanır; HTML yorumlanmaz', async () => {
    ciz(async () => 'a​b <i>c</i>');
    await fireEvent.click(screen.getByRole('button', { name: 'Metni göster' }));
    const alan = await screen.findByRole('region', { name: "EVREN'e gidecek metin" });
    expect(alan.textContent).toBe('a⟨U+200B⟩b <i>c</i>');
    expect(alan.querySelector('i')).toBeNull();
  });

  it('eklenti ayrıntısı olmayan öğede de düğme vardır', () => {
    ciz(async () => 'x', {
      ogeler: [{ cagri: 'c9', kaynak: 'eklenti:kalfa-rag:gomme', bayt: 10 }],
    });
    expect(screen.getByRole('button', { name: 'Metni göster' })).toBeTruthy();
  });

  it('eklenti önizlemesinden başkasında ve karar verildikten sonra düğme çıkmaz', () => {
    const { unmount } = render(OnizlemeKarti, {
      props: {
        oge: onizleme({ neden: 'Onizleme', ogeler: [{ cagri: 'c1', kaynak: 'dosya:a.rs:1-2', bayt: 5 }] }),
        onkarar: () => {},
        onmetin: async () => 'x',
      },
    });
    expect(screen.queryByRole('button', { name: 'Metni göster' })).toBeNull();
    unmount();

    const { unmount: kapat } = render(OnizlemeKarti, {
      props: { oge: onizleme({ durum: 'Yanitlandi' }), onkarar: () => {}, onmetin: async () => 'x' },
    });
    expect(screen.queryByRole('button', { name: 'Metni göster' })).toBeNull();
    kapat();

    render(OnizlemeKarti, { props: { oge: onizleme(), onkarar: () => {} } });
    expect(screen.queryByRole('button', { name: 'Metni göster' })).toBeNull();
  });

  it('konuşmada eklenti_istegi komutu oturum ve işlemle çağrılır', async () => {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Önizleme', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';
    arka.eklentiIstekleri.set('islem-9', '{"input":["merhaba"]}');
    render(Ana, { props: { durum: new AnaDurum() } });
    await screen.findByRole('heading', { name: 'Önizleme' });
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'GonderimOnizlemesi',
        islem: 'islem-9',
        neden: 'Eklenti',
        ogeler: [{ cagri: 'c9', kaynak: 'eklenti:kalfa-rag:gomme', bayt: 2048, eklenti: OZET }],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Metni göster' }));
    expect((await screen.findByRole('region', { name: "EVREN'e gidecek metin" })).textContent).toBe(
      '{"input":["merhaba"]}',
    );
    expect(arka.komutlar('eklenti_istegi')).toEqual([{ oturum: 'o1', islem: 'islem-9' }]);
  });
});

describe('durum çubuğunda günlük sınır (§9.5)', () => {
  /** Yerel saatle 12 Ekim 03:00; saat dilimi ne olursa olsun "03:00" yazılır. */
  const SIFIRLANMA = new Date(2026, 9, 12, 3, 0).toISOString();

  async function ciz(ayar: (a: SahteArka) => void = () => {}) {
    const arka = new SahteArka().kur();
    arka.oturumEkle(
      { oturum: 'o1', gecmis: { ogeler: [], daha_eski_var: false } },
      { baslik: 'Sınır', durum: { tip: 'Tamamlandi' } },
    );
    arka.uygulama.son_oturum = 'o1';
    ayar(arka);
    const durum = new AnaDurum();
    render(Ana, { props: { durum } });
    await screen.findByRole('heading', { name: 'Sınır' });
    return { arka, durum };
  }

  const cubuk = () => screen.getByRole('contentinfo', { name: 'Durum çubuğu' });

  it('sınır yoksa öğe çıkmaz; açılışta bir kez sorulur', async () => {
    const { arka } = await ciz();
    expect(arka.komutlar('gunluk_sinir')).toEqual([{}]);
    expect(within(cubuk()).queryByText(/Günlük sınır/)).toBeNull();
  });

  it('sınır doluysa sıfırlanma anı yerel saatle yazılır', async () => {
    await ciz((a) => (a.gunlukSinir = { sifirlanma: SIFIRLANMA }));
    const oge = await within(cubuk()).findByText('Günlük sınır doldu · sıfırlanma 03:00');
    expect(oge.getAttribute('title')).toContain('günlük token sınırı doldu');
  });

  it.each([
    ['sıfırlanma yok', null],
    ['zaman damgası değil', 'yarın sabah'],
    ['yalnızca rakam', '12'],
    ['geçersiz tarih', '2026-13-45T25:61:00Z'],
  ])('%s: yalnızca "Günlük sınır doldu"', async (_ad, sifirlanma) => {
    await ciz((a) => (a.gunlukSinir = { sifirlanma }));
    const oge = await within(cubuk()).findByText(/Günlük sınır doldu/);
    expect(oge.textContent?.trim()).toBe('Günlük sınır doldu');
  });

  it('hatayla biten tur sınırı gösterir; normal biten tur kaldırır', async () => {
    const { arka } = await ciz();
    expect(arka.komutlar('gunluk_sinir')).toHaveLength(1);

    arka.gunlukSinir = { sifirlanma: SIFIRLANMA };
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'Hata', mesaj: "EVREN'in günlük token sınırına ulaşıldı." },
      { tip: 'Tamamlandi', neden: { tip: 'Hata' } },
    ]);
    expect(await within(cubuk()).findByText(/Günlük sınır doldu/)).toBeTruthy();
    expect(arka.komutlar('gunluk_sinir')).toHaveLength(2);

    arka.gunlukSinir = null;
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'TokenParcasi', metin: 'Merhaba' },
      { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
    ]);
    await waitFor(() => expect(within(cubuk()).queryByText(/Günlük sınır/)).toBeNull());
    expect(arka.komutlar('gunluk_sinir')).toHaveLength(3);
  });

  it('iptalle biten tur sınıra yeniden bakmaz', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [{ tip: 'TurBasladi' }, { tip: 'Tamamlandi', neden: { tip: 'Iptal' } }]);
    await waitFor(() => expect(arka.komutlar('oturumlari_listele').length).toBeGreaterThan(1));
    expect(arka.komutlar('gunluk_sinir')).toHaveLength(1);
  });

  it('sorgu hata verirse önceki durum korunur', async () => {
    const { arka, durum } = await ciz((a) => (a.gunlukSinir = { sifirlanma: SIFIRLANMA }));
    await within(cubuk()).findByText(/Günlük sınır doldu/);
    arka.hatalar.set('gunluk_sinir', { kod: 'Ic', mesaj: 'Sınır okunamadı.' });
    await durum.gunlukSiniriYenile();
    expect(durum.gunlukSinir).toEqual({ sifirlanma: SIFIRLANMA });
    expect(durum.hata).toBe('Sınır okunamadı.');
  });
});
