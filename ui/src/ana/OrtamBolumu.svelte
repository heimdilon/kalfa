<!--
  Sağ panelin Ortam bölümü (masaüstü arayüzü tasarımı §6.2): çalışma alanı
  ve git dalı, izin kipi, yalıtım, model ve efor; son isteğin bağlam
  kullanımı; bu oturumda değişen dosyalar. Değişen dosyalar ve özetleme
  sayısı bütün kayıttan Rust'ta toplanır (yüklenen geçmiş sayfasından
  bağımsız); yeni tur özeti, geri alma ya da bağlam özeti gelince yenilenir.
  Dosyaya tıklamak Değişiklikler bölümünde açar.
-->
<script lang="ts">
  import { EFOR_ADI, KIP_ADI, m } from '../dil/tr';
  import * as ipc from '../ipc/ana';
  import { komutHatasi } from '../ipc/hata';
  import type { CalismaAlaniOzeti } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { projeAdi, sayi } from './bicim';

  let { durum, ondosya }: { durum: AnaDurum; ondosya: (yol: string) => void } = $props();

  const ekran = $derived(durum.aktif);
  let ozet = $state<CalismaAlaniOzeti | null>(null);
  let hata = $state<string | null>(null);

  /** Toplamları değiştiren canlı öğelerin sayısı; değişince özet yeniden okunur. */
  const degisimler = $derived(
    ekran?.ogeler.filter(
      (o) => o.tip === 'TurOzeti' || o.tip === 'GeriAlma' || o.tip === 'OturumOzeti',
    ).length ?? 0,
  );

  const oturum = $derived(ekran?.oturum);

  // Yalnız oturum ya da toplamı değiştiren öğe sayısı değişince: her akış
  // parçasında bütün kaydı yeniden okutmaz.
  $effect(() => {
    void degisimler;
    if (!oturum) return;
    ipc.calismaAlaniOzeti(oturum).then(
      (o) => {
        ozet = o;
        hata = null;
      },
      (e) => (hata = komutHatasi(e).mesaj),
    );
  });
</script>

{#if ekran}
  {@const yalitim = durum.uygulama?.yalitim}
  <div class="ortam">
    <section>
      <dl>
        <dt>{m('sag.calisma_alani')}</dt>
        <dd>
          <span class="ad">{projeAdi(ekran.calisma_alani)}</span>
          {#if ozet}<span class="yol" title={ekran.calisma_alani}>{ozet.kisa_yol}</span>{/if}
        </dd>
        <dt>{m('sag.git_dali')}</dt>
        <dd>
          {#if hata}
            <span class="soluk">{m('sag.okunamadi', { neden: hata })}</span>
          {:else if ozet?.git.tip === 'Dal'}
            <code>{ozet.git.ad}</code>
          {:else if ozet?.git.tip === 'AyrikHead'}
            {m('sag.git_ayrik')}
          {:else if ozet}
            <span class="soluk">{m('sag.git_yok')}</span>
          {/if}
        </dd>
        {#if ekran.ozet.kip}
          <dt>{m('sag.izin_kipi')}</dt>
          <dd>{m(KIP_ADI[ekran.ozet.kip])}</dd>
        {/if}
        {#if yalitim}
          <dt>{m('sag.yalitim')}</dt>
          <dd>
            {yalitim.tip === 'Hazir'
              ? m('durum.yalitimli_ipucu', { arka_uc: yalitim.arka_uc })
              : durum.uygulama?.yalitimsiz_izinli
                ? m('durum.yalitimsiz')
                : m('durum.komutlar_kapali')}
          </dd>
        {/if}
        {#if ekran.ozet.model}
          <dt>{m('sag.model')}</dt>
          <dd><code>{ekran.ozet.model}</code></dd>
        {/if}
        {#if ekran.ozet.efor}
          <dt>{m('sag.efor')}</dt>
          <dd>{m(EFOR_ADI[ekran.ozet.efor])}</dd>
        {/if}
      </dl>
    </section>

    <section>
      <h3>{m('sag.baglam')}</h3>
      {#if ekran.baglam === null || ekran.baglam.esik <= 0}
        <p class="soluk">{m('sag.baglam_bilinmiyor')}</p>
      {:else}
        <!-- Oturumun yaydığı tahmin ve compact eşiği (etkin pencerenin %90'ı). -->
        {@const { tahmin, esik } = ekran.baglam}
        <progress max={esik} value={Math.min(tahmin, esik)}></progress>
        <p>
          {m('sag.baglam_kullanim', {
            token: sayi(tahmin),
            esik: sayi(esik),
            yuzde: Math.min(100, Math.round((tahmin / esik) * 100)),
          })}
        </p>
      {/if}
      {#if ozet && ozet.ozetlenen > 0}
        <p class="soluk">{m('sag.baglam_ozetleme', { sayi: ozet.ozetlenen })}</p>
      {/if}
    </section>

    <section>
      <h3>{m('sag.ciktilar')}</h3>
      {#if !ozet || ozet.ciktilar.length === 0}
        <p class="soluk">{m('sag.ciktilar_bos')}</p>
      {:else}
        <ul>
          {#each ozet.ciktilar as { yol, eklenen, silinen } (yol)}
            <li>
              <button type="button" onclick={() => ondosya(yol)}>
                <span class="yol-ad">{yol}</span>
                <span class="arti">+{eklenen}</span>
                <span class="eksi">−{silinen}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}

<style>
  .ortam {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  section {
    padding: 12px 16px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h3 {
    margin: 0 0 8px;
    font: var(--type-meta);
    font-weight: var(--fw-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 12px;
    margin: 0;
    font: var(--type-small);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .ad {
    font-weight: var(--fw-semibold);
  }
  .yol {
    display: block;
    font: var(--type-meta);
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  code {
    font: var(--type-code);
  }
  p {
    margin: 4px 0 0;
    font: var(--type-small);
    color: var(--text-body);
  }
  .soluk {
    color: var(--text-muted);
  }
  progress {
    width: 100%;
    height: 6px;
    accent-color: var(--copper-300);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 6px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: var(--type-small);
    text-align: left;
    color: var(--text-secondary);
    cursor: pointer;
  }
  li button:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .yol-ad {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--text-micro);
  }
  .arti {
    color: var(--status-ok);
  }
  .eksi {
    color: var(--status-danger);
  }
</style>
