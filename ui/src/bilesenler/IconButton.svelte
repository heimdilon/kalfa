<script lang="ts">
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    icon: IkonAdi;
    /** Zorunlu: aria-label ve ipucu olur. */
    label: string;
    size?: 'sm' | 'md' | 'lg';
    variant?: 'ghost' | 'primary';
    active?: boolean;
    class?: string;
  }

  let {
    icon,
    label,
    size = 'md',
    variant = 'ghost',
    active = false,
    class: sinif,
    ...geri
  }: Props = $props();

  const ikonBoyu = $derived(size === 'sm' ? 14 : size === 'lg' ? 18 : 16);
</script>

<button
  type="button"
  {...geri}
  aria-label={label}
  title={label}
  aria-pressed={active || undefined}
  class={['ikon-dugme', `b-${size}`, `v-${variant}`, { etkin: active }, sinif]}
>
  <Icon name={icon} size={ikonBoyu} />
</button>

<style>
  .ikon-dugme {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    border-radius: var(--radius-md);
    border: var(--border-width) solid transparent;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .b-sm {
    width: 26px;
    height: 26px;
  }
  .b-lg {
    width: 40px;
    height: 40px;
  }
  .v-ghost:hover:not(:disabled) {
    background: var(--surface-raised);
  }
  .etkin {
    background: var(--surface-selected);
    color: var(--text-accent);
    border-color: var(--border-accent);
  }
  .v-primary {
    background: var(--action-primary);
    color: var(--action-primary-fg);
  }
  .v-primary:hover:not(:disabled) {
    background: var(--action-primary-hover);
  }
  .ikon-dugme:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
</style>
