<!--
  Kod ve araç çıktısı. `highlight` açıksa ve dil destekleniyorsa Shiki
  tokenlarıyla renklendirilir; renkler `style:` yönergesiyle verilir (CSP
  satır içi stil özniteliğine izin vermez). Renklendirme modülü yalnızca
  gerekince yüklenir.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import type { Parca } from '../markdown/vurgula';

  interface Props {
    code: string;
    /** Markdown kod bloğunun dil etiketi. */
    lang?: string;
    highlight?: boolean;
    /** İlk satırın numarası: araç çıktısı çoğu zaman dosyanın bir dilimidir. */
    startLine?: number;
    showLineNumbers?: boolean;
    /** Çıktı kırpıldıysa "···" devam işareti. */
    truncated?: boolean;
    /** CSS uzunluğu, ör. "320px". */
    maxHeight?: string;
    class?: string;
  }

  let {
    code,
    lang,
    highlight = false,
    startLine = 1,
    showLineNumbers = true,
    truncated = false,
    maxHeight,
    class: sinif,
  }: Props = $props();

  const satirlar = $derived(code.replace(/\n$/, '').split('\n'));
  let renkli: Parca[][] | null = $state.raw(null);

  $effect(() => {
    renkli = null;
    if (!highlight || !lang) return;
    const [kod, dil] = [code, lang];
    let gecersiz = false;
    import('../markdown/vurgula')
      .then(({ vurgula }) => vurgula(kod, dil))
      .then((sonuc) => {
        if (!gecersiz) renkli = sonuc;
      })
      .catch(() => {
        // Renklendirilemeyen kod düz gösterilir.
      });
    return () => {
      gecersiz = true;
    };
  });
</script>

<div class={['kod', sinif]}>
  <div class="kaydirma" style:max-height={maxHeight}>
    <table>
      <tbody>
        {#each satirlar as satir, i (i)}
          <tr>
            {#if showLineNumbers}<td class="no">{startLine + i}</td>{/if}
            <td class="satir"
              >{#if renkli?.[i]}{#each renkli[i] as parca, j (j)}<span style:color={parca.renk}
                    >{parca.metin}</span
                  >{/each}{:else}{satir}{/if}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if truncated}
    <div class="devami" title={m('kod.kirpildi')}>
      <span aria-hidden="true">···</span>
      <span class="gizli-erisilebilir">{m('kod.kirpildi')}</span>
    </div>
  {/if}
</div>

<style>
  .kod {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }
  .kaydirma {
    overflow: auto;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font: var(--type-code);
  }
  .no {
    width: 1px;
    white-space: nowrap;
    text-align: right;
    padding: 0 12px 0 14px;
    color: var(--warm-400);
    user-select: none;
    vertical-align: top;
  }
  .satir {
    padding: 0 14px;
    white-space: pre;
    color: var(--code-punct);
  }
  .no + .satir {
    padding-left: 0;
  }
  .devami {
    text-align: center;
    padding: 4px 0 8px;
    color: var(--text-muted);
    font: var(--type-meta);
  }
</style>
