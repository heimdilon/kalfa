<!--
  Menü (1c tasarımı §8.1): Oturumlar, Eklentiler (aşama 4 tasarımı §14), Ayarlar
  (aşama 2 tasarımı §10.4) ve "Anahtarı değiştir".
-->
<script lang="ts">
  import Icon from '../bilesenler/Icon.svelte';
  import { m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();
</script>

<nav class="menu codex-serit" aria-label={m('menu.ad')}>
  <div class="ust" title={`${m('uygulama.adi')} — ${m('uygulama.slogan')}`}>
    <span class="amblem" aria-hidden="true">
      <Icon name="hexagon" size={20} />
    </span>
    <span class="gizli-erisilebilir">{m('uygulama.adi')}</span>
  </div>

  <div class="liste">
    <button
      type="button"
      class={['serit-dugme', { etkin: durum.gorunum === 'oturumlar' }]}
      aria-current={durum.gorunum === 'oturumlar' ? 'page' : undefined}
      title={m('menu.oturumlar')}
      onclick={() => durum.oturumlaraDon()}
    >
      <Icon name="message-square" size={18} />
      <span class="gizli-erisilebilir">{m('menu.oturumlar')}</span>
    </button>

    <button
      type="button"
      class={['serit-dugme', { etkin: durum.gorunum === 'eklentiler' }]}
      aria-current={durum.gorunum === 'eklentiler' ? 'page' : undefined}
      title={m('menu.eklentiler')}
      onclick={() => durum.eklentileriAc()}
    >
      <Icon name="puzzle" size={18} />
      <span class="gizli-erisilebilir">{m('menu.eklentiler')}</span>
    </button>

    {#if durum.secili}
      <button
        type="button"
        class={['serit-dugme', { etkin: durum.gunlukAcik }]}
        aria-current={durum.gunlukAcik ? 'page' : undefined}
        title={m('menu.gunluk')}
        onclick={() => durum.gunluguAc()}
      >
        <Icon name="send" size={18} />
        <span class="gizli-erisilebilir">{m('menu.gunluk')}</span>
      </button>
    {/if}
  </div>

  <div class="alt">
    <button
      type="button"
      class="serit-dugme"
      title={m('menu.anahtar')}
      onclick={() => durum.anahtariDegistir()}
    >
      <Icon name="key-round" size={18} />
      <span class="gizli-erisilebilir">{m('menu.anahtar')}</span>
    </button>

    <button
      type="button"
      class={['serit-dugme', { etkin: durum.gorunum === 'ayarlar' }]}
      aria-current={durum.gorunum === 'ayarlar' ? 'page' : undefined}
      title={m('menu.ayarlar')}
      onclick={() => durum.ayarlariAc()}
    >
      <Icon name="settings" size={18} />
      <span class="gizli-erisilebilir">{m('menu.ayarlar')}</span>
    </button>
  </div>
</nav>

<style>
  .codex-serit {
    display: flex;
    flex-direction: column;
    align-items: center;
    width: 48px;
    flex: 0 0 48px;
    height: 100%;
    padding: 10px 0;
    box-sizing: border-box;
    background: var(--bg-rail);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .ust {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    margin-bottom: 8px;
    border-radius: var(--radius-md);
    color: var(--text-accent);
    background: color-mix(in srgb, var(--copper-400) 14%, transparent);
  }
  .amblem {
    display: inline-flex;
  }
  .liste {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 0;
  }
  .alt {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding-top: 8px;
    border-top: var(--border-width) solid var(--border-subtle);
  }
  .serit-dugme {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    padding: 0;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .serit-dugme:hover {
    color: var(--text-body);
    background: var(--surface-raised);
  }
  .serit-dugme.etkin {
    color: var(--text-accent);
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
</style>
