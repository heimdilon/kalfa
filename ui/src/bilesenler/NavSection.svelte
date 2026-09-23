<!-- Menü içinde büyük harfli, aralıklı grup etiketi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    class?: string;
    children?: Snippet;
  }

  let { label, class: sinif, children }: Props = $props();
</script>

<div class={['bolum', sinif]}>
  <div class="etiket">{label}</div>
  {@render children?.()}
</div>

<style>
  .bolum {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-top: var(--space-6);
  }
  .etiket {
    font: var(--type-label);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
    color: var(--text-muted);
    padding: 0 var(--space-5) var(--space-3);
  }
</style>
