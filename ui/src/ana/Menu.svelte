<!--
  Menü (1c tasarımı §8.1): Oturumlar, Eklentiler (aşama 4 tasarımı §14), Ayarlar
  (aşama 2 tasarımı §10.4) ve "Anahtarı değiştir".
-->
<script lang="ts">
  import NavItem from '../bilesenler/NavItem.svelte';
  import NavRail from '../bilesenler/NavRail.svelte';
  import { m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();
</script>

<NavRail label={m('menu.ad')} class="menu">
  {#snippet header()}
    <div class="marka">{m('uygulama.adi')}</div>
    <div class="slogan">{m('uygulama.slogan')}</div>
  {/snippet}
  <NavItem
    icon="message-square"
    active={durum.gorunum === 'oturumlar'}
    onclick={() => durum.oturumlaraDon()}
  >
    {m('menu.oturumlar')}
  </NavItem>
  <NavItem
    icon="puzzle"
    active={durum.gorunum === 'eklentiler'}
    onclick={() => durum.eklentileriAc()}
  >
    {m('menu.eklentiler')}
  </NavItem>
  <NavItem
    icon="settings"
    active={durum.gorunum === 'ayarlar'}
    onclick={() => durum.ayarlariAc()}
  >
    {m('menu.ayarlar')}
  </NavItem>
  <NavItem icon="key-round" onclick={() => durum.anahtariDegistir()}>
    {m('menu.anahtar')}
  </NavItem>
  {#if durum.secili}
    <NavItem
      icon="send"
      active={durum.gunlukAcik}
      onclick={() => durum.gunluguAc()}
    >
      {m('menu.gunluk')}
    </NavItem>
  {/if}
  {#snippet footer()}
    <div class="cizgi"></div>
    <div class="alt-not">{m('uygulama.alt_not')}</div>
  {/snippet}
</NavRail>

<style>
  .marka {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
  .slogan {
    margin-top: 4px;
    font: var(--type-tagline);
    color: var(--text-muted);
  }
  .cizgi {
    width: 34px;
    height: 2px;
    margin-bottom: 10px;
    background: var(--copper-400);
  }
  .alt-not {
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
