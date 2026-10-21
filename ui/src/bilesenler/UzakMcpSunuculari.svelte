<script lang="ts">
  import {
    m,
    UZAK_MCP_DURUMU_ADI,
    UZAK_MCP_ROZET_ACIKLAMASI,
    UZAK_MCP_ROZETI,
    UZAK_MCP_YETKI_ADI,
  } from '../dil/tr';
  import type {
    UzakMcpSunucuAyrintisi,
    UzakMcpSunucuFormu,
    UzakMcpSunucuSatiri,
  } from '../ipc/tipler';
  import UzakMcpFormu from './UzakMcpFormu.svelte';

  interface Props {
    sunucular: UzakMcpSunucuSatiri[];
    ayrintilar?: Record<string, UzakMcpSunucuAyrintisi>;
    onKaydet: (eskiAd: string | null, form: UzakMcpSunucuFormu) => Promise<boolean | void> | boolean | void;
    onSil: (ad: string) => Promise<boolean | void> | boolean | void;
    onEtkinDegistir: (ad: string, etkin: boolean) => Promise<void> | void;
    onBaglan: (ad: string) => Promise<void> | void;
    onOAuthBagla?: (ad: string) => Promise<void> | void;
    onKimlikKaldir: (ad: string) => Promise<boolean | void> | boolean | void;
    onAyrintiYukle?: (ad: string) => Promise<void> | void;
  }

  let {
    sunucular,
    ayrintilar = {},
    onKaydet,
    onSil,
    onEtkinDegistir,
    onBaglan,
    onOAuthBagla,
    onKimlikKaldir,
    onAyrintiYukle,
  }: Props = $props();

  let yeniFormAcik = $state(false);
  let duzenlenenAd = $state<string | null>(null);
  let acikAyrintiAd = $state<string | null>(null);
  let geriBildirim = $state<string | null>(null);

  async function kaydetIsle(eskiAd: string | null, form: UzakMcpSunucuFormu) {
    const sonuc = await onKaydet(eskiAd, form);
    if (sonuc === false) {
      geriBildirim = m('mcp.reddedildi');
      return;
    }
    yeniFormAcik = false;
    duzenlenenAd = null;
    geriBildirim = m('mcp.kaydedildi', { ad: form.ad });
  }

  async function silIsle(ad: string) {
    const sonuc = await onSil(ad);
    if (sonuc === false) {
      geriBildirim = m('mcp.reddedildi');
      return;
    }
    geriBildirim = m('mcp.silindi', { ad });
  }

  async function kimlikKaldirIsle(ad: string) {
    const sonuc = await onKimlikKaldir(ad);
    if (sonuc === false) {
      geriBildirim = m('mcp.reddedildi');
    }
  }

  async function ayrintiDegistir(ad: string) {
    if (acikAyrintiAd === ad) {
      acikAyrintiAd = null;
      return;
    }
    acikAyrintiAd = ad;
    if (onAyrintiYukle) {
      await onAyrintiYukle(ad);
    }
  }
</script>

<section class="uzak-mcp-bolumu" aria-label={m('uzak_mcp.baslik')}>
  <header class="uzak-mcp-ust">
    <div>
      <h3>{m('uzak_mcp.baslik')}</h3>
      <p>{m('uzak_mcp.aciklama')}</p>
    </div>
    <button
      type="button"
      onclick={() => {
        yeniFormAcik = true;
        duzenlenenAd = null;
      }}
    >
      {m('uzak_mcp.ekle')}
    </button>
  </header>

  {#if geriBildirim}
    <p class="uzak-mcp-geri-bildirim" role="status">{geriBildirim}</p>
  {/if}

  {#if yeniFormAcik}
    <UzakMcpFormu
      onKaydet={kaydetIsle}
      onVazgec={() => {
        yeniFormAcik = false;
      }}
    />
  {/if}

  {#if sunucular.length === 0}
    <p class="uzak-mcp-bos">{m('uzak_mcp.bos')}</p>
  {:else}
    <ul class="uzak-mcp-listesi" aria-label={m('uzak_mcp.liste')}>
      {#each sunucular as satir (satir.ozet.ad)}
        {@const ad = satir.ozet.ad}
        {@const ayrinti = ayrintilar[ad]}
        <li class="uzak-mcp-satiri" data-sunucu={ad}>
          <div class="uzak-mcp-satir-ust">
            <strong class="uzak-mcp-ad">{ad}</strong>
            <span
              class="uzak-mcp-rozet"
              data-rozet={satir.rozet}
              title={m(UZAK_MCP_ROZET_ACIKLAMASI[satir.rozet])}
            >
              {m(UZAK_MCP_ROZETI[satir.rozet])}
            </span>
            <span class="uzak-mcp-durum" data-durum={satir.durum}>
              {m(UZAK_MCP_DURUMU_ADI[satir.durum])}
            </span>
            <span class="uzak-mcp-yetki">
              {m(UZAK_MCP_YETKI_ADI[satir.ozet.yetki_turu])}
            </span>
            {#if satir.ozet.yetki_turu !== 'yetkisiz'}
              <span class="uzak-mcp-kimlik-durumu">
                {satir.ozet.kimlik_bagli ? m('uzak_mcp.kimlik_bagli') : m('uzak_mcp.kimlik_bagli_degil')}
              </span>
            {/if}
          </div>

          <div class="uzak-mcp-satir-orta">
            <code class="uzak-mcp-url">{satir.ozet.uc_url}</code>
            <span class="uzak-mcp-arac-sayisi">
              {m('mcp.arac_sayisi', { sayi: satir.arac_sayisi })}
            </span>
            {#if satir.protokol_surumu}
              <span class="uzak-mcp-protokol">
                {m('uzak_mcp.protokol', { surum: satir.protokol_surumu })}
              </span>
            {/if}
            {#if satir.reddedilen_arac_sayisi > 0}
              <span class="uzak-mcp-reddedilen-sayisi">
                {m('uzak_mcp.reddedilen_sayisi', { sayi: satir.reddedilen_arac_sayisi })}
              </span>
            {/if}
          </div>

          {#if satir.belirsiz_yazma_uyarisi}
            <div class="uzak-mcp-belirsiz-uyari" role="alert">
              <strong>{m('uzak_mcp.belirsiz_uyari_baslik')}</strong>
              <span>{satir.belirsiz_yazma_uyarisi}</span>
            </div>
          {/if}

          {#if satir.son_hata}
            <p class="uzak-mcp-son-hata" role="alert">
              {m('mcp.hata', { mesaj: satir.son_hata })}
            </p>
          {/if}

          <div class="uzak-mcp-eylemler">
            <label class="uzak-mcp-etkin">
              <input
                type="checkbox"
                checked={satir.ozet.etkin}
                onchange={(ev) => onEtkinDegistir(ad, (ev.currentTarget as HTMLInputElement).checked)}
              />
              <span>{m('mcp.etkin')}</span>
            </label>

            <button type="button" onclick={() => onBaglan(ad)}>
              {m('uzak_mcp.baglan')}
            </button>

            {#if satir.ozet.yetki_turu === 'oauth' && onOAuthBagla}
              <button type="button" onclick={() => onOAuthBagla(ad)}>
                {m('uzak_mcp.oauth_bagla')}
              </button>
            {/if}

            {#if satir.ozet.kimlik_bagli}
              <button type="button" onclick={() => kimlikKaldirIsle(ad)}>
                {m('uzak_mcp.kimlik_kaldir')}
              </button>
            {/if}

            <button type="button" onclick={() => ayrintiDegistir(ad)}>
              {acikAyrintiAd === ad ? m('mcp.ayrinti_gizle') : m('mcp.ayrinti')}
            </button>

            <button
              type="button"
              onclick={async () => {
                if (onAyrintiYukle && !ayrintilar[ad]) {
                  await onAyrintiYukle(ad);
                }
                duzenlenenAd = ad;
                yeniFormAcik = false;
              }}
            >
              {m('mcp.duzenle')}
            </button>

            <button type="button" onclick={() => silIsle(ad)}>
              {m('mcp.sil')}
            </button>
          </div>

          {#if duzenlenenAd === ad && ayrinti}
            <UzakMcpFormu
              mevcut={ayrinti.ayrinti}
              onKaydet={kaydetIsle}
              onVazgec={() => {
                duzenlenenAd = null;
              }}
            />
          {/if}

          {#if acikAyrintiAd === ad && ayrinti}
            <div class="uzak-mcp-ayrinti">
              <h4>{m('mcp.ayrinti.araclar')}</h4>
              {#if ayrinti.araclar.length === 0}
                <p>{m('mcp.ayrinti.arac_yok')}</p>
              {:else}
                <ul>
                  {#each ayrinti.araclar as aracAdi (aracAdi)}
                    <li><code>{aracAdi}</code></li>
                  {/each}
                </ul>
              {/if}

              {#if ayrinti.reddedilenler.length > 0}
                <p class="uzak-mcp-reddedilenler">
                  {m('mcp.ayrinti.reddedilenler', { liste: ayrinti.reddedilenler.join(', ') })}
                </p>
              {/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
