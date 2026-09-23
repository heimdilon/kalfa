<!-- Durum çubuğunda tek bir bilgi. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon?: IkonAdi;
    tone?: 'ok' | 'warn' | 'danger' | 'info' | 'accent';
    /** Fareyle üzerine gelince görünen açıklama. */
    title?: string;
    class?: string;
    children?: Snippet;
  }

  let { icon, tone, title, class: sinif, children }: Props = $props();
</script>

<span class={['durum-ogesi', tone && `t-${tone}`, sinif]} {title}>
  {#if icon}<Icon name={icon} size={14} />{/if}
  {@render children?.()}
</span>

<style>
  .durum-ogesi {
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .t-ok {
    color: var(--status-ok);
  }
  .t-warn {
    color: var(--status-warn);
  }
  .t-danger {
    color: var(--status-danger);
  }
  .t-info {
    color: var(--status-info);
  }
  .t-accent {
    color: var(--text-accent);
  }
</style>
