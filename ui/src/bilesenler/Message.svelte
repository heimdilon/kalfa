<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';

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
  <div class="govde">
    <header class={{ 'gizli-erisilebilir': kullanici || !time }}>
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
  }
  .kullanici {
    width: fit-content;
    margin-left: auto;
    max-width: min(80%, 640px);
    padding: 9px 14px;
    background: var(--surface-raised);
    border-radius: 14px;
  }
  .govde {
    flex: 1;
    min-width: 0;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 10px;
    margin-bottom: 4px;
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
    font: var(--fw-regular) 14.5px / 1.65 var(--font-sans);
    color: var(--text-body);
    text-wrap: pretty;
  }
  .imlec {
    display: inline-block;
    width: 7px;
    height: 15px;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--copper-200);
    animation: kalfa-imlec 1s steps(2) infinite;
  }
</style>
