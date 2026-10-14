<!--
  Menü (1c tasarımı §8.1): kenar çubuğunun altındaki gezinme. Eklentiler
  (aşama 4 tasarımı §14), Gönderim günlüğü, "Anahtarı değiştir" ve Ayarlar
  (aşama 2 tasarımı §10.4). Ayarlar ya da Eklentiler açıkken en üstte
  "Oturumlar" konuşmaya döndürür.
-->
<script lang="ts">
  import Icon from '../bilesenler/Icon.svelte';
  import type { IkonAdi } from '../bilesenler/ikonlar';
  import { m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();
</script>

{#snippet oge(ikon: IkonAdi, etiket: string, etkin: boolean, onclick: () => void)}
  <button
    type="button"
    class={['oge', { etkin }]}
    aria-label={etiket}
    aria-current={etkin ? 'page' : undefined}
    {onclick}
  >
    <Icon name={ikon} size={16} />
    <span>{etiket}</span>
  </button>
{/snippet}

<nav class="menu" aria-label={m('menu.ad')}>
  {#if durum.gorunum !== 'oturumlar'}
    {@render oge('message-square', m('menu.oturumlar'), false, () => durum.oturumlaraDon())}
  {/if}
  {@render oge('puzzle', m('menu.eklentiler'), durum.gorunum === 'eklentiler', () =>
    durum.eklentileriAc(),
  )}
  {#if durum.secili}
    {@render oge('send', m('menu.gunluk'), durum.gunlukAcik, () => durum.gunluguAc())}
  {/if}
  {@render oge('key-round', m('menu.anahtar'), false, () => durum.anahtariDegistir())}
  {@render oge('settings', m('menu.ayarlar'), durum.gorunum === 'ayarlar', () => durum.ayarlariAc())}
</nav>

<style>
  .menu {
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 8px;
    border-top: var(--border-width) solid var(--border-subtle);
  }
  .oge {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font: var(--type-small);
    text-align: left;
    cursor: pointer;
    transition: var(--transition-control);
  }
  .oge:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .oge.etkin {
    background: var(--surface-selected);
    color: var(--text-title);
  }
</style>
