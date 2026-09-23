<!--
  Oturumun altındaki bakır çerçeveli giriş: mesaj, model ve izin kipi tek
  çerçevede; kullanıcı ikisini görmeden gönderemez. Enter gönderir,
  Shift+Enter yeni satır açar (1c tasarımı §9.5). Tur sürerken gönder
  düğmesinin yerinde durdurma düğmesi durur.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import IconButton from './IconButton.svelte';
  import Select, { type Secenek } from './Select.svelte';
  import Textarea from './Textarea.svelte';

  interface Props {
    value?: string;
    models: Secenek[];
    model: string;
    modes: Secenek[];
    mode: string;
    /** Tur sürüyor: gönderme kapalı, durdurma açık. */
    running?: boolean;
    disabled?: boolean;
    placeholder?: string;
    onsend?: (metin: string) => void;
    oncancel?: () => void;
    onmodelchange?: (model: string) => void;
    onmodechange?: (kip: string) => void;
    class?: string;
  }

  let {
    value = $bindable(''),
    models,
    model,
    modes,
    mode,
    running = false,
    disabled = false,
    placeholder = m('composer.yer_tutucu'),
    onsend,
    oncancel,
    onmodelchange,
    onmodechange,
    class: sinif,
  }: Props = $props();

  const gonderilebilir = $derived(!running && !disabled && value.trim().length > 0);

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
    }
  }
</script>

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
      icon="box"
      tone="accent"
      label={m('composer.model')}
      value={model}
      options={models}
      disabled={disabled || running}
      onchange={onmodelchange}
    />
    <Select
      size="sm"
      icon="shield"
      label={m('composer.kip')}
      value={mode}
      options={modes}
      disabled={disabled || running}
      onchange={onmodechange}
    />
    <span class="bosluk"></span>
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

<style>
  .bosluk {
    flex: 1;
  }
</style>
