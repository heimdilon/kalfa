<!--
  Açılır seçim listesi. Odak düğmede kalır; seçenekler aria-activedescendant
  ile gösterilir. Ok tuşları, Enter, Boşluk ve Escape çalışır. Liste,
  kartların `overflow`'una takılmasın diye pencereye göre konumlanır; altta
  yer yoksa yukarı, sağda yer yoksa sola doğru açılır.
-->
<script lang="ts" module>
  export interface Secenek {
    value: string;
    label: string;
    /** Sağa yaslı tek aralıklı ipucu: model kimliği, kısayol. */
    hint?: string;
  }
</script>

<script lang="ts">
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    value: string;
    options: Secenek[];
    /** Ekran okuyucunun okuduğu ad; görünmez. */
    label: string;
    icon?: IkonAdi;
    tone?: 'default' | 'accent';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
    onchange?: (value: string) => void;
    class?: string;
  }

  let {
    value,
    options,
    label,
    icon,
    tone = 'default',
    size = 'md',
    disabled = false,
    onchange,
    class: sinif,
  }: Props = $props();

  const kimlik = $props.id();
  let acik = $state(false);
  let vurgu = $state(0);
  let kok = $state<HTMLDivElement>();

  const secili = $derived(options.find((o) => o.value === value));

  /** Tetikle liste arası ve pencere kenarı payı (px). */
  const ARA = 6;
  const KENAR = 8;
  /** Bir seçeneğin yaklaşık yüksekliği; yalnız yön seçiminde kullanılır. */
  const SATIR = 34;

  interface Konum {
    ust: number | null;
    alt: number | null;
    sol: number | null;
    sag: number | null;
    genislik: number;
    yukseklik: number;
  }
  let konum = $state<Konum>();

  function konumla() {
    const r = kok?.getBoundingClientRect();
    if (!r) return;
    const { innerHeight: h, innerWidth: w } = window;
    const asagi = h - r.bottom - ARA - KENAR;
    const yukari = r.top - ARA - KENAR;
    const istenen = options.length * SATIR + 10;
    const yukariAc = asagi < istenen && yukari > asagi;
    const solaAc = r.left > w / 2;
    konum = {
      ust: yukariAc ? null : r.bottom + ARA,
      alt: yukariAc ? h - r.top + ARA : null,
      sol: solaAc ? null : r.left,
      sag: solaAc ? w - r.right : null,
      genislik: r.width,
      yukseklik: Math.max(0, yukariAc ? yukari : asagi),
    };
  }

  function ac() {
    if (disabled) return;
    vurgu = Math.max(
      0,
      options.findIndex((o) => o.value === value),
    );
    konumla();
    acik = true;
  }

  function yenidenKonumla() {
    if (acik) konumla();
  }

  const px = (n: number | null | undefined) => (n == null ? null : `${n}px`);

  function sec(secenek: Secenek) {
    acik = false;
    if (secenek.value !== value) onchange?.(secenek.value);
  }

  function tus(e: KeyboardEvent) {
    if (!acik) {
      if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(e.key)) {
        e.preventDefault();
        ac();
      }
      return;
    }
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      vurgu = Math.min(options.length - 1, vurgu + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      vurgu = Math.max(0, vurgu - 1);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const secenek = options[vurgu];
      if (secenek) sec(secenek);
    } else if (e.key === 'Escape' || e.key === 'Tab') {
      acik = false;
    }
  }

  function disariBasildi(e: PointerEvent) {
    if (acik && kok && !kok.contains(e.target as Node)) acik = false;
  }
</script>

<svelte:window
  onpointerdown={disariBasildi}
  onresize={yenidenKonumla}
  onscrollcapture={yenidenKonumla}
/>

<div bind:this={kok} class={['secim', sinif]}>
  <button
    type="button"
    role="combobox"
    class={['tetik', `b-${size}`, `ton-${tone}`, { acik }]}
    {disabled}
    aria-haspopup="listbox"
    aria-expanded={acik}
    aria-controls={`${kimlik}-liste`}
    aria-label={label}
    aria-activedescendant={acik ? `${kimlik}-${vurgu}` : undefined}
    onclick={() => (acik ? (acik = false) : ac())}
    onkeydown={tus}
  >
    {#if icon}<span class="ikon-yeri"><Icon name={icon} size={15} /></span>{/if}
    <span class="deger">{secili?.label ?? value}</span>
    <span class="ok"><Icon name="chevron-down" size={14} /></span>
  </button>
  {#if acik}
    <div
      id={`${kimlik}-liste`}
      role="listbox"
      aria-label={label}
      class="liste"
      style:top={px(konum?.ust)}
      style:bottom={px(konum?.alt)}
      style:left={px(konum?.sol)}
      style:right={px(konum?.sag)}
      style:min-width={px(konum?.genislik)}
      style:max-height={px(konum?.yukseklik)}
    >
      {#each options as secenek, i (secenek.value)}
        <button
          type="button"
          id={`${kimlik}-${i}`}
          role="option"
          tabindex="-1"
          aria-selected={secenek.value === value}
          class={['secenek', { vurgulu: i === vurgu, secili: secenek.value === value }]}
          onpointerdown={(e) => e.preventDefault()}
          onpointermove={() => (vurgu = i)}
          onclick={() => sec(secenek)}
        >
          <span class="etiket">{secenek.label}</span>
          {#if secenek.hint}<span class="ipucu">{secenek.hint}</span>{/if}
          {#if secenek.value === value}<Icon name="check" size={14} />{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .secim {
    position: relative;
    display: inline-block;
  }
  .tetik {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: var(--control-height);
    padding: 0 10px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-md);
    color: var(--text-body);
    font: var(--type-small);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .tetik:hover:not(:disabled) {
    background: var(--surface-raised);
  }
  .tetik:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .b-sm {
    height: 28px;
  }
  .b-lg {
    height: var(--control-height-lg);
  }
  .acik {
    border-color: var(--border-accent);
  }
  .ikon-yeri,
  .ok {
    display: inline-flex;
    color: var(--text-muted);
  }
  .ton-accent {
    color: var(--text-accent);
  }
  .ton-accent .ikon-yeri {
    color: var(--text-accent);
  }
  .deger {
    flex: 1;
    text-align: left;
  }
  .liste {
    position: fixed;
    z-index: 40;
    overflow-y: auto;
    padding: 4px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
    white-space: nowrap;
  }
  .secenek {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 7px 9px;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-body);
    font: var(--type-small);
    text-align: left;
    cursor: pointer;
  }
  .vurgulu {
    background: var(--surface-raised);
  }
  .secili {
    background: var(--surface-selected);
    color: var(--text-accent);
  }
  .etiket {
    flex: 1;
  }
  .ipucu {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
