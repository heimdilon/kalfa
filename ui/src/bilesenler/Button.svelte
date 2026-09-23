<!--
  Kalfa'nın eylem düğmesi. İzin kararları approve/caution/danger üçlüsünü
  kullanır; karar hiçbir zaman sıradan bir düğme gibi görünmez.
-->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    variant?: 'primary' | 'secondary' | 'ghost' | 'approve' | 'caution' | 'danger';
    size?: 'sm' | 'md' | 'lg';
    iconLeft?: IkonAdi;
    iconRight?: IkonAdi;
    fullWidth?: boolean;
    loading?: boolean;
    class?: string;
    children?: Snippet;
  }

  let {
    variant = 'secondary',
    size = 'md',
    iconLeft,
    iconRight,
    fullWidth = false,
    loading = false,
    disabled = false,
    type = 'button',
    class: sinif,
    children,
    ...geri
  }: Props = $props();

  const ikonBoyu = $derived(size === 'sm' ? 14 : 16);
</script>

<button
  {...geri}
  {type}
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  class={['dugme', `v-${variant}`, `b-${size}`, { tam: fullWidth }, sinif]}
>
  {#if loading}
    <Icon name="loader-circle" size={ikonBoyu} class="donen" />
  {:else if iconLeft}
    <Icon name={iconLeft} size={ikonBoyu} />
  {/if}
  {@render children?.()}
  {#if iconRight}<Icon name={iconRight} size={ikonBoyu} />{/if}
</button>

<style>
  .dugme {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 34px;
    padding: 0 14px;
    font: var(--fw-medium) var(--text-small) / 1 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--fg);
    background: var(--bg);
    border: var(--border-width) solid var(--bd);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .dugme:hover:not(:disabled) {
    background: var(--bg-uzerinde);
  }
  .dugme:active:not(:disabled) {
    background: var(--bg-basili);
    transform: translateY(0.5px);
  }
  .dugme:disabled {
    color: var(--text-muted);
    background: transparent;
    border-color: var(--border-subtle);
    cursor: not-allowed;
    opacity: 0.55;
  }
  .tam {
    width: 100%;
  }
  .b-sm {
    height: 28px;
    padding: 0 10px;
    gap: 6px;
    font-size: var(--text-micro);
  }
  .b-lg {
    height: 42px;
    padding: 0 20px;
    font-size: var(--text-body-size);
  }
  .v-primary {
    --fg: var(--action-primary-fg);
    --bg: var(--action-primary);
    --bd: transparent;
    --bg-uzerinde: var(--action-primary-hover);
    --bg-basili: var(--action-primary-press);
  }
  .v-secondary {
    --fg: var(--text-body);
    --bg: transparent;
    --bd: var(--border-default);
    --bg-uzerinde: var(--surface-raised);
    --bg-basili: var(--surface-sunken);
  }
  .v-ghost {
    --fg: var(--text-secondary);
    --bg: transparent;
    --bd: transparent;
    --bg-uzerinde: var(--surface-raised);
    --bg-basili: var(--surface-sunken);
  }
  .v-approve {
    --fg: var(--status-ok);
    --bg: color-mix(in oklab, var(--green-400) 16%, transparent);
    --bd: color-mix(in oklab, var(--green-400) 45%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--green-400) 26%, transparent);
    --bg-basili: color-mix(in oklab, var(--green-400) 34%, transparent);
  }
  .v-caution {
    --fg: var(--status-warn);
    --bg: color-mix(in oklab, var(--amber-200) 14%, transparent);
    --bd: color-mix(in oklab, var(--amber-200) 42%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--amber-200) 24%, transparent);
    --bg-basili: color-mix(in oklab, var(--amber-200) 32%, transparent);
  }
  .v-danger {
    --fg: var(--status-danger);
    --bg: color-mix(in oklab, var(--red-400) 16%, transparent);
    --bd: color-mix(in oklab, var(--red-400) 45%, transparent);
    --bg-uzerinde: color-mix(in oklab, var(--red-400) 26%, transparent);
    --bg-basili: color-mix(in oklab, var(--red-400) 34%, transparent);
  }
  :global(.donen) {
    animation: kalfa-don 1s linear infinite;
  }
</style>
