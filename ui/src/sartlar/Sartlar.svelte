<!--
  Şart penceresi (1c tasarımı §7): anahtar formu, EVREN'in şart metni ya da
  kilitli görünüm. Açık temayla açılır (tasarım sistemindeki OnboardingScreen
  görsel referanstır; içeriği EVREN'in metnidir).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { m } from '../dil/tr';
  import AnahtarFormu from './AnahtarFormu.svelte';
  import KilitliEkran from './KilitliEkran.svelte';
  import SartMetniEkrani from './SartMetniEkrani.svelte';
  import { SartDurum } from './sartDurum.svelte';

  let { durum = new SartDurum() }: { durum?: SartDurum } = $props();

  onMount(() => {
    void durum.yukle();
  });
</script>

<main class="sartlar">
  <div class="kolon">
    <header class="marka">
      <div class="ad">{m('uygulama.adi')}</div>
      <div class="slogan">{m('uygulama.slogan')}</div>
    </header>

    <div class="kart">
      {#if durum.durum?.tip === 'AnahtarGerekli'}
        <AnahtarFormu {durum} neden={durum.durum.neden} />
      {:else if durum.durum?.tip === 'SartGerekli'}
        <SartMetniEkrani {durum} metin={durum.durum.metin} />
      {:else if durum.durum?.tip === 'Kilitli'}
        <KilitliEkran {durum} />
      {:else if durum.durum?.tip === 'Tamam'}
        <p class="tamam" role="status">{m('sartlar.tamam')}</p>
      {:else if durum.hata}
        <p class="hata" role="alert">{durum.hata}</p>
      {/if}
    </div>

    <p class="alt-not">{m('sartlar.alt_not')}</p>
  </div>
</main>

<style>
  .sartlar {
    height: 100%;
    overflow-y: auto;
    display: flex;
    justify-content: center;
    padding: 44px 28px;
    box-sizing: border-box;
    background: var(--bg-app);
    color: var(--text-body);
  }
  .kolon {
    width: min(760px, 100%);
  }
  .marka {
    text-align: center;
    margin-bottom: 30px;
  }
  .ad {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
  .slogan {
    margin-top: 6px;
    font: var(--type-tagline);
    color: var(--text-muted);
  }
  .kart {
    padding: 32px 36px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-paper);
  }
  .tamam {
    margin: 0;
    font: var(--type-body);
    color: var(--text-secondary);
  }
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .alt-not {
    margin-top: 22px;
    text-align: center;
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
