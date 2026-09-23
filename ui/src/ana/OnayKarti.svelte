<!--
  Konuşmadaki onay isteği (1c tasarımı §8.2): küçük bir kart. "İncele" onay
  sayfasını açar; sayfa karar verilmeden kapatılırsa istek burada bekler.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import { m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import { kisaAd } from './onay';

  let {
    oge,
    onincele,
  }: { oge: Extract<EkranOgesi, { tip: 'OnayIstegi' }>; onincele: () => void } = $props();

  const uyariSayisi = $derived(oge.ogeler.reduce((t, o) => t + o.uyarilar.length, 0));
</script>

<Card
  tone={oge.durum === 'Bekliyor' ? 'accent' : 'default'}
  icon="shield"
  title={m('onay.kart.baslik', { sayi: oge.ogeler.length })}
>
  {#snippet actions()}
    {#if oge.durum === 'Bekliyor'}
      <Button variant="primary" size="sm" onclick={onincele}>{m('onay.kart.incele')}</Button>
    {:else if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('onay.kart.yanitlandi')}</Badge>
    {:else}
      <Badge>{m('onay.kart.gecersiz')}</Badge>
    {/if}
  {/snippet}
  <ul class="hedefler">
    {#each oge.ogeler as o (o.cagri)}
      <li>
        <span class="ad">{kisaAd(o)}</span>
        <span class="aciklama">{o.aciklama}</span>
      </li>
    {/each}
  </ul>
  {#if uyariSayisi > 0}
    <Badge tone="warn" icon="triangle-alert">{m('onay.kart.uyari', { sayi: uyariSayisi })}</Badge>
  {/if}
</Card>

<style>
  .hedefler {
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    gap: 10px;
    font: var(--type-small);
    min-width: 0;
  }
  .ad {
    font-family: var(--font-mono);
    color: var(--text-body);
    flex: none;
  }
  .aciklama {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
