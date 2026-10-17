<!--
  Ayarlar'daki MCP sunucuları bölümü (1.1a tasarımı §9). WASM
  eklentilerinden ayrıdır. Her sunucu: ad, erişim rozeti ve açıklaması,
  etkinlik, çalışma alanı başına durum, araç sayısı, son hata, başlatma izni
  ve geri alma, yeniden başlatma; ayrıntı, düzenleme ve silme. Görüntü
  kanaldan gelir (`durum.mcp`); eski revizyon yenisini ezmez. Kayıt, silme
  ve tam erişime geçiş Rust yerel pencerede sorulur.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import { MCP_DURUMU_ADI, MCP_ROZET_ACIKLAMASI, MCP_ROZETI, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { McpDurumEtiketi, McpSunucuAyari, OturumId } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import McpAyrinti from './McpAyrinti.svelte';
  import McpFormu from './McpFormu.svelte';
  import McpIceAktarma from './McpIceAktarma.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  const DURUM_TONU: Record<McpDurumEtiketi, 'neutral' | 'info' | 'ok' | 'danger'> = {
    durdu: 'neutral',
    basliyor: 'info',
    calisiyor: 'ok',
    bosta_kapandi: 'neutral',
    hata: 'danger',
  };

  const sunucular = $derived(durum.mcp?.sunucular ?? []);
  /** Açık panel: yeni/düzenleme formu ya da içe aktarma; aynı anda biri. */
  let panel = $state<
    { tur: 'form'; baslangic: McpSunucuAyari | null } | { tur: 'ice' } | null
  >(null);
  let ayrintiAcik = $state<Record<string, boolean>>({});
  let hatalar = $state<Record<string, string>>({});
  let mesgul = $state(false);

  onMount(() => {
    void durum.mcpYenile();
  });

  /** Açık oturumlardan alanın görünen yolu; yoksa kısa kimlik. */
  function alanAdi(kimlik: string): string {
    for (const e of durum.ekranlar.values()) {
      if (e.calisma_alani_kimligi === kimlik && e.calisma_alani) return e.calisma_alani;
    }
    return m('mcp.alan_kimligi', { kimlik: kimlik.slice(0, 8) });
  }

  /** Alanda yazılabilir açık oturum (yeniden başlatma oturumla gider). */
  function alanOturumu(kimlik: string): OturumId | null {
    for (const e of durum.ekranlar.values()) {
      if (e.calisma_alani_kimligi === kimlik && !e.salt_okunur && !e.kapandi) return e.oturum;
    }
    return null;
  }

  async function calistir(ad: string, is: () => Promise<unknown>): Promise<void> {
    if (mesgul) return;
    mesgul = true;
    delete hatalar[ad];
    try {
      await is();
    } catch (e) {
      hatalar[ad] = komutHatasi(e).mesaj;
    } finally {
      mesgul = false;
    }
  }

  function duzenle(ad: string): Promise<void> {
    return calistir(ad, async () => {
      const a = await ipc.mcpSunucuAyrintisi(ad);
      panel = { tur: 'form', baslangic: a.ayrinti };
    });
  }

  function sil(ad: string): Promise<void> {
    return calistir(ad, async () => {
      durum.duyuru = (await ipc.mcpSil(ad))
        ? m('mcp.silindi', { ad })
        : m('mcp.reddedildi');
    });
  }
</script>

<Card title={m('mcp.baslik')} icon="box">
  {#snippet actions()}
    <Button
      size="sm"
      iconLeft="plus"
      disabled={panel !== null}
      onclick={() => (panel = { tur: 'form', baslangic: null })}
    >
      {m('mcp.ekle')}
    </Button>
    <Button size="sm" disabled={panel !== null} onclick={() => (panel = { tur: 'ice' })}>
      {m('mcp.ice_aktar')}
    </Button>
  {/snippet}
  <p class="aciklama">{m('mcp.aciklama')}</p>

  {#if panel?.tur === 'form'}
    <div class="panel">
      <McpFormu
        baslangic={panel.baslangic}
        oturum={durum.secili}
        onkapat={() => (panel = null)}
        onkaydedildi={(ad) => {
          panel = null;
          durum.duyuru = m('mcp.kaydedildi', { ad });
        }}
      />
    </div>
  {:else if panel?.tur === 'ice'}
    <div class="panel">
      <McpIceAktarma
        oturum={durum.secili}
        onkapat={() => (panel = null)}
        onkaydedildi={() => (panel = null)}
      />
    </div>
  {/if}

  {#if sunucular.length === 0}
    <p class="bos">{m('mcp.bos')}</p>
  {:else}
    <ul class="liste" aria-label={m('mcp.liste')}>
      {#each sunucular as s (s.ozet.ad)}
        {@const ad = s.ozet.ad}
        {@const tam = s.ozet.kip === 'tam_erisim'}
        <li>
          <div class="sunucu" role="group" aria-label={gorunurMetin(ad)}>
            <div class="ust">
              <strong class="ad"><GorunurMetin metin={ad} /></strong>
              <Badge tone={tam ? 'danger' : 'accent'}>{m(MCP_ROZETI[s.ozet.kip])}</Badge>
              <span class="bosluk"></span>
              <Checkbox
                checked={s.ozet.etkin}
                disabled={mesgul}
                label={m('mcp.etkin')}
                onchange={(v) => void calistir(ad, () => ipc.mcpEtkinAyarla(ad, v))}
              />
            </div>
            <p class={['rozet-aciklama', { tehlike: tam }]}>
              {m(MCP_ROZET_ACIKLAMASI[s.ozet.kip])}
            </p>

            {#if s.alanlar.length === 0}
              <p class="bos">{m('mcp.alan_yok')}</p>
            {:else}
              <ul class="alanlar" aria-label={m('mcp.alanlar')}>
                {#each s.alanlar as a (a.alan_kimligi)}
                  {@const oturum = alanOturumu(a.alan_kimligi)}
                  <li>
                    <div class="alan-ust">
                      <span class="yol" title={gorunurMetin(alanAdi(a.alan_kimligi))}>
                        <GorunurMetin metin={alanAdi(a.alan_kimligi)} />
                      </span>
                      <Badge tone={DURUM_TONU[a.durum]}>{m(MCP_DURUMU_ADI[a.durum])}</Badge>
                      <span class="sayi">{m('mcp.arac_sayisi', { sayi: a.arac_sayisi })}</span>
                    </div>
                    {#if a.hata}
                      <p class="hata"><GorunurMetin metin={m('mcp.hata', { mesaj: a.hata })} /></p>
                    {/if}
                    <div class="alan-alt">
                      <span class="izin">
                        {a.izin === 'tam_erisim'
                          ? m('mcp.izin.tam_erisim')
                          : a.izin === 'yalitimli'
                            ? m('mcp.izin.yalitimli')
                            : m('mcp.izin.yok')}
                      </span>
                      {#if a.izin}
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={mesgul}
                          onclick={() =>
                            void calistir(ad, () =>
                              ipc.mcpBaslatmaIzniGeriAl(a.alan_kimligi, ad),
                            )}
                        >
                          {m('mcp.izni_geri_al')}
                        </Button>
                      {/if}
                      <Button
                        size="sm"
                        variant="ghost"
                        iconLeft="rotate-ccw"
                        disabled={mesgul || oturum === null}
                        title={oturum === null ? m('mcp.yeniden_baslat_oturum_yok') : undefined}
                        onclick={() => {
                          if (oturum) void calistir(ad, () => ipc.mcpYenidenBaslat(oturum, ad));
                        }}
                      >
                        {m('mcp.yeniden_baslat')}
                      </Button>
                    </div>
                  </li>
                {/each}
              </ul>
            {/if}

            {#if hatalar[ad]}
              <p class="hata" role="alert"><GorunurMetin metin={hatalar[ad]} /></p>
            {/if}
            <div class="dugmeler">
              <Button
                size="sm"
                variant="ghost"
                aria-expanded={ayrintiAcik[ad] ?? false}
                onclick={() => (ayrintiAcik[ad] = !ayrintiAcik[ad])}
              >
                {ayrintiAcik[ad] ? m('mcp.ayrinti_gizle') : m('mcp.ayrinti')}
              </Button>
              <Button
                size="sm"
                iconLeft="square-pen"
                disabled={mesgul || panel !== null}
                onclick={() => void duzenle(ad)}
              >
                {m('mcp.duzenle')}
              </Button>
              <Button size="sm" iconLeft="trash-2" disabled={mesgul} onclick={() => void sil(ad)}>
                {m('mcp.sil')}
              </Button>
            </div>
            {#if ayrintiAcik[ad]}
              <McpAyrinti {ad} {alanAdi} />
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .aciklama,
  .bos,
  .rozet-aciklama,
  .hata,
  .izin,
  .sayi {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    margin-top: 10px;
    color: var(--text-muted);
  }
  .rozet-aciklama {
    margin-top: 4px;
    color: var(--status-warn);
  }
  .rozet-aciklama.tehlike,
  .hata {
    color: var(--status-danger);
  }
  .panel {
    margin-top: 12px;
  }
  .liste,
  .alanlar {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .alanlar {
    margin-top: 8px;
    gap: 6px;
  }
  .sunucu {
    padding: 10px 12px;
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
  }
  .ust,
  .alan-ust,
  .alan-alt,
  .dugmeler {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .ad {
    font: var(--type-body);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .bosluk {
    flex: 1;
  }
  .alanlar li {
    padding: 6px 10px;
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
  .dugmeler {
    margin-top: 10px;
  }
</style>
