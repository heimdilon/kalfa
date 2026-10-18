<!--
  Windows'ta ana pencerenin başlık çubuğu (masaüstü arayüzü tasarımı §5).
  Pencere çerçevesizdir: çubuğun boş yeri sürükleme alanıdır, çift tıklama
  büyütür ya da geri yükler. Sağda sağ panelin bölüm düğmeleri (§6.1). Pencere düğmeleri Tauri'nin `core:window`
  izinleri yerine ana pencerenin kendi komutlarını çağırır.
-->
<script lang="ts">
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { m } from '../dil/tr';
  import { pencereBuyutDegistir, pencereKapat, pencereKucult, pencereSurukle } from '../ipc/ana';
  import type { AnaDurum } from './anaDurum.svelte';
  import PanelDugmeleri from './PanelDugmeleri.svelte';
  import { projeAdi, satirBasligi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  // ponytail: yalnız düğmeyle ve çift tıklamayla değişen hâl izlenir; Win+Yukarı
  // ya da kenara sürükleyerek büyütülünce simge eski kalır.
  let buyuk = $state(false);

  const satir = $derived(
    durum.gorunum === 'oturumlar' ? durum.liste.find((s) => s.oturum === durum.secili) : undefined,
  );
  const alan = $derived(durum.aktif?.calisma_alani ?? null);

  async function buyutDegistir() {
    buyuk = (await pencereBuyutDegistir()) ?? false;
  }

  /** Boş yerde sol tuş sürükler, çift tıklama büyütür; düğmeler kendi işini yapar. */
  function bas(e: MouseEvent) {
    if (e.button !== 0 || (e.target as Element).closest('button')) return;
    if (e.detail === 2) void buyutDegistir();
    else void pencereSurukle();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="baslik-cubugu" onmousedown={bas}>
  <div class="sol">
    <button type="button" class="marka" onclick={() => durum.oturumlaraDon()}>
      <span class="amblem" aria-hidden="true"><Icon name="hexagon" size={12} /></span>
      <span>{m('uygulama.adi')}</span>
    </button>
    <IconButton
      icon="panel-left"
      size="sm"
      label={durum.kenarAcik ? m('kenar.gizle') : m('kenar.goster')}
      onclick={() => (durum.kenarAcik = !durum.kenarAcik)}
    />
  </div>

  <div class="orta">
    {#if satir}
      <span class="oturum">{satirBasligi(satir)}</span>
      {#if alan}<span class="proje" title={alan}>{projeAdi(alan)}</span>{/if}
    {/if}
  </div>

  <div class="panel">
    <PanelDugmeleri {durum} />
  </div>

  <div class="pencere">
    <button type="button" aria-label={m('pencere.kucult')} title={m('pencere.kucult')} onclick={pencereKucult}>
      <Icon name="minus" size={15} />
    </button>
    <button
      type="button"
      aria-label={buyuk ? m('pencere.geri_yukle') : m('pencere.buyut')}
      title={buyuk ? m('pencere.geri_yukle') : m('pencere.buyut')}
      onclick={buyutDegistir}
    >
      <Icon name={buyuk ? 'copy' : 'square'} size={13} />
    </button>
    <button
      type="button"
      class="kapat"
      aria-label={m('pencere.kapat')}
      title={m('pencere.kapat')}
      onclick={pencereKapat}
    >
      <Icon name="x" size={16} />
    </button>
  </div>
</div>

<style>
  .baslik-cubugu {
    flex: 0 0 36px;
    display: flex;
    align-items: stretch;
    height: 36px;
    background: var(--bg-rail);
    border-bottom: var(--border-width) solid var(--border-subtle);
    user-select: none;
  }
  .sol,
  .orta {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .sol {
    padding: 0 6px 0 10px;
  }
  .orta {
    flex: 1;
    justify-content: center;
    font: var(--type-small);
  }
  .marka {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 3px 6px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-body);
    cursor: pointer;
  }
  .marka:hover {
    background: var(--surface-raised);
  }
  .amblem {
    display: inline-flex;
    color: var(--text-accent);
  }
  .oturum {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-body);
  }
  .proje {
    flex: none;
    color: var(--text-muted);
  }
  .proje::before {
    content: '·';
    margin-right: 6px;
  }
  .panel {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 8px;
    border-right: var(--border-width) solid var(--border-subtle);
    margin: 6px 0;
  }
  .pencere {
    display: flex;
  }
  .pencere button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 46px;
    padding: 0;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: default;
  }
  .pencere button:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .pencere .kapat:hover {
    background: var(--red-400);
    color: var(--text-body);
  }
  .pencere button:focus-visible {
    outline-offset: -2px;
  }
</style>
