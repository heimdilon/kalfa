// Yerel MCP sunucuları (1.1a tasarımı §5.1, §9): Ayarlar bölümü, form, JSON
// içe aktarma, ayrıntı, sohbetteki başlatma kartı, onay sayfası ve araç kartı.
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { McpBaslatmaKarti, McpSunucuSatiri } from '../ipc/tipler';
import { onayOgesi, sahteAlanKimligi, SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const YOL = 'C:\\projeler\\kalfa';
const ALAN = sahteAlanKimligi(YOL);
const SIR = 'gizli-deger-7f3a9c';

function sunucu(ek: Partial<McpSunucuSatiri['ozet']> = {}, alanlar = true): McpSunucuSatiri {
  return {
    ozet: { ad: 'notlar', etkin: true, kip: 'yalitimli', gizli_ortam_sayisi: 0, ...ek },
    alanlar: alanlar
      ? [
          {
            alan_kimligi: ALAN,
            durum: 'calisiyor',
            kusak: 3,
            hata: null,
            izin: 'yalitimli',
            arac_sayisi: 2,
          },
        ]
      : [],
  };
}

function kart(ek: Partial<McpBaslatmaKarti> = {}): McpBaslatmaKarti {
  return {
    istek_kimligi: 'istek-1',
    alan_kimligi: ALAN,
    sunucu: 'notlar',
    kip: 'yalitimli',
    erisim_kipi: 'Yalıtılmış süreç',
    komut_ozeti: 'npx -y notlar@1.2.3',
    ag_listesi: ['api.ornek.com:443'],
    uyari: 'Sunucu başladığı andan kapanana kadar, araç çağrılmasa da çalışır.',
    wde_kodlu: false,
    ...ek,
  };
}

/** Oturum `o1` açık (alanı `YOL`); `ayar` arka ucu çizmeden önce hazırlar. */
async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle({ oturum: 'o1' }, { baslik: 'MCP oturumu' });
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('heading', { name: 'MCP oturumu' });
  return { arka, durum };
}

async function ayarlariAc() {
  const menu = screen.getByRole('navigation', { name: 'Menü' });
  await fireEvent.click(within(menu).getByRole('button', { name: 'Ayarlar' }));
  const ayarlar = await screen.findByRole('region', { name: 'Ayarlar' });
  return ayarlar;
}

function satir(ad: string) {
  return screen.findByRole('group', { name: ad });
}

describe('Ayarlar: MCP sunucuları bölümü', () => {
  it('Eklentiler’den ayrı; rozet, açıklama, durum, araç sayısı ve izin görünür', async () => {
    await ciz((a) => (a.mcp.sunucular = [sunucu(), sunucu({ ad: 'tam', kip: 'tam_erisim' })]));
    const ayarlar = await ayarlariAc();
    expect(within(ayarlar).getByText('MCP sunucuları')).toBeTruthy();
    const s = await satir('notlar');
    expect(within(s).getByText('Yalıtılmış süreç')).toBeTruthy();
    expect(within(s).getByText(/başladığı andan kapanana kadar sürer: çalışma alanına yazabilir/)).toBeTruthy();
    expect(within(s).getByText('Çalışıyor')).toBeTruthy();
    expect(within(s).getByText('2 araç')).toBeTruthy();
    expect(within(s).getByText(YOL)).toBeTruthy();
    expect(within(s).getByText('Başlatma izni var (kalıcı)')).toBeTruthy();
    const t = await satir('tam');
    expect(within(t).getByText('Tam sistem erişimi')).toBeTruthy();
    expect(within(t).getByText(/geride çalışan süreçler kalabilir/)).toBeTruthy();
  });

  it('etkinlik, izni geri alma ve yeniden başlatma arka uca gider', async () => {
    const { arka } = await ciz((a) => (a.mcp.sunucular = [sunucu()]));
    await ayarlariAc();
    const s = await satir('notlar');
    await fireEvent.click(within(s).getByRole('checkbox', { name: 'Etkin' }));
    await waitFor(() => expect(arka.komutlar('mcp_etkin_ayarla')).toEqual([{ ad: 'notlar', etkin: false }]));
    await fireEvent.click(within(s).getByRole('button', { name: 'Yeniden başlat' }));
    await waitFor(() =>
      expect(arka.komutlar('mcp_yeniden_baslat')).toEqual([{ oturum: 'o1', ad: 'notlar' }]),
    );
    await fireEvent.click(within(s).getByRole('button', { name: 'İzni geri al' }));
    await waitFor(() =>
      expect(arka.komutlar('mcp_baslatma_izni_geri_al')).toEqual([{ alan_kimligi: ALAN, ad: 'notlar' }]),
    );
    expect(await within(s).findByText('Başlatma izni yok')).toBeTruthy();
  });

  it('alanda açık oturum yoksa yeniden başlatma kapalıdır', async () => {
    await ciz((a) => {
      const yabanci = sunucu();
      yabanci.alanlar[0].alan_kimligi = 'alan:baska';
      a.mcp.sunucular = [yabanci];
    });
    await ayarlariAc();
    const s = await satir('notlar');
    const dugme = within(s).getByRole<HTMLButtonElement>('button', { name: 'Yeniden başlat' });
    expect(dugme.disabled).toBe(true);
    expect(within(s).getByText('Çalışma alanı alan:bas')).toBeTruthy();
  });

  it('eski revizyonlu görüntü yenisini ezmez', async () => {
    const { arka } = await ciz();
    // Bölüm açılırken okunan görüntü kanaldan gelen yeniden eskiyse atılır.
    arka.ozel.set('mcp_durumu', () => ({
      revizyon: 1,
      sunucular: [sunucu({ ad: 'eski' })],
      baslatma_kartlari: [],
    }));
    arka.mcpDurumuYaz({ sunucular: [sunucu({ ad: 'yeni' })] });
    await ayarlariAc();
    await waitFor(() => expect(arka.komutlar('mcp_durumu')).toHaveLength(1));
    expect(await satir('yeni')).toBeTruthy();
    expect(screen.queryByRole('group', { name: 'eski' })).toBeNull();
    // Kanalın kendisi de eski revizyonu atar.
    arka.mcpPaketi({ revizyon: 1, sunucular: [sunucu({ ad: 'eski' })], baslatma_kartlari: [] });
    await new Promise((r) => setTimeout(r, 0));
    expect(screen.queryByRole('group', { name: 'eski' })).toBeNull();
    expect(screen.getByRole('group', { name: 'yeni' })).toBeTruthy();
  });

  it('silme yerel pencerede reddedilirse sunucu kalır', async () => {
    const { arka, durum } = await ciz((a) => {
      a.mcp.sunucular = [sunucu()];
      a.mcpOnayi = false;
    });
    await ayarlariAc();
    await fireEvent.click(within(await satir('notlar')).getByRole('button', { name: 'Sil' }));
    await waitFor(() => expect(durum.duyuru).toBe('Yerel pencerede vazgeçildi; hiçbir şey değişmedi.'));
    expect(arka.mcp.sunucular).toHaveLength(1);
    expect(screen.getByRole('group', { name: 'notlar' })).toBeTruthy();
  });
});

describe('MCP formu', () => {
  async function formuAc() {
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: 'Sunucu ekle' }));
    return screen.findByRole('form', { name: 'Yeni MCP sunucusu' });
  }

  it('yeni form yalıtımlıdır; tam erişimde ağ gizlenir, “sınırsız” ve kalıcı uyarı görünür', async () => {
    await ciz();
    const form = await formuAc();
    const yalitimli = within(form).getByRole<HTMLInputElement>('radio', { name: 'Yalıtılmış süreç' });
    expect(yalitimli.checked).toBe(true);
    expect(within(form).getByRole('button', { name: 'Ağ izni ekle' })).toBeTruthy();
    expect(within(form).queryByRole('note')).toBeNull();

    await fireEvent.click(within(form).getByRole('radio', { name: 'Tam sistem erişimi' }));
    expect(within(form).queryByRole('button', { name: 'Ağ izni ekle' })).toBeNull();
    expect(within(form).getByText('Sınırsız')).toBeTruthy();
    expect(within(form).getByRole('note').textContent).toMatch(/geride çalışan süreçler kalabilir/);
  });

  it('Gözat yerel seçiciyi açar; form kimlik ve gizli değerle kaydedilir, sır DOM’da kalmaz', async () => {
    const { arka } = await ciz();
    const form = await formuAc();
    await fireEvent.input(within(form).getByRole('textbox', { name: 'Ad' }), {
      target: { value: 'notlar' },
    });
    await fireEvent.click(within(form).getByRole('radio', { name: 'Dosya seç' }));
    await fireEvent.click(within(form).getByRole('button', { name: 'Gözat' }));
    expect(await within(form).findByText('sunucu.exe')).toBeTruthy();
    await fireEvent.input(within(form).getByRole('textbox', { name: 'Argümanlar' }), {
      target: { value: '--kip\nsessiz\n' },
    });
    await fireEvent.click(within(form).getByRole('button', { name: 'Değişken ekle' }));
    await fireEvent.input(within(form).getByRole('textbox', { name: 'Değişken adı' }), {
      target: { value: 'API_ANAHTARI' },
    });
    await fireEvent.click(within(form).getByRole('checkbox', { name: 'Gizli' }));
    const deger = within(form).getByLabelText<HTMLInputElement>('Değer');
    expect(deger.type).toBe('password');
    await fireEvent.input(deger, { target: { value: SIR } });
    await fireEvent.click(within(form).getByRole('button', { name: 'Kaydet' }));

    await waitFor(() => expect(arka.komutlar('mcp_taslak_kaydet')).toHaveLength(1));
    expect(arka.komutlar('mcp_taslak_olustur')).toEqual([
      {
        eski_ad: null,
        form: {
          ad: 'notlar',
          etkin: true,
          komut: { tur: 'secili_yurutulebilir', kimlik: 'komut-1', argumanlar: ['--kip', 'sessiz'] },
          ortam: [{ tur: 'yeni_gizli', ad: 'API_ANAHTARI', deger: SIR }],
          kip: 'yalitimli',
          ag_izinleri: [],
          baslatma_zamani_asimi_sn: 30,
          cagri_zamani_asimi_sn: 120,
        },
      },
    ]);
    expect(arka.komutlar('mcp_taslak_kaydet')[0]).toMatchObject({ oturum: 'o1', kusak: 1 });
    expect(await satir('notlar')).toBeTruthy();
    expect(screen.queryByRole('form')).toBeNull();
    expect(document.body.innerHTML).not.toContain(SIR);
  });

  it('yerel pencerede ret: kayıt olmaz, form açık kalır ve nedeni yazılır', async () => {
    const { arka } = await ciz((a) => (a.mcpOnayi = false));
    const form = await formuAc();
    await fireEvent.input(within(form).getByRole('textbox', { name: 'Ad' }), {
      target: { value: 'notlar' },
    });
    await fireEvent.input(within(form).getByRole('textbox', { name: 'Program adı' }), {
      target: { value: 'npx' },
    });
    await fireEvent.click(within(form).getByRole('button', { name: 'Kaydet' }));
    expect(await within(form).findByRole('alert')).toHaveProperty(
      'textContent',
      'Yerel pencerede vazgeçildi; hiçbir şey değişmedi.',
    );
    expect(arka.mcp.sunucular).toHaveLength(0);
  });

  it('eksik alanlar arka uca gitmeden söylenir', async () => {
    const { arka } = await ciz();
    const form = await formuAc();
    await fireEvent.click(within(form).getByRole('button', { name: 'Kaydet' }));
    expect((await within(form).findByRole('alert')).textContent).toBe('Sunucu için bir ad yazın.');
    expect(arka.komutlar('mcp_taslak_olustur')).toHaveLength(0);
  });

  it('düzenlemede kayıtlı gizli değer gösterilmez ve kimliğiyle korunur', async () => {
    const { arka } = await ciz((a) => {
      a.mcp.sunucular = [sunucu()];
      a.mcpAyrintilari.set('notlar', {
        ayrinti: {
          ad: 'notlar',
          etkin: false,
          komut: { tur: 'program_adi', ad: 'npx', argumanlar: ['-y', 'notlar@1.2.3'] },
          ortam: [
            { ad: 'API_ANAHTARI', gizli: true, gizli_var: true, gizli_kimlik: 'gizli-1' },
            { ad: 'DIL', gizli: false, deger_acik: 'tr', gizli_var: false },
          ],
          kip: 'yalitimli',
          ag_izinleri: [{ host: 'api.ornek.com', port: 443 }],
          baslatma_zamani_asimi_sn: 30,
          cagri_zamani_asimi_sn: 120,
        },
        alanlar: [],
      });
    });
    await ayarlariAc();
    await fireEvent.click(within(await satir('notlar')).getByRole('button', { name: 'Düzenle' }));
    const form = await screen.findByRole('form', { name: 'notlar sunucusunu düzenle' });
    expect(within(form).getByText('Kayıtlı gizli değer (gösterilmez)')).toBeTruthy();
    await fireEvent.click(within(form).getByRole('button', { name: 'Kaydet' }));
    await waitFor(() => expect(arka.komutlar('mcp_taslak_olustur')).toHaveLength(1));
    expect(arka.komutlar('mcp_taslak_olustur')[0]).toMatchObject({
      eski_ad: 'notlar',
      form: {
        etkin: false,
        komut: { tur: 'program_adi', ad: 'npx', argumanlar: ['-y', 'notlar@1.2.3'] },
        ortam: [
          { tur: 'korunan_gizli', ad: 'API_ANAHTARI', kimlik: 'gizli-1' },
          { tur: 'duz', ad: 'DIL', deger: 'tr' },
        ],
        ag_izinleri: [{ host: 'api.ornek.com', port: 443, ozel_ag: false }],
      },
    });
  });
});

describe('MCP JSON içe aktarma', () => {
  it('metin önizlemeye gidince alandan silinir; gizli değer gösterilmez; “gizli say” iki yönde arka uca gider', async () => {
    const json = JSON.stringify({ mcpServers: { notlar: { command: 'npx', env: { TOKEN: SIR } } } });
    const { arka } = await ciz((a) => {
      a.mcpIceAktarma = {
        taslak_kimligi: 'ice-1',
        kusak: 1,
        sunucular: [
          {
            ad: 'notlar',
            etkin: false,
            kip: 'yalitimli',
            komut_gorunumu: 'npx',
            argumanlar: ['-y', 'notlar'],
            ortam: [
              { ad: 'TOKEN', gizli: true, gizli_var: true },
              { ad: 'DIL', gizli: false, deger_acik: 'tr', gizli_var: false },
            ],
          },
        ],
        atlanan_url_sunuculari: [{ ad: 'uzak', neden: 'URL sunucuları 1.1b’de' }],
      };
    });
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: "JSON'dan içe aktar" }));
    const alan = await screen.findByRole('textbox', { name: 'JSON yapılandırması' });
    await fireEvent.input(alan, { target: { value: json } });
    await fireEvent.click(screen.getByRole('button', { name: 'Önizle' }));

    const liste = await screen.findByRole('list', { name: 'Eklenecek sunucular' });
    expect(arka.komutlar('mcp_ice_aktar_onizle')).toEqual([{ json }]);
    expect(screen.queryByRole('textbox', { name: 'JSON yapılandırması' })).toBeNull();
    expect(document.body.innerHTML).not.toContain(SIR);
    expect(within(liste).getByText('gizli')).toBeTruthy();
    expect(screen.getByText('uzak atlandı: URL sunucuları 1.1b’de')).toBeTruthy();

    const gizliSay = within(liste).getAllByRole<HTMLInputElement>('checkbox', { name: 'Gizli say' });
    // Kısa düz değerler (8 karakterden kısa) gizli kalamaz; düze çevrilebilir.
    expect(gizliSay[0].disabled).toBe(false);
    await fireEvent.click(gizliSay[0]);
    await waitFor(() => expect(within(liste).getByText('düz değer olarak kaydedilir')).toBeTruthy());
    await fireEvent.click(gizliSay[1]);
    await waitFor(() =>
      expect(arka.komutlar('mcp_ice_aktar_gizlilik')).toEqual([
        { taslak_kimligi: 'ice-1', sunucu: 'notlar', ortam: 'TOKEN', gizli: false },
        { taslak_kimligi: 'ice-1', sunucu: 'notlar', ortam: 'DIL', gizli: true },
      ]),
    );
    await waitFor(() => expect(within(liste).queryByText('tr')).toBeNull());

    await fireEvent.click(screen.getByRole('button', { name: 'İçe aktar' }));
    await waitFor(() =>
      expect(arka.komutlar('mcp_ice_aktar_kaydet')).toEqual([
        { taslak_kimligi: 'ice-1', kusak: 2, oturum: 'o1' },
      ]),
    );
    expect(await satir('notlar')).toBeTruthy();
  });
});

describe('MCP JSON içe aktarma: hata', () => {
  it('önizleme başarısız olsa da yapıştırılan metin alanda kalmaz', async () => {
    const json = JSON.stringify({ mcpServers: { notlar: { command: 'npx', env: { TOKEN: SIR } } } });
    await ciz();
    const ayarlar = await ayarlariAc();
    await fireEvent.click(within(ayarlar).getByRole('button', { name: "JSON'dan içe aktar" }));
    const alan = await screen.findByRole<HTMLTextAreaElement>('textbox', { name: 'JSON yapılandırması' });
    await fireEvent.input(alan, { target: { value: json } });
    await fireEvent.click(screen.getByRole('button', { name: 'Önizle' }));
    expect((await screen.findByRole('alert')).textContent).toBe('Geçersiz MCP JSON yapılandırması.');
    expect(alan.value).toBe('');
    expect(document.body.innerHTML).not.toContain(SIR);
  });
});

describe('MCP ayrıntısı', () => {
  it('denetim durumları, kapatılan araçlar ve maskeli stderr görünür', async () => {
    await ciz((a) => {
      a.mcp.sunucular = [sunucu()];
      const arac = (ek: object) => ({
        birlesik_ad: 'mcp__notlar__oku',
        uzak_ad: 'oku',
        aciklama: 'Notu okur.',
        d_ozeti_hex: 'ab',
        denetim_metni: 'oku: Notu okur.',
        denetim_sonucu: null,
        sunulur: true,
        uyari: null,
        onbellekten: false,
        ...ek,
      });
      a.mcpAyrintilari.set('notlar', {
        ayrinti: {
          ad: 'notlar',
          etkin: true,
          komut: { tur: 'program_adi', ad: 'npx' },
          ortam: [],
          kip: 'yalitimli',
          ag_izinleri: [],
          baslatma_zamani_asimi_sn: 30,
          cagri_zamani_asimi_sn: 120,
        },
        alanlar: [
          {
            alan_kimligi: ALAN,
            araclar: [
              arac({}),
              arac({
                birlesik_ad: 'mcp__notlar__yaz',
                denetim_sonucu: { tip: 'Supheli', kategori: 'enjeksiyon', gerekce: 'Talimat içeriyor' },
              }),
              arac({
                birlesik_ad: 'mcp__notlar__sil',
                sunulur: false,
                denetim_sonucu: { tip: 'Supheli', kategori: 'enjeksiyon', gerekce: 'Gizli talimat' },
              }),
            ],
            reddedilenler: ['3', '5'],
            stderr: ['uyarı: ***'],
          },
        ],
      });
    });
    await ayarlariAc();
    const s = await satir('notlar');
    await fireEvent.click(within(s).getByRole('button', { name: 'Ayrıntı' }));
    expect(await within(s).findByText('Tanım denetimi kapalı')).toBeTruthy();
    expect(within(s).getByText('Şüpheli, uyarıyla sunuluyor')).toBeTruthy();
    expect(within(s).getByText('Şüpheli, sunulmuyor')).toBeTruthy();
    expect(within(s).getByText('Modele sunulmuyor')).toBeTruthy();
    expect(within(s).getByText(/Kapatılan araçlar \(sıra no\): 3, 5/)).toBeTruthy();
    expect(within(s).getByText('uyarı: ***')).toBeTruthy();
  });
});

describe('sohbette başlatma kartı', () => {
  it('yalnız aynı çalışma alanındaki oturumda görünür; Başlat kartı yanıtlar ve kaldırır', async () => {
    const { arka } = await ciz((a) => {
      a.mcp.baslatma_kartlari = [kart(), kart({ istek_kimligi: 'istek-2', alan_kimligi: 'alan:baska', sunucu: 'diger' })];
    });
    const grup = await screen.findByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' });
    expect(screen.queryByRole('group', { name: 'diger bu çalışma alanında başlatılsın mı?' })).toBeNull();
    expect(within(grup).getByText('Bu kartı model açamaz ve yanıtlayamaz.')).toBeTruthy();
    expect(within(grup).getByText('npx -y notlar@1.2.3')).toBeTruthy();
    expect(within(grup).getByText('api.ornek.com:443')).toBeTruthy();
    expect(within(grup).getByText(/araç çağrılmasa da çalışır/)).toBeTruthy();
    expect(within(grup).getByText(/kalıcıdır/)).toBeTruthy();
    // Kart sohbeti bekletmez.
    expect(screen.getByRole<HTMLTextAreaElement>('textbox', { name: /Bir şey sorun/ }).disabled).toBe(false);

    await fireEvent.click(within(grup).getByRole('button', { name: 'Başlat' }));
    await waitFor(() =>
      expect(arka.komutlar('mcp_baslatma_karti_yanitla')).toEqual([{ istek_kimligi: 'istek-1', kabul: true }]),
    );
    await waitFor(() =>
      expect(screen.queryByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' })).toBeNull(),
    );
  });

  it('salt okunur oturumda kart görünmez', async () => {
    await ciz((a) => {
      a.oturumEkle(
        { oturum: 'o1', salt_okunur: { tip: 'CalismaAlaniKayitsiz' } },
        { baslik: 'MCP oturumu' },
      );
      a.mcp.baslatma_kartlari = [kart()];
    });
    expect(
      (await screen.findAllByText(/Bu kayıt çalışma alanını yazmıyor/)).length,
    ).toBeGreaterThan(0);
    expect(screen.queryByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' })).toBeNull();
  });

  it('tam erişim kartı sınırsız ağı ve yerel onayı söyler; ret duyurulur', async () => {
    const { arka, durum } = await ciz((a) => {
      a.mcp.baslatma_kartlari = [kart({ kip: 'tam_erisim', erisim_kipi: 'Tam sistem erişimi', ag_listesi: [] })];
      a.mcpKartSonucu = false;
    });
    const grup = await screen.findByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' });
    expect(within(grup).getByText('Sınırsız')).toBeTruthy();
    expect(within(grup).getByText(/yerel bir pencerede ayrıca onay ister/)).toBeTruthy();
    await fireEvent.click(within(grup).getByRole('button', { name: 'Başlat' }));
    await waitFor(() => expect(durum.duyuru).toBe('notlar başlatılmadı.'));
    expect(arka.komutlar('mcp_baslatma_karti_yanitla')).toHaveLength(1);
  });

  it('modelin araç çağrısı onayı başlatma kartının yerine geçmez', async () => {
    const { arka } = await ciz((a) => (a.mcp.baslatma_kartlari = [kart()]));
    await screen.findByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' });
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i1',
        ogeler: [
          onayOgesi({
            cagri: 'c1',
            arac: 'mcp__notlar__oku',
            aciklama: 'notlar sunucusunun oku aracını çağırmak istiyor.',
            mcp: { sunucu: 'notlar', arac: 'oku', erisim: 'Yalitimli', ag_hostlari: [], ag_sinirsiz: false },
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Tümünü onayla' }));
    await waitFor(() => expect(arka.komutlar('arac_onayi')).toHaveLength(1));
    expect(arka.komutlar('mcp_baslatma_karti_yanitla')).toHaveLength(0);
    expect(screen.getByRole('group', { name: 'notlar bu çalışma alanında başlatılsın mı?' })).toBeTruthy();
  });
});

describe('onay sayfası ve araç kartı', () => {
  it('MCP öğesi kaynağı, rozeti ve erişim nedenini söyler; kısayol cümlesi MCP’dir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      {
        tip: 'OnayGerekiyor',
        islem: 'i1',
        ogeler: [
          onayOgesi({
            cagri: 'c1',
            arac: 'mcp__notlar__oku',
            aciklama: 'notlar sunucusunun oku aracını çağırmak istiyor.',
            mcp: {
              sunucu: 'notlar',
              arac: 'oku',
              erisim: 'Yalitimli',
              ag_hostlari: ['api.ornek.com:443'],
              ag_sinirsiz: false,
            },
            kisayol: { tip: 'Mcp', sunucu: 'notlar', arac: 'oku' },
          }),
          onayOgesi({
            cagri: 'c2',
            arac: 'mcp__tam__calistir',
            yerel_onay: true,
            mcp: { sunucu: 'tam', arac: 'calistir', erisim: 'TamErisim', ag_hostlari: [], ag_sinirsiz: true },
          }),
        ],
      },
    ]);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = await screen.findByRole('dialog');
    expect(within(sayfa).getByText('notlar MCP sunucusu · oku')).toBeTruthy();
    expect(within(sayfa).getByText('Yalıtılmış süreç')).toBeTruthy();
    expect(
      within(sayfa).getByText(/şu adreslere çıkabilir: api.ornek.com:443/),
    ).toBeTruthy();
    expect(
      within(sayfa).getByRole('checkbox', {
        name: 'Bu oturumda notlar sunucusunun oku aracını onaysız çağırabilir.',
      }),
    ).toBeTruthy();
    expect(within(sayfa).getByText(/Tam sistem erişimli MCP araçları/)).toBeTruthy();
    expect(within(sayfa).queryByText(/Yalıtımsız komutlar/)).toBeNull();

    await fireEvent.click(within(sayfa).getByRole('button', { name: 'tam · calistir değişikliğini göster' }));
    expect(within(sayfa).getByText('Tam sistem erişimi')).toBeTruthy();
    expect(within(sayfa).getByText(/hesabınızın erişebildiği her şeye erişir, ağ sınırsızdır/)).toBeTruthy();
  });

  it('MCP araç kartında sunucu adı ve rozet görünür', async () => {
    const { arka } = await ciz((a) => (a.mcp.sunucular = [sunucu({ ad: 'not_lar' })]));
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'AracCagrisiBasladi', cagri: 'c1', arac: 'mcp__not_lar__oku', aciklama: 'not' },
      { tip: 'AracSonucu', cagri: 'c1', arac: 'mcp__not_lar__oku', durum: 'Basarili', icerik: 'tamam' },
    ]);
    expect(await screen.findByText('not_lar · oku')).toBeTruthy();
    expect(screen.getByText('Yalıtılmış süreç')).toBeTruthy();
  });
});
