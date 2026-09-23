import { fireEvent, render, screen } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import Button from './Button.svelte';
import Checkbox from './Checkbox.svelte';
import IconButton from './IconButton.svelte';
import Select from './Select.svelte';

const metin = (m: string) => createRawSnippet(() => ({ render: () => `<span>${m}</span>` }));

const MODELLER = [
  { value: 'glm-5.3', label: 'Derin', hint: 'glm-5.3' },
  { value: 'auto', label: 'Otomatik' },
];
const KIPLER = [
  { value: 'Siki', label: 'Sıkı' },
  { value: 'Serbest', label: 'Serbest' },
];

describe('temel bileşenler', () => {
  it('Button tıklanır; yüklenirken ve kapalıyken tıklanmaz', async () => {
    const onclick = vi.fn();
    const { rerender } = render(Button, { props: { onclick, children: metin('Kaydet') } });
    const dugme = screen.getByRole<HTMLButtonElement>('button', { name: 'Kaydet' });
    await fireEvent.click(dugme);
    expect(onclick).toHaveBeenCalledOnce();
    await rerender({ onclick, loading: true, children: metin('Kaydet') });
    expect(dugme.disabled).toBe(true);
    expect(dugme.getAttribute('aria-busy')).toBe('true');
  });

  it('IconButton etiketini erişilebilir ad olarak taşır', () => {
    render(IconButton, { props: { icon: 'x', label: 'Kapat', active: true } });
    const dugme = screen.getByRole('button', { name: 'Kapat' });
    expect(dugme.getAttribute('title')).toBe('Kapat');
    expect(dugme.getAttribute('aria-pressed')).toBe('true');
  });

  it('Checkbox gerçek onay kutusudur; karışık durumu bildirir', async () => {
    const onchange = vi.fn();
    render(Checkbox, { props: { label: 'a.rs', indeterminate: true, onchange } });
    const kutu = screen.getByRole<HTMLInputElement>('checkbox', { name: 'a.rs' });
    expect(kutu.indeterminate).toBe(true);
    await fireEvent.click(kutu);
    expect(onchange).toHaveBeenCalledWith(true);
  });
});

describe('Select', () => {
  it('klavyeyle açılır, okla gezilir, Enter ile seçilir', async () => {
    const onchange = vi.fn();
    render(Select, { props: { label: 'Model', value: 'glm-5.3', options: MODELLER, onchange } });
    const tetik = screen.getByRole('combobox', { name: 'Model' });
    expect(tetik.textContent).toContain('Derin');
    await fireEvent.keyDown(tetik, { key: 'ArrowDown' });
    expect(tetik.getAttribute('aria-expanded')).toBe('true');
    expect(screen.getAllByRole('option')).toHaveLength(2);
    await fireEvent.keyDown(tetik, { key: 'ArrowDown' });
    await fireEvent.keyDown(tetik, { key: 'Enter' });
    expect(onchange).toHaveBeenCalledWith('auto');
    expect(screen.queryByRole('listbox')).toBeNull();
  });

  it('Escape kapatır, seçim değişmez', async () => {
    const onchange = vi.fn();
    render(Select, { props: { label: 'Kip', value: 'Siki', options: KIPLER, onchange } });
    const tetik = screen.getByRole('combobox', { name: 'Kip' });
    await fireEvent.click(tetik);
    await fireEvent.keyDown(tetik, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).toBeNull();
    expect(onchange).not.toHaveBeenCalled();
  });
});
