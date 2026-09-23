<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
    icon?: IkonAdi;
    /** İkon yerine dolu bir durum noktası. */
    dot?: boolean;
    shape?: 'pill' | 'square';
    class?: string;
    children?: Snippet;
  }

  let { tone = 'neutral', icon, dot = false, shape = 'pill', class: sinif, children }: Props =
    $props();
</script>

<span class={['rozet', `ton-${tone}`, { kare: shape === 'square' }, sinif]}>
  {#if dot}<span class="nokta"></span>{/if}
  {#if icon}<Icon name={icon} size={12} />{/if}
  {@render children?.()}
</span>

<style>
  .rozet {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 9px;
    border-radius: var(--radius-pill);
    font: var(--fw-medium) var(--text-micro) / 1 var(--font-sans);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
    white-space: nowrap;
  }
  .kare {
    border-radius: var(--radius-sm);
  }
  .nokta {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--fg);
    flex: none;
  }
  .ton-neutral {
    --fg: var(--text-secondary);
    --bg: color-mix(in oklab, var(--warm-200) 10%, transparent);
    --bd: color-mix(in oklab, var(--warm-200) 22%, transparent);
  }
  .ton-accent {
    --fg: var(--text-accent);
    --bg: var(--surface-selected);
    --bd: color-mix(in oklab, var(--copper-400) 45%, transparent);
  }
  .ton-ok {
    --fg: var(--status-ok);
    --bg: color-mix(in oklab, var(--green-400) 14%, transparent);
    --bd: color-mix(in oklab, var(--green-400) 38%, transparent);
  }
  .ton-warn {
    --fg: var(--status-warn);
    --bg: color-mix(in oklab, var(--amber-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--amber-200) 38%, transparent);
  }
  .ton-danger {
    --fg: var(--status-danger);
    --bg: color-mix(in oklab, var(--red-400) 14%, transparent);
    --bd: color-mix(in oklab, var(--red-400) 38%, transparent);
  }
  .ton-info {
    --fg: var(--status-info);
    --bg: color-mix(in oklab, var(--blue-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--blue-200) 38%, transparent);
  }
</style>
