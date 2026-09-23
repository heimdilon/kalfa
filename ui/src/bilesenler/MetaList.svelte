<script lang="ts" module>
  export interface MetaOgesi {
    label: string;
    value: string;
    /** Değer tek aralıklı: yol, boyut, kimlik, zaman. */
    mono?: boolean;
    tone?: 'default' | 'accent';
  }
</script>

<script lang="ts">
  interface Props {
    items: MetaOgesi[];
    labelWidth?: number;
    dense?: boolean;
    class?: string;
  }

  let { items, labelWidth = 112, dense = false, class: sinif }: Props = $props();
</script>

<dl class={['meta', { sik: dense }, sinif]} style:grid-template-columns={`${labelWidth}px 1fr`}>
  {#each items as oge, i (i)}
    <dt>{oge.label}</dt>
    <dd class={{ mono: oge.mono, vurgulu: oge.tone === 'accent' }}>{oge.value}</dd>
  {/each}
</dl>

<style>
  .meta {
    display: grid;
    row-gap: 10px;
    column-gap: 12px;
    margin: 0;
  }
  .sik {
    row-gap: 6px;
  }
  dt {
    font: var(--type-small);
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    font: var(--type-small);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .mono {
    font: var(--type-meta);
  }
  .vurgulu {
    color: var(--text-accent);
  }
</style>
