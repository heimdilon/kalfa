<!--
  MCP sunucusunun ayrıntısı (1.1a tasarımı §9): çalışma alanı başına
  denetlenen araçlar ve tanım metinleri, tanım denetiminin durumu (kapalı,
  denetlendi, uyarılı, katı, denetlenemedi), kapatılan araçlar ve maskeli
  son stderr satırları. Kapatılan araçların adı gösterilmez; sunucunun
  yazdığı metin görünmez karakterler kaçışlanarak basılır.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Badge from '../bilesenler/Badge.svelte';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import { m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { McpAracOzeti, McpSunucuAyrintisi } from '../ipc/tipler';

  let { ad, alanAdi }: { ad: string; alanAdi: (kimlik: string) => string } = $props();

  let ayrinti = $state<McpSunucuAyrintisi | null>(null);
  let hata = $state<string | null>(null);

  onMount(() => {
    ipc
      .mcpSunucuAyrintisi(ad)
      .then((a) => (ayrinti = a))
      .catch((e: unknown) => (hata = komutHatasi(e).mesaj));
  });

  type Ton = 'neutral' | 'ok' | 'warn' | 'danger';

  /** Denetim kipi ayrı alan değildir: sonuç ve "sunulur" birlikte okunur. */
  function denetim(a: McpAracOzeti): { ton: Ton; metin: string; neden: string | null } {
    const s = a.denetim_sonucu;
    if (!s) return { ton: 'neutral', metin: m('mcp.ayrinti.denetim_kapali'), neden: null };
    if (s.tip === 'Guvenli') return { ton: 'ok', metin: m('mcp.ayrinti.denetim_guvenli'), neden: null };
    if (s.tip === 'Bilinmiyor')
      return { ton: 'warn', metin: m('mcp.ayrinti.denetim_bilinmiyor'), neden: s.neden };
    return a.sunulur
      ? { ton: 'warn', metin: m('mcp.ayrinti.denetim_uyarili'), neden: s.gerekce }
      : { ton: 'danger', metin: m('mcp.ayrinti.denetim_kati'), neden: s.gerekce };
  }
</script>

<div class="ayrinti">
  {#if hata}
    <p class="hata" role="alert">{hata}</p>
  {:else if ayrinti}
    {#each ayrinti.alanlar as alan (alan.alan_kimligi)}
      <section class="alan" aria-label={alanAdi(alan.alan_kimligi)}>
        <h4><GorunurMetin metin={alanAdi(alan.alan_kimligi)} /></h4>
        {#if alan.araclar.length === 0}
          <p class="bos">{m('mcp.ayrinti.arac_yok')}</p>
        {:else}
          <ul class="araclar" aria-label={m('mcp.ayrinti.araclar')}>
            {#each alan.araclar as a (a.birlesik_ad)}
              {@const d = denetim(a)}
              <li>
                <div class="arac-ust">
                  <span class="mono"><GorunurMetin metin={a.birlesik_ad} /></span>
                  <Badge tone={d.ton}>{d.metin}</Badge>
                  {#if !a.sunulur}<Badge tone="danger">{m('mcp.ayrinti.sunulmuyor')}</Badge>{/if}
                </div>
                {#if a.aciklama}<p class="aciklama"><GorunurMetin metin={a.aciklama} /></p>{/if}
                {#if d.neden}<p class="neden"><GorunurMetin metin={d.neden} /></p>{/if}
                {#if a.uyari}<p class="neden"><GorunurMetin metin={a.uyari} /></p>{/if}
                <details>
                  <summary>{m('mcp.ayrinti.denetim_metni')}</summary>
                  <pre><GorunurMetin metin={a.denetim_metni} /></pre>
                </details>
              </li>
            {/each}
          </ul>
        {/if}
        {#if alan.reddedilenler.length > 0}
          <p class="neden">
            {m('mcp.ayrinti.reddedilenler', { liste: alan.reddedilenler.join(', ') })}
          </p>
        {/if}
        {#if alan.stderr.length > 0}
          <details>
            <summary>{m('mcp.ayrinti.stderr')}</summary>
            <pre><GorunurMetin metin={alan.stderr.join('\n')} /></pre>
          </details>
        {/if}
      </section>
    {:else}
      <p class="bos">{m('mcp.alan_yok')}</p>
    {/each}
  {:else}
    <p class="bos" role="status">…</p>
  {/if}
</div>

<style>
  .ayrinti {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-top: 10px;
  }
  h4 {
    margin: 0 0 6px;
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .araclar {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .araclar li {
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .arac-ust {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .mono,
  pre {
    font: var(--type-meta);
  }
  pre {
    margin: 6px 0 0;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
    color: var(--text-body);
  }
  details {
    margin-top: 6px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  summary {
    cursor: pointer;
  }
  .aciklama,
  .neden,
  .bos,
  .hata {
    margin: 4px 0 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .neden {
    color: var(--status-warn);
  }
  .hata {
    color: var(--status-danger);
  }
</style>
