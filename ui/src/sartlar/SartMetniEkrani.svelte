<!--
  EVREN'in şart metni (1c tasarımı §7.4). Başlıkta sürüm ve değişikliğin esaslı
  olup olmadığı yazar. Metin özetlenmeden, güvenli Markdown ile gösterilir;
  görseller yüklenmez. Onay kutusu yoktur: yalnızca iki düğme.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import { m } from '../dil/tr';
  import type { SartMetni } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum, metin }: { durum: SartDurum; metin: SartMetni } = $props();

  const kimlik = $props.id();
</script>

<header class="ust">
  <h1 id="{kimlik}-baslik">{m('sart.baslik')}</h1>
  <div class="rozetler">
    <Badge tone="accent">{m('sart.surum', { surum: metin.surum })}</Badge>
    {#if metin.esasli}
      <Badge tone="warn">{m('sart.esasli')}</Badge>
    {:else}
      <Badge>{m('sart.esasli_degil')}</Badge>
    {/if}
  </div>
</header>
<p class="aciklama">{m('sart.aciklama')}</p>

{#if durum.not}
  <p class="not" role="status">{durum.not}</p>
{/if}
{#if durum.depo === 'Bellekte'}
  <p class="uyari" role="status">{m('hata.anahtar_bellekte')}</p>
{/if}

<!-- Kaydırılabilir metin klavyeyle de kaydırılabilsin diye odaklanabilir. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="metin" tabindex="0" role="region" aria-labelledby="{kimlik}-baslik">
  <Markdown metin={metin.metin} belge baglantiAc={durum.baglantiAc} />
</div>

{#if durum.hata}
  <p class="hata" role="alert">{durum.hata}</p>
{/if}

<div class="dugmeler">
  <Button size="lg" disabled={durum.mesgul} onclick={() => durum.reddet()}>
    {m('sart.reddet')}
  </Button>
  <Button
    size="lg"
    variant="primary"
    loading={durum.mesgul}
    disabled={durum.mesgul}
    onclick={() => durum.kabul(metin.surum)}
  >
    {m('sart.kabul')}
  </Button>
</div>

<style>
  .ust {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: var(--type-display);
    color: var(--text-title);
  }
  .rozetler {
    display: flex;
    gap: 8px;
  }
  .aciklama {
    margin: 0 0 20px;
    font: var(--type-subtitle);
    color: var(--text-secondary);
  }
  .not,
  .uyari {
    margin: 0 0 16px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font: var(--type-small);
  }
  /* Zemin durum renginden türetilir: tasarım sistemindeki --amber-tint ve
     --blue-tint yalnızca koyu temada tanımlı, bu pencere açık temada. */
  .not {
    color: var(--status-info);
    background: color-mix(in srgb, var(--status-info) 12%, transparent);
  }
  .uyari {
    color: var(--status-warn);
    background: color-mix(in srgb, var(--status-warn) 14%, transparent);
  }
  .metin {
    max-height: 52vh;
    overflow-y: auto;
    padding: 20px 4px 20px 0;
    border-top: var(--border-width) solid var(--border-subtle);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .metin:focus-visible {
    outline: none;
    box-shadow: var(--shadow-focus);
  }
  .hata {
    margin: 16px 0 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .dugmeler {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 22px;
  }
</style>
