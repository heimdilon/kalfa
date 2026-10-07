<!--
  Onay sayfası (1c tasarımı §8.2, §9.5; tasarım sistemindeki ApprovalSheet).
  Değişiklik listesi, seçilenin diff'i, "ne" ve "nerede", uyarılar, ret
  gerekçesi ve üç karar. Seçilmeyen çağrılar gerekçeyle reddedilir. Odak
  sayfanın içinde kalır; Escape karar vermeden kapatır. Seçilen öğelerin hepsi
  geri alınabilir dosya değişikliğiyse başlıkta "Bu değişiklikler geri
  alınabilir" yazar; bilgiyi çekirdek verir (aşama 3 tasarımı §5.11). Kabuk
  çağrısında diff yerine komut, dizin, ağ ve yalıtım gösterilir (aşama 2
  tasarımı §10.2).
-->
<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import Button from '../bilesenler/Button.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle, diffSayilari } from '../bilesenler/diff';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import { kisayolCumlesi, m } from '../dil/tr';
  import type { CagriId, OnayGirdisi, OnayOgesi, VeriCikisi } from '../ipc/tipler';
  import { geriAlinabilirlikMetni } from './geriAlma';
  import { kararlariUret, kisaAd, riskliMi, type OnaySecimi } from './onay';

  function veriCikisiMetni(v?: VeriCikisi): string {
    switch (v) {
      case 'DosyaIcerigi':
        return m('onay.veri.DosyaIcerigi');
      case 'KomutCiktisi':
        return m('onay.veri.KomutCiktisi');
      case 'AramaSonuclari':
        return m('onay.veri.AramaSonuclari');
      case 'DosyaAdlari':
        return m('onay.veri.DosyaAdlari');
      case 'YalnizcaYazildi':
        return m('onay.veri.YalnizcaYazildi');
      default:
        return m('onay.veri.Bilinmiyor');
    }
  }

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
  // Riskli öğeler önceden seçilmez; kullanıcı tek tek seçer.
  const secilenler = new SvelteSet<CagriId>(
    untrack(() => ogeler.filter((o) => !riskliMi(o)).map((o) => o.cagri)),
  );
  // "Kapsam" satırında kısayolu seçilen çağrılar (aşama 3 tasarımı §9.1).
  const kisayolSecili = new SvelteSet<CagriId>();
  const riskliVar = $derived(ogeler.some(riskliMi));
  let aktif = $state(0);
  let gerekce = $state('');
  let gonderiliyor = $state(false);
  let pencere = $state<HTMLDivElement>();

  const aktifOge = $derived(ogeler[aktif] ?? ogeler[0]);
  const parcalar = $derived(aktifOge?.diff ? diffCozumle(aktifOge.diff) : []);
  const tumu = $derived(secilenler.size === ogeler.length);
  /** Seçilen öğelerin hepsi geri alınabilir dosya değişikliği. */
  const hepsiGeriAlinabilir = $derived(
    secilenler.size > 0 &&
      ogeler
        .filter((o) => secilenler.has(o.cagri))
        .every((o) => o.geri_alinabilir?.tip === 'Evet'),
  );

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
      await onkarar(kararlariUret(ogeler, secim, secilenler, gerekce, kisayolSecili));
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
        {#if hepsiGeriAlinabilir}
          <p class="geri-alinabilir">
            <Icon name="rotate-ccw" size={14} />{m('onay.sayfa.hepsi_geri_alinabilir')}
          </p>
        {/if}
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
              label={gorunurMetin(kisaAd(o))}
              hint={s ? `+${s.eklenen} −${s.silinen}` : undefined}
              onchange={(v) => (v ? secilenler.add(o.cagri) : secilenler.delete(o.cagri))}
            />
            <IconButton
              icon="chevron-right"
              size="sm"
              active={i === aktif}
              label={m('onay.sayfa.goster', { ad: gorunurMetin(kisaAd(o)) })}
              onclick={() => (aktif = i)}
            />
          </div>
        {/each}
      </div>

      <div class="onizleme">
        <div class="onizleme-ust">
          <Icon name={aktifOge.kabuk ? 'square-terminal' : 'file-text'} size={16} />
          <div class="yol-blogu">
            <div class="ad"><GorunurMetin metin={kisaAd(aktifOge)} /></div>
            {#if aktifOge.hedefler[0]}<div class="yol">
                <GorunurMetin metin={aktifOge.hedefler[0]} />
              </div>{/if}
          </div>
        </div>
        {#if aktifOge.kabuk}
          {@const k = aktifOge.kabuk}
          <dl class="kabuk">
            <dt>
              {k.bicim.tip === 'Betik'
                ? m('onay.kabuk.betik', { yorumlayici: k.bicim.yorumlayici })
                : m('onay.kabuk.komut')}
            </dt>
            <dd><pre class="komut"><GorunurMetin metin={k.komut} /></pre></dd>
            <dt>{m('onay.kabuk.dizin')}</dt>
            <dd class="mono"><GorunurMetin metin={k.dizin} /></dd>
            <dt>{m('onay.kabuk.ag')}</dt>
            <dd class={{ uyari: k.ag }}>
              {#if !k.ag}
                {m('onay.kabuk.ag_kapali')}
              {:else if k.yalitim.tip === 'Yalitimsiz'}
                {m('onay.kabuk.ag_yalitimsiz')}
              {:else if !k.ag_hostlari || k.ag_hostlari.length === 0}
                {m('onay.kabuk.ag_varsayilan')}
              {:else}
                <div>{m('onay.kabuk.ag_ek_adresler')}</div>
                <div class="ek-hostlar">
                  {#each k.ag_hostlari as host (host)}
                    <span class="mono ek-host"><GorunurMetin metin={host} /></span>
                  {/each}
                </div>
              {/if}
            </dd>
            <dt>{m('onay.kabuk.yalitim')}</dt>
            <dd class={{ uyari: k.yalitim.tip === 'Yalitimsiz' }}>
              {k.yalitim.tip === 'Yalitimli'
                ? m('onay.kabuk.yalitimli', { arka_uc: k.yalitim.arka_uc })
                : m('onay.kabuk.yalitimsiz')}
            </dd>
            {#if aktifOge.onaysiz_degisiklikler}
              {@const o = aktifOge.onaysiz_degisiklikler}
              <dt>{m('onay.kabuk.onaysiz')}</dt>
              <dd class="onaysiz">
                {#if o.yollar.length === 0 && !o.komut_calisti}
                  <span>{m('onay.kabuk.onaysiz_yok')}</span>
                {/if}
                {#each o.yollar as y (y)}<span class="mono"><GorunurMetin metin={y} /></span>{/each}
                {#if o.komut_calisti}<span class="uyari">{m('onay.kabuk.onaysiz_komut')}</span>{/if}
                {#if o.derleme_dosyalari.length > 0}
                  <span class="uyari rozet">
                    <Icon name="triangle-alert" size={14} />
                    {m('onay.kabuk.derleme', { dosyalar: o.derleme_dosyalari.join(', ') })}
                  </span>
                {/if}
              </dd>
            {/if}
          </dl>
        {:else}
          <div class="diff">
            {#each parcalar as p, i (i)}
              <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
            {:else}
              <p class="bos">{m('onay.sayfa.diff_yok')}</p>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <dl class="bilgiler">
      <div class="bilgi">
        <dt><Icon name="info" size={16} />{m('onay.sayfa.ne')}</dt>
        <dd><GorunurMetin metin={aktifOge.aciklama} /></dd>
      </div>
      {#if aktifOge.hedefler.length > 0}
        <div class="bilgi">
          <dt><Icon name="folder" size={16} />{m('onay.sayfa.nerede')}</dt>
          <dd class="mono">
            {#each aktifOge.hedefler as h (h)}<span><GorunurMetin metin={h} /></span>{/each}
          </dd>
        </div>
      {/if}
      <div class="bilgi">
        <dt><Icon name="send" size={16} />{m('onay.sayfa.veri_cikisi')}</dt>
        <dd>{veriCikisiMetni(aktifOge.veri_cikisi)}</dd>
      </div>
      {#if aktifOge.geri_alinabilir}
        <div class={['bilgi', { uyari: aktifOge.geri_alinabilir.tip === 'Hayir' }]}>
          <dt><Icon name="rotate-ccw" size={16} />{m('onay.sayfa.geri_alinabilir')}</dt>
          <dd>{geriAlinabilirlikMetni(aktifOge.geri_alinabilir)}</dd>
        </div>
      {/if}
      {#if aktifOge.kisayol}
        {@const oge = aktifOge}
        {@const teklif = aktifOge.kisayol}
        <div class="bilgi">
          <dt><Icon name="key-round" size={16} />{m('onay.sayfa.kapsam')}</dt>
          <dd class="kapsam">
            {#if !kisayolSecili.has(oge.cagri)}<span>{m('onay.sayfa.kapsam_tek')}</span>{/if}
            <Checkbox
              checked={kisayolSecili.has(oge.cagri)}
              label={kisayolCumlesi(teklif)}
              onchange={(v) => (v ? kisayolSecili.add(oge.cagri) : kisayolSecili.delete(oge.cagri))}
            />
          </dd>
        </div>
      {/if}
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

    {#if riskliVar}
      <p class="yerel"><Icon name="shield-alert" size={16} />{m('onay.sayfa.riskli')}</p>
    {/if}

    {#if ogeler.some((o) => o.yerel_onay)}
      <p class="yerel"><Icon name="shield-alert" size={16} />{m('onay.kabuk.yerel')}</p>
    {/if}

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
          disabled={gonderiliyor || riskliVar}
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
  .baslik-blogu .geri-alinabilir {
    display: flex;
    align-items: center;
    gap: 8px;
    font: var(--type-small);
    color: var(--status-ok);
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
  .kabuk {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 10px 14px;
    margin: 0;
    padding: 14px 16px;
    font: var(--type-small);
  }
  .kabuk dt {
    color: var(--text-muted);
  }
  .kabuk dd {
    min-width: 0;
    color: var(--text-body);
  }
  .komut {
    margin: 0;
    max-height: 240px;
    overflow: auto;
    font: var(--text-small) / 1.5 var(--font-mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .kabuk .uyari {
    color: var(--status-warn);
  }
  .onaysiz {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .onaysiz .mono {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .rozet {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .yerel {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0 0 16px;
    font: var(--type-small);
    color: var(--status-warn);
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
