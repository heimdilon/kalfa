import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { UzakMcpSunucuAyrintisi, UzakMcpSunucuSatiri } from '../ipc/tipler';
import UzakMcpSunuculari from './UzakMcpSunuculari.svelte';

afterEach(() => {
  cleanup();
});

const CANARY_SIR = 'CANARY-UI-SECRET-TOKEN-99887766';

function ornekSatirlar(): UzakMcpSunucuSatiri[] {
  return [
    {
      ozet: {
        ad: 'bulut-analiz',
        etkin: true,
        uc_url: 'https://mcp.ornek.com/analiz',
        yetki_turu: 'oauth',
        kimlik_bagli: true,
      },
      rozet: 'uzak_streamable_http',
      durum: 'bagli',
      protokol_surumu: '2025-11-25',
      arac_sayisi: 2,
      reddedilen_arac_sayisi: 1,
      son_hata: null,
      belirsiz_yazma_uyarisi: null,
    },
    {
      ozet: {
        ad: 'yazma-servisi',
        etkin: true,
        uc_url: 'https://mcp.ornek.com/yazma',
        yetki_turu: 'sabit_baslik',
        kimlik_bagli: true,
      },
      rozet: 'uzak_streamable_http',
      durum: 'sonucu_belirsiz',
      protokol_surumu: '2025-06-18',
      arac_sayisi: 1,
      reddedilen_arac_sayisi: 0,
      son_hata: 'Bağlantı kesildi; sır maskelendi: [gizli]',
      belirsiz_yazma_uyarisi:
        'Bağlantı kesildi; `yazma-servisi` uzak sunucusundaki `kayit_olustur` işleminin tamamlanıp tamamlanmadığı bilinmiyor. Otomatik tekrar yapılmadı.',
    },
  ];
}

describe('UzakMcpSunuculari', () => {
  it('uzak MCP rozetini yerel yalıtımdan ayırır, sır sızdırmaz ve belirsiz yazma uyarısını gösterir', async () => {
    const onKaydet = vi.fn();
    const onSil = vi.fn();
    const onEtkinDegistir = vi.fn();
    const onBaglan = vi.fn();
    const onOAuthBagla = vi.fn();
    const onKimlikKaldir = vi.fn();

    const ayrintilar: Record<string, UzakMcpSunucuAyrintisi> = {
      'bulut-analiz': {
        ayrinti: {
          ad: 'bulut-analiz',
          etkin: true,
          uc_url: 'https://mcp.ornek.com/analiz',
          yetki_turu: 'oauth',
          oauth_istemci_kimligi: 'istemci-1',
          oauth_kapsamlar: ['tools:read'],
          oauth_kaynak_url: 'https://mcp.ornek.com/analiz',
          kimlik_bagli: true,
          baslatma_zamani_asimi_sn: 15,
          cagri_zamani_asimi_sn: 30,
        },
        rozet: 'uzak_streamable_http',
        durum: 'bagli',
        protokol_surumu: '2025-11-25',
        araclar: ['mcp__bulut-analiz__rapor_getir', 'mcp__bulut-analiz__metrik_oku'],
        reddedilenler: ['#3'],
        son_hata: null,
        belirsiz_yazma_uyarisi: null,
      },
    };

    const { container } = render(UzakMcpSunuculari, {
      sunucular: ornekSatirlar(),
      ayrintilar,
      onKaydet,
      onSil,
      onEtkinDegistir,
      onBaglan,
      onOAuthBagla,
      onKimlikKaldir,
    });

    // Uzak sunucu rozeti yerel AppContainer/Yalıtılmış süreç rozeti taşımaz.
    expect(screen.getAllByText('Uzak HTTP sunucusu (yerel yalıtım yok)').length).toBe(2);
    expect(container.textContent).not.toContain('Yalıtılmış süreç');
    expect(container.textContent).not.toContain('AppContainer');
    expect(container.textContent).not.toContain(CANARY_SIR);

    // Belirsiz yazma uyarısı ve maskeli hata görünür.
    expect(screen.getByText(/Otomatik tekrar yapılmadı/)).toBeTruthy();
    expect(screen.getByText(/Son işlem sonucu belirsiz/)).toBeTruthy();

    // Bağlan, OAuth bağla ve Kimliği kaldır aksiyonları çalışır.
    const baglanDugmeleri = screen.getAllByText('Bağlan ve araçları keşfet');
    await fireEvent.click(baglanDugmeleri[0]);
    expect(onBaglan).toHaveBeenCalledWith('bulut-analiz');

    const oauthDugmesi = screen.getByText('Hesabı bağla (OAuth)');
    await fireEvent.click(oauthDugmesi);
    expect(onOAuthBagla).toHaveBeenCalledWith('bulut-analiz');

    const kimlikKaldirDugmeleri = screen.getAllByText('Kimliği kaldır');
    await fireEvent.click(kimlikKaldirDugmeleri[0]);
    expect(onKimlikKaldir).toHaveBeenCalledWith('bulut-analiz');

    // Ayrıntı açıldığında denetlenmiş araçlar ve sıra numaralı reddedilen araç gösterilir.
    const ayrintiDugmeleri = screen.getAllByText('Ayrıntı');
    await fireEvent.click(ayrintiDugmeleri[0]);
    expect(screen.getByText('mcp__bulut-analiz__rapor_getir')).toBeTruthy();
    expect(screen.getByText(/#3/)).toBeTruthy();
  });

  it('yeni uzak MCP sunucusu eklerken https:// ve ad doğrulaması yapar', async () => {
    const onKaydet = vi.fn().mockResolvedValue(true);

    render(UzakMcpSunuculari, {
      sunucular: [],
      onKaydet,
      onSil: vi.fn(),
      onEtkinDegistir: vi.fn(),
      onBaglan: vi.fn(),
      onKimlikKaldir: vi.fn(),
    });

    await fireEvent.click(screen.getByText('Uzak sunucu ekle'));

    const adGirdisi = screen.getByLabelText('Sunucu adı') as HTMLInputElement;
    const urlGirdisi = screen.getByLabelText('HTTPS uç adresi (URL)') as HTMLInputElement;

    // 1. http:// adresi reddedilir.
    await fireEvent.input(adGirdisi, { target: { value: ' güvensiz-servis ' } });
    await fireEvent.input(urlGirdisi, { target: { value: 'http://mcp.ornek.com/mcp' } });
    await fireEvent.click(screen.getByText('Kaydet'));
    expect(screen.getByRole('alert').textContent).toContain('Sunucu adı yalnız küçük harf');
    expect(onKaydet).not.toHaveBeenCalled();

    // 2. Geçerli ad ama http:// URL reddedilir.
    await fireEvent.input(adGirdisi, { target: { value: 'yeni-uzak' } });
    await fireEvent.click(screen.getByText('Kaydet'));
    expect(screen.getByRole('alert').textContent).toContain('Geçerli bir https://');
    expect(onKaydet).not.toHaveBeenCalled();

    // 3. Geçerli https:// URL ile kaydedilir.
    await fireEvent.input(urlGirdisi, { target: { value: 'https://mcp.ornek.com/mcp' } });
    await fireEvent.click(screen.getByText('Kaydet'));
    expect(onKaydet).toHaveBeenCalledWith(null, {
      ad: 'yeni-uzak',
      etkin: true,
      uc_url: 'https://mcp.ornek.com/mcp',
      yetki: { tur: 'yetkisiz' },
      baslatma_zamani_asimi_sn: 15,
      cagri_zamani_asimi_sn: 30,
    });
  });
});
