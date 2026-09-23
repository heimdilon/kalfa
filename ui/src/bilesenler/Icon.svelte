<!-- Lucide ikonu. Kalfa kendi glifini çizmez (tasarım sistemi: Icon). -->
<script lang="ts">
  import { IKONLAR, type IkonAdi } from './ikonlar';

  interface Props {
    name: IkonAdi;
    /** 14 satır içi, 16 varsayılan, 18 menü, 20 başlık. */
    size?: number;
    strokeWidth?: number;
    /** Erişilebilir ad; süs ikonlarında verilmez. */
    title?: string;
    class?: string;
  }

  let { name, size = 16, strokeWidth = 1.75, title, class: sinif = '' }: Props = $props();

  const Bilesen = $derived(IKONLAR[name]);
  const siniflar = $derived(sinif ? `ikon ${sinif}` : 'ikon');
</script>

{#if title}
  <Bilesen {size} {strokeWidth} class={siniflar} role="img" aria-label={title} />
{:else}
  <Bilesen {size} {strokeWidth} class={siniflar} aria-hidden="true" />
{/if}
