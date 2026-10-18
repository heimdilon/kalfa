<!--
  Ana pencere (1c tasarımı §8.1): Windows'ta en üstte başlık çubuğu (masaüstü
  arayüzü §5), solda kenar çubuğu (oturumlar ve menü),
  ortada konuşma, istenirse sağda inceleme bölmesi; en altta durum çubuğu.
  Onay sayfası konuşmanın üstünde açılır. Menüden Ayarlar ya da Eklentiler
  seçilince konuşmanın yerine o ekran gelir. Ctrl+N yeni oturum açar, Ctrl+K
  oturumlarda arar, Ctrl+B kenar çubuğunu gizler.
-->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { m } from '../dil/tr';
  import { AnaDurum, yerelOku, yerelYaz } from './anaDurum.svelte';
  import Ayarlar from './Ayarlar.svelte';
  import BaslikCubugu from './BaslikCubugu.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Eklentiler from './Eklentiler.svelte';
  import GeriAlmaSayfasi from './GeriAlmaSayfasi.svelte';
  import GonderimGunlugu from './GonderimGunlugu.svelte';
  import Konusma from './Konusma.svelte';
  import Menu from './Menu.svelte';
  import OnaySayfasi from './OnaySayfasi.svelte';
  import OturumListesi from './OturumListesi.svelte';
  import SagPanel from './SagPanel.svelte';

  let { durum = new AnaDurum() }: { durum?: AnaDurum } = $props();

  onMount(() => {
    void durum.baslat();
  });

  // Sağ panel: %28'den %50'ye (yarı yarıya) kadar çekilebilir (2'şer puan adımlarla).
  // Genişlik bu makinede hatırlanır (masaüstü arayüzü tasarımı §6.1).
  const SAG_ORAN_ANAHTARI = 'kalfa.sagOran';
  let sagOran = $state(Math.min(50, Math.max(28, Number(yerelOku(SAG_ORAN_ANAHTARI)) || 38)));
  let suruklenen = $state(false);
  let govdeKutu = $state<HTMLDivElement>();

  $effect(() => yerelYaz(SAG_ORAN_ANAHTARI, String(Math.round(sagOran))));

  function sagOranSinifi(oran: number): string {
    const sinirli = Math.min(50, Math.max(28, Math.round(oran / 2) * 2));
    return `sag-oran-${sinirli}`;
  }

  function surukleBaslat(e: PointerEvent) {
    e.preventDefault();
    (e.currentTarget as HTMLElement | null)?.setPointerCapture?.(e.pointerId);
    suruklenen = true;
  }

  function surukleHareket(e: PointerEvent) {
    if (!suruklenen || !govdeKutu) return;
    const dikdortgen = govdeKutu.getBoundingClientRect();
    if (dikdortgen.width > 0) {
      const sagGenislik = dikdortgen.right - e.clientX;
      const yuzde = (sagGenislik / dikdortgen.width) * 100;
      sagOran = Math.min(50, Math.max(28, yuzde));
    }
  }

  function surukleBitir() {
    if (suruklenen) suruklenen = false;
  }

  function tutamacTus(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      sagOran = Math.min(50, sagOran + 2);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      sagOran = Math.max(28, sagOran - 2);
    }
  }

  /** Ctrl+N, Ctrl+K ve Ctrl+B (macOS'ta Cmd). Açık bir pencere varken çalışmaz. */
  async function kisayol(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey) return;
    if (durum.onay || durum.gunlukOturum || durum.aktif?.geri_alma) return;
    const tus = e.key.toLocaleLowerCase('tr-TR');
    if (tus === 'n') {
      e.preventDefault();
      await durum.yeniOturum();
    } else if (tus === 'b') {
      e.preventDefault();
      durum.kenarAcik = !durum.kenarAcik;
    } else if (tus === 'k') {
      e.preventDefault();
      durum.kenarAcik = true;
      await tick();
      govdeKutu?.querySelector<HTMLInputElement>('.kenar input[type="search"]')?.focus();
    }
  }

  const onayIstegi = $derived.by(() => {
    const acik = durum.onay;
    if (!acik) return null;
    const oge = durum.ekranlar
      .get(acik.oturum)
      ?.ogeler.find((o) => o.tip === 'OnayIstegi' && o.islem === acik.islem);
    return oge?.tip === 'OnayIstegi' && oge.durum === 'Bekliyor' ? oge : null;
  });

  /** Seçili oturumun açık geri alma planı ve turun ilk mesajı. */
  const geriAlma = $derived.by(() => {
    const ekran = durum.aktif;
    const onizleme = ekran?.geri_alma;
    if (!ekran || !onizleme) return null;
    const ilk = ekran.ogeler.find((o) => o.tip === 'KullaniciMesaji' && o.tur === onizleme.tur);
    return {
      oturum: ekran.oturum,
      onizleme,
      baslik: ilk?.tip === 'KullaniciMesaji' ? ilk.metin : null,
    };
  });
</script>

<svelte:window
  onkeydown={kisayol}
  onpointermove={surukleHareket}
  onpointerup={surukleBitir}
  onpointercancel={surukleBitir}
  onblur={surukleBitir}
/>

<div class={['ana', { surukleniyor: suruklenen }]}>
  {#if durum.ozelCerceve}
    <BaslikCubugu {durum} />
  {/if}
  <div bind:this={govdeKutu} class="govde">
    {#if durum.kenarAcik}
      <div class="kenar">
        <OturumListesi {durum} />
        <Menu {durum} />
      </div>
    {/if}
    {#if durum.gorunum === 'ayarlar'}
      <Ayarlar {durum} />
    {:else if durum.gorunum === 'eklentiler'}
      <Eklentiler {durum} />
    {:else}
      <Konusma {durum} />

      {#if durum.gorunenSagBolum && durum.aktif}
        {@const bolum = durum.gorunenSagBolum}
        <button
          type="button"
          aria-label={m('konusma.bolme_boyutlandir')}
          class={['bolme-tutamaci', { etkin: suruklenen }]}
          onpointerdown={surukleBaslat}
          onlostpointercapture={surukleBitir}
          onkeydown={tutamacTus}
        ></button>

        <aside
          class={['sag-bolme', sagOranSinifi(sagOran)]}
          aria-label={m(`sag.${bolum}`)}
        >
          {#key durum.aktif.oturum}
            <SagPanel {durum} {bolum} />
          {/key}
        </aside>
      {/if}
    {/if}
  </div>
  <DurumCubugu {durum} />
  {#if durum.onay && onayIstegi}
    {@const { oturum, islem } = durum.onay}
    <OnaySayfasi
      ogeler={onayIstegi.ogeler}
      onkarar={(kararlar) => durum.onayla(oturum, islem, kararlar)}
      onkapat={() => durum.onayiKapat()}
    />
  {/if}
  {#if geriAlma && !durum.onay}
    {@const { oturum, onizleme, baslik } = geriAlma}
    <GeriAlmaSayfasi
      {onizleme}
      {baslik}
      gonderiliyor={durum.geriAliniyor}
      ongeri={() => durum.turuGeriAl(oturum)}
      onkapat={() => durum.geriAlmayiKapat(oturum)}
    />
  {/if}
  {#if durum.gunlukOturum}
    <GonderimGunlugu
      oturum={durum.gunlukOturum}
      istekSayaci={durum.ekranlar.get(durum.gunlukOturum)?.istek_sayaci ?? 0}
      onkapat={() => durum.gunluguKapat()}
    />
  {/if}
  <div class="gizli-erisilebilir" aria-live="polite">{durum.duyuru}</div>
</div>

<style>
  .ana {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-app);
  }
  .ana.surukleniyor {
    user-select: none;
    cursor: col-resize;
  }
  .govde {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .kenar {
    flex: 0 0 256px;
    width: 256px;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-rail);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .bolme-tutamaci {
    position: relative;
    width: 5px;
    flex: 0 0 5px;
    padding: 0;
    border: none;
    cursor: col-resize;
    background: linear-gradient(var(--border-subtle), var(--border-subtle)) center / 1px 100% no-repeat;
    transition: background-color var(--dur-fast) var(--ease-standard);
    z-index: 15;
  }
  .bolme-tutamaci::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 3px;
    height: 28px;
    border-radius: var(--radius-pill);
    background: transparent;
    transition: background 120ms ease;
  }
  .bolme-tutamaci:hover,
  .bolme-tutamaci.etkin,
  .bolme-tutamaci:focus-visible {
    background-color: color-mix(in srgb, var(--copper-300) 35%, transparent);
    outline: none;
  }
  .bolme-tutamaci:hover::after,
  .bolme-tutamaci.etkin::after,
  .bolme-tutamaci:focus-visible::after {
    background: var(--copper-200);
  }
  .sag-bolme {
    flex: 0 0 auto;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    min-width: 260px;
    max-width: 50%;
    overflow: hidden;
  }
  .sag-oran-28 {
    width: 28%;
  }
  .sag-oran-30 {
    width: 30%;
  }
  .sag-oran-32 {
    width: 32%;
  }
  .sag-oran-34 {
    width: 34%;
  }
  .sag-oran-36 {
    width: 36%;
  }
  .sag-oran-38 {
    width: 38%;
  }
  .sag-oran-40 {
    width: 40%;
  }
  .sag-oran-42 {
    width: 42%;
  }
  .sag-oran-44 {
    width: 44%;
  }
  .sag-oran-46 {
    width: 46%;
  }
  .sag-oran-48 {
    width: 48%;
  }
  .sag-oran-50 {
    width: 50%;
  }
</style>
