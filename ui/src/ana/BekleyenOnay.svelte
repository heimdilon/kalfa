<!--
  Yazma alanının üstündeki yapışık onay kartı (masaüstü arayüzü tasarımı §3).
  Öğe başına komut, dizin, yalıtım/ağ rozetleri, MCP erişimi, uyarılar,
  geri alınabilirlik ve kısa diff görünür. Karar burada verilir: Reddet, "Bu oturumda izin ver"
  (yalnız teklifi olan tek öğede), "Bir kez izin ver" ya da "İncele" ile tam
  sayfa. Kararlar sayfanın mühürlü `kararlariUret`'iyle üretilir; riskli öğe
  içeren istek burada onaylanamaz. Yerel onaylı öğe Rust'ın penceresinde
  ayrıca sorulur. Kısayol tuşu yoktur: karar yalnız düğmeyle verilir.
-->
<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle, diffKirp, diffSayilari } from '../bilesenler/diff';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import { kisayolCumlesi, m } from '../dil/tr';
  import { geriAlinabilirlikMetni } from './geriAlma';
  import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';
  import { kartKarariUret, kartKararlari, kisaAd, riskliMi, type KartKarari } from './onay';

  const ONIZLEME_SATIRI = 12;

  let {
    ogeler,
    onkarar,
    onincele,
  }: {
    ogeler: OnayOgesi[];
    onkarar: (kararlar: OnayGirdisi[]) => Promise<void>;
    onincele: () => void;
  } = $props();

  let gonderiliyor = $state(false);
  const acikDiffler = new SvelteSet<CagriId>();

  const kararlar = $derived(kartKararlari(ogeler));
  const tek = $derived(ogeler.length === 1);
  const riskliVar = $derived(ogeler.some(riskliMi));
  const kisayol = $derived(tek ? ogeler[0].kisayol : undefined);

  const diffler = $derived(
    new Map(
      ogeler.map((o) => {
        const parcalar = o.diff ? diffCozumle(o.diff) : [];
        const satir = parcalar.reduce((t, p) => t + p.satirlar.length, 0);
        return [
          o.cagri,
          { parcalar, kisa: diffKirp(parcalar, ONIZLEME_SATIRI), satir, sayi: diffSayilari(parcalar) },
        ] as const;
      }),
    ),
  );

  async function karar(k: KartKarari) {
    const girdiler = kartKarariUret(ogeler, k);
    if (!girdiler) return;
    gonderiliyor = true;
    try {
      await onkarar(girdiler);
    } finally {
      gonderiliyor = false;
    }
  }
</script>

<section class="bekleyen" aria-label={m('onay.bekleyen.baslik', { sayi: ogeler.length })}>
  <header class="ust">
    <span class="ikon"><Icon name="shield" size={15} /></span>
    <span class="baslik">{m('onay.bekleyen.baslik', { sayi: ogeler.length })}</span>
  </header>

  <ul class="ogeler">
    {#each ogeler as o (o.cagri)}
      {@const d = diffler.get(o.cagri)}
      <li>
        <div class="satir">
          <span class="ad">{gorunurMetin(kisaAd(o))}</span>
          {#if d && d.parcalar.length > 0}
            <span class="arti">+{d.sayi.eklenen}</span>
            <span class="eksi">−{d.sayi.silinen}</span>
          {/if}
          <span class="aciklama"><GorunurMetin metin={o.aciklama} /></span>
        </div>
        {#if o.kabuk}
          <code class="komut"><GorunurMetin metin={o.kabuk.komut} /></code>
          <div class="rozetler">
            <span class="dizin"><GorunurMetin metin={o.kabuk.dizin} /></span>
            {#if o.kabuk.yalitim.tip === 'Yalitimli'}
              <Badge tone="accent">{m('onay.kabuk.yalitimli', { arka_uc: o.kabuk.yalitim.arka_uc })}</Badge>
            {:else}
              <Badge tone="danger">{m('onay.kabuk.yalitimsiz')}</Badge>
            {/if}
            {#if o.kabuk.ag}
              <Badge tone="warn">
                {m('onay.bekleyen.ag', { hostlar: (o.kabuk.ag_hostlari ?? []).join(', ') || '—' })}
              </Badge>
            {/if}
          </div>
        {:else if o.mcp}
          <!-- Erişim ve ağ kapsamı sayfadakiyle aynı metinlerle. -->
          {@const tam = o.mcp.erisim === 'TamErisim'}
          <div class="rozetler">
            <Badge tone={tam ? 'danger' : 'accent'}>
              {m(tam ? 'mcp.rozet.tam_erisim' : 'mcp.rozet.yalitimli')}
            </Badge>
            <span class="kapsam">
              {#if tam}
                {m('mcp.onay.tam_erisim')}
              {:else if o.mcp.ag_hostlari.length === 0}
                {m('mcp.onay.yalitimli_ag_yok')}
              {:else}
                <GorunurMetin
                  metin={m('mcp.onay.yalitimli_ag', { hostlar: o.mcp.ag_hostlari.join(', ') })}
                />
              {/if}
            </span>
          </div>
        {/if}
        {#if d && d.parcalar.length > 0}
          {#each acikDiffler.has(o.cagri) ? d.parcalar : d.kisa.parcalar as p, j (j)}
            <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
          {/each}
          {#if d.kisa.kirpildi}
            <Button
              size="sm"
              variant="ghost"
              onclick={() =>
                acikDiffler.has(o.cagri) ? acikDiffler.delete(o.cagri) : acikDiffler.add(o.cagri)}
            >
              {acikDiffler.has(o.cagri)
                ? m('onay.bekleyen.daralt')
                : m('onay.bekleyen.tamami', { sayi: d.satir })}
            </Button>
          {/if}
        {/if}
        {#if o.geri_alinabilir}
          <p class={['geri', { uyari: o.geri_alinabilir.tip === 'Hayir' }]}>
            <Icon name="rotate-ccw" size={13} />
            {m('onay.sayfa.geri_alinabilir')}: {geriAlinabilirlikMetni(o.geri_alinabilir)}
          </p>
        {/if}
        {#each o.uyarilar as u (u)}
          <p class="uyari"><Icon name="triangle-alert" size={13} /><GorunurMetin metin={u} /></p>
        {/each}
      </li>
    {/each}
  </ul>

  {#if riskliVar}
    <p class="not">{m('onay.bekleyen.riskli')}</p>
  {/if}
  {#if kisayol}
    <p class="not">{kisayolCumlesi(kisayol)}</p>
  {/if}

  <div class="dugmeler">
    <Button size="sm" variant="ghost" disabled={gonderiliyor} onclick={() => void karar('reddet')}>
      {tek ? m('onay.bekleyen.reddet') : m('onay.bekleyen.tumunu_reddet')}
    </Button>
    <span class="bosluk"></span>
    <Button size="sm" variant={riskliVar ? 'primary' : 'ghost'} disabled={gonderiliyor} onclick={onincele}>
      {m('onay.kart.incele')}
    </Button>
    {#if kararlar.includes('oturum')}
      <Button size="sm" disabled={gonderiliyor} onclick={() => void karar('oturum')}>
        {m('onay.bekleyen.oturum')}
      </Button>
    {/if}
    {#if kararlar.includes('bir_kez')}
      <Button size="sm" variant="primary" loading={gonderiliyor} onclick={() => void karar('bir_kez')}>
        {tek ? m('onay.bekleyen.bir_kez') : m('onay.bekleyen.tumunu_onayla')}
      </Button>
    {/if}
  </div>
</section>

<style>
  .bekleyen {
    display: flex;
    flex-direction: column;
    gap: 10px;
    max-height: 45vh;
    margin-bottom: 10px;
    padding: 12px 14px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-accent);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
  }
  .ust {
    display: flex;
    align-items: center;
    gap: 8px;
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-body);
  }
  .ikon {
    display: inline-flex;
    color: var(--text-accent);
  }
  /* Düğmeler hep görünür kalsın diye yalnız öğe listesi kayar. */
  .ogeler {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  li :global(.diff) {
    align-self: stretch;
  }
  li > :global(button) {
    align-self: flex-start;
  }
  .satir {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    font: var(--type-small);
  }
  .ad {
    flex: none;
    font-family: var(--font-mono);
    color: var(--text-body);
  }
  .arti {
    color: var(--status-ok);
  }
  .eksi {
    color: var(--status-danger);
  }
  .aciklama {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-muted);
  }
  .komut {
    display: block;
    padding: 6px 8px;
    background: var(--surface-sunken);
    border-radius: var(--radius-sm);
    font: var(--type-code);
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--text-body);
  }
  .rozetler {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .kapsam {
    font: var(--type-small);
    color: var(--text-muted);
  }
  .dizin {
    font: var(--type-meta);
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  .uyari,
  .not,
  .geri {
    margin: 0;
    font: var(--type-small);
  }
  .geri {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-muted);
  }
  .uyari {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--status-warn);
  }
  .not {
    color: var(--text-muted);
  }
  .dugmeler {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bosluk {
    flex: 1;
  }
</style>
