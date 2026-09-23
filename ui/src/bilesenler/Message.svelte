<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';
  import IconTile from './IconTile.svelte';

  interface Props {
    author?: 'user' | 'kalfa';
    /** Verilmezse "Siz" ya da "Kalfa". */
    name?: string;
    /** Kısa saat, "14:28". */
    time?: string;
    /** Token'lar gelmeye devam ederken yanıp sönen bakır imleç. */
    streaming?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { author = 'kalfa', name, time, streaming = false, class: sinif, children }: Props =
    $props();

  const kullanici = $derived(author === 'user');
</script>

<article class={['mesaj', { kullanici }, sinif]}>
  <IconTile icon={kullanici ? 'user' : 'hexagon'} tone={kullanici ? 'neutral' : 'accent'} />
  <div class="govde">
    <header>
      <span class="ad">{name ?? (kullanici ? m('mesaj.kullanici') : m('mesaj.kalfa'))}</span>
      {#if time}<span class="zaman">{time}</span>{/if}
    </header>
    <div class="icerik">
      {@render children?.()}
      {#if streaming}<span class="imlec" aria-hidden="true"></span>{/if}
    </div>
  </div>
</article>

<style>
  .mesaj {
    display: flex;
    gap: 12px;
    padding: 14px 16px;
  }
  .kullanici {
    background: var(--surface-raised);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
  }
  .govde {
    flex: 1;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 5px;
  }
  .ad {
    font: var(--fw-semibold) var(--text-small) / 1 var(--font-sans);
    color: var(--text-title);
  }
  .zaman {
    margin-left: auto;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .icerik {
    font: var(--type-body);
    color: var(--text-body);
    text-wrap: pretty;
  }
  .imlec {
    display: inline-block;
    width: 8px;
    height: 15px;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--copper-200);
    animation: kalfa-imlec 1s steps(2) infinite;
  }
</style>
