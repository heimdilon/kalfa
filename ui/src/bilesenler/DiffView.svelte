<script lang="ts">
  import type { DiffSatiri } from './diff';

  interface Props {
    /** Birleşik diff blok başlığı, ör. "@@ -42,10 +42,18 @@ async fn main()". */
    hunkHeader?: string;
    lines: DiffSatiri[];
    class?: string;
  }

  let { hunkHeader, lines, class: sinif }: Props = $props();

  const ISARET = { add: '+', del: '−', ctx: ' ' } as const;
</script>

<div class={['diff', sinif]}>
  {#if hunkHeader}<div class="blok-basligi">{hunkHeader}</div>{/if}
  <table>
    <tbody>
      {#each lines as satir, i (i)}
        <tr class={`t-${satir.kind}`}>
          <td class="no">{satir.old ?? ''}</td>
          <td class="no yeni">{satir.neu ?? ''}</td>
          <td class="isaret">{ISARET[satir.kind]}</td>
          <td class="metin">{satir.text}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .diff {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: auto;
    font: var(--type-code);
  }
  .blok-basligi {
    padding: 6px 14px;
    color: var(--text-muted);
    background: color-mix(in oklab, var(--warm-200) 5%, transparent);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  td {
    color: var(--fg, var(--code-punct));
  }
  .no {
    width: 1px;
    white-space: nowrap;
    text-align: right;
    padding: 0 8px 0 12px;
    color: var(--warm-400);
    user-select: none;
    background: var(--oluk, transparent);
  }
  .no.yeni {
    padding: 0 10px 0 4px;
  }
  .isaret {
    width: 1px;
    padding: 0 8px;
    user-select: none;
  }
  .metin {
    padding: 0 14px 0 0;
    white-space: pre;
  }
  .t-add {
    background: var(--diff-add-bg);
    --fg: var(--diff-add-fg);
    --oluk: var(--diff-add-gutter);
  }
  .t-del {
    background: var(--diff-del-bg);
    --fg: var(--diff-del-fg);
    --oluk: var(--diff-del-gutter);
  }
</style>
