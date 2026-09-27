<!-- Gerçek bir onay kutusunun üstüne çizilir: klavye ve ekran okuyucu desteği yerleşiktir. -->
<script lang="ts">
  import Icon from './Icon.svelte';

  interface Props {
    checked?: boolean;
    /** Karışık durum: kısmen seçili dosya listesinin "Tümünü seç" satırı. */
    indeterminate?: boolean;
    label?: string;
    /** Sağdaki tek aralıklı not, ör. "+12 −4". */
    hint?: string;
    disabled?: boolean;
    onchange?: (sonraki: boolean) => void;
    class?: string;
  }

  let {
    checked = false,
    indeterminate = false,
    label,
    hint,
    disabled = false,
    onchange,
    class: sinif,
  }: Props = $props();

  let girdi = $state<HTMLInputElement>();

  $effect(() => {
    if (girdi) girdi.indeterminate = indeterminate;
  });
</script>

<label class={['onay-kutusu', { 'devre-disi': disabled }, sinif]}>
  <input
    bind:this={girdi}
    type="checkbox"
    class="gizli-erisilebilir"
    {checked}
    {disabled}
    onchange={(e) => {
      // Denetimli: kutunun durumunu `checked` belirler. Üst bileşen değeri
      // değiştirmezse (ör. yerel pencerede vazgeçildi) kutu eski hâline döner.
      const istenen = e.currentTarget.checked;
      e.currentTarget.checked = checked;
      onchange?.(istenen);
    }}
  />
  <span class={['kutu', { dolu: checked || indeterminate }]} aria-hidden="true">
    {#if indeterminate}
      <Icon name="minus" size={12} strokeWidth={3} />
    {:else if checked}
      <Icon name="check" size={12} strokeWidth={3} />
    {/if}
  </span>
  {#if label}<span class="etiket">{label}</span>{/if}
  {#if hint}<span class="ipucu">{hint}</span>{/if}
</label>

<style>
  .onay-kutusu {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 9px;
    cursor: pointer;
  }
  .devre-disi {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .kutu {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 17px;
    height: 17px;
    flex: none;
    border-radius: var(--radius-xs);
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-strong);
    color: var(--action-primary-fg);
    transition: var(--transition-control);
  }
  .dolu {
    background: var(--action-primary);
    border-color: var(--action-primary);
  }
  input:focus-visible + .kutu {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .etiket {
    font: var(--type-small);
    color: var(--text-body);
  }
  .ipucu {
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
