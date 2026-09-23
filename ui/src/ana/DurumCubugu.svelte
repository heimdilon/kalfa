<!--
  Durum çubuğu (1c tasarımı §8.3): oturumun hâli, bağlantı ve kimlik deposu
  uyarıları; son gözlenen bakiye ve zamanı, token kullanımı (düşünce dökümüyle),
  yönlendirilen model, izin kipi. Bağlam doluluğu gösterilmez.
-->
<script lang="ts">
  import StatusBar from '../bilesenler/StatusBar.svelte';
  import StatusItem from '../bilesenler/StatusItem.svelte';
  import { KIP_ADI, m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';
  import { kredi, saat, sayi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  const ekran = $derived(durum.aktif);
  const ozet = $derived(ekran?.ozet ?? null);
</script>

<StatusBar label={m('durum.ad')}>
  {#snippet left()}
    {#if ekran?.anlik === 'Calisiyor'}
      <StatusItem icon="loader-circle" tone="warn">{m('durum.calisiyor')}</StatusItem>
    {:else if ekran?.anlik === 'OnayBekliyor'}
      <StatusItem icon="shield" tone="warn">{m('oturum.durum.onay')}</StatusItem>
    {:else if ekran?.anlik === 'SartBekliyor'}
      <StatusItem icon="shield" tone="warn">{m('oturum.durum.sart')}</StatusItem>
    {:else}
      <StatusItem icon="circle-check" tone="ok">{m('durum.hazir')}</StatusItem>
    {/if}
    {#if durum.uygulama?.baglanti === 'Yok'}
      <StatusItem icon="wifi-off" tone="danger">{m('durum.baglanti_yok')}</StatusItem>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
      <StatusItem icon="key-round" tone="warn" title={m('hata.anahtar_bellekte')}>
        {m('durum.anahtar_bellekte')}
      </StatusItem>
    {/if}
  {/snippet}
  {#snippet right()}
    {#if ozet?.son_kredi?.kalan}
      <StatusItem icon="coins" title={m('durum.bakiye_ipucu')}>
        {m('durum.bakiye', {
          kalan: kredi(ozet.son_kredi.kalan),
          saat: saat(ozet.son_kredi.zaman_ms),
        })}
      </StatusItem>
    {/if}
    {#if ozet}
      <StatusItem
        title={m('durum.token_ipucu', {
          istem: sayi(ozet.istem),
          tamamlama: sayi(ozet.tamamlama),
          dusunce: sayi(ozet.dusunce),
        })}
      >
        {m('durum.token', { sayi: sayi(ozet.istem + ozet.tamamlama) })}
      </StatusItem>
      {#if ozet.yonlendirilen_model}
        <StatusItem icon="cpu" title={m('durum.model_ipucu')}>{ozet.yonlendirilen_model}</StatusItem>
      {/if}
      {#if ozet.kip}
        <StatusItem icon="shield" tone="accent">{m(KIP_ADI[ozet.kip])}</StatusItem>
      {/if}
    {/if}
  {/snippet}
</StatusBar>
