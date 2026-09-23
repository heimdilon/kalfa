<!--
  Onay sayfası (1c tasarımı §8.2, §9.5; tasarım sistemindeki ApprovalSheet).
  Değişiklik listesi, seçilenin diff'i, "ne" ve "nerede", uyarılar, ret
  gerekçesi ve üç karar. Seçilmeyen çağrılar gerekçeyle reddedilir. Odak
  sayfanın içinde kalır; Escape karar vermeden kapatır. "Bu değişiklikler geri
  alınabilir" cümlesi geri alma gelene kadar yazılmaz.
-->
<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import Button from '../bilesenler/Button.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle, diffSayilari } from '../bilesenler/diff';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import { m } from '../dil/tr';
  import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';
  import { kararlariUret, kisaAd, type OnaySecimi } from './onay';

  let {
    ogeler,
    onkarar,
    onkapat,
  }: {
    ogeler: OnayOgesi[];
    onkarar: (kararlar: OnayGirdisi[]) => Promise<void>;
    onkapat: () => void;
  } = $props();

  const kimlik = $props.id();
  const secilenler = new SvelteSet<CagriId>(untrack(() => ogeler.map((o) => o.cagri)));
  let aktif = $state(0);
  let gerekce = $state('');
  let gonderiliyor = $state(false);
  let pencere = $state<HTMLDivElement>();

  const aktifOge = $derived(ogeler[aktif] ?? ogeler[0]);
  const parcalar = $derived(aktifOge?.diff ? diffCozumle(aktifOge.diff) : []);
  const tumu = $derived(secilenler.size === ogeler.length);

  onMount(() => {
    const onceki = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pencere?.focus();
    return () => onceki?.focus();
  });

  function tus(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onkapat();
      return;
    }
    if (e.key !== 'Tab' || !pencere) return;
    const odaklanabilir = [
      ...pencere.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex="0"]',
      ),
    ];
    const ilk = odaklanabilir[0];
    const son = odaklanabilir.at(-1);
    if (!ilk || !son) return;
    const odak = document.activeElement;
    if (e.shiftKey && (odak === ilk || odak === pencere || !pencere.contains(odak))) {
      e.preventDefault();
      son.focus();
    } else if (!e.shiftKey && (odak === son || !pencere.contains(odak))) {
      e.preventDefault();
      ilk.focus();
    }
  }

  function tumunuSec(sec: boolean) {
    secilenler.clear();
    if (sec) for (const o of ogeler) secilenler.add(o.cagri);
  }

  async function karar(secim: OnaySecimi) {
    gonderiliyor = true;
    try {
      await onkarar(kararlariUret(ogeler, secim, secilenler, gerekce));
    } finally {
      gonderiliyor = false;
    }
  }
</script>

<svelte:window onkeydown={tus} />

<div class="perde">
  <div
    class="pencere"
    bind:this={pencere}
    role="dialog"
    aria-modal="true"
    aria-labelledby="{kimlik}-baslik"
    aria-describedby="{kimlik}-alt"
    tabindex="-1"
  >
    <header class="ust">
      <IconTile icon="shield" tone="accent" size={40} />
      <div class="baslik-blogu">
        <h2 id="{kimlik}-baslik">{m('onay.sayfa.baslik', { sayi: ogeler.length })}</h2>
        <p id="{kimlik}-alt">{m('onay.sayfa.alt')}</p>
      </div>
      <IconButton icon="x" label={m('genel.kapat')} onclick={onkapat} />
    </header>

    <div class="izgara">
      <div class="dosyalar">
        <div class="dosyalar-ust">
          <Checkbox
            checked={tumu}
            indeterminate={!tumu && secilenler.size > 0}
            label={m('onay.sayfa.tumunu_sec')}
            onchange={tumunuSec}
          />
          <span class="sayi">
            {m('onay.sayfa.secili', { secili: secilenler.size, toplam: ogeler.length })}
          </span>
        </div>
        {#each ogeler as o, i (o.cagri)}
          {@const s = o.diff ? diffSayilari(diffCozumle(o.diff)) : null}
          <div class={['dosya', { etkin: i === aktif }]}>
            <Checkbox
              checked={secilenler.has(o.cagri)}
              label={kisaAd(o)}
              hint={s ? `+${s.eklenen} −${s.silinen}` : undefined}
              onchange={(v) => (v ? secilenler.add(o.cagri) : secilenler.delete(o.cagri))}
            />
            <IconButton
              icon="chevron-right"
              size="sm"
              active={i === aktif}
              label={m('onay.sayfa.goster', { ad: kisaAd(o) })}
              onclick={() => (aktif = i)}
            />
          </div>
        {/each}
      </div>

      <div class="onizleme">
        <div class="onizleme-ust">
          <Icon name="file-text" size={16} />
          <div class="yol-blogu">
            <div class="ad">{kisaAd(aktifOge)}</div>
            {#if aktifOge.hedefler[0]}<div class="yol">{aktifOge.hedefler[0]}</div>{/if}
          </div>
        </div>
        <div class="diff">
          {#each parcalar as p, i (i)}
            <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
          {:else}
            <p class="bos">{m('onay.sayfa.diff_yok')}</p>
          {/each}
        </div>
      </div>
    </div>

    <dl class="bilgiler">
      <div class="bilgi">
        <dt><Icon name="info" size={16} />{m('onay.sayfa.ne')}</dt>
        <dd>{aktifOge.aciklama}</dd>
      </div>
      <div class="bilgi">
        <dt><Icon name="folder" size={16} />{m('onay.sayfa.nerede')}</dt>
        <dd class="mono">
          {#each aktifOge.hedefler as h (h)}<span>{h}</span>{/each}
        </dd>
      </div>
      {#if aktifOge.uyarilar.length > 0}
        <div class="bilgi uyari">
          <dt><Icon name="triangle-alert" size={16} />{m('onay.sayfa.uyarilar')}</dt>
          <dd>
            <ul>
              {#each aktifOge.uyarilar as u, i (i)}<li>{u}</li>{/each}
            </ul>
          </dd>
        </div>
      {/if}
    </dl>

    <div class="alt">
      <div class="gerekce">
        <span id="{kimlik}-gerekce">{m('onay.sayfa.gerekce')}</span>
        <Textarea
          rows={2}
          bind:value={gerekce}
          placeholder={m('onay.sayfa.gerekce_yer')}
          aria-labelledby="{kimlik}-gerekce"
        />
      </div>
      <div class="dugmeler">
        <Button
          variant="approve"
          size="lg"
          iconLeft="check"
          disabled={gonderiliyor}
          onclick={() => karar('tumu')}
        >
          {m('onay.sayfa.tumunu_onayla')}
        </Button>
        <Button
          variant="caution"
          size="lg"
          iconLeft="check"
          disabled={gonderiliyor || secilenler.size === 0}
          onclick={() => karar('secilenler')}
        >
          {m('onay.sayfa.secilenleri_onayla')}
        </Button>
        <Button
          variant="danger"
          size="lg"
          iconLeft="x"
          disabled={gonderiliyor}
          onclick={() => karar('hicbiri')}
        >
          {m('onay.sayfa.reddet')}
        </Button>
      </div>
    </div>
  </div>
</div>

<style>
  .perde {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 28px;
    background: var(--scrim);
    backdrop-filter: var(--blur-scrim);
  }
  .pencere {
    width: min(1140px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 26px 28px;
    background: var(--bg-panel);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-overlay);
  }
  .pencere:focus {
    outline: none;
  }
  .ust {
    display: flex;
    align-items: flex-start;
    gap: 18px;
    margin-bottom: 22px;
  }
  .baslik-blogu {
    flex: 1;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 23px / 1.3 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
  }
  .baslik-blogu p {
    margin: 6px 0 0;
    font: var(--type-subtitle);
    color: var(--text-muted);
  }
  .izgara {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 18px;
    margin-bottom: 20px;
  }
  .dosyalar,
  .onizleme {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .dosyalar {
    padding: 10px;
  }
  .dosyalar-ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px 12px;
    margin-bottom: 8px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .sayi {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .dosya {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 6px 6px 10px;
    border-radius: var(--radius-md);
    transition: var(--transition-control);
  }
  .dosya :global(.onay-kutusu) {
    flex: 1;
  }
  .etkin {
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .onizleme {
    overflow: hidden;
    min-width: 0;
  }
  .onizleme-ust {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    color: var(--text-muted);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .yol-blogu {
    min-width: 0;
  }
  .ad {
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-mono);
    color: var(--text-body);
  }
  .yol {
    font: var(--type-meta);
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .diff {
    max-height: 360px;
    overflow: auto;
  }
  .bos {
    margin: 16px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .bilgiler {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin: 0 0 20px;
    padding: 20px 0;
    border-top: var(--border-width) solid var(--border-subtle);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .bilgi {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 12px;
    align-items: start;
  }
  dt {
    display: flex;
    align-items: center;
    gap: 10px;
    font: var(--fw-medium) var(--text-small) / 1.5 var(--font-sans);
    color: var(--text-body);
  }
  dd {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .mono {
    display: flex;
    flex-direction: column;
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .uyari dt,
  .uyari dd {
    color: var(--status-warn);
  }
  ul {
    margin: 0;
    padding-left: 18px;
  }
  .alt {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 22px;
    align-items: end;
  }
  .gerekce span {
    display: block;
    margin-bottom: 8px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .dugmeler {
    display: flex;
    gap: 12px;
  }
</style>
