<!--
  Ayarlar (aşama 2 tasarımı §10.4): yalıtımın durumu ve talimatı, algılamayı
  yeniden çalıştırma; Linux'ta yalıtıma açılan araç zinciri dizinleri. Dizin
  yerel klasör seçiciyle eklenir; arayüz yol göndermez. Windows'un düğmeleri
  aşama 2d'de gelir.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { YALITIM_NEDENI, m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  const yalitim = $derived(durum.uygulama?.yalitim ?? null);
  const linux = $derived(durum.uygulama?.platform === 'linux');
</script>

<section class="ayarlar" aria-label={m('ayarlar.baslik')}>
  {#if durum.hata}
    <div class="serit" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}
  <header class="ust">
    <h2>{m('ayarlar.baslik')}</h2>
  </header>
  <div class="govde">
    <Card title={m('ayarlar.yalitim')} icon="shield">
      {#snippet actions()}
        {#if linux}
          <Button
            size="sm"
            iconLeft="rotate-ccw"
            loading={durum.algilaniyor}
            onclick={() => durum.yalitimiAlgila()}
          >
            {m('ayarlar.yalitim.yeniden_dene')}
          </Button>
        {/if}
      {/snippet}
      {#if yalitim?.tip === 'Hazir'}
        <p class="durum ok">
          <Icon name="shield-check" size={16} />
          {m('ayarlar.yalitim.hazir', { arka_uc: yalitim.arka_uc, surum: yalitim.surum })}
        </p>
        <p class="aciklama">{m('ayarlar.yalitim.hazir_aciklama')}</p>
      {:else if yalitim}
        <p class="durum uyari">
          <Icon name="shield-alert" size={16} />
          {m('ayarlar.yalitim.yok', { neden: m(YALITIM_NEDENI[yalitim.neden]) })}
        </p>
        <p class="aciklama">{yalitim.talimat}</p>
      {/if}
    </Card>

    {#if linux}
      <Card title={m('ayarlar.arac_zinciri')} icon="folder">
        {#snippet actions()}
          <Button size="sm" iconLeft="folder-plus" onclick={() => durum.aracZinciriEkle()}>
            {m('ayarlar.arac_zinciri.ekle')}
          </Button>
        {/snippet}
        <p class="aciklama">{m('ayarlar.arac_zinciri.aciklama')}</p>
        {#if durum.aracZinciri.length > 0}
          <ul class="dizinler" aria-label={m('ayarlar.arac_zinciri')}>
            {#each durum.aracZinciri as yol (yol)}
              <li>
                <span class="yol">{yol}</span>
                <IconButton
                  icon="x"
                  size="sm"
                  label={m('ayarlar.arac_zinciri.kaldir', { yol })}
                  onclick={() => durum.aracZinciriKaldir(yol)}
                />
              </li>
            {/each}
          </ul>
        {:else}
          <p class="bos">{m('ayarlar.arac_zinciri.bos')}</p>
        {/if}
      </Card>
    {/if}
  </div>
</section>

<style>
  .ayarlar {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  .serit {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    font: var(--type-small);
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 14%, transparent);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  .ust {
    padding: 14px 22px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 20px / 1.2 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
  }
  .govde {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px 22px;
    max-width: 760px;
  }
  .durum {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
  }
  .ok {
    color: var(--status-ok);
  }
  .uyari {
    color: var(--status-warn);
  }
  .aciklama,
  .bos {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    margin-top: 10px;
    color: var(--text-muted);
  }
  .dizinler {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .yol {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--type-meta);
    color: var(--text-body);
  }
</style>
