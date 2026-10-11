<!--
  Eklenti isteğinin ayrıntısı (aşama 4 tasarımı §9.3, §9.4): uç, model, metin
  sayısı, boyut ve okunan dosyalar. Önizleme kartı ve gönderim günlüğü aynı
  görünümü kullanır; günlük ayrıca özeti gösterir.
-->
<script lang="ts">
  import MetaList, { type MetaOgesi } from '../bilesenler/MetaList.svelte';
  import { EKLENTI_UC_ADI, m } from '../dil/tr';
  import type { EklentiIstekOzeti } from '../ipc/tipler';
  import { sayi } from './bicim';
  import { boyutMetni } from './geriAlma';

  let {
    ozet,
    ozetGoster = false,
  }: { ozet: EklentiIstekOzeti; ozetGoster?: boolean } = $props();

  const satirlar: MetaOgesi[] = $derived([
    { label: m('eklenti.uc'), value: m(EKLENTI_UC_ADI[ozet.uc]) },
    { label: m('eklenti.model'), value: ozet.model, mono: true },
    { label: m('eklenti.metin_sayisi'), value: sayi(ozet.metin_sayisi) },
    { label: m('eklenti.boyut'), value: boyutMetni(ozet.bayt) },
    ...(ozetGoster ? [{ label: m('eklenti.ozet'), value: ozet.ozet, mono: true }] : []),
  ]);
</script>

<div class="eklenti-ayrinti">
  <MetaList items={satirlar} dense />
  <div class="dosyalar">
    <span class="baslik">{m('eklenti.dosyalar')}</span>
    {#if ozet.dosyalar.length > 0}
      <ul class="liste">
        {#each ozet.dosyalar as dosya, i (i)}
          <li class="yol">{dosya}</li>
        {/each}
      </ul>
      {#if ozet.dosya_fazlasi > 0}
        <span class="fazla">{m('eklenti.dosya_fazlasi', { sayi: sayi(ozet.dosya_fazlasi) })}</span>
      {/if}
    {:else}
      <span class="fazla">{m('eklenti.dosya_yok')}</span>
    {/if}
  </div>
</div>

<style>
  .eklenti-ayrinti {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  .dosyalar {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .baslik {
    font: var(--fw-semibold) var(--type-meta);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .liste {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 200px;
    overflow-y: auto;
  }
  .yol {
    font: var(--type-meta);
    font-family: var(--font-mono);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .fazla {
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
