<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLTextareaAttributes, 'class' | 'value'> {
    value?: string;
    rows?: number;
    /** Bakır kenarlık hep açık: Composer kullanır. */
    accent?: boolean;
    /** Metin alanının çerçevesi içinde, metnin altında duran denetimler. */
    footer?: Snippet;
    /** Metin alanı öğesi; odak vermek için bağlanır. */
    element?: HTMLTextAreaElement;
    class?: string;
  }

  let {
    value = $bindable(''),
    rows = 3,
    accent = false,
    disabled = false,
    footer,
    element = $bindable(),
    class: sinif,
    ...geri
  }: Props = $props();
</script>

<div class={['metin-alani', { vurgulu: accent, 'devre-disi': disabled }, sinif]}>
  <textarea bind:this={element} {...geri} {rows} {disabled} bind:value></textarea>
  {#if footer}<div class="alt">{@render footer()}</div>{/if}
</div>

<style>
  .metin-alani {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: 10px 12px;
    transition: var(--transition-control);
  }
  .metin-alani:focus-within,
  .vurgulu {
    border: var(--border-width-accent) solid var(--border-accent);
  }
  .devre-disi {
    opacity: 0.55;
  }
  textarea {
    width: 100%;
    border: none;
    outline: none;
    background: transparent;
    resize: none;
    color: var(--text-body);
    font: var(--type-body);
  }
  .alt {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
</style>
