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

  it('akan düşüncenin yalnızca sonu çizilir; tamamı akış bitince açılınca gelir', async () => {
    const { arka } = await ciz();
    // Çok uzun düşünce her karede baştan Markdown'a çözülürse arayüz geri kalır
    // (e2e "duraklayan arayüz"). Akarken yalnızca son kısım düz metin gösterilir.
    const uzun = `BAS ${'a'.repeat(20_000)} SON`;
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'DusunceParcasi', metin: uzun },
    ]);
    const canli = await screen.findByText(/SON$/);
    expect(canli.textContent!.length).toBeLessThan(2_100);
    expect(document.body.textContent).not.toContain('BAS a');

    arka.olaylar('o1', [{ tip: 'Tamamlandi', neden: { tip: 'Normal' } }]);
    const dugme = await screen.findByRole('button', { name: 'Düşünce' });
    await waitFor(() => expect(dugme.getAttribute('aria-expanded')).toBe('false'));
    await fireEvent.click(dugme);
    expect(await screen.findByText(/^BAS a+ SON$/)).toBeTruthy();
  });

  it('kabuk çağrısının çıktısı terminalde akar; kart çıkış kodunu gösterir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'AracCagrisiBasladi', cagri: 'k1', arac: 'kabuk', aciklama: '`cargo test` komutunu çalıştırmak istiyor.' },
      {
        tip: 'AracCiktisiParcasi',
        cagri: 'k1',
        akis: 'Standart',
        metin: '\x1b[32mtest ok\x1b[0m\n<b>kalın değil</b>\n',
        atlanan_bayt: 0,
      },
    ]);
    const terminal = await screen.findByLabelText('Komut çıktısı');
    expect(terminal.textContent).toContain('test ok');
    expect(terminal.textContent).toContain('<b>kalın değil</b>');
    expect(terminal.querySelector('b')).toBeNull();
    expect(terminal.querySelector('.r-yesil')?.textContent).toBe('test ok');
    expect(screen.getByText('Komut çalışıyor')).toBeTruthy();
    arka.olaylar('o1', [
      {
        tip: 'AracSonucu',
        cagri: 'k1',
        arac: 'kabuk',
        durum: 'Basarisiz',
        icerik: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 1,0 sn',
      },
    ]);
    expect(await screen.findByText('Çıkış kodu: 101')).toBeTruthy();
    expect(screen.getByLabelText('Komut çıktısı').textContent).toContain('test ok');
  });

  it('durum çubuğu yalıtım durumunu gösterir', async () => {
    await ciz();
    // Yalıtım yok ve yalıtımsız çalıştırma kapalı: komutlar kapalıdır (§9.8).
    const oge = screen.getByText('Komutlar kapalı', { selector: '.durum-ogesi' });
    expect(oge.getAttribute('title')).toContain('güvenle çalıştırılamıyor');
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

  it('güvenlik uyarısı konuşmada uyarı kartı olarak çizilir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'GuvenlikUyarisi', cagri: 'c1', mesaj: '`.git` kenara alındı.' },
    ]);
    const kart = (await screen.findByText('Güvenlik uyarısı')).closest('section');
    expect(within(kart as HTMLElement).getByText('`.git` kenara alındı.')).toBeTruthy();
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
          { cagri: 'c1', muhur: 'm1', karar: { tip: 'Onayla' }, kisayol: false },
          {
            cagri: 'c2',
            muhur: 'm2',
            karar: { tip: 'Reddet', gerekce: 'lib.rs kalsın' },
            kisayol: false,
          },
        ],
      },
    ]);
    expect(screen.getByText('Yanıtlandı')).toBeTruthy();
  });

  it('kabuk öğesinde diff yerine komut, dizin, ağ ve yalıtım; yerel onay notu', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i2',
        ogeler: [
          onayOgesi({
            cagri: 'k1',
            arac: 'kabuk',
            aciklama: '`cargo test` komutunu çalıştırmak istiyor.',
            uyarilar: ['Yalıtımsız: komut senin bütün yetkilerinle çalışacak.'],
            kabuk: {
              komut: 'cargo test --features e2e',
              bicim: { tip: 'Program' },
              dizin: 'crates/kalfa',
              ag: true,
              yalitim: { tip: 'Yalitimsiz', talimat: 'yok' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
            yerel_onay: true,
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('cargo test --features e2e')).toBeTruthy();
    expect(within(sayfa).getByText('crates/kalfa')).toBeTruthy();
    expect(within(sayfa).getByText('Sınırsız (yalıtımsız)')).toBeTruthy();
    expect(within(sayfa).getByText('Yalıtımsız')).toBeTruthy();
    expect(within(sayfa).getByText(/yerel bir pencerede bir kez daha sorulur/)).toBeTruthy();
    expect(within(sayfa).queryByText('Bu değişiklik için önizleme yok.')).toBeNull();
    // Ağlı komut riskli: toplu onaylanmaz, önceden seçili değildir.
    const tumu = within(sayfa).getByRole('button', { name: 'Tümünü onayla' });
    expect((tumu as HTMLButtonElement).disabled).toBe(true);
    expect(within(sayfa).getByText(/tek tek seçilerek onaylanır/)).toBeTruthy();
    await fireEvent.click(within(sayfa).getByRole('checkbox', { name: 'cargo' }));
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Seçilenleri onayla' }));
    await waitFor(() => expect(arka.komutlar('arac_onayi')).toHaveLength(1));
  });

  it('kabuk kartı ağ satırını ve ek hostları gösterir (aşama 3d Görev 5)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i_ag',
        ogeler: [
          onayOgesi({
            cagri: 'k_ag',
            arac: 'kabuk',
            aciklama: '`curl` komutunu çalıştırmak istiyor.',
            kabuk: {
              komut: 'curl https://github.com',
              bicim: { tip: 'Program' },
              dizin: '.',
              ag: true,
              ag_hostlari: ['github.com:443', 'api.custom.org:8080'],
              yalitim: { tip: 'Yalitimli', arka_uc: 'bwrap', profil: 'p1' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(
      within(sayfa).getByText(
        'Açık; paket kayıtları (crates.io, npm, PyPI), GitHub indirmeleri ve şu adresler:'
      )
    ).toBeTruthy();
    expect(within(sayfa).getByText('github.com:443')).toBeTruthy();
    expect(within(sayfa).getByText('api.custom.org:8080')).toBeTruthy();
  });

  it('bilgi vermeyen araç için onay kartı "araç sonucu" yazar, veri çıkışı olan için türünü yazar', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i_vc',
        ogeler: [
          onayOgesi({
            cagri: 'c_bilinmeyen',
            arac: 'ozel_arac',
            aciklama: 'özel bir iş yapacak',
          }),
          onayOgesi({
            cagri: 'c_oku',
            arac: 'dosya_oku',
            aciklama: 'dosya okuyacak',
            veri_cikisi: 'DosyaIcerigi',
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('Veri çıkışı')).toBeTruthy();
    expect(within(sayfa).getByText('Araç sonucu')).toBeTruthy();

    await fireEvent.click(within(sayfa).getByRole('button', { name: /dosya_oku değişikliğini göster/ }));
    expect(within(sayfa).getByText('Dosya içeriği')).toBeTruthy();
  });

  it('eklenti çıktısının onay sayfası modele gideceğini söyler (aşama 4b)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i_eklenti',
        ogeler: [
          onayOgesi({
            cagri: 'c_eklenti',
            arac: 'anlamsal_ara',
            aciklama: 'kalfa-rag eklentisinin aracını çalıştırmak istiyor',
            veri_cikisi: 'EklentiCiktisi',
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('Veri çıkışı')).toBeTruthy();
    expect(within(sayfa).getByText('Eklentinin ürettiği metin modele gidecek.')).toBeTruthy();
  });

  it('kabuk onayı onaysız değişen dosyaları ve derleme dosyası uyarısını gösterir (Bulgu 3)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i4',
        ogeler: [
          onayOgesi({
            cagri: 'k3',
            arac: 'kabuk',
            aciklama: '`cargo test` komutunu çalıştırmak istiyor.',
            kabuk: {
              komut: 'cargo test',
              bicim: { tip: 'Program' },
              dizin: '.',
              ag: false,
              yalitim: { tip: 'Yalitimli', arka_uc: 'linux', profil: 'p' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
            onaysiz_degisiklikler: {
              yollar: ['build.rs', 'src/a.rs'],
              komut_calisti: true,
              derleme_dosyalari: ['build.rs'],
            },
          }),
          onayOgesi({ cagri: 'c9', hedefler: ['src/b.rs'], muhur: 'm9' }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    await fireEvent.click(within(sayfa).getByRole('button', { name: /cargo değişikliğini göster/ }));
    expect(within(sayfa).getByText('Bu oturumda onayınız olmadan değişen dosyalar')).toBeTruthy();
    expect(within(sayfa).getByText('src/a.rs')).toBeTruthy();
    expect(within(sayfa).getByText(/Onaysız bir komut da çalıştı/)).toBeTruthy();
    expect(within(sayfa).getByText(/Derleme ya da betik dosyası değişti.*build\.rs/)).toBeTruthy();
    const tumu = within(sayfa).getByRole('button', { name: 'Tümünü onayla' });
    expect((tumu as HTMLButtonElement).disabled).toBe(true);
    // Riskli olmayan öğe seçili, riskli olan seçili değil.
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Seçilenleri onayla' }));
    await waitFor(() => expect(arka.komutlar('arac_onayi')).toHaveLength(1));
    const kararlar = (arka.komutlar('arac_onayi')[0] as { kararlar: { cagri: string; karar: { tip: string } }[] })
      .kararlar;
    expect(kararlar.find((k) => k.cagri === 'c9')?.karar.tip).toBe('Onayla');
    expect(kararlar.find((k) => k.cagri === 'k3')?.karar.tip).toBe('Reddet');
  });

  it('görünmez ve yön değiştiren karakterler kaçışla gösterilir (Bulgu 7)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i3',
        ogeler: [
          onayOgesi({
            cagri: 'k2',
            arac: 'kabuk',
            aciklama: 'komut',
            kabuk: {
              komut: 'echo "‮txt.exe"',
              bicim: { tip: 'Program' },
              dizin: '.',
              ag: false,
              yalitim: { tip: 'Yalitimli', arka_uc: 'linux', profil: 'p' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    const kacis = within(sayfa).getByText('⟨U+202E⟩');
    expect(kacis.classList.contains('kacis')).toBe(true);
    expect(sayfa.textContent).not.toContain('‮');
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

describe('tur özeti ve geri alma (aşama 3 tasarımı §5.11)', () => {
  const ozet = {
    tur: 't1',
    dosyalar: [
      {
        yol: 'src/a.rs',
        islem: 'Degisti' as const,
        eklenen: 2,
        silinen: 1,
        sonucu_bilinmiyor: false,
        geri_alinabilir: { tip: 'Evet' as const },
      },
    ],
    komutlar: [
      {
        cagri: 'k1',
        komut: 'cargo build',
        durum: 'Basarili' as const,
        degisiklikler: { dosyalar: [{ yol: 'target/x', tur: 'Eklendi' as const }], toplam: 1, eksik: false },
      },
    ],
    istekler: { sayi: 2, istem: 100, tamamlama: 20, dusunce: 0 },
    guvenlik_uyarisi: 0,
  };

  const plan = {
    ogeler: [
      {
        yol: 'src/a.rs',
        eylem: { tip: 'GeriYukle' as const, ozet: 'x' },
        simdiki: 'y',
        uyarilar: [],
        diff: DIFF,
      },
      {
        yol: 'b.rs',
        eylem: { tip: 'Cakisma' as const, neden: { tip: 'Elle' as const } },
        simdiki: 'z',
        uyarilar: [],
        diff: null,
      },
      {
        yol: '.env',
        eylem: { tip: 'GeriYukle' as const, ozet: 'e' },
        simdiki: 'w',
        uyarilar: ['Gizli dosya; içerik gösterilmiyor.'],
        diff: null,
      },
    ],
    komutlar: ozet.komutlar,
    veri_silindi: false,
  };

  it('tur özeti satırları ve notu çizilir; geri alınamayan turda düğme yok', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [{ tip: 'TurOzeti', ozet }]);
    const blok = await screen.findByRole('region', { name: 'Tur özeti' });
    expect(within(blok).getByText('src/a.rs +2 −1')).toBeTruthy();
    expect(within(blok).getByText('cargo build: 1 dosya değişti')).toBeTruthy();
    expect(within(blok).getByText('Kabukla yapılan değişiklikler geri alınamaz.')).toBeTruthy();
    expect(within(blok).getByRole('button', { name: 'Turu geri al…' })).toBeTruthy();

    arka.olaylar('o1', [
      {
        tip: 'TurOzeti',
        ozet: {
          ...ozet,
          tur: 't2',
          komutlar: [],
          dosyalar: [{ ...ozet.dosyalar[0], geri_alinabilir: { tip: 'Hayir', neden: 'Buyuk' } }],
        },
      },
    ]);
    await waitFor(() => expect(screen.getAllByRole('region', { name: 'Tur özeti' })).toHaveLength(2));
    const ikinci = screen.getAllByRole('region', { name: 'Tur özeti' })[1];
    expect(within(ikinci).queryByRole('button', { name: 'Turu geri al…' })).toBeNull();
    // Tur sonu çizgisi ayrı kalır: uçtan uca testler onu sayar.
    expect(document.querySelectorAll('.tur-sonu')).toHaveLength(0);
  });

  it('"Turu geri al…" önizler; sayfa eylemleri, çakışmayı ve gizli dosyayı gösterir; "Geri al" mühürle yürütür', async () => {
    const { arka } = await ciz((a) => {
      a.geriAlmaPlani = plan;
    });
    arka.olaylar('o1', [{ tip: 'TurOzeti', ozet }]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Turu geri al…' }));
    expect(arka.komutlar('geri_alma_onizle')).toEqual([{ oturum: 'o1', tur: 't1' }]);
    const sayfa = await screen.findByRole('dialog', { name: 'Turu geri al' });
    expect(within(sayfa).getByText('Tur: Dosyaları incele')).toBeTruthy();
    expect(within(sayfa).getAllByText('Önceki hâline döner').length).toBeGreaterThan(0);
    expect(within(sayfa).getByText('yeni();')).toBeTruthy();
    expect(within(sayfa).getByText('cargo build')).toBeTruthy();

    await fireEvent.click(within(sayfa).getByRole('button', { name: 'b.rs için ne olacağını göster' }));
    expect(within(sayfa).getByText("Dosya Kalfa'dan sonra değişti.")).toBeTruthy();
    await fireEvent.click(within(sayfa).getByRole('button', { name: '.env için ne olacağını göster' }));
    expect(within(sayfa).getByText('Gizli dosya; içerik gösterilmiyor.')).toBeTruthy();
    expect(within(sayfa).getByText('Bu dosya için önizleme yok.')).toBeTruthy();

    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Geri al' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(arka.komutlar('turu_geri_al')).toEqual([{ oturum: 'o1', tur: 't1', muhur: 'muhur-t1' }]);
    expect(await screen.findByText(/Tur geri alındı: 2 dosya önceki hâline döndü/)).toBeTruthy();
    expect(screen.getByText('Bu tur geri alındı.')).toBeTruthy();
  });

  it('yapılacak bir şey yoksa "Geri al" kapalı; Escape kapatır', async () => {
    const { arka } = await ciz((a) => {
      a.geriAlmaPlani = {
        ogeler: [{ yol: 'a', eylem: { tip: 'ZatenOnceki' }, simdiki: 'x', uyarilar: [], diff: null }],
        veri_silindi: false,
      };
    });
    arka.olaylar('o1', [{ tip: 'TurOzeti', ozet }]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Turu geri al…' }));
    const sayfa = await screen.findByRole('dialog', { name: 'Turu geri al' });
    expect(within(sayfa).getByText('Geri alınacak bir değişiklik yok.')).toBeTruthy();
    const dugme = within(sayfa).getByRole('button', { name: 'Geri al' }) as HTMLButtonElement;
    expect(dugme.disabled).toBe(true);
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(arka.komutlar('turu_geri_al')).toEqual([]);
  });

  it('onay sayfası geri alınabilirliği gösterir; kabuk öğesi seçiliyken başlık cümlesi yok', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i3',
        ogeler: [
          onayOgesi({ cagri: 'y1', diff: DIFF, geri_alinabilir: { tip: 'Evet' } }),
          onayOgesi({
            cagri: 'y2',
            arac: 'kabuk',
            geri_alinabilir: { tip: 'Hayir', neden: 'Kabuk' },
            kabuk: {
              komut: 'make',
              bicim: { tip: 'Program' },
              dizin: '.',
              ag: false,
              yalitim: { tip: 'Yalitimli', arka_uc: 'bwrap', profil: 'p' },
              ilk_cagri: false,
              alan_hazirligi: false,
            },
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('Evet: turu geri alarak')).toBeTruthy();
    expect(within(sayfa).queryByText('Bu değişiklikler geri alınabilir.')).toBeNull();
    await fireEvent.click(within(sayfa).getByRole('checkbox', { name: 'make' }));
    expect(within(sayfa).getByText('Bu değişiklikler geri alınabilir.')).toBeTruthy();
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'make değişikliğini göster' }));
    expect(within(sayfa).getByText('Hayır: komutların etkileri geri alınamaz')).toBeTruthy();
  });

  it('oturum listesindeki "Sil" oturumu siler; vazgeçilirse oturum kalır', async () => {
    const { arka } = await ciz();
    const liste = screen.getByRole('complementary', { name: 'Oturumlar' });
    arka.silmeOnayi = false;
    await fireEvent.click(within(liste).getByRole('button', { name: 'Dosyaları incele oturumunu sil' }));
    await waitFor(() => expect(arka.komutlar('oturumu_sil')).toHaveLength(1));
    expect(screen.getByRole('heading', { name: 'Dosyaları incele' })).toBeTruthy();

    arka.silmeOnayi = true;
    await fireEvent.click(within(liste).getByRole('button', { name: 'Dosyaları incele oturumunu sil' }));
    await waitFor(() => expect(within(liste).getByText('Henüz oturum yok. Yeni bir oturumla başlayın.')).toBeTruthy());
    expect(screen.queryByRole('heading', { name: 'Dosyaları incele' })).toBeNull();
  });

  it('inceleme bölmesi başlıktan açılır ve kapanır; tutamaçla boyutlandırılır', async () => {
    await ciz();
    expect(screen.queryByRole('complementary', { name: 'Değişiklikler' })).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'İnceleme bölmesini aç' }));
    expect(screen.getByRole('complementary', { name: 'Değişiklikler' })).toBeTruthy();

    // Tutamaç klavye ile bölmeyi %50'ye (yarı yarıya) kadar genişletir.
    const tutamac = screen.getByRole('button', { name: 'Bölme genişliğini ayarla' });
    for (let i = 0; i < 10; i++) {
      await fireEvent.keyDown(tutamac, { key: 'ArrowLeft' });
    }
    expect(document.querySelector('.sag-oran-50')).toBeTruthy();

    // Pointer sürüklemesi blur ile temizlenir.
    await fireEvent.pointerDown(tutamac, { pointerId: 1 });
    expect(tutamac.classList.contains('etkin')).toBe(true);
    await fireEvent.blur(window);
    expect(tutamac.classList.contains('etkin')).toBe(false);

    const konusma = screen.getByRole('region', { name: 'Konuşma' });
    await fireEvent.click(within(konusma).getByRole('button', { name: 'İnceleme bölmesini kapat' }));
    expect(screen.queryByRole('complementary', { name: 'Değişiklikler' })).toBeNull();
  });

  it('Ctrl+B kenar çubuğunu gizler; başlıktaki düğme geri açar', async () => {
    await ciz();
    await fireEvent.keyDown(window, { key: 'b', ctrlKey: true });
    expect(screen.queryByRole('complementary', { name: 'Oturumlar' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Kenar çubuğunu göster' }));
    expect(screen.getByRole('complementary', { name: 'Oturumlar' })).toBeTruthy();
  });

  it('Ctrl+N yeni oturum açar, Ctrl+K aramaya odaklanır', async () => {
    const { arka } = await ciz();
    await fireEvent.keyDown(window, { key: 'n', ctrlKey: true });
    await waitFor(() => expect(arka.komutlar('oturum_baslat')).toHaveLength(1));
    await fireEvent.keyDown(window, { key: 'k', ctrlKey: true });
    await waitFor(() =>
      expect(document.activeElement).toBe(screen.getByRole('searchbox', { name: 'Oturumlarda ara…' })),
    );
  });

  it('değişiklikler bölmesi tamamlanan tur diffini gösterir ve yanıtlanan onayları dışlar', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(screen.getByRole('button', { name: 'İnceleme bölmesini aç' }));
    const bolme = screen.getByRole('complementary', { name: 'Değişiklikler' });
    expect(within(bolme).getAllByText('C:\\projeler\\kalfa\\src\\main.rs').length).toBeGreaterThan(0);

    // Onay isteği yanıtlandığında bekleyen öneri sağ bölmeden kalkar; tamamlanan tur özeti diff ile görünür.
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = screen.getByRole('dialog');
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Reddet' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(within(bolme).queryByText('C:\\projeler\\kalfa\\src\\main.rs')).toBeNull();

    arka.olaylar('o1', [
      {
        tip: 'TurOzeti',
        ozet: {
          ...ozet,
          dosyalar: [{ ...ozet.dosyalar[0], diff: DIFF }],
        },
      },
    ]);
    await waitFor(() => expect(within(bolme).getAllByText('src/a.rs').length).toBeGreaterThan(0));
    expect(within(bolme).getByText('yeni();')).toBeTruthy();
  });

  it('aynı klasör adını paylaşan projeler üst dizinle ayırt edilir', async () => {
    await ciz((a) => {
      a.oturumEkle(
        { oturum: 'o2', calisma_alani: 'C:\\bir\\uygulama' },
        { baslik: 'Birinci iş', calisma_alani: 'C:\\bir\\uygulama' },
      );
      a.oturumEkle(
        { oturum: 'o3', calisma_alani: 'C:\\iki\\uygulama' },
        { baslik: 'İkinci iş', calisma_alani: 'C:\\iki\\uygulama' },
      );
    });
    const liste = screen.getByRole('complementary', { name: 'Oturumlar' });
    expect(within(liste).getByText('C:\\bir\\uygulama')).toBeTruthy();
    expect(within(liste).getByText('C:\\iki\\uygulama')).toBeTruthy();
  });
});

