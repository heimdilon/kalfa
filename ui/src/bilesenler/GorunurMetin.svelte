<!-- Görünmez karakterleri vurgulu kaçışla çizen metin (güvenlik analizi Bulgu 7). -->
<script lang="ts">
  import { gorunurYap } from './gorunmez';

  let { metin }: { metin: string } = $props();

  const parcalar = $derived(gorunurYap(metin));
</script>

{#each parcalar as p, i (i)}{#if p.kacis}<span class="kacis">{p.metin}</span
    >{:else}{p.metin}{/if}{/each}

<style>
  .kacis {
    padding: 0 2px;
    border-radius: var(--radius-sm);
    color: var(--status-danger);
    background: color-mix(in oklab, var(--status-danger) 14%, transparent);
    font-family: var(--font-mono);
  }
</style>
