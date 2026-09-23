<!--
  Kilitli görünüm (1c tasarımı §7.4): şartlar reddedildi. Anahtar saklı kalır;
  metin yeniden gösterilebilir.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import { m } from '../dil/tr';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum }: { durum: SartDurum } = $props();
</script>

<div class="kilitli">
  <IconTile icon="lock" tone="warn" size={40} />
  <h1>{m('kilitli.baslik')}</h1>
  <p>{m('kilitli.metin')}</p>
  {#if durum.hata}
    <p class="hata" role="alert">{durum.hata}</p>
  {/if}
  <Button variant="primary" size="lg" loading={durum.mesgul} onclick={() => durum.metniGoster()}>
    {m('kilitli.goster')}
  </Button>
</div>

<style>
  .kilitli {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font: var(--type-display);
    color: var(--text-title);
  }
  p {
    margin: 0;
    max-width: 62ch;
    font: var(--type-body);
    color: var(--text-secondary);
  }
  .hata {
    font: var(--type-small);
    color: var(--status-danger);
  }
</style>
