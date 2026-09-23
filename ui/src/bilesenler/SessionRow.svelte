<!--
  295 px'lik oturum listesinde bir oturum. Nokta rengi durumu gösterir
  (1c tasarımı §8.1): çalışıyor, onay ya da şart bekliyor, tamamlandı,
  hata, yarım kaldı.
-->
<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Durum = 'running' | 'waiting' | 'done' | 'failed' | 'partial' | 'idle';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children' | 'title'> {
    title: string;
    /** İki satıra kırpılan kısa açıklama; çoğunlukla çalışma alanı. */
    preview?: string;
    /** Sağa yaslı tek aralıklı zaman. */
    time?: string;
    status?: Durum;
    /** Durumun ekran okuyucuya okunan adı. */
    statusLabel?: string;
    active?: boolean;
    class?: string;
  }

  let {
    title,
    preview,
    time,
    status,
    statusLabel,
    active = false,
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<button
  type="button"
  {...geri}
  aria-current={active ? 'true' : undefined}
  class={['oturum-satiri', { etkin: active }, sinif]}
>
  <span class="ust">
    {#if status}
      <span class={['nokta', `d-${status}`]} aria-hidden="true"></span>
      {#if statusLabel}<span class="gizli-erisilebilir">{statusLabel}</span>{/if}
    {/if}
    <span class="baslik">{title}</span>
    {#if time}<span class="zaman">{time}</span>{/if}
  </span>
  {#if preview}<span class="onizleme">{preview}</span>{/if}
</button>

<style>
  .oturum-satiri {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    width: 100%;
    text-align: left;
    padding: var(--space-5) var(--space-6);
    cursor: pointer;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    transition: var(--transition-control);
  }
  .oturum-satiri:hover {
    background: var(--surface-raised);
  }
  .etkin,
  .etkin:hover {
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .ust {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
  }
  .nokta {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-pill);
    flex: 0 0 auto;
    background: var(--text-muted);
  }
  .d-running {
    background: var(--status-warn);
  }
  .d-waiting {
    background: var(--text-accent);
  }
  .d-done {
    background: var(--status-ok);
  }
  .d-failed {
    background: var(--status-danger);
  }
  .d-partial {
    background: var(--status-info);
  }
  .baslik {
    flex: 1;
    min-width: 0;
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-sans);
    color: var(--text-body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .etkin .baslik {
    color: var(--text-title);
  }
  .zaman {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .onizleme {
    font: var(--fw-regular) var(--text-micro) / 1.45 var(--font-sans);
    color: var(--text-muted);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
