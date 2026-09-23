<!--
  Güvenli Markdown gösterimi (1c tasarımı §9.1, §9.2). Sohbette, düşünce
  panelinde, araç çıktısında ve şart metninde aynı çizici kullanılır.
  Akan metin ekran karesi başına en çok bir kez yeniden çözülür; tamamlanan
  metin bir kez çözülür.
-->
<script lang="ts">
  import type { Root } from 'mdast';
  import { untrack } from 'svelte';
  import { cozumle } from './cozumle';
  import Dugum from './Dugum.svelte';

  interface Props {
    metin: string;
    akista?: boolean;
    /** Bağlantı tıklanınca; Rust adresi yerel iletişim kutusunda gösterip açar. */
    baglantiAc: (adres: string) => void;
    /** Şart metni gibi uzun belgeler için serif yazı düzeni. */
    belge?: boolean;
    class?: string;
  }

  let { metin, akista = false, baglantiAc, belge = false, class: sinif }: Props = $props();

  let agac: Root = $state.raw(untrack(() => cozumle(metin)));
  let bekleyen: number | null = null;

  $effect(() => {
    const guncel = metin;
    if (!akista) {
      if (bekleyen !== null) cancelAnimationFrame(bekleyen);
      bekleyen = null;
      agac = cozumle(guncel);
      return;
    }
    if (bekleyen !== null) return;
    bekleyen = requestAnimationFrame(() => {
      bekleyen = null;
      agac = cozumle(untrack(() => metin));
    });
  });

  $effect(() => () => {
    if (bekleyen !== null) cancelAnimationFrame(bekleyen);
  });
</script>

<div class={['markdown', { belge }, sinif]}>
  {#each agac.children as dugum, i (i)}
    <Dugum {dugum} {akista} {baglantiAc} />
  {/each}
</div>

<style>
  .markdown {
    font: var(--type-body);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .belge {
    font: var(--type-doc);
  }
  .markdown > :global(:first-child) {
    margin-top: 0;
  }
  .markdown > :global(:last-child) {
    margin-bottom: 0;
  }
  .markdown :global(p) {
    margin: 0 0 0.75em;
  }
  .markdown :global(h1),
  .markdown :global(h2),
  .markdown :global(h3),
  .markdown :global(h4),
  .markdown :global(h5),
  .markdown :global(h6) {
    margin: 1.1em 0 0.5em;
    color: var(--text-title);
    font-weight: var(--fw-semibold);
    line-height: var(--lh-snug);
  }
  .markdown :global(h1) {
    font-size: var(--text-title-lg);
  }
  .markdown :global(h2) {
    font-size: var(--text-title-size);
  }
  .markdown :global(h3),
  .markdown :global(h4),
  .markdown :global(h5),
  .markdown :global(h6) {
    font-size: var(--text-subtitle);
  }
  .markdown :global(ul),
  .markdown :global(ol) {
    margin: 0 0 0.75em;
    padding-left: 1.4em;
  }
  .markdown :global(li > p) {
    margin: 0;
  }
  .markdown :global(li.gorev) {
    list-style: none;
    margin-left: -1.4em;
  }
  .markdown :global(li.gorev > input[type='checkbox']) {
    margin: 0 8px 0 0;
    vertical-align: -2px;
  }
  .markdown :global(li.gorev > p) {
    display: inline;
  }
  .markdown :global(blockquote) {
    margin: 0 0 0.75em;
    padding: 2px 12px;
    border-left: 3px solid var(--border-strong);
    color: var(--text-secondary);
  }
  .markdown :global(code) {
    font: var(--type-meta);
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--surface-sunken);
  }
  .markdown :global(.ham-html) {
    color: var(--text-secondary);
  }
  .markdown :global(hr) {
    border: none;
    border-top: var(--border-width) solid var(--border-subtle);
    margin: 1em 0;
  }
  .markdown :global(.baglanti) {
    display: inline;
    padding: 0;
    border: none;
    border-bottom: 1px solid color-mix(in oklab, var(--text-link) 35%, transparent);
    background: none;
    color: var(--text-link);
    font: inherit;
    cursor: pointer;
  }
  .markdown :global(.baglanti:hover) {
    color: var(--text-link-hover);
    border-bottom-color: currentColor;
  }
  .markdown :global(.gorsel) {
    color: var(--text-muted);
    font-style: italic;
  }
  .markdown :global(.tablo) {
    overflow-x: auto;
    margin: 0 0 0.75em;
  }
  .markdown :global(.tablo table) {
    border-collapse: collapse;
    font: var(--type-small);
  }
  .markdown :global(.tablo th),
  .markdown :global(.tablo td) {
    padding: 5px 10px;
    border: var(--border-width) solid var(--border-subtle);
    text-align: left;
  }
  .markdown :global(.tablo th) {
    color: var(--text-title);
    font-weight: var(--fw-semibold);
  }
  .markdown :global(.tablo .orta) {
    text-align: center;
  }
  .markdown :global(.tablo .sag) {
    text-align: right;
  }
  .markdown :global(.dipnot) {
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .markdown :global(.kod) {
    margin: 0 0 0.75em;
  }
</style>
