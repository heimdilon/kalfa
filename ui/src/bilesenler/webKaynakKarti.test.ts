import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import WebKaynakKarti from './WebKaynakKarti.svelte';

describe('WebKaynakKarti', () => {
  it('kaynak URL, yönlendirilen son URL, başlık ve kırpma/imleç bilgisini açıkça gösterir', () => {
    const { container } = render(WebKaynakKarti, {
      props: {
        istenenUrl: 'https://belge.ornek.test/eski?token=[MASKELENDI]',
        sonUrl: 'https://belge.ornek.test/yeni?token=[MASKELENDI]',
        baslik: 'Kalfa Web Mimarisi — Kurulum',
        icerikTuru: 'text/html',
        acilmisBayt: 131_072,
        ciktiBayt: 65_536,
        kirpildi: true,
        devamImleci: 'web1:0123456789abcdef:420',
        govdeOnizleme: 'Kurulum adımı 1: Bağımlılıkları doğrulayın.',
      },
    });

    expect(screen.getByRole('heading', { name: 'Kalfa Web Mimarisi — Kurulum' })).toBeTruthy();
    expect(container.textContent).toContain('Dış web kaynağı (güvenilmez içerik)');
    expect(container.textContent).toContain('Kırpıldı');
    expect(container.textContent).toContain('https://belge.ornek.test/eski?token=[MASKELENDI]');
    expect(container.textContent).toContain('Son URL (yönlendirildi)');
    expect(container.textContent).toContain('https://belge.ornek.test/yeni?token=[MASKELENDI]');
    expect(container.textContent).toContain('web1:0123456789abcdef:420');
    expect(container.textContent).toContain('Kurulum adımı 1: Bağımlılıkları doğrulayın.');
  });

  it('dış içerikteki HTML ve script etiketlerini çalıştırmaz, düz metin olarak basar ve sabit style özniteliği kullanmaz', () => {
    const { container } = render(WebKaynakKarti, {
      props: {
        istenenUrl: 'https://belge.ornek.test/test',
        sonUrl: 'https://belge.ornek.test/test',
        baslik: '<img src=x onerror=alert(1)>Başlık',
        acilmisBayt: 256,
        ciktiBayt: 256,
        kirpildi: false,
        govdeOnizleme: '<script>window.__SALDIRI__ = true;</script><b onclick="alert(1)">Metin</b>',
      },
    });

    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('img')).toBeNull();
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toContain('<script>window.__SALDIRI__ = true;</script>');
    expect(container.querySelectorAll('[style]').length).toBe(0);
  });

  it('harici bağlantıyı kullanıcı açıkça onaylamadan açmaz; vazgeçilince çağırmaz', async () => {
    const onBaglantiAc = vi.fn();
    render(WebKaynakKarti, {
      props: {
        istenenUrl: 'https://belge.ornek.test/kilavuz',
        sonUrl: 'https://belge.ornek.test/kilavuz',
        baslik: 'Kılavuz',
        acilmisBayt: 512,
        ciktiBayt: 512,
        onBaglantiAc,
      },
    });

    expect(onBaglantiAc).not.toHaveBeenCalled();

    const ilkDugme = screen.getByRole('button', { name: 'Harici bağlantıyı aç' });
    await fireEvent.click(ilkDugme);
    expect(onBaglantiAc).not.toHaveBeenCalled();

    const vazgecDugme = screen.getByRole('button', { name: 'Vazgeç' });
    await fireEvent.click(vazgecDugme);
    expect(onBaglantiAc).not.toHaveBeenCalled();

    await fireEvent.click(screen.getByRole('button', { name: 'Harici bağlantıyı aç' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Tarayıcıda aç' }));
    expect(onBaglantiAc).toHaveBeenCalledOnce();
    expect(onBaglantiAc).toHaveBeenCalledWith('https://belge.ornek.test/kilavuz');
  });

  it('javascript:, file: veya kimlik bilgisi içeren güvensiz bağlantıların açılmasını engeller', () => {
    const onBaglantiAc = vi.fn();
    const { rerender } = render(WebKaynakKarti, {
      props: {
        istenenUrl: 'javascript:alert(1)',
        sonUrl: 'javascript:alert(1)',
        acilmisBayt: 64,
        ciktiBayt: 64,
        onBaglantiAc,
      },
    });

    expect(screen.getByRole('status').textContent).toContain('Güvensiz bağlantı şeması engellendi');
    expect(screen.queryByRole('button', { name: 'Harici bağlantıyı aç' })).toBeNull();

    rerender({
      istenenUrl: 'https://kullanici:parola@belge.ornek.test/',
      sonUrl: 'https://kullanici:parola@belge.ornek.test/',
      acilmisBayt: 64,
      ciktiBayt: 64,
      onBaglantiAc,
    });
    expect(screen.getByRole('status').textContent).toContain('Güvensiz bağlantı şeması engellendi');
    expect(onBaglantiAc).not.toHaveBeenCalled();
  });
});
