<!--
  MCP başlatma kartı (1.1a tasarımı §5.1): sohbette, model dışında görünür.
  Sunucu izin verilene kadar başlamaz ve araçları modele sunulmaz. Kart
  modelin araç çağrısı onayı değildir; onun yerine kullanılamaz. Tam
  erişimde "Başlat"ı Rust yerel pencerede ayrıca sorar. Kart sohbeti
  bekletmez: Composer açık kalır.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import { MCP_ROZETI, m } from '../dil/tr';
  import type { McpBaslatmaKarti } from '../ipc/tipler';

  let {
    kart,
    onyanit,
  }: { kart: McpBaslatmaKarti; onyanit: (kabul: boolean) => Promise<void> } = $props();

  let gonderiliyor = $state(false);
  const tam = $derived(kart.kip === 'tam_erisim');

  async function yanitla(kabul: boolean): Promise<void> {
    gonderiliyor = true;
    try {
      await onyanit(kabul);
    } finally {
      gonderiliyor = false;
    }
  }
</script>

<div role="group" aria-label={gorunurMetin(m('mcp.kart.baslik', { sunucu: kart.sunucu }))}>
  <Card
    icon="box"
    tone={tam ? 'danger' : 'accent'}
    title={gorunurMetin(m('mcp.kart.baslik', { sunucu: kart.sunucu }))}
    subtitle={m('mcp.kart.model_disi')}
  >
    <dl class="bilgiler">
      <dt>{m('mcp.kart.erisim')}</dt>
      <dd>
        <Badge tone={tam ? 'danger' : 'accent'}>{m(MCP_ROZETI[kart.kip])}</Badge>
      </dd>
      <dt>{m('mcp.kart.komut')}</dt>
      <dd><pre class="komut"><GorunurMetin metin={kart.komut_ozeti} /></pre></dd>
      <dt>{m('mcp.kart.ag')}</dt>
      <dd>
        {#if tam}
          {m('mcp.kart.ag_sinirsiz')}
        {:else if kart.ag_listesi.length === 0}
          {m('mcp.kart.ag_yok')}
        {:else}
          {#each kart.ag_listesi as host (host)}<span class="mono host"
              ><GorunurMetin metin={host} /></span
            >{/each}
        {/if}
      </dd>
    </dl>
    <p class={['uyari', { tehlike: tam }]}>{kart.uyari}</p>
    {#if kart.wde_kodlu}
      <p class="not">{m('mcp.kart.wde')}</p>
    {/if}
    <p class="not">{m(tam ? 'mcp.kart.sure.tam_erisim' : 'mcp.kart.sure.yalitimli')}</p>
    <div class="dugmeler">
      <Button
        size="sm"
        variant={tam ? 'caution' : 'primary'}
        iconLeft="check"
        disabled={gonderiliyor}
        onclick={() => void yanitla(true)}
      >
        {m('mcp.kart.baslat')}
      </Button>
      <Button size="sm" iconLeft="x" disabled={gonderiliyor} onclick={() => void yanitla(false)}>
        {m('mcp.kart.baslatma')}
      </Button>
    </div>
  </Card>
</div>

<style>
  .bilgiler {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 12px;
    margin: 0;
    font: var(--type-small);
  }
  dt {
    color: var(--text-secondary);
  }
  dd {
    margin: 0;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    color: var(--text-body);
  }
  .komut {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font: var(--type-meta);
  }
  .mono {
    font: var(--type-meta);
  }
  .host {
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
  }
  .uyari {
    margin: 12px 0 0;
    font: var(--type-small);
    color: var(--status-warn);
  }
  .uyari.tehlike {
    color: var(--status-danger);
  }
  .not {
    margin: 6px 0 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .dugmeler {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
</style>
