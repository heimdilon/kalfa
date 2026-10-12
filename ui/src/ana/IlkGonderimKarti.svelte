<!--
  Eklentinin bir çalışma alanındaki ilk EVREN gönderimi (aşama 4 tasarımı
  §8.4, §14): hangi eklenti, hangi uçlara, kaç dosyadan veri gönderebilecek ve
  neden izin istiyor. Açıklama onay sayfasındaki "Ne" satırıyla aynı güvenli
  yolla gösterilir (görünmez karakterler kaçışlanır). Karar düğmeleri onay
  sayfasındadır; kalıcı izin yerel pencerede ayrıca sorulur.
-->
<script lang="ts">
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import MetaList, { type MetaOgesi } from '../bilesenler/MetaList.svelte';
  import { evrenUcuAdi, m } from '../dil/tr';
  import type { OnayOgesi } from '../ipc/tipler';
  import { mbMetni, sayi } from './bicim';

  let { oge }: { oge: OnayOgesi } = $props();

  const tarif = $derived(oge.eklenti);
  const satirlar: MetaOgesi[] = $derived(
    tarif
      ? [
          { label: m('ilk_gonderim.eklenti'), value: `${tarif.ad} ${tarif.surum}`, mono: true },
          ...(tarif.evren_uclari.length > 0
            ? [{ label: m('ilk_gonderim.uclar'), value: tarif.evren_uclari.map(evrenUcuAdi).join(', ') }]
            : []),
          {
            label: m('ilk_gonderim.dosyalar'),
            value: m('ilk_gonderim.dosya_degeri', {
              sayi: sayi(tarif.dosya_sayisi),
              boyut: mbMetni(tarif.bayt),
            }),
          },
        ]
      : [],
  );
</script>

{#if tarif}
  <section class="ilk-gonderim" aria-label={m('ilk_gonderim.baslik', { ad: tarif.ad })}>
    <h3>{m('ilk_gonderim.baslik', { ad: tarif.ad })}</h3>
    <MetaList items={satirlar} labelWidth={150} dense />
    <h4>{m('ilk_gonderim.neden')}</h4>
    <p class="neden"><GorunurMetin metin={oge.aciklama} /></p>
    {#if oge.yerel_onay}
      <p class="yerel"><Icon name="shield-alert" size={16} />{m('ilk_gonderim.yerel')}</p>
    {/if}
  </section>
{/if}

<style>
  .ilk-gonderim {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
  }
  h3,
  h4 {
    margin: 0;
    color: var(--text-title);
  }
  h3 {
    font: var(--fw-semibold) var(--text-small) / 1.3 var(--font-sans);
  }
  h4 {
    font: var(--fw-semibold) var(--type-meta);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .neden {
    margin: 0;
    max-height: 240px;
    overflow-y: auto;
    font: var(--type-small);
    color: var(--text-body);
    /* Açıklamanın paragrafları korunur; fazladan boşluk birleşir. */
    white-space: pre-line;
    overflow-wrap: anywhere;
  }
  .yerel {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    font: var(--type-small);
    color: var(--status-warn);
  }
</style>
