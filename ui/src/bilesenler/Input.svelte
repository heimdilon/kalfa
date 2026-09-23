<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLInputAttributes, 'class' | 'size' | 'type' | 'value'> {
    value?: string;
    type?: 'text' | 'password' | 'search';
    iconLeft?: IkonAdi;
    iconRight?: IkonAdi;
    /** Tek aralıklı değer: anahtar, istek kimliği, yol. */
    mono?: boolean;
    size?: 'md' | 'lg';
    invalid?: boolean;
    class?: string;
  }

  let {
    value = $bindable(''),
    type = 'text',
    iconLeft,
    iconRight,
    mono = false,
    size = 'md',
    invalid = false,
    disabled = false,
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<div class={['girdi', `b-${size}`, { gecersiz: invalid, 'devre-disi': disabled }, sinif]}>
  {#if iconLeft}<span class="ikon-yeri"><Icon name={iconLeft} size={15} /></span>{/if}
  <input
    {...geri}
    {type}
    {value}
    {disabled}
    aria-invalid={invalid || undefined}
    class={{ mono }}
    oninput={(e) => (value = e.currentTarget.value)}
  />
  {#if iconRight}<span class="ikon-yeri"><Icon name={iconRight} size={15} /></span>{/if}
</div>

<style>
  .girdi {
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--control-height);
    padding: 0 10px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-md);
    transition: var(--transition-control);
  }
  .b-lg {
    height: var(--control-height-lg);
  }
  .girdi:focus-within {
    border-color: var(--border-accent);
    box-shadow: var(--shadow-focus);
  }
  .gecersiz,
  .gecersiz:focus-within {
    border-color: var(--status-danger);
  }
  .devre-disi {
    opacity: 0.55;
  }
  .ikon-yeri {
    display: inline-flex;
    color: var(--text-muted);
  }
  input {
    flex: 1;
    min-width: 0;
    border: none;
    outline: none;
    background: transparent;
    color: var(--text-body);
    font: var(--type-small);
  }
  input.mono {
    font: var(--type-meta);
  }
</style>
