<!-- Mesaj yazarının ve günlük satırının simgesi olarak kullanılan kare karo. -->
<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon: IkonAdi;
    tone?: 'neutral' | 'accent' | 'ok' | 'warn' | 'danger' | 'info';
    /** 28 sık satırda, 32 varsayılan, 40 başlıkta. */
    size?: 28 | 32 | 40;
    class?: string;
  }

  let { icon, tone = 'neutral', size = 32, class: sinif }: Props = $props();
</script>

<span class={['karo', `ton-${tone}`, `b-${size}`, sinif]}>
  <Icon name={icon} size={Math.round(size * 0.5)} />
</span>

<style>
  .karo {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    border-radius: var(--radius-md);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
  }
  .b-28 {
    width: 28px;
    height: 28px;
  }
  .b-32 {
    width: 32px;
    height: 32px;
  }
  .b-40 {
    width: 40px;
    height: 40px;
  }
  .ton-neutral {
    --fg: var(--text-secondary);
    --bg: var(--surface-sunken);
    --bd: var(--border-subtle);
  }
  .ton-accent {
    --fg: var(--text-accent);
    --bg: var(--surface-selected);
    --bd: color-mix(in oklab, var(--copper-400) 40%, transparent);
  }
  .ton-ok {
    --fg: var(--status-ok);
    --bg: var(--green-tint);
    --bd: color-mix(in oklab, var(--green-400) 35%, transparent);
  }
  .ton-warn {
    --fg: var(--status-warn);
    --bg: var(--amber-tint);
    --bd: color-mix(in oklab, var(--amber-200) 35%, transparent);
  }
  .ton-danger {
    --fg: var(--status-danger);
    --bg: var(--red-tint);
    --bd: color-mix(in oklab, var(--red-400) 35%, transparent);
  }
  .ton-info {
    --fg: var(--status-info);
    --bg: var(--blue-tint);
    --bd: color-mix(in oklab, var(--blue-200) 35%, transparent);
  }
</style>
