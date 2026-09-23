<!-- Sol menüde ya da iç içe listede tek satır. Etkin satır bakır kenarlık alır. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    icon?: IkonAdi;
    active?: boolean;
    /** Sağdaki tek aralıklı sayı ya da not. */
    badge?: string | number;
    /** Girinti düzeyi; her düzey 14 px ekler. */
    depth?: number;
    class?: string;
    children?: Snippet;
  }

  let {
    icon,
    active = false,
    badge,
    depth = 0,
    class: sinif,
    children,
    ...geri
  }: Props = $props();
</script>

<button
  type="button"
  {...geri}
  aria-current={active ? 'page' : undefined}
  class={['menu-ogesi', { etkin: active }, sinif]}
  style:padding-left={`${10 + depth * 14}px`}
>
  {#if icon}<span class="ikon-yeri"><Icon name={icon} size={18} /></span>{/if}
  <span class="etiket">{@render children?.()}</span>
  {#if badge !== undefined}<span class="rozet">{badge}</span>{/if}
</button>

<style>
  .menu-ogesi {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    height: var(--row-height);
    padding-right: var(--space-6);
    border: 0;
    border-radius: var(--radius-md);
    cursor: pointer;
    font: var(--fw-regular) var(--text-small) / 1 var(--font-sans);
    color: var(--text-secondary);
    background: transparent;
    transition: var(--transition-control);
    text-align: left;
  }
  .menu-ogesi:hover {
    color: var(--text-body);
    background: var(--surface-raised);
  }
  .etkin,
  .etkin:hover {
    font-weight: var(--fw-semibold);
    color: var(--text-title);
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .ikon-yeri {
    display: inline-flex;
  }
  .etkin .ikon-yeri {
    color: var(--text-accent);
  }
  .etiket {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rozet {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
