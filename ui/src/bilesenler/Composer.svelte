<!--
  Oturumun altındaki bakır çerçeveli giriş: mesaj, model ve izin kipi tek
  çerçevede; kullanıcı ikisini görmeden gönderemez. Enter gönderir,
  Shift+Enter yeni satır açar (1c tasarımı §9.5). Tur sürerken gönder
  düğmesinin yerinde durdurma düğmesi durur.
-->
<script lang="ts">
  import { EFOR_ADI, m } from '../dil/tr';
  import { EFORLAR, type DusunceEforu } from '../ipc/tipler';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Select, { type Secenek } from './Select.svelte';
  import Textarea from './Textarea.svelte';

  interface Props {
    value?: string;
    models: Secenek[];
    model: string;
    modes: Secenek[];
    mode: string;
    /** Seçili düşünce eforu (varsayılan: Orta). */
    efor?: DusunceEforu;
    /** Seçili modelin desteklediği efor düzeyleri. */
    eforlar?: readonly DusunceEforu[];
    /** Tur sürüyor: gönderme kapalı, durdurma açık. */
    running?: boolean;
    disabled?: boolean;
    placeholder?: string;
    onsend?: (metin: string) => void;
    oncancel?: () => void;
    onmodelchange?: (model: string) => void;
    onmodechange?: (kip: string) => void;
    oneforchange?: (efor: DusunceEforu) => void;
    onmodelyenile?: () => void;
    class?: string;
  }

  let {
    value = $bindable(''),
    models,
    model,
    modes,
    mode,
    efor = 'orta',
    eforlar = EFORLAR,
    running = false,
    disabled = false,
    placeholder = m('composer.yer_tutucu'),
    onsend,
    oncancel,
    onmodelchange,
    onmodechange,
    oneforchange,
    onmodelyenile,
    class: sinif,
  }: Props = $props();

  let eforAcik = $state(false);
  let eforKutu = $state<HTMLDivElement>();

  const gonderilebilir = $derived(!running && !disabled && value.trim().length > 0);
  const aktifEforlar = $derived(eforlar.length > 0 ? eforlar : EFORLAR);
  const eforIndeksi = $derived(Math.max(0, aktifEforlar.indexOf(efor)));
  const eforEtiketi = $derived(m(EFOR_ADI[efor]));
  const seciliModel = $derived(models.find((o) => o.value === model));

  function gonder() {
    if (!gonderilebilir) return;
    const metin = value;
    value = '';
    onsend?.(metin);
  }

  function tus(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      gonder();
    } else if (e.key === 'Escape' && eforAcik) {
      eforAcik = false;
    }
  }

  function eforSec(yeni: DusunceEforu) {
    if (disabled || running) return;
    if (yeni !== efor) oneforchange?.(yeni);
  }

  function kaydiriciDegisti(e: Event & { currentTarget: HTMLInputElement }) {
    const idx = Number(e.currentTarget.value);
    const secilen = aktifEforlar[idx];
    if (secilen) eforSec(secilen);
  }

  function disariBasildi(e: PointerEvent) {
    if (eforAcik && eforKutu && !eforKutu.contains(e.target as Node)) {
      eforAcik = false;
    }
  }
</script>

<svelte:window onpointerdown={disariBasildi} />

<div class="composer-sarmal">
  <Textarea
    bind:value
    accent
    rows={2}
    {placeholder}
    {disabled}
    aria-label={placeholder}
    onkeydown={tus}
    class={sinif}
  >
    {#snippet footer()}
      <Select
        size="sm"
        icon="shield"
        label={m('composer.kip')}
        value={mode}
        options={modes}
        disabled={disabled || running}
        onchange={onmodechange}
      />
      <Select
        size="sm"
        icon="box"
        tone="accent"
        label={m('composer.model')}
        value={model}
        options={models}
        disabled={disabled || running}
        onchange={onmodelchange}
      />
      <span class="bosluk"></span>
      <div bind:this={eforKutu} class="efor-sarmal">
        <button
          type="button"
          class={['efor-tetik', { acik: eforAcik }]}
          disabled={disabled || running}
          aria-label={m('composer.efor')}
          aria-expanded={eforAcik}
          onclick={() => (eforAcik = !eforAcik)}
        >
          <span class="efor-model">{model || seciliModel?.label || 'auto'}</span>
          <span class="efor-deger">{eforEtiketi}</span>
          <Icon name="chevron-down" size={13} />
        </button>

        {#if eforAcik}
          <div class="efor-balon" role="dialog" aria-label={m('composer.efor_baslik')}>
            <div class="balon-ust">
              <div class="balon-basliklar">
                <span class="balon-efor">{eforEtiketi}</span>
                <span class="balon-model">
                  {model}
                  {#if seciliModel?.label && seciliModel.label !== model}
                    · {seciliModel.label}
                  {/if}
                </span>
              </div>
              <div class="balon-eylemler">
                {#if onmodelyenile}
                  <IconButton
                    icon="rotate-ccw"
                    size="sm"
                    label={m('composer.yenile')}
                    onclick={() => onmodelyenile()}
                  />
                {/if}
              </div>
            </div>

            {#if models.length > 1}
              <div class="model-chipler" role="group" aria-label={m('composer.model')}>
                {#each models as s (s.value)}
                  <button
                    type="button"
                    class={['model-chip', { secili: s.value === model }]}
                    onclick={() => onmodelchange?.(s.value)}
                  >
                    <span>{s.value}</span>
                    {#if s.label && s.label !== s.value}
                      <span class="chip-ipucu">{s.label}</span>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}

            <div class={['kaydirici-kutu', `efor-adim-${eforIndeksi}`]}>
              <div class="kaydirici-iz">
                <div class="kaydirici-dolu"></div>
                {#each aktifEforlar as k, i (k)}
                  <button
                    type="button"
                    role="radio"
                    aria-checked={k === efor}
                    aria-label={m(EFOR_ADI[k])}
                    class={['durak', { etkin: i <= eforIndeksi, secili: k === efor }]}
                    onclick={() => eforSec(k)}
                  >
                    <span class="durak-nokta"></span>
                  </button>
                {/each}
              </div>
              <input
                type="range"
                class="kaydirici-girdi"
                aria-label={m('composer.efor')}
                min={0}
                max={Math.max(0, aktifEforlar.length - 1)}
                step={1}
                value={eforIndeksi}
                oninput={kaydiriciDegisti}
              />
            </div>

            <div class="efor-etiketler">
              {#each aktifEforlar as k (k)}
                <button
                  type="button"
                  class={['efor-etiket', { secili: k === efor }]}
                  onclick={() => eforSec(k)}
                >
                  {m(EFOR_ADI[k])}
                </button>
              {/each}
            </div>
          </div>
        {/if}
      </div>
      {#if running}
        <IconButton icon="square" label={m('composer.durdur')} variant="primary" onclick={oncancel} />
      {:else}
        <IconButton
          icon="send"
          label={m('composer.gonder')}
          variant="primary"
          disabled={!gonderilebilir}
          onclick={gonder}
        />
      {/if}
    {/snippet}
  </Textarea>
</div>

<style>
  .composer-sarmal {
    position: relative;
  }
  .bosluk {
    flex: 1;
  }
  .efor-sarmal {
    position: relative;
    display: inline-flex;
  }
  .efor-tetik {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    background: var(--surface-raised);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-pill);
    color: var(--text-body);
    font: var(--type-small);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .efor-tetik:hover:not(:disabled),
  .efor-tetik.acik {
    border-color: var(--border-accent);
    background: var(--surface-selected);
  }
  .efor-tetik:disabled {
    cursor: not-allowed;
    opacity: 0.55;
  }
  .efor-model {
    font-family: var(--font-mono);
    font-size: var(--text-micro);
    color: var(--text-secondary);
  }
  .efor-deger {
    font-weight: var(--fw-semibold);
    color: var(--text-accent);
  }
  .efor-balon {
    position: absolute;
    bottom: calc(100% + 10px);
    right: 0;
    width: 280px;
    z-index: 45;
    padding: 14px 16px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-overlay);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .balon-ust {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
  }
  .balon-basliklar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .balon-efor {
    font: var(--fw-semibold) 16px / 1.2 var(--font-sans);
    color: var(--text-title);
  }
  .balon-model {
    font: var(--type-meta);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .balon-eylemler {
    display: inline-flex;
    gap: 4px;
  }
  .model-chipler {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .model-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 8px;
    border-radius: var(--radius-pill);
    border: var(--border-width) solid var(--border-subtle);
    background: var(--surface-sunken);
    color: var(--text-secondary);
    font: var(--type-meta);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .model-chip:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .model-chip.secili {
    border-color: var(--border-accent);
    background: var(--surface-selected);
    color: var(--text-accent);
  }
  .chip-ipucu {
    color: var(--text-muted);
  }
  .kaydirici-kutu {
    position: relative;
    height: 30px;
    display: flex;
    align-items: center;
  }
  .kaydirici-iz {
    position: relative;
    width: 100%;
    height: 26px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    overflow: hidden;
  }
  .kaydirici-dolu {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    background: color-mix(in srgb, var(--copper-300) 28%, var(--surface-selected));
    border-radius: var(--radius-pill);
    transition: width 140ms ease;
    width: 16%;
  }
  .efor-adim-0 .kaydirici-dolu {
    width: 16%;
  }
  .efor-adim-1 .kaydirici-dolu {
    width: 42%;
  }
  .efor-adim-2 .kaydirici-dolu {
    width: 68%;
  }
  .efor-adim-3 .kaydirici-dolu {
    width: 100%;
  }
  .durak {
    position: relative;
    z-index: 2;
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }
  .durak-nokta {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-pill);
    background: var(--text-muted);
    transition: var(--transition-control);
  }
  .durak.etkin .durak-nokta {
    background: var(--copper-200);
  }
  .durak.secili .durak-nokta {
    width: 12px;
    height: 12px;
    background: var(--copper-200);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--copper-300) 35%, transparent);
  }
  .kaydirici-girdi {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
    margin: 0;
    z-index: 3;
  }
  .efor-etiketler {
    display: flex;
    justify-content: space-between;
    gap: 4px;
  }
  .efor-etiket {
    padding: 2px 4px;
    border: none;
    background: transparent;
    font: var(--type-meta);
    color: var(--text-muted);
    cursor: pointer;
  }
  .efor-etiket.secili {
    color: var(--text-accent);
    font-weight: var(--fw-semibold);
  }
</style>
