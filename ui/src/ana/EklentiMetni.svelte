<!--
  "Metni göster" (aşama 4 tasarımı §9.4): önizlemede bekleyen eklenti isteğinin
  EVREN'e gidecek gövdesi. Kart metnin kendisini taşımaz; metin tıklanınca
  host'tan istenir, ikinci tıkta gizlenir. Metin eklentinin ürettiğidir: görünmez
  karakterler kaçışlanarak, kaydırılabilir eş aralıklı bir alanda gösterilir.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import { m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';

  let { yukle }: { yukle: () => Promise<string> } = $props();

  let metin = $state<string | null>(null);
  let hata = $state<string | null>(null);
  let acik = $state(false);
  let bekliyor = $state(false);

  async function degistir(): Promise<void> {
    if (acik) {
      acik = false;
      return;
    }
    if (metin === null) {
      bekliyor = true;
      hata = null;
      try {
        metin = await yukle();
      } catch (e) {
        hata = komutHatasi(e).mesaj;
        return;
      } finally {
        bekliyor = false;
      }
    }
    acik = true;
  }
</script>

<div class="metin">
  <div>
    <Button
      variant="ghost"
      size="sm"
      iconLeft="file-text"
      loading={bekliyor}
      aria-expanded={acik}
      onclick={degistir}
    >
      {acik ? m('eklenti.metin_gizle') : m('eklenti.metin_goster')}
    </Button>
  </div>
  {#if hata}
    <p class="hata" role="alert">{hata}</p>
  {/if}
  {#if acik && metin !== null}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre class="govde" role="region" aria-label={m('eklenti.metin_baslik')} tabindex="0"><GorunurMetin
        {metin}
      /></pre>
  {/if}
</div>

<style>
  .metin {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .govde {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px 12px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font: var(--text-small) / 1.5 var(--font-mono);
    color: var(--text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
</style>
