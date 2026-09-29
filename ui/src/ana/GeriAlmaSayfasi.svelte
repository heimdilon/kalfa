<!--
  Geri alma sayfası (aşama 3 tasarımı §5.6, §5.11): onay sayfasının düzeni.
  Dosyalar ve her birine ne olacağı, seçilenin geri alma yönündeki diff'i,
  çakışma nedenleri ve kabuğun değiştirdiği dosyalar (salt liste). Onay
  gösterilen plana mühürle bağlıdır: dosyalar önizlemeden sonra değişirse
  çekirdek yeni planı gönderir ve sayfa onu gösterir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle } from '../bilesenler/diff';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import { m } from '../dil/tr';
  import type { GeriAlmaOnizlemesi } from '../durum/oturum';
  import { eylemMetni, yazilacakMi } from './geriAlma';

  let {
    onizleme,
    baslik,
    gonderiliyor,
    ongeri,
    onkapat,
  }: {
    onizleme: GeriAlmaOnizlemesi;
    /** Turun ilk mesajı; bilinmiyorsa yok. */
    baslik: string | null;
    gonderiliyor: boolean;
    ongeri: () => void;
    onkapat: () => void;
  } = $props();

  const kimlik = $props.id();
  let aktif = $state(0);
  let pencere = $state<HTMLDivElement>();

  const plan = $derived(onizleme.plan);
  const aktifOge = $derived(plan.ogeler[aktif] ?? plan.ogeler[0]);
  const parcalar = $derived(aktifOge?.diff ? diffCozumle(aktifOge.diff) : []);
  const yapilacak = $derived(plan.ogeler.some((o) => yazilacakMi(o.eylem)));
  const komutlar = $derived(plan.komutlar ?? []);

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
      ...pencere.querySelectorAll<HTMLElement>('button:not([disabled]), [tabindex="0"]'),
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

  function ton(tip: string): 'ok' | 'warn' | 'danger' | 'neutral' {
    if (tip === 'GeriYukle' || tip === 'Sil') return 'ok';
    if (tip === 'Cakisma') return 'warn';
    if (tip === 'GeriAlinamaz') return 'danger';
    return 'neutral';
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
      <IconTile icon="rotate-ccw" tone="accent" size={40} />
      <div class="baslik-blogu">
        <h2 id="{kimlik}-baslik">{m('geri_alma.sayfa.baslik')}</h2>
        <p id="{kimlik}-alt">{m('geri_alma.sayfa.alt')}</p>
        {#if baslik}<p class="tur">{m('geri_alma.sayfa.tur', { baslik })}</p>{/if}
      </div>
      <IconButton icon="x" label={m('genel.kapat')} onclick={onkapat} />
    </header>

    {#if plan.veri_silindi}
      <p class="serit"><Icon name="triangle-alert" size={16} />{m('geri_alma.sayfa.veri_silindi')}</p>
    {/if}

    {#if plan.ogeler.length > 0}
      <div class="izgara">
        <div class="dosyalar">
          {#each plan.ogeler as o, i (o.yol)}
            {@const e = eylemMetni(o.eylem)}
            <button
              type="button"
              class={['dosya', { etkin: i === aktif }]}
              aria-current={i === aktif ? 'true' : undefined}
              aria-label={m('geri_alma.sayfa.goster', { ad: o.yol })}
              onclick={() => (aktif = i)}
            >
              <span class="yol"><GorunurMetin metin={o.yol} /></span>
              <Badge tone={ton(o.eylem.tip)}>{e.ad}</Badge>
            </button>
          {/each}
        </div>

        <div class="onizleme">
          {#if aktifOge}
            {@const e = eylemMetni(aktifOge.eylem)}
            <div class="onizleme-ust">
              <Icon name="file-text" size={16} />
              <span class="ad"><GorunurMetin metin={aktifOge.yol} /></span>
              <span class="eylem">{e.ad}</span>
            </div>
            {#if e.neden}<p class="neden">{e.neden}</p>{/if}
            {#if aktifOge.uyarilar.length > 0}
              <ul class="uyarilar">
                {#each aktifOge.uyarilar as u, i (i)}
                  <li><Icon name="triangle-alert" size={14} />{u}</li>
                {/each}
              </ul>
            {/if}
            <div class="diff">
              {#each parcalar as p, i (i)}
                <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
              {:else}
                <p class="bos">{m('geri_alma.sayfa.diff_yok')}</p>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    {#if komutlar.length > 0}
      <section class="komutlar" aria-label={m('geri_alma.sayfa.komutlar')}>
        <h3><Icon name="square-terminal" size={16} />{m('geri_alma.sayfa.komutlar')}</h3>
        <p class="not">{m('tur_ozeti.not_kabuk')}</p>
        {#each komutlar as k (k.cagri)}
          {@const g = k.degisiklikler}
          <div class="komut">
            <pre><GorunurMetin metin={k.komut} /></pre>
            {#if g}
              <span class="sayi">{m('geri_alma.sayfa.komut_dosyalari', { sayi: g.toplam })}</span>
              <ul class="komut-dosyalari">
                {#each g.dosyalar as f (f.yol)}<li><GorunurMetin metin={f.yol} /></li>{/each}
              </ul>
              {#if g.eksik || g.dosyalar.length < g.toplam}
                <p class="not">{m('tur_ozeti.not_eksik')}</p>
              {/if}
            {/if}
          </div>
        {/each}
      </section>
    {/if}

    {#if !yapilacak}
      <p class="bos">{m('geri_alma.sayfa.yapilacak_yok')}</p>
    {/if}

    <div class="dugmeler">
      <Button variant="ghost" size="lg" onclick={onkapat}>{m('geri_alma.sayfa.vazgec')}</Button>
      <Button
        variant="caution"
        size="lg"
        iconLeft="rotate-ccw"
        disabled={gonderiliyor || !yapilacak}
        onclick={ongeri}
      >
        {m('geri_alma.sayfa.geri_al')}
      </Button>
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
  .baslik-blogu .tur {
    font: var(--type-small);
    overflow-wrap: anywhere;
  }
  .serit {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px;
    font: var(--type-small);
    color: var(--status-warn);
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
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .dosya {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    cursor: pointer;
    text-align: left;
    color: inherit;
    transition: var(--transition-control);
  }
  .dosya:hover {
    background: var(--surface-raised);
  }
  .etkin,
  .etkin:hover {
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .yol {
    min-width: 0;
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-mono);
    color: var(--text-body);
    overflow-wrap: anywhere;
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
  .ad {
    flex: 1;
    min-width: 0;
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-mono);
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .eylem {
    font: var(--type-meta);
  }
  .neden {
    margin: 12px 16px 0;
    font: var(--type-small);
    color: var(--status-warn);
  }
  .uyarilar {
    margin: 12px 16px 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font: var(--type-small);
    color: var(--status-warn);
  }
  .uyarilar li {
    display: flex;
    align-items: center;
    gap: 8px;
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
  .komutlar {
    margin: 0 0 20px;
    padding: 16px 0;
    border-top: var(--border-width) solid var(--border-subtle);
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 6px;
    font: var(--fw-medium) var(--text-small) / 1.5 var(--font-sans);
    color: var(--text-body);
  }
  .not {
    margin: 0 0 10px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .komut {
    margin-bottom: 10px;
  }
  .komut pre {
    margin: 0 0 4px;
    font: var(--text-small) / 1.5 var(--font-mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--text-body);
  }
  .sayi {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .komut-dosyalari {
    margin: 4px 0 0;
    padding-left: 18px;
    max-height: 160px;
    overflow: auto;
    font: var(--text-small) / 1.5 var(--font-mono);
    color: var(--text-secondary);
  }
  .dugmeler {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
  }
</style>
