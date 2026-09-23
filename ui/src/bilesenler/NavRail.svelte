<!-- Masaüstü uygulamasının 213 px'lik sabit sol menüsü. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Ekran okuyucunun okuduğu menü adı. */
    label: string;
    /** Üstteki marka alanı. */
    header?: Snippet;
    /** Alta sabitlenen alan. */
    footer?: Snippet;
    class?: string;
    children?: Snippet;
  }

  let { label, header, footer, class: sinif, children }: Props = $props();
</script>

<nav class={['menu', sinif]} aria-label={label}>
  {#if header}<div class="ust">{@render header()}</div>{/if}
  <div class="liste">{@render children?.()}</div>
  {#if footer}<div class="alt">{@render footer()}</div>{/if}
</nav>

<style>
  .menu {
    display: flex;
    flex-direction: column;
    width: var(--rail-width);
    flex: 0 0 auto;
    height: 100%;
    background: var(--bg-rail);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .ust {
    padding: var(--space-6) var(--space-6) var(--space-4);
  }
  .liste {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-2) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .alt {
    padding: var(--space-4) var(--space-6);
    border-top: var(--border-width) solid var(--border-subtle);
  }
</style>
