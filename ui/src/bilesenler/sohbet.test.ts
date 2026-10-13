import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Composer from './Composer.svelte';
import SessionRow from './SessionRow.svelte';
import ThoughtPanel from './ThoughtPanel.svelte';
import ToolCallCard from './ToolCallCard.svelte';

const metin = (m: string) => createRawSnippet(() => ({ render: () => `<span>${m}</span>` }));

const MODELLER = [
  { value: 'glm-5.3', label: 'Derin', hint: 'glm-5.3' },
  { value: 'auto', label: 'Otomatik' },
];
const KIPLER = [
  { value: 'Siki', label: 'Sıkı' },
  { value: 'Serbest', label: 'Serbest' },
];

describe('Composer (1c tasarımı §8.1, §9.5)', () => {
  function ciz(ek: Record<string, unknown> = {}) {
    const onsend = vi.fn();
    const oncancel = vi.fn();
    render(Composer, {
      props: { models: MODELLER, model: 'glm-5.3', modes: KIPLER, mode: 'Siki', onsend, oncancel, ...ek },
    });
    return { onsend, oncancel, alan: screen.getByRole<HTMLTextAreaElement>('textbox') };
  }

  it('Enter gönderir ve alanı boşaltır; Shift+Enter göndermez', async () => {
    const { onsend, alan } = ciz();
    await fireEvent.input(alan, { target: { value: 'selam' } });
    await fireEvent.keyDown(alan, { key: 'Enter', shiftKey: true });
    expect(onsend).not.toHaveBeenCalled();
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).toHaveBeenCalledWith('selam');
    expect(alan.value).toBe('');
  });

  it('boş mesaj gönderilmez', async () => {
    const { onsend, alan } = ciz();
    await fireEvent.input(alan, { target: { value: '   ' } });
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).not.toHaveBeenCalled();
    expect(screen.getByRole<HTMLButtonElement>('button', { name: 'Gönder' }).disabled).toBe(true);
  });

  it('tur sürerken gönder yerine durdur vardır', async () => {
    const { onsend, oncancel, alan } = ciz({ running: true });
    expect(screen.queryByRole('button', { name: 'Gönder' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Turu durdur' }));
    expect(oncancel).toHaveBeenCalledOnce();
    await fireEvent.input(alan, { target: { value: 'x' } });
    await fireEvent.keyDown(alan, { key: 'Enter' });
    expect(onsend).not.toHaveBeenCalled();
  });

  it('düşünce eforu balonu açılır, kaydırıcıyla efor seçilir ve modeller yenilenir', async () => {
    const oneforchange = vi.fn();
    const onmodelyenile = vi.fn();
    ciz({ efor: 'orta', oneforchange, onmodelyenile });
    const tetik = screen.getByRole('button', { name: 'Düşünce eforu' });
    expect(tetik.textContent).toContain('Orta');
    await fireEvent.click(tetik);
    expect(screen.getByRole('dialog', { name: 'Düşünce eforu ve model' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('radio', { name: 'Yüksek' }));
    expect(oneforchange).toHaveBeenCalledWith('yuksek');
    await fireEvent.click(screen.getByRole('button', { name: 'Modelleri yenile' }));
    expect(onmodelyenile).toHaveBeenCalledOnce();
  });
});

describe('sohbet ve gezinme bileşenleri', () => {
  it('ThoughtPanel kapalı başlar, düğmeyle açılır; akışta canlı açık görünür', async () => {
    const { rerender } = render(ThoughtPanel, {
      props: { children: metin('iç düşünce'), akista: true },
    });
    const dugme = screen.getByRole('button', { name: /Düşünce/ });
    expect(dugme.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getByText('Düşünüyor…')).toBeTruthy();
    expect(screen.getByText('iç düşünce').textContent).toBe('iç düşünce');

    await rerender({ children: metin('iç düşünce'), akista: false });
    expect(dugme.getAttribute('aria-expanded')).toBe('false');
    expect(screen.queryByText('iç düşünce')).toBeNull();
    await fireEvent.click(dugme);
    expect(screen.getByText('iç düşünce').textContent).toBe('iç düşünce');
  });

  it('ToolCallCard durumu ekran okuyucuya adıyla okunur', () => {
    render(ToolCallCard, {
      props: { tool: 'Dosya okundu', target: 'a.rs', status: 'unknown', statusLabel: 'Sonucu bilinmiyor' },
    });
    expect(screen.getAllByRole('img', { name: 'Sonucu bilinmiyor' })).toHaveLength(1);
  });

  it('SessionRow durum adını gizli metin olarak taşır', () => {
    render(SessionRow, { props: { title: 'Oturum', status: 'waiting', statusLabel: 'Onay bekliyor' } });
    expect(screen.getAllByRole('button', { name: /Onay bekliyor/ })).toHaveLength(1);
  });
});
