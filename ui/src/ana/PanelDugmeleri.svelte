<!--
  Sağ panelin bölüm düğmeleri (masaüstü arayüzü tasarımı §6.1). Windows'ta
  başlık çubuğunda, Linux'ta konuşma başlığında durur. Düğme bölümü açar;
  açık bölümün düğmesi paneli kapatır.
-->
<script lang="ts">
  import IconButton from '../bilesenler/IconButton.svelte';
  import type { IkonAdi } from '../bilesenler/ikonlar';
  import { m, type MetinAnahtari } from '../dil/tr';
  import { SAG_BOLUMLER, type AnaDurum, type SagBolum } from './anaDurum.svelte';

  let {
    durum,
    bolumler = SAG_BOLUMLER,
  }: { durum: AnaDurum; bolumler?: readonly SagBolum[] } = $props();

  const IKON: Record<SagBolum, IkonAdi> = {
    ortam: 'info',
    degisiklikler: 'code',
    dosyalar: 'folder',
  };
  const AD: Record<SagBolum, MetinAnahtari> = {
    ortam: 'sag.ortam',
    degisiklikler: 'sag.degisiklikler',
    dosyalar: 'sag.dosyalar',
  };
</script>

{#each bolumler as b (b)}
  <IconButton
    icon={IKON[b]}
    size="sm"
    label={m(AD[b])}
    active={durum.sagBolum === b}
    disabled={!durum.aktif || (b !== 'degisiklikler' && !!durum.aktif.salt_okunur)}
    onclick={() => durum.sagBolumuDegistir(b)}
  />
{/each}
