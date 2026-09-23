<!-- Her panelin ya da sütunun üstündeki 44 px'lik başlık çubuğu. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    icon?: IkonAdi;
    title: string;
    /** Başlığın yanındaki tek aralıklı bilgi: yol, sayı, zaman. */
    subtitle?: string;
    /** Sağa yaslı denetimler. */
    actions?: Snippet;
    class?: string;
  }

  let { icon, title, subtitle, actions, class: sinif }: Props = $props();
</script>

<header class={['panel-basligi', sinif]}>
  {#if icon}<span class="ikon-yeri"><Icon name={icon} size={20} /></span>{/if}
  <div class="metinler">
    <h2 class="baslik">{title}</h2>
    {#if subtitle}<span class="alt-baslik">{subtitle}</span>{/if}
  </div>
  {#if actions}<div class="eylemler">{@render actions()}</div>{/if}
</header>

<style>
  .panel-basligi {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    height: var(--titlebar-height);
    flex: 0 0 auto;
    padding: 0 var(--space-8);
    background: var(--bg-panel);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .ikon-yeri {
    display: inline-flex;
    color: var(--text-muted);
  }
  .metinler {
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: var(--space-5);
  }
  .baslik {
    margin: 0;
    font: var(--type-title);
    color: var(--text-title);
    letter-spacing: var(--tracking-tight);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .alt-baslik {
    font: var(--type-meta);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .eylemler {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>
