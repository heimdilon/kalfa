<script lang="ts" module>
  export interface Dilim {
    label: string;
    value: number;
    /** Herhangi bir renk tokenı; verilmezse bakır. */
    color?: string;
  }
</script>

<script lang="ts">
  interface Props {
    segments: Dilim[];
    /** Bağlam penceresi; verilirse çubuk oranla dolar. */
    total?: number;
    used?: number;
    class?: string;
  }

  let { segments, total, used, class: sinif }: Props = $props();

  const toplam = $derived(segments.reduce((a, d) => a + d.value, 0) || 1);
  const doluluk = $derived(total ? Math.min(1, (used ?? toplam) / total) : 1);
</script>

<div class={['olcer', sinif]}>
  <div class="cubuk">
    <div class="dolgu" style:width={`${doluluk * 100}%`}>
      {#each segments as dilim, i (i)}
        <span
          style:width={`${(dilim.value / toplam) * 100}%`}
          style:background={dilim.color ?? 'var(--copper-400)'}
        ></span>
      {/each}
    </div>
  </div>
  <div class="aciklama">
    {#each segments as dilim, i (i)}
      <span class="kalem">
        <span class="nokta" style:background={dilim.color ?? 'var(--copper-400)'}></span>
        {dilim.label} ({dilim.value.toLocaleString('tr-TR')})
      </span>
    {/each}
  </div>
</div>

<style>
  .cubuk {
    display: flex;
    height: 6px;
    border-radius: var(--radius-pill);
    overflow: hidden;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
  }
  .dolgu {
    display: flex;
    min-width: 2px;
  }
  .aciklama {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 16px;
    margin-top: 10px;
  }
  .kalem {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font: var(--type-meta);
    color: var(--text-secondary);
  }
  .nokta {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
</style>
