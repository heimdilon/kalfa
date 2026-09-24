<!--
  Kabuk çağrısının canlı çıktısı (aşama 2 tasarımı §10.1). stdout ve stderr
  aynı akışta, stderr ayrı renkte. ANSI'den yalnızca renk ve kalınlık çizilir;
  metin düğümü olarak eklenir, HTML olarak yorumlanmaz. Kullanıcı en alttaysa
  yeni çıktıyla birlikte aşağı kayar.
-->
<script lang="ts">
  import { m } from '../dil/tr';
  import type { Terminal } from '../durum/terminal';
  import { ansiParcala } from './ansi';

  let { terminal }: { terminal: Terminal } = $props();

  let kutu = $state<HTMLPreElement>();
  let altta = true;

  $effect.pre(() => {
    void terminal.satirlar;
    if (kutu) altta = kutu.scrollHeight - kutu.scrollTop - kutu.clientHeight < 24;
  });

  $effect(() => {
    void terminal.satirlar;
    if (kutu && altta) kutu.scrollTop = kutu.scrollHeight;
  });
</script>

{#if terminal.atilan_satir > 0}
  <p class="atilan">{m('terminal.atilan', { sayi: terminal.atilan_satir })}</p>
{/if}
<pre class="terminal" bind:this={kutu} aria-label={m('terminal.etiket')}
  >{#each terminal.satirlar as satir, i (terminal.atilan_satir + i)}{#if i > 0}{'\n'}{/if}<span
      class={['satir', `a-${satir.akis}`]}
      >{#each ansiParcala(satir.metin) as p, j (j)}<span
          class={[p.renk && `r-${p.renk}`, { kalin: p.kalin }]}>{p.metin}</span
        >{/each}{#if satir.kesik}<span class="kesik" title={m('terminal.kesik')}>…</span>{/if}</span
    >{/each}</pre>

<style>
  .terminal {
    margin: 0;
    max-height: 320px;
    overflow: auto;
    padding: 10px 12px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font: var(--text-small) / 1.5 var(--font-mono);
    color: var(--text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .a-Hata {
    color: var(--status-warn);
  }
  .kesik {
    color: var(--text-muted);
  }
  .a-Bilgi {
    color: var(--text-muted);
    font-style: italic;
  }
  .r-kirmizi {
    color: var(--status-danger);
  }
  .r-yesil {
    color: var(--status-ok);
  }
  .r-sari {
    color: var(--status-warn);
  }
  .r-mavi {
    color: var(--status-info);
  }
  .r-soluk {
    color: var(--text-muted);
  }
  .kalin {
    font-weight: var(--fw-semibold);
  }
  .atilan {
    margin: 0 0 6px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
