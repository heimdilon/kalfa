<!--
  Kaynak etiketi (tasarım §6.3, §6.5): dosya, kabuk, kullanıcı mesajı, sistem
  iletisi ya da araç tanımlarını gösterir. Tıklanabilir olduğunda gönderilen
  öğenin tam metnini açar.
-->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';
  import { boyutMetni } from '../ana/geriAlma';

  interface Props {
    kaynak: string;
    bayt?: number;
    sira?: number | null;
    secili?: boolean;
    onclick?: () => void;
    class?: string;
  }

  let {
    kaynak,
    bayt,
    sira,
    secili = false,
    onclick,
    class: sinif,
  }: Props = $props();

  interface KaynakAyrinti {
    icon: IkonAdi;
    tur: string;
    ayrinti: string;
  }

  const bilgi = $derived.by((): KaynakAyrinti => {
    if (kaynak.startsWith('dosya:')) {
      return { icon: 'file-text', tur: 'dosya', ayrinti: kaynak.slice(6) };
    }
    if (kaynak.startsWith('kabuk:')) {
      return { icon: 'square-terminal', tur: 'kabuk', ayrinti: kaynak.slice(6) };
    }
    if (kaynak === 'mesaj:kullanıcı') {
      return { icon: 'user', tur: 'mesaj', ayrinti: 'kullanıcı' };
    }
    if (kaynak === 'kalfa:sistem') {
      return { icon: 'cpu', tur: 'kalfa', ayrinti: 'sistem' };
    }
    if (kaynak === 'kalfa:araçlar') {
      return { icon: 'code', tur: 'kalfa', ayrinti: 'araçlar' };
    }
    if (kaynak === 'kalfa:sonuç') {
      return { icon: 'info', tur: 'kalfa', ayrinti: 'sonuç' };
    }
    if (kaynak === 'kalfa:not') {
      return { icon: 'info', tur: 'kalfa', ayrinti: 'not' };
    }
    if (kaynak.startsWith('arama:')) {
      return { icon: 'search', tur: 'arama', ayrinti: kaynak.slice(6) };
    }
    if (kaynak.startsWith('listeleme:')) {
      return { icon: 'folder', tur: 'listeleme', ayrinti: kaynak.slice(10) };
    }
    const ikiNokta = kaynak.indexOf(':');
    if (ikiNokta > 0) {
      return {
        icon: 'info',
        tur: kaynak.slice(0, ikiNokta),
        ayrinti: kaynak.slice(ikiNokta + 1),
      };
    }
    return { icon: 'info', tur: kaynak, ayrinti: '' };
  });

  const boyutYazisi = $derived(bayt !== undefined ? boyutMetni(bayt) : null);
</script>

{#if onclick}
  <button
    type="button"
    class={['kaynak-etiketi', 'tiklanabilir', { secili }, sinif]}
    {onclick}
  >
    <Icon name={bilgi.icon} size={14} />
    <span class="tur">{bilgi.tur}</span>
    {#if bilgi.ayrinti}
      <span class="ayrinti">{bilgi.ayrinti}</span>
    {/if}
    {#if boyutYazisi}
      <span class="boyut">{boyutYazisi}</span>
    {/if}
  </button>
{:else}
  <span class={['kaynak-etiketi', { secili }, sinif]}>
    <Icon name={bilgi.icon} size={14} />
    <span class="tur">{bilgi.tur}</span>
    {#if bilgi.ayrinti}
      <span class="ayrinti">{bilgi.ayrinti}</span>
    {/if}
    {#if boyutYazisi}
      <span class="boyut">{boyutYazisi}</span>
    {/if}
  </span>
{/if}

<style>
  .kaynak-etiketi {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    background: var(--bg-surface-elevated, var(--bg-surface));
    color: var(--text-body);
    font: var(--type-small);
    line-height: 1.2;
    text-align: left;
    max-width: 100%;
    overflow: hidden;
  }
  .tiklanabilir {
    cursor: pointer;
    transition: var(--transition-control);
  }
  .tiklanabilir:hover {
    border-color: var(--border-focus);
    background: var(--bg-hover);
  }
  .secili {
    border-color: var(--copper-400);
    background: var(--bg-selected, var(--bg-hover));
  }
  .tur {
    font-weight: var(--fw-semibold);
    color: var(--text-secondary);
    text-transform: lowercase;
    flex-shrink: 0;
  }
  .ayrinti {
    font-family: var(--font-mono);
    color: var(--text-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .boyut {
    margin-left: auto;
    font: var(--type-meta);
    color: var(--text-muted);
    flex-shrink: 0;
  }
</style>
