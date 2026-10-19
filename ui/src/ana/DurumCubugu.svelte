<!--
  Durum çubuğu (1c tasarımı §8.3): oturumun hâli, bağlantı ve kimlik deposu
  uyarıları; son gözlenen bakiye ve zamanı, token kullanımı (düşünce dökümüyle),
  yönlendirilen model, izin kipi. Bağlam doluluğu gösterilmez. Yalıtım
  göstergesine tıklanınca Ayarlar açılır (aşama 2 tasarımı §10.3). EVREN'in
  günlük token sınırı dolduysa "Günlük sınır doldu" ve sıfırlanma saati
  görünür (aşama 4 tasarımı §9.5).
-->
<script lang="ts">
  import StatusBar from '../bilesenler/StatusBar.svelte';
  import StatusItem from '../bilesenler/StatusItem.svelte';
  import { DENETCI_KIPI_ADI, KIP_ADI, m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';
  import { gunlukSinirMetni, kredi, saat, sayi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  const ekran = $derived(durum.aktif);
  const ozet = $derived(ekran?.ozet ?? null);
  const butce = $derived(ekran?.butce ?? null);
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
    {#if durum.uygulama}
      {@const yalitim = durum.uygulama.yalitim}
      <button type="button" class="yalitim" onclick={() => durum.ayarlariAc()}>
        {#if yalitim.tip === 'Hazir'}
          <StatusItem
            icon="shield-check"
            tone="ok"
            title={m('durum.yalitimli_ipucu', { arka_uc: yalitim.arka_uc })}
          >
            {m('durum.yalitimli')}
          </StatusItem>
        {:else if durum.uygulama.yalitimsiz_izinli}
          <StatusItem icon="shield-alert" tone="warn" title={m('durum.yalitimsiz_ipucu')}>
            {m('durum.yalitimsiz')}
          </StatusItem>
        {:else}
          <StatusItem icon="shield-alert" tone="warn" title={m('durum.komutlar_kapali_ipucu')}>
            {m('durum.komutlar_kapali')}
          </StatusItem>
        {/if}
      </button>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
      <StatusItem icon="key-round" tone="warn" title={m('hata.anahtar_bellekte')}>
        {m('durum.anahtar_bellekte')}
      </StatusItem>
    {/if}
    {#if durum.gunlukSinir}
      <StatusItem icon="triangle-alert" tone="warn" title={m('durum.gunluk_sinir_ipucu')}>
        {gunlukSinirMetni(durum.gunlukSinir)}
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
      {#if butce}
        <StatusItem icon="coins">
          {m('durum.istek_token', {
            istek: sayi(butce.baslatilan),
            token: sayi(butce.bilinen_token),
          })}
        </StatusItem>
        {#if butce.bilinmeyen > 0}
          <StatusItem icon="triangle-alert" tone="warn">
            {m('durum.butce_bilinmeyen', { sayi: sayi(butce.bilinmeyen) })}
          </StatusItem>
        {/if}
      {/if}
      {#if ekran?.baglam}
        {@const baglam = ekran.baglam}
        {@const yuzde = baglam.esik > 0 ? Math.round((baglam.tahmin / baglam.esik) * 100) : 0}
        <StatusItem
          tone={yuzde > 80 ? 'warn' : undefined}
          title={m('durum.baglam_ipucu', {
            tahmin: sayi(baglam.tahmin),
            esik: sayi(baglam.esik),
          })}
        >
          {m('durum.baglam', { yuzde })}
        </StatusItem>
      {/if}
      {#if ozet.yonlendirilen_model}
        <StatusItem icon="cpu" title={m('durum.model_ipucu')}>{ozet.yonlendirilen_model}</StatusItem>
      {/if}
      {#if ozet.kip}
        <StatusItem icon="shield" tone="accent">{m(KIP_ADI[ozet.kip])}</StatusItem>
      {/if}
    {/if}
    {#if durum.denetci}
      <StatusItem
        icon={durum.denetci.ayar_hatasi ? 'triangle-alert' : 'shield-check'}
        tone={durum.denetci.ayar_hatasi ? 'warn' : undefined}
        title={durum.denetci.ayar_hatasi ?? m('durum.denetci_ipucu')}
      >
        {m('durum.denetci', { kip: m(DENETCI_KIPI_ADI[durum.denetci.kip]) })}
      </StatusItem>
    {/if}
  {/snippet}
</StatusBar>

<style>
  .yalitim {
    display: inline-flex;
    padding: 0;
    border: 0;
    background: none;
    font: inherit;
    cursor: pointer;
  }
  .yalitim:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-sm);
  }
</style>
