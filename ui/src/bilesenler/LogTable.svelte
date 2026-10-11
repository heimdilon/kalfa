<!--
  Gönderim günlüğü tablosu (tasarım §6.5): zaman, amaç, yönlendirilen model,
  token, istek kimliği ve sonucu listeler. Seçilen satırın ayrıntı paneli açılır.
-->
<script lang="ts">
  import Badge from './Badge.svelte';
  import { m } from '../dil/tr';
  import type { GunlukSatiri } from '../ipc/tipler';
  import { saat, sayi } from '../ana/bicim';

  interface Props {
    satirlar: GunlukSatiri[];
    seciliSira?: number | null;
    onsec?: (satir: GunlukSatiri) => void;
    class?: string;
  }

  let {
    satirlar,
    seciliSira = null,
    onsec,
    class: sinif,
  }: Props = $props();

  function amacMetni(amac: string): string {
    if (amac === 'ana') return m('gunluk.amac.ana');
    if (amac === 'denetci') return m('gunluk.amac.denetci');
    if (amac.startsWith('eklenti:')) {
      return m('eklenti.amac', { ad: amac.slice('eklenti:'.length) });
    }
    return amac;
  }

  function tuslaSec(e: KeyboardEvent, satir: GunlukSatiri) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onsec?.(satir);
    }
  }
</script>

<div class={['tablo-kapsayici', sinif]}>
  <table class="gunluk-tablosu">
    <thead>
      <tr>
        <th>{m('gunluk.kolon.zaman')}</th>
        <th>{m('gunluk.kolon.amac')}</th>
        <th>{m('gunluk.kolon.model')}</th>
        <th>{m('gunluk.kolon.token')}</th>
        <th>{m('gunluk.kolon.istek_id')}</th>
        <th>{m('gunluk.kolon.sonuc')}</th>
      </tr>
    </thead>
    <tbody>
      {#each satirlar as satir (satir.sira)}
        {@const secili = seciliSira === satir.sira}
        <tr
          class={['satir', { secili }]}
          tabindex="0"
          role="button"
          onclick={() => onsec?.(satir)}
          onkeydown={(e) => tuslaSec(e, satir)}
        >
          <td class="zaman">{satir.zaman ? saat(satir.zaman) : '-'}</td>
          <td class="amac">{amacMetni(satir.amac)}</td>
          <td class="model">{satir.yonlendirilen_model ?? satir.model ?? '-'}</td>
          <td class="token">
            {satir.token ? sayi(satir.token.istem + satir.token.tamamlama) : '-'}
          </td>
          <td class="istek-id">{satir.istek_id ?? '-'}</td>
          <td class="sonuc" title={satir.sonuc.tip === 'Hata' ? satir.sonuc.mesaj : undefined}>
            {#if satir.sonuc.tip === 'Basarili'}
              <Badge tone="ok">{m('gunluk.sonuc.basarili')}</Badge>
            {:else if satir.sonuc.tip === 'Gonderilmedi'}
              <Badge tone="neutral">
                {m('gunluk.sonuc.gonderilmedi', { neden: satir.sonuc.neden })}
              </Badge>
            {:else}
              <Badge tone="danger">
                {m('gunluk.sonuc.hata', { mesaj: satir.sonuc.asama === 'Baglanti' ? 'Bağlantı' : 'Yanıt' })}
              </Badge>
            {/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .tablo-kapsayici {
    width: 100%;
    overflow-x: auto;
  }
  .gunluk-tablosu {
    width: 100%;
    border-collapse: collapse;
    font: var(--type-small);
    text-align: left;
  }
  th {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    font-weight: var(--fw-semibold);
    color: var(--text-secondary);
    background: var(--bg-surface-elevated, var(--bg-surface));
    position: sticky;
    top: 0;
    z-index: 1;
  }
  td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-body);
  }
  .satir {
    cursor: pointer;
    transition: var(--transition-control);
  }
  .satir:hover {
    background: var(--bg-hover);
  }
  .satir:focus-visible {
    outline: 2px solid var(--border-focus);
    outline-offset: -2px;
  }
  .satir.secili {
    background: var(--bg-selected, var(--bg-hover));
  }
  .model,
  .istek-id {
    font-family: var(--font-mono);
  }
  .token {
    font-variant-numeric: tabular-nums;
  }
</style>
