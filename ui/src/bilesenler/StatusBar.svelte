<!-- 46 px'lik alt çubuk: bakiye, token kullanımı, model, izin kipi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Ekran okuyucunun okuduğu ad. */
    label: string;
    left?: Snippet;
    right?: Snippet;
    class?: string;
  }

  let { label, left, right, class: sinif }: Props = $props();
</script>

<footer class={['durum-cubugu', sinif]} aria-label={label}>
  {@render left?.()}
  <div class="sag">{@render right?.()}</div>
</footer>

<style>
  .durum-cubugu {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: var(--statusbar-height);
    flex: 0 0 auto;
    padding: 0 var(--space-8);
    background: var(--bg-rail);
    border-top: var(--border-width) solid var(--border-subtle);
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .sag {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-8);
  }
</style>
