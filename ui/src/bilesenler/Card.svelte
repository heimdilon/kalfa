<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    title?: string;
    /** Başlığın altındaki tek aralıklı satır: yol, sayı, süre. */
    subtitle?: string;
    icon?: IkonAdi;
    /** Başlığın sağındaki denetimler. */
    actions?: Snippet;
    tone?: 'default' | 'sunken' | 'accent' | 'danger';
    padded?: boolean;
    selected?: boolean;
    class?: string;
    children?: Snippet;
  }

  let {
    title,
    subtitle,
    icon,
    actions,
    tone = 'default',
    padded = true,
    selected = false,
    class: sinif,
    children,
  }: Props = $props();
</script>

<section class={['kart', `ton-${tone}`, { secili: selected }, sinif]}>
  {#if title || actions}
    <header class={{ ayrik: Boolean(children) }}>
      {#if icon}
        <span class="karo"><Icon name={icon} size={15} /></span>
      {/if}
      <div class="baslik-blogu">
        <div class="baslik">{title}</div>
        {#if subtitle}<div class="alt-baslik">{subtitle}</div>{/if}
      </div>
      {#if actions}<div class="eylemler">{@render actions()}</div>{/if}
    </header>
  {/if}
  {#if children}
    <div class={{ ic: true, bosluklu: padded }}>{@render children()}</div>
  {/if}
</section>

<style>
  .kart {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  .ton-sunken {
    background: var(--surface-sunken);
  }
  .ton-accent,
  .secili {
    border-color: var(--border-accent);
  }
  .ton-danger {
    border-color: color-mix(in oklab, var(--red-400) 40%, transparent);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
  }
  header.ayrik {
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .karo {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text-secondary);
    flex: none;
  }
  .baslik-blogu {
    min-width: 0;
    flex: 1;
  }
  .baslik {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .alt-baslik {
    font: var(--type-meta);
    color: var(--text-muted);
    margin-top: 3px;
  }
  .eylemler {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: none;
  }
  .bosluklu {
    padding: var(--pad-card);
  }
</style>
