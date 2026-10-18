<!--
  Sağ panelin Dosyalar bölümü (masaüstü arayüzü tasarımı §6.3): çalışma
  alanının salt okur ağacı. Arayüz yol göndermez; dizinler ve dosyalar
  Rust'ın verdiği oturum kimlikleriyle istenir. Dizinler kapalı gelir ve
  açılınca yüklenir; bağlantılar açılmaz. Dosyaya tıklamak önizler; gizli,
  ikili ve başka adı olan dosyanın içeriği gelmez.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';
  import Icon from '../bilesenler/Icon.svelte';
  import { m } from '../dil/tr';
  import * as ipc from '../ipc/ana';
  import { komutHatasi } from '../ipc/hata';
  import type { DizinIcerigi, DosyaGirdisi, DosyaIcerigi, OturumId } from '../ipc/tipler';

  let { oturum }: { oturum: OturumId } = $props();

  /** Yüklenen dizinler; anahtar dizinin kimliği, kök için `null`. */
  const dizinler = new SvelteMap<number | null, DizinIcerigi>();
  const aciklar = new SvelteSet<number>();
  let secili = $state<DosyaGirdisi | null>(null);
  let icerik = $state<DosyaIcerigi | null>(null);
  let hata = $state<string | null>(null);

  async function yukle(dizin: number | null) {
    try {
      dizinler.set(dizin, await ipc.calismaAlaniDizini(oturum, dizin));
      hata = null;
    } catch (e) {
      hata = komutHatasi(e).mesaj;
    }
  }

  onMount(() => void yukle(null));

  async function dizinDegistir(g: DosyaGirdisi) {
    if (aciklar.has(g.kimlik)) {
      aciklar.delete(g.kimlik);
      return;
    }
    aciklar.add(g.kimlik);
    if (!dizinler.has(g.kimlik)) await yukle(g.kimlik);
  }

  /** Yavaş bir önceki okuma sonradan dönerse yok sayılır: yalnız seçili dosya gösterilir. */
  async function dosyaSec(g: DosyaGirdisi) {
    secili = g;
    icerik = null;
    hata = null;
    try {
      const yeni = await ipc.calismaAlaniDosyasi(oturum, g.kimlik);
      if (secili?.kimlik === g.kimlik) icerik = yeni;
    } catch (e) {
      if (secili?.kimlik === g.kimlik) hata = komutHatasi(e).mesaj;
    }
  }
</script>

{#snippet agac(dizin: number | null, derinlik: number)}
  {@const icerigi = dizinler.get(dizin)}
  {#if icerigi}
    <ul role={derinlik === 0 ? 'tree' : 'group'} aria-label={derinlik === 0 ? m('sag.dosyalar') : undefined}>
      {#each icerigi.girdiler as g (g.kimlik)}
        <li role="treeitem" aria-selected={secili?.kimlik === g.kimlik} aria-expanded={g.tur === 'Dizin' ? aciklar.has(g.kimlik) : undefined}>
          {#if g.tur === 'Dizin'}
            <button type="button" class={`d-${derinlik}`} onclick={() => dizinDegistir(g)}>
              <Icon name={aciklar.has(g.kimlik) ? 'chevron-down' : 'chevron-right'} size={13} />
              <Icon name="folder" size={14} />
              <span class="ad">{g.ad}</span>
            </button>
            {#if aciklar.has(g.kimlik)}
              {@render agac(g.kimlik, derinlik + 1)}
            {/if}
          {:else if g.tur === 'Dosya'}
            <button
              type="button"
              class={[`d-${derinlik}`, 'dosya', { secili: secili?.kimlik === g.kimlik }]}
              onclick={() => dosyaSec(g)}
            >
              <Icon name="file-text" size={14} />
              <span class="ad">{g.ad}</span>
            </button>
          {:else}
            <span class={[`d-${derinlik}`, 'baglanti']}>
              <Icon name="external-link" size={13} />
              <span class="ad">{g.ad}</span>
              <span class="not">{m('sag.baglanti')}</span>
            </span>
          {/if}
        </li>
      {/each}
    </ul>
    {#if icerigi.kirpildi}
      <p class="not">{m('sag.dizin_kirpildi')}</p>
    {/if}
  {/if}
{/snippet}

<div class="dosyalar">
  <div class="agac">
    {@render agac(null, 0)}
  </div>
  <div class="onizleme" aria-live="polite">
    {#if hata}
      <p class="not">{m('sag.okunamadi', { neden: hata })}</p>
    {:else if !secili}
      <p class="not">{m('sag.dosya_sec')}</p>
    {:else}
      <div class="baslik">{secili.ad}</div>
      {#if icerik?.tip === 'Metin'}
        {#if icerik.kirpildi}<p class="not">{m('sag.kirpildi')}</p>{/if}
        <pre>{icerik.metin}</pre>
      {:else if icerik?.tip === 'Ikili'}
        <p class="not">{m('sag.ikili')}</p>
      {:else if icerik?.tip === 'Gizli'}
        <p class="not">{m('sag.gizli')}</p>
      {:else if icerik?.tip === 'CokAdli'}
        <p class="not">{m('sag.cok_adli')}</p>
      {/if}
    {/if}
  </div>
</div>

<style>
  .dosyalar {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .agac {
    flex: 0 1 45%;
    min-height: 80px;
    overflow: auto;
    padding: 6px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li > button,
  li > span {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    padding: 3px 6px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: var(--type-small);
    text-align: left;
    color: var(--text-secondary);
    white-space: nowrap;
  }
  li > button {
    cursor: pointer;
  }
  li > button:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  li > button.secili {
    background: var(--surface-selected);
    color: var(--text-title);
  }
  /* Satır içi stil yasak (CSP): derinlik sınıfla girintilenir. */
  .d-1 { padding-left: 22px; }
  .d-2 { padding-left: 38px; }
  .d-3 { padding-left: 54px; }
  .d-4 { padding-left: 70px; }
  .d-5 { padding-left: 86px; }
  .d-6 { padding-left: 102px; }
  .d-7 { padding-left: 118px; }
  .dosya {
    padding-left: 24px;
  }
  .dosya.d-1 { padding-left: 40px; }
  .dosya.d-2 { padding-left: 56px; }
  .dosya.d-3 { padding-left: 72px; }
  .dosya.d-4 { padding-left: 88px; }
  .dosya.d-5 { padding-left: 104px; }
  .dosya.d-6 { padding-left: 120px; }
  .dosya.d-7 { padding-left: 136px; }
  .ad {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .baglanti {
    color: var(--text-muted);
  }
  .not {
    margin: 4px 6px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .onizleme {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 8px 12px;
  }
  .baslik {
    margin-bottom: 6px;
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    font-family: var(--font-mono);
    color: var(--text-body);
  }
  pre {
    margin: 0;
    font: var(--type-code);
    color: var(--text-body);
    white-space: pre;
  }
</style>
