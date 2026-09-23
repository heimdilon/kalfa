<!-- Modelin düşünce dökümü. Kalfa bunu kapalı gösterir: düşünce sunulur, dayatılmaz. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';
  import Icon from './Icon.svelte';

  interface Props {
    /** Özet satırı; verilmezse katalogdaki genel metin. */
    summary?: string;
    open?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { summary, open = $bindable(false), class: sinif, children }: Props = $props();

  const kimlik = $props.id();
</script>

<section class={['dusunce', sinif]}>
  <button
    type="button"
    aria-expanded={open}
    aria-controls={`${kimlik}-ic`}
    onclick={() => (open = !open)}
  >
    <span class="ok"><Icon name={open ? 'chevron-down' : 'chevron-right'} size={15} /></span>
    <span class="metinler">
      <span class="baslik">{m('dusunce.baslik')}</span>
      <span class="ozet">{summary ?? m('dusunce.ozet')}</span>
    </span>
  </button>
  {#if open}
    <div id={`${kimlik}-ic`} class="ic">{@render children?.()}</div>
  {/if}
</section>

<style>
  .dusunce {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 12px 14px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }
  .ok {
    display: inline-flex;
    color: var(--text-muted);
  }
  .metinler {
    flex: 1;
  }
  .baslik {
    display: block;
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .ozet {
    display: block;
    margin-top: 3px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .ic {
    padding: 12px 14px 14px 39px;
    border-top: var(--border-width) solid var(--border-subtle);
    font: var(--type-small);
    color: var(--text-secondary);
  }
</style>
