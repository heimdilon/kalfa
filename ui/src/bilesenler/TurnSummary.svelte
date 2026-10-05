<!--
  Turun kapanış bloğu (tasarım sistemindeki TurnSummary): ne değişti ve tur
  gerçekten geri alınabiliyorsa geri alma düğmesi. Kabuk ve ağ etkileri geri
  alınamaz; `note` bunu söyler.
-->
<script lang="ts">
  import Button from './Button.svelte';
  import IconTile from './IconTile.svelte';

  export type SummaryItem = string | { text: string; onclick?: () => void };

  interface Props {
    title: string;
    /** Değişiklikler, madde başına bir satır. */
    items: SummaryItem[];
    time?: string;
    /** Yalnızca tur gerçekten geri alınabiliyorsa verilir. */
    onUndo?: () => void;
    undoLabel?: string;
    /** Uyarı satırları, ör. "Kabukla yapılan değişiklikler geri alınamaz." */
    note?: string[];
    class?: string;
  }

  let { title, items, time, onUndo, undoLabel, note = [], class: sinif }: Props = $props();
</script>

<section class={['tur-ozeti', sinif]} aria-label={title}>
  <IconTile icon="file-check" />
  <div class="govde">
    <div class="ust">
      <span class="baslik">{title}</span>
      <span class="bosluk"></span>
      {#if onUndo}
        <Button size="sm" iconLeft="rotate-ccw" onclick={onUndo}>{undoLabel}</Button>
      {/if}
      {#if time}<span class="zaman">{time}</span>{/if}
    </div>
    {#if items.length > 0}
      <ul>
        {#each items as it, i (i)}
          <li>
            {#if typeof it === 'string'}
              {it}
            {:else if it.onclick}
              <button type="button" class="baglanti" onclick={it.onclick}>
                {it.text}
              </button>
            {:else}
              {it.text}
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
    {#each note as n, i (i)}<p class="not">{n}</p>{/each}
  </div>
</section>

<style>
  .tur-ozeti {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
  }
  .govde {
    flex: 1;
    min-width: 0;
  }
  .ust {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }
  .baslik {
    font: var(--fw-semibold) var(--text-small) / 1 var(--font-sans);
    color: var(--text-title);
  }
  .bosluk {
    flex: 1;
  }
  .zaman {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    font: var(--type-body);
    color: var(--text-body);
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
  .not {
    margin: 10px 0 0;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .baglanti {
    background: none;
    border: none;
    padding: 0;
    margin: 0;
    font: inherit;
    color: var(--copper-400);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
    text-align: left;
  }
  .baglanti:hover {
    color: var(--copper-300, var(--copper-400));
  }
</style>
