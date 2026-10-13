<!-- Modelin düşünce dökümü. Kalfa bunu kapalı gösterir: düşünce sunulur, dayatılmaz. -->
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
      <span class="ozet">
        {summary ?? (akista ? m('dusunce.suruyor') : m('dusunce.bitti'))}
      </span>
    </span>
  </button>
  {#if gorunur}
    <div id={`${kimlik}-ic`} class="ic">{@render children?.()}</div>
  {/if}
</section>

<style>
  .dusunce {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: var(--transition-control);
  }
  .dusunce.akista {
    border-left: 2px solid var(--copper-300);
    background: color-mix(in srgb, var(--copper-400) 6%, var(--surface-card));
  }
  button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }
  button:disabled {
    cursor: default;
  }
  button:hover:not(:disabled) {
    background: var(--surface-raised);
  }
  .ok {
    display: inline-flex;
    color: var(--text-muted);
  }
  .canli-nokta {
    width: 7px;
    height: 7px;
    border-radius: var(--radius-pill);
    background: var(--copper-200);
    box-shadow: 0 0 8px var(--copper-300);
    animation: kalfa-imlec 1s steps(2) infinite;
  }
  .metinler {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .baslik {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .akista .baslik {
    color: var(--text-accent);
  }
  .ozet {
    font: var(--type-meta);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ic {
    padding: 10px 14px 12px 34px;
    border-top: var(--border-width) solid var(--border-subtle);
    font: var(--type-small);
    color: var(--text-secondary);
    max-height: 260px;
    overflow-y: auto;
  }
</style>
