<!--
  Gönderim günlüğü ekranı (tasarım §6.5): açık oturumun EVREN isteklerini,
  kaynak etiketlerini ve token kullanımını gösterir. Sayfalıdır; yeni bir
  istek bilgisi gelince son sayfasını yeniden okur.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import LogTable from '../bilesenler/LogTable.svelte';
  import SourceTag from '../bilesenler/SourceTag.svelte';
  import { m } from '../dil/tr';
  import * as ipc from '../ipc/ana';
  import EklentiAyrinti from './EklentiAyrinti.svelte';
  import type { GunlukSatiri, GunlukSayfasi, OturumId } from '../ipc/tipler';

  let {
    oturum,
    istekSayaci = 0,
    onkapat,
  }: {
    oturum: OturumId;
    istekSayaci?: number;
    onkapat: () => void;
  } = $props();

  let sayfa = $state<GunlukSayfasi | null>(null);
  let yukleniyor = $state(false);
  let dahaEskiYukleniyor = $state(false);
  let seciliSira = $state<number | null>(null);
  let seciliMetin = $state<{ sira: number; metin: string } | null>(null);
  let metinYukleniyor = $state(false);
  let pencere = $state<HTMLDivElement>();

  const seciliSatir = $derived(
    sayfa?.satirlar.find((s) => s.sira === seciliSira) ?? sayfa?.satirlar[0] ?? null,
  );

  onMount(() => {
    const onceki = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pencere?.focus();
    void yukle();
    return () => onceki?.focus();
  });

  // İstekBilgisi gelince son sayfayı yeniden oku
  $effect(() => {
    void istekSayaci;
    void yukle();
  });

  async function yukle() {
    yukleniyor = true;
    try {
      const yeni = await ipc.gonderimGunlugu(oturum, null);
      sayfa = yeni;
      if (seciliSira === null && yeni.satirlar.length > 0) {
        seciliSira = yeni.satirlar[0].sira;
      }
    } catch {
      // Hata durumunda var olan sayfa korunur
    } finally {
      yukleniyor = false;
    }
  }

  async function dahaEskiYukle() {
    if (!sayfa || dahaEskiYukleniyor) return;
    const enEski = sayfa.satirlar.at(-1)?.sira;
    if (enEski === undefined) return;
    dahaEskiYukleniyor = true;
    try {
      const eski = await ipc.gonderimGunlugu(oturum, enEski);
      sayfa = {
        satirlar: [...sayfa.satirlar, ...eski.satirlar],
        daha_eski_var: eski.daha_eski_var,
      };
    } finally {
      dahaEskiYukleniyor = false;
    }
  }

  function satirSec(satir: GunlukSatiri) {
    seciliSira = satir.sira;
    seciliMetin = null;
  }

  async function metinAc(sira: number) {
    if (seciliMetin?.sira === sira) return;
    metinYukleniyor = true;
    try {
      const metin = await ipc.gonderimOgesi(oturum, sira);
      seciliMetin = { sira, metin };
    } catch {
      seciliMetin = null;
    } finally {
      metinYukleniyor = false;
    }
  }

  function tus(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onkapat();
    }
  }
</script>

<div
  bind:this={pencere}
  class="arka-perde"
  role="dialog"
  aria-modal="true"
  aria-label={m('gunluk.baslik')}
  tabindex="-1"
  onkeydown={tus}
>
  <div class="pencere">
    <header class="ust">
      <h2>{m('gunluk.baslik')}</h2>
      <IconButton icon="x" label={m('genel.kapat')} onclick={onkapat} />
    </header>

    <div class="govde">
      <div class="sol-kolon">
        {#if sayfa && sayfa.satirlar.length > 0}
          <LogTable
            satirlar={sayfa.satirlar}
            seciliSira={seciliSatir?.sira ?? null}
            onsec={satirSec}
          />
          {#if sayfa.daha_eski_var}
            <div class="daha-eski-alani">
              <Button
                variant="ghost"
                size="sm"
                iconLeft="chevron-down"
                disabled={dahaEskiYukleniyor}
                onclick={dahaEskiYukle}
              >
                {m('gunluk.daha_eski')}
              </Button>
            </div>
          {/if}
        {:else if yukleniyor}
          <p class="durum-mesaji">{m('gunluk.metin_yukleniyor')}</p>
        {:else}
          <p class="durum-mesaji">{m('gunluk.ayrinti_yok')}</p>
        {/if}
      </div>

      <div class="sag-kolon">
        {#if seciliSatir}
          <div class="ayrinti-paneli">
            {#if seciliSatir.yeni.length === 0 && seciliSatir.onceki === 0 && !seciliSatir.eklenti}
              <p class="ayrinti-yok">{m('gunluk.ayrinti_yok')}</p>
            {:else}
              <div class="ogeler-alani">
                {#each seciliSatir.yeni as o (o.kaynak + (o.sira ?? ''))}
                  <SourceTag
                    kaynak={o.kaynak}
                    bayt={o.bayt}
                    sira={o.sira}
                    secili={seciliMetin?.sira === o.sira}
                    onclick={o.sira != null ? () => metinAc(o.sira!) : undefined}
                  />
                {/each}
                {#if seciliSatir.onceki > 0}
                  <span class="onceki-mesaj">
                    {m('gunluk.onceki_mesaj', { sayi: seciliSatir.onceki })}
                  </span>
                {/if}
              </div>

              {#if seciliSatir.eklenti}
                <EklentiAyrinti ozet={seciliSatir.eklenti} ozetGoster />
                <p class="metin-kayitsiz">{m('eklenti.metin_kayitsiz')}</p>
              {/if}

              {#if metinYukleniyor}
                <p class="durum-mesaji">{m('gunluk.metin_yukleniyor')}</p>
              {:else if seciliMetin}
                <div class="metin-onizleme">
                  <div class="metin-baslik">{m('gunluk.metin_baslik')}</div>
                  <CodeBlock code={seciliMetin.metin} maxHeight="280px" />
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .arka-perde {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--scrim);
    padding: 24px;
  }
  .pencere {
    display: flex;
    flex-direction: column;
    width: 900px;
    max-width: 100%;
    height: 600px;
    max-height: 100%;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-modal);
    overflow: hidden;
  }
  .ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface-elevated, var(--bg-surface));
  }
  .ust h2 {
    margin: 0;
    font: var(--fw-semibold) var(--text-base) / 1.2 var(--font-sans);
    color: var(--text-title);
  }
  .govde {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .sol-kolon {
    flex: 1.2;
    overflow-y: auto;
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
  }
  .sag-kolon {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  .ayrinti-paneli {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .ogeler-alani {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .onceki-mesaj {
    font: var(--type-small);
    color: var(--text-muted);
    font-style: italic;
  }
  .ayrinti-yok {
    font: var(--type-small);
    color: var(--text-muted);
    margin: 0;
  }
  .durum-mesaji {
    padding: 20px;
    text-align: center;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .daha-eski-alani {
    padding: 10px;
    display: flex;
    justify-content: center;
  }
  .metin-onizleme {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .metin-baslik {
    font: var(--fw-semibold) var(--type-meta);
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .metin-kayitsiz {
    margin: 0;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
