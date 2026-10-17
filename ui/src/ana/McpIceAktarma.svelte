<!--
  MCP JSON içe aktarma (1.1a tasarımı §9). Metni arka uç ayrıştırır; mutlak
  komut yolları orada kimliğe çevrilir. Metin sır içerebileceği için
  önizlemeye gönderilir gönderilmez alandan silinir. Önizleme gizli
  değerleri göstermez; düz bir değeri "gizli say" ile kasaya
  yönlendirebilirsiniz. Sunucular yalıtımlı ve kapalı eklenir, `url`
  içerenler atlanır. Kaydı Rust yerel pencerede sorar.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import { MCP_ROZETI, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { McpIceAktarmaOnizlemesi, OturumId } from '../ipc/tipler';

  let {
    oturum,
    onkapat,
    onkaydedildi,
  }: {
    oturum: OturumId | null;
    onkapat: () => void;
    onkaydedildi: (sayi: number) => void;
  } = $props();

  const kimlik = $props.id();
  let metin = $state('');
  let onizleme = $state<McpIceAktarmaOnizlemesi | null>(null);
  let kusak = 0;
  let hata = $state<string | null>(null);
  let mesgul = $state(false);

  async function calistir(is: () => Promise<void>): Promise<void> {
    mesgul = true;
    hata = null;
    try {
      await is();
    } catch (e) {
      hata = komutHatasi(e).mesaj;
    } finally {
      mesgul = false;
    }
  }

  function onizle(): Promise<void> {
    const json = metin;
    metin = '';
    return calistir(async () => {
      onizleme = await ipc.mcpIceAktarOnizle(json);
      kusak = onizleme.kusak;
    });
  }

  function gizliSay(sunucu: string, ortam: string): Promise<void> {
    const o = onizleme;
    if (!o) return Promise.resolve();
    return calistir(async () => {
      kusak = await ipc.mcpIceAktarGizlilik(o.taslak_kimligi, sunucu, ortam, true);
      const degisken = o.sunucular
        .find((s) => s.ad === sunucu)
        ?.ortam.find((d) => d.ad === ortam);
      if (degisken) {
        degisken.gizli = true;
        degisken.deger_acik = undefined;
      }
    });
  }

  function kaydet(): Promise<void> {
    const o = onizleme;
    if (!o) return Promise.resolve();
    return calistir(async () => {
      if (await ipc.mcpIceAktarKaydet(o.taslak_kimligi, kusak, oturum)) {
        onkaydedildi(o.sunucular.length);
      } else {
        hata = m('mcp.reddedildi');
      }
    });
  }
</script>

<Card icon="box" title={m('mcp.ice.baslik')}>
  <div class="govde">
    <p class="aciklama">{m('mcp.ice.aciklama')}</p>
    {#if !onizleme}
      <span id="{kimlik}-metin" class="etiket">{m('mcp.ice.metin')}</span>
      <Textarea
        bind:value={metin}
        rows={6}
        spellcheck={false}
        aria-labelledby="{kimlik}-metin"
      />
      <div class="dugmeler">
        <Button
          size="sm"
          variant="primary"
          loading={mesgul}
          disabled={!metin.trim()}
          onclick={() => void onizle()}
        >
          {m('mcp.ice.onizle')}
        </Button>
        <Button size="sm" disabled={mesgul} onclick={onkapat}>{m('mcp.form.vazgec')}</Button>
      </div>
    {:else}
      <ul class="sunucular" aria-label={m('mcp.ice.sunucular')}>
        {#each onizleme.sunucular as s (s.ad)}
          <li>
            <div class="ust">
              <strong><GorunurMetin metin={s.ad} /></strong>
              <Badge tone="accent">{m(MCP_ROZETI[s.kip])}</Badge>
            </div>
            <pre><GorunurMetin metin={[s.komut_gorunumu, ...s.argumanlar].join(' ')} /></pre>
            {#each s.ortam as d (d.ad)}
              <div class="ortam">
                <span class="mono"><GorunurMetin metin={d.ad} /></span>
                {#if d.gizli}
                  <span class="gizli">{m('mcp.ice.gizli')}</span>
                {:else}
                  <span class="mono"><GorunurMetin metin={d.deger_acik ?? ''} /></span>
                {/if}
                <Checkbox
                  checked={d.gizli}
                  disabled={d.gizli || mesgul}
                  label={m('mcp.ice.gizli_say')}
                  onchange={() => void gizliSay(s.ad, d.ad)}
                />
              </div>
            {/each}
          </li>
        {/each}
      </ul>
      {#each onizleme.atlanan_url_sunuculari as a (a.ad)}
        <p class="aciklama"><GorunurMetin metin={m('mcp.ice.atlanan', a)} /></p>
      {/each}
      <p class="aciklama">{m('mcp.form.yerel_onay')}</p>
      <div class="dugmeler">
        <Button
          size="sm"
          variant="primary"
          loading={mesgul}
          disabled={onizleme.sunucular.length === 0}
          onclick={() => void kaydet()}
        >
          {m('mcp.ice.kaydet')}
        </Button>
        <Button size="sm" disabled={mesgul} onclick={onkapat}>{m('mcp.form.vazgec')}</Button>
      </div>
    {/if}
    {#if hata}<p class="hata" role="alert"><GorunurMetin metin={hata} /></p>{/if}
  </div>
</Card>

<style>
  .govde {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .etiket {
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .aciklama,
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .hata {
    color: var(--status-danger);
  }
  .sunucular {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .sunucular li {
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .ust,
  .ortam {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .ust {
    font: var(--type-small);
  }
  pre,
  .mono {
    font: var(--type-meta);
  }
  pre {
    margin: 6px 0;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .gizli {
    font: var(--type-small);
    color: var(--text-muted);
  }
  .dugmeler {
    display: flex;
    gap: 8px;
  }
</style>
