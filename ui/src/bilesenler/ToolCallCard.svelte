<!--
  Araç çağrısı: tek satır ("Dosya okundu src/auth.rs · 48 satır"). Çıktısı
  varsa satıra tıklanınca açılır; çalışırken ya da hata verince kendiliğinden
  açık durur. Çıktı kapalıyken de belgede kalır.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  type Durum = 'running' | 'done' | 'failed' | 'denied' | 'unknown';

  interface Props {
    /** Ne oldu, geçmiş zamanla: "Dosya okundu". */
    tool: string;
    /** Tek aralıklı hedef: yol, desen. */
    target?: string;
    /** Kısa özet: "412 satır · 18 KB". */
    meta?: string;
    status?: Durum;
    /** Durumun ekran okuyucuya okunan adı. */
    statusLabel: string;
    icon?: IkonAdi;
    class?: string;
    /** Çıktı önizlemesi; çoğunlukla bir CodeBlock. */
    children?: Snippet;
    /** Satırın altında rozetler (örneğin güvenlik denetçisinin, 3c). */
    rozet?: Snippet;
    /** Çıktı bittiğinde de açık başlar (örneğin dosya diff'i). */
    acikBaslar?: boolean;
  }

  let {
    tool,
    target,
    meta,
    status = 'done',
    statusLabel,
    icon = 'file-text',
    class: sinif,
    children,
    rozet,
    acikBaslar = false,
  }: Props = $props();

  const DURUM_IKONU: Record<Durum, IkonAdi> = {
    running: 'loader-circle',
    done: 'circle-check',
    failed: 'triangle-alert',
    denied: 'circle-x',
    unknown: 'circle-help',
  };
</script>

{#snippet satir()}
  <span class="ikon"><Icon name={icon} size={14} /></span>
  <span class="ad">{tool}</span>
  {#if target}<span class="hedef" title={target}>{target}</span>{/if}
  <span class="bosluk"></span>
  {#if meta}<span class="ozet">{meta}</span>{/if}
  <span class={['durum', `d-${status}`]}>
    <Icon
      name={DURUM_IKONU[status]}
      size={14}
      title={statusLabel}
      class={status === 'running' ? 'donen' : undefined}
    />
  </span>
{/snippet}

<section class={['arac', sinif]}>
  {#if children}
    <details open={acikBaslar || status === 'running' || status === 'failed'}>
      <summary class="satir">{@render satir()}</summary>
      <div class="cikti">{@render children()}</div>
    </details>
  {:else}
    <div class="satir">{@render satir()}</div>
  {/if}
  {#if rozet}<div class="rozetler">{@render rozet()}</div>{/if}
</section>

<style>
  .satir {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 26px;
    padding: 2px 6px;
    margin: 0 -6px;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font: var(--type-small);
  }
  summary.satir {
    list-style: none;
    cursor: pointer;
    transition: var(--transition-control);
  }
  summary.satir::-webkit-details-marker {
    display: none;
  }
  summary.satir:hover {
    background: var(--surface-card);
    color: var(--text-body);
  }
  .ikon {
    display: inline-flex;
    color: var(--text-muted);
  }
  .ad {
    flex: none;
  }
  .hedef {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--fw-regular) var(--text-micro) / 1.4 var(--font-mono);
    color: var(--text-body);
  }
  .bosluk {
    flex: 1;
  }
  .ozet {
    flex: none;
    font: var(--type-meta);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .durum {
    display: inline-flex;
  }
  .d-running {
    color: var(--status-warn);
  }
  .d-done {
    color: var(--text-muted);
  }
  .d-failed,
  .d-denied {
    color: var(--status-danger);
  }
  .d-unknown {
    color: var(--text-muted);
  }
  .cikti {
    margin: 6px 0 4px 22px;
  }
  .rozetler {
    margin: 2px 0 0 22px;
  }
</style>
