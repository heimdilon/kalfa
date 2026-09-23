import { render } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';
import CodeBlock from './CodeBlock.svelte';
import DiffView from './DiffView.svelte';
import MetaList from './MetaList.svelte';
import TokenMeter from './TokenMeter.svelte';

describe('veri bileşenleri', () => {
  it('CodeBlock satır numaralarıyla çizer; kırpıldıysa söyler', () => {
    const { container } = render(CodeBlock, {
      props: { code: 'a\nb\n', startLine: 41, truncated: true },
    });
    expect([...container.querySelectorAll('.no')].map((n) => n.textContent)).toEqual(['41', '42']);
    expect(container.textContent).toContain('Çıktının devamı gösterilmiyor');
  });

  it('DiffView eklenen ve silinen satırları işaretler', () => {
    const { container } = render(DiffView, {
      props: {
        hunkHeader: '@@ -1 +1 @@',
        lines: [
          { kind: 'del', old: 1, text: 'eski' },
          { kind: 'add', neu: 1, text: 'yeni' },
        ],
      },
    });
    expect([...container.querySelectorAll('tr')].map((t) => t.className.split(' ')[0])).toEqual([
      't-del',
      't-add',
    ]);
    expect(container.textContent).toContain('@@ -1 +1 @@');
  });

  it('MetaList etiket ve değerleri tanım listesi olarak yazar', () => {
    const { container } = render(MetaList, {
      props: { items: [{ label: 'Nerede', value: 'src/a.rs', mono: true }] },
    });
    expect(container.querySelector('dt')?.textContent).toBe('Nerede');
    expect(container.querySelector('dd.mono')?.textContent).toBe('src/a.rs');
  });

  it('TokenMeter sayıları Türkçe biçimle yazar', () => {
    const { container } = render(TokenMeter, {
      props: { segments: [{ label: 'İstem', value: 12840 }] },
    });
    expect(container.textContent).toContain('İstem (12.840)');
  });
});
