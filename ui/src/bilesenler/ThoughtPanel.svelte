<!--
  Modelin düşünce dökümü: tek satır. Akarken açık durur, bitince kapanır;
  düşünce sunulur, dayatılmaz.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { m } from '../dil/tr';
  import Icon from './Icon.svelte';

  interface Props {
    /** Özet satırı; verilmezse katalogdaki genel metin. */
    summary?: string;
    open?: boolean;
    /** Model hâlâ düşünce üretiyorsa canlı akış görünümü açılır. */
    akista?: boolean;
    class?: string;
    children?: Snippet;
  }

  let {
    summary,
    open = $bindable(false),
    akista = false,
    class: sinif,
    children,
  }: Props = $props();

  const kimlik = $props.id();
  const gorunur = $derived(open || akista);

  function degistir() {
    if (akista) return;
    open = !open;
  }
</script>

<section class={['dusunce', { akista, acik: gorunur }, sinif]}>
  <button
    type="button"
    disabled={akista}
    aria-expanded={gorunur}
    aria-controls={`${kimlik}-ic`}
    onclick={degistir}
  >
    <span class="ok"><Icon name={gorunur ? 'chevron-down' : 'chevron-right'} size={15} /></span>
    {#if akista}
      <span class="canli-nokta" aria-hidden="true"></span>
    {/if}
    <span class="metinler">
      <span class="baslik">{m('dusunce.baslik')}</span>
      {#if summary || akista}
        <span class="ozet">{summary ?? m('dusunce.suruyor')}</span>
      {/if}
    </span>
  </button>
  {#if gorunur}
    <div id={`${kimlik}-ic`} class="ic">{@render children?.()}</div>
  {/if}
</section>

<style>
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 0;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-muted);
    transition: var(--transition-control);
  }
  button:disabled {
    cursor: default;
  }
  button:hover:not(:disabled) {
    color: var(--text-secondary);
  }
  .ok {
    display: inline-flex;
  }
  .canli-nokta {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-pill);
    background: var(--action-primary);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--action-primary) 25%, transparent);
    animation: kalfa-imlec 1.2s ease-in-out infinite;
  }
  .metinler {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .baslik {
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-sans);
  }
  .akista .baslik {
    color: var(--text-secondary);
  }
  .ozet {
    font: var(--type-small);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ic {
    margin: 6px 0 2px 7px;
    padding: 2px 0 2px 14px;
    border-left: 2px solid var(--border-default);
    font: var(--type-small);
    color: var(--text-muted);
    max-height: 260px;
    overflow-y: auto;
  }
  .ic :global(:is(p, li)) {
    color: inherit;
  }
</style>
