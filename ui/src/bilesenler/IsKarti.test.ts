import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import IsKarti, { terminalMetniniTemizle } from './IsKarti.svelte';

describe('IsKarti (G2b / G2c arka plan işi ve terminal kartı)', () => {
  it('OSC 8 bağlantılarını, OSC 52 pano kaçışlarını ve DCS/APC dizilerini ayıklar, SGR renklerini korur', () => {
    const girdi =
      '\x1b]8;;https://zararli.ornek\x07Tikla\x1b]8;;\x07 ' +
      '\x1b]52;c;Z2l6bGk=\x1b\\ ' +
      '\x1bP1$r0m\x1b\\ ' +
      '\x1b_APC-VERISI\x1b\\ ' +
      '\x1b[32mYESIL\x1b[0m';

    const temiz = terminalMetniniTemizle(girdi);
    expect(temiz).not.toContain('https://zararli.ornek');
    expect(temiz).not.toContain('Z2l6bGk=');
    expect(temiz).not.toContain('APC-VERISI');

    const { container } = render(IsKarti, {
      props: {
        isId: 'is-11111111-2222-3333-4444-555555555555',
        gosterim: 'cargo test',
        kip: { kip: 'terminal', boyut: { sutun: 80, satir: 24 } },
        durum: { durum: 'calisiyor' },
        cikti: girdi,
        toplamBayt: 64,
      },
    });

    expect(container.querySelectorAll('a')).toHaveLength(0);
    expect(screen.getByText('YESIL').classList.contains('r-yesil')).toBe(true);
    expect(container.textContent).toContain('Tikla');
    expect(container.textContent).not.toContain('https://zararli.ornek');
  });

  it('modelin reddedilen stdin isteğini yerel kullanıcı denetimlerinden ayırır ve işi çalışır tutar', async () => {
    const onDurdur = vi.fn();
    const onKullaniciGirdi = vi.fn();
    const onKullaniciEof = vi.fn();

    render(IsKarti, {
      props: {
        isId: 'is-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
        gosterim: 'python -i',
        kip: { kip: 'terminal', boyut: { sutun: 100, satir: 30 } },
        durum: { durum: 'calisiyor' },
        cikti: '>>> ',
        toplamBayt: 4,
        modelGirdiDurumu: 'reddedildi',
        modelGirdiAciklama: 'import os',
        onDurdur,
        onKullaniciGirdi,
        onKullaniciEof,
      },
    });

    expect(
      screen.getByText('Modelin standart girdi isteği reddedildi; iş çalışmaya devam ediyor'),
    ).toBeTruthy();
    expect(screen.getByText('Çalışıyor')).toBeTruthy();

    const girdiAlani = screen.getByRole<HTMLInputElement>('textbox', {
      name: 'Standart girdi metni',
    });
    await fireEvent.input(girdiAlani, { target: { value: 'print(42)' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Girdi gönder' }));
    expect(onKullaniciGirdi).toHaveBeenCalledWith(
      'is-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee',
      'print(42)\n',
    );

    await fireEvent.click(screen.getByRole('button', { name: 'EOF gönder' }));
    expect(onKullaniciEof).toHaveBeenCalledWith('is-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee');

    await fireEvent.click(screen.getByRole('button', { name: 'İşi durdur' }));
    expect(onDurdur).toHaveBeenCalledWith('is-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee');
  });

  it('sonucu bilinmiyor durumunda uyarıyı gösterir ve durdur/girdi denetimlerini gizler', () => {
    render(IsKarti, {
      props: {
        isId: 'is-99999999-8888-7777-6666-555555555555',
        gosterim: 'npm run dev',
        kip: { kip: 'borulu' },
        durum: {
          durum: 'sonucu_bilinmiyor',
          neden: 'Uygulama beklenmedik biçimde kapandı; eski süreç kimliklerine sinyal gönderilmedi.',
        },
        tasti: true,
        uyarilar: ['Yeni yapılandırma dosyası algılandı: .vscode/tasks.json'],
        onDurdur: vi.fn(),
        onKullaniciGirdi: vi.fn(),
      },
    });

    expect(screen.getByText('Sonucu bilinmiyor')).toBeTruthy();
    expect(
      screen.getByText(
        'Uygulama beklenmedik biçimde kapandı; eski süreç kimliklerine sinyal gönderilmedi.',
      ),
    ).toBeTruthy();
    expect(
      screen.getByText('Yeni yapılandırma dosyası algılandı: .vscode/tasks.json'),
    ).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'İşi durdur' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Girdi gönder' })).toBeNull();
  });
});
