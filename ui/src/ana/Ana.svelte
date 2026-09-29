<!--
  Ana pencere (1c tasarımı §8.1): menü, oturum listesi, konuşma ve en altta
  durum çubuğu. Onay sayfası konuşmanın üstünde açılır. Menüden Ayarlar
  seçilince liste ve konuşmanın yerine Ayarlar gelir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { AnaDurum } from './anaDurum.svelte';
  import Ayarlar from './Ayarlar.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Konusma from './Konusma.svelte';
  import GeriAlmaSayfasi from './GeriAlmaSayfasi.svelte';
  import Menu from './Menu.svelte';
  import OnaySayfasi from './OnaySayfasi.svelte';
  import OturumListesi from './OturumListesi.svelte';

  let { durum = new AnaDurum() }: { durum?: AnaDurum } = $props();

  onMount(() => {
    void durum.baslat();
  });

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

<div class="ana">
  <div class="govde">
    <Menu {durum} />
    {#if durum.gorunum === 'ayarlar'}
      <Ayarlar {durum} />
    {:else}
      <OturumListesi {durum} />
      <Konusma {durum} />
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
  .govde {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
