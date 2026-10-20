// Konuşmadaki onay kartı (masaüstü arayüzü tasarımı §3): kararlar sayfanın
// mühürlü `kararlariUret`'iyle aynıdır; riskli öğe karttan onaylanamaz.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { OnayOgesi } from '../ipc/tipler';
import { onayOgesi } from '../sahte/arka';
import BekleyenOnay from './BekleyenOnay.svelte';
import OnaySayfasi from './OnaySayfasi.svelte';
import { kararlariUret, kartKarariUret, kartKararlari } from './onay';

const KISAYOL = { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' } as const;

function kabuk(ag: boolean): OnayOgesi {
  return onayOgesi({
    cagri: 'k1',
    arac: 'kabuk',
    aciklama: 'testleri çalıştırır',
    muhur: 'mk',
    kabuk: {
      komut: 'python -m unittest -v',
      bicim: { tip: 'Program' },
      dizin: '.',
      ag,
      ag_hostlari: ag ? ['pypi.org:443'] : [],
      yalitim: { tip: 'Yalitimli', arka_uc: 'windows AppContainer', profil: 'p' },
      ilk_cagri: false,
    } as OnayOgesi['kabuk'],
  });
}

const duzenleme = onayOgesi({
  cagri: 'd1',
  arac: 'dosya_duzenle',
  hedefler: ['C:\\proje\\src\\gorevler.py'],
  diff: '@@ -1 +1 @@\n-eski\n+yeni\n',
  muhur: 'md',
  kisayol: KISAYOL,
});

describe('kart kararları', () => {
  it('riskli öğe varsa yalnız ret; kısayol yalnız teklifi olan tek öğede', () => {
    expect(kartKararlari([kabuk(true)])).toEqual(['reddet']);
    expect(kartKararlari([duzenleme])).toEqual(['reddet', 'oturum', 'bir_kez']);
    expect(kartKararlari([kabuk(false)])).toEqual(['reddet', 'bir_kez']);
    expect(kartKararlari([duzenleme, kabuk(false)])).toEqual(['reddet', 'bir_kez']);
  });

  it('kararlar sayfanınkiyle aynı mühürlü girdilerdir', () => {
    const ogeler = [duzenleme];
    const hepsi = new Set(['d1']);
    expect(kartKarariUret(ogeler, 'bir_kez')).toEqual(kararlariUret(ogeler, 'tumu', new Set(), ''));
    expect(kartKarariUret(ogeler, 'oturum')).toEqual(
      kararlariUret(ogeler, 'secilenler', hepsi, '', hepsi),
    );
    expect(kartKarariUret(ogeler, 'oturum')?.[0]).toMatchObject({
      muhur: 'md',
      karar: { tip: 'Onayla' },
      kisayol: true,
    });
    expect(kartKarariUret(ogeler, 'reddet')?.[0]).toMatchObject({
      karar: { tip: 'Reddet', gerekce: null },
      kisayol: false,
    });
  });

  it('kartta verilemeyen karar üretilmez', () => {
    expect(kartKarariUret([kabuk(true)], 'bir_kez')).toBeNull();
    expect(kartKarariUret([kabuk(false)], 'oturum')).toBeNull();
  });
});

describe('BekleyenOnay', () => {
  function ciz(ogeler: OnayOgesi[]) {
    const onkarar = vi.fn(async () => {});
    const onincele = vi.fn();
    render(BekleyenOnay, { props: { ogeler, onkarar, onincele } });
    return { onkarar, onincele };
  }

  it('komut, dizin ve yalıtım görünür; "Bir kez izin ver" mühürle onaylar', async () => {
    const { onkarar } = ciz([kabuk(false)]);
    expect(screen.getByText('python -m unittest -v')).toBeTruthy();
    expect(screen.getByText('Yalıtımlı (windows AppContainer)')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Bu oturumda izin ver' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Bir kez izin ver' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'k1', muhur: 'mk', karar: { tip: 'Onayla' }, kisayol: false },
    ]);
  });

  it('düzenlemede diff görünür; "Bu oturumda izin ver" kısayolu ister', async () => {
    const { onkarar } = ciz([duzenleme]);
    expect(screen.getByText('gorevler.py')).toBeTruthy();
    expect(screen.getByText('yeni')).toBeTruthy();
    expect(screen.getByText(/src\/ altındaki dosyaları/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Bu oturumda izin ver' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'd1', muhur: 'md', karar: { tip: 'Onayla' }, kisayol: true },
    ]);
  });

  it('ağlı komut karttan onaylanamaz; yalnız ret ve İncele', async () => {
    const { onkarar, onincele } = ciz([kabuk(true)]);
    expect(screen.queryByRole('button', { name: 'Bir kez izin ver' })).toBeNull();
    expect(screen.getByText(/onayı İncele'den verin/)).toBeTruthy();
    expect(screen.getByText('Ağ: pypi.org:443')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'İncele' }));
    expect(onincele).toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Reddet' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'k1', muhur: 'mk', karar: { tip: 'Reddet', gerekce: null }, kisayol: false },
    ]);
  });

  it('yalıtımlı MCP çağrısında erişim ve izinli hostlar görünür', () => {
    ciz([
      onayOgesi({
        cagri: 'm1',
        arac: 'mcp__takvim__ekle',
        muhur: 'mm',
        mcp: {
          sunucu: 'takvim',
          arac: 'ekle',
          erisim: 'Yalitimli',
          ag_hostlari: ['api.takvim.ornek:443'],
          ag_sinirsiz: false,
        },
      }),
    ]);
    expect(screen.getByText('Yalıtılmış süreç')).toBeTruthy();
    expect(screen.getByText(/api\.takvim\.ornek:443/)).toBeTruthy();
  });

  it('uzak MCP çağrısında yerel yalıtım iddia etmez; uzak origin, kimlik kapsamı ve veri çıkışını gösterir (SP-04)', () => {
    const uzakOge = onayOgesi({
      cagri: 'u1',
      arac: 'mcp__uzak-ornek__sorgula',
      muhur: 'mu1',
      veri_cikisi: 'UzakMcpCagrisi',
      mcp: {
        sunucu: 'uzak-ornek',
        arac: 'sorgula',
        erisim: 'UzakStreamableHttp',
        ag_hostlari: ['mcp.ornek.tr:443'],
        ag_sinirsiz: false,
        uzak_origin: 'https://mcp.ornek.tr',
        kimlik_kapsami: 'OAuth 2.0 (PKCE) · kapsamlar: mcp:tools',
      },
    });
    const { container } = render(BekleyenOnay, {
      props: { ogeler: [uzakOge], onkarar: vi.fn(async () => {}), onincele: vi.fn() },
    });
    expect(screen.getByText('Uzak HTTP sunucusu (yerel yalıtım yok)')).toBeTruthy();
    expect(screen.getByText(/Uzak uç \(origin\): https:\/\/mcp\.ornek\.tr/)).toBeTruthy();
    expect(screen.getByText(/Kimlik kapsamı: OAuth 2\.0 \(PKCE\) · kapsamlar: mcp:tools/)).toBeTruthy();
    expect(
      screen.getByText(/Çağrı girdileri uzak MCP sunucusuna gönderilir; sunucunun döndürdüğü yanıt modele gidecek\./),
    ).toBeTruthy();
    expect(container.textContent).not.toContain('Yalıtılmış süreç');

    const sayfa = render(OnaySayfasi, {
      props: { ogeler: [uzakOge], onkarar: vi.fn(async () => {}), onkapat: vi.fn() },
    });
    expect(sayfa.container.textContent).not.toContain('Yalıtılmış süreç');
    expect(sayfa.container.textContent).toContain('https://mcp.ornek.tr');
    expect(sayfa.container.textContent).toContain('OAuth 2.0 (PKCE) · kapsamlar: mcp:tools');
    expect(sayfa.container.textContent).toContain(
      'Çağrı girdileri uzak MCP sunucusuna gönderilir; sunucunun döndürdüğü yanıt modele gidecek.',
    );
  });

  it('öğe başına geri alınabilirlik sayfadaki metinle görünür (§3.2)', () => {
    ciz([
      { ...duzenleme, geri_alinabilir: { tip: 'Evet' } },
      { ...kabuk(false), geri_alinabilir: { tip: 'Hayir', neden: 'Kabuk' } },
    ]);
    expect(screen.getByText('Geri alınabilir mi: Evet: turu geri alarak')).toBeTruthy();
    const hayir = screen.getByText('Geri alınabilir mi: Hayır: komutların etkileri geri alınamaz');
    expect(hayir.classList.contains('uyari')).toBe(true);
  });

  it('birden çok öğede toplu düğmeler', () => {
    ciz([duzenleme, kabuk(false)]);
    expect(screen.getByRole('button', { name: 'Tümünü onayla' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Tümünü reddet' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Bu oturumda izin ver' })).toBeNull();
  });
});
