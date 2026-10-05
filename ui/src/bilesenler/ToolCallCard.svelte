<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import IconTile from './IconTile.svelte';
  import type { IkonAdi } from './ikonlar';

  type Durum = 'running' | 'done' | 'failed' | 'denied' | 'unknown';

  interface Props {
    /** Ne oldu, geçmiş zamanla: "Dosya okundu". */
    tool: string;
    /** Tek aralıklı hedef: yol, desen. */
    target?: string;
    /** Tek aralıklı özet: "412 satır · 18 KB". */
    meta?: string;
    status?: Durum;
    /** Durumun ekran okuyucuya okunan adı. */
    statusLabel: string;
    icon?: IkonAdi;
    class?: string;
    /** Çıktı önizlemesi; çoğunlukla bir CodeBlock. */
    children?: Snippet;
    /** Başlığın altında rozetler (örneğin güvenlik denetçisinin, 3c). */
    rozet?: Snippet;
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
  }: Props = $props();

  const DURUM_IKONU: Record<Durum, IkonAdi> = {
    running: 'loader-circle',
    done: 'circle-check',
    failed: 'triangle-alert',
    denied: 'circle-x',
    unknown: 'circle-help',
  };
</script>

<section class={['arac', sinif]}>
  <header>
    <IconTile {icon} />
    <div class="metinler">
      <div class="ad">{tool}</div>
      {#if target}<div class="hedef">{target}</div>{/if}
      {#if meta}<div class="ozet">{meta}</div>{/if}
      {#if rozet}<div class="rozetler">{@render rozet()}</div>{/if}
    </div>
    <span class={['durum', `d-${status}`]}>
      <Icon
        name={DURUM_IKONU[status]}
        size={19}
        title={statusLabel}
        class={status === 'running' ? 'donen' : undefined}
      />
    </span>
  </header>
  {#if children}<div class="cikti">{@render children()}</div>{/if}
</section>

<style>
  .arac {
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-xl);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
  }
  .metinler {
    flex: 1;
    min-width: 0;
  }
  .rozetler {
    margin-top: 6px;
  }
  .ad {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-title);
  }
  .hedef {
    margin-top: 3px;
    font: var(--type-meta);
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }
  .ozet {
    margin-top: 6px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .durum {
    display: inline-flex;
  }
  .d-running {
    color: var(--status-warn);
  }
  .d-done {
    color: var(--status-ok);
  }
  .d-failed,
  .d-denied {
    color: var(--status-danger);
  }
  .d-unknown {
    color: var(--text-muted);
  }
  .cikti {
    padding: 0 14px 14px;
  }
</style>
