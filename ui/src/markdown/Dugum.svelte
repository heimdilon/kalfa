<!--
  Markdown sözdizimi ağacının bir düğümü. Hiçbir şey HTML olarak çizilmez:
  ham HTML metin olarak görünür; bağlantı yalnızca http/https ise düğmeye
  dönüşür; görselin yerine alt metni yazılır, hiçbir görsel yüklenmez
  (1c tasarımı §9.1).
-->
<script lang="ts">
  import type { AlignType, Parent, RootContent } from 'mdast';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import { m } from '../dil/tr';
  import { guvenliAdres } from './adres';
  import Dugum from './Dugum.svelte';

  interface Props {
    dugum: RootContent;
    /** Akan mesajda kod blokları renklendirilmez (1c tasarımı §9.2). */
    akista: boolean;
    baglantiAc: (adres: string) => void;
  }

  let { dugum, akista, baglantiAc }: Props = $props();

  function hiza(a: AlignType | undefined) {
    return a === 'right' ? 'sag' : a === 'center' ? 'orta' : undefined;
  }
</script>

{#snippet cocuklar(d: Parent)}
  {#each d.children as cocuk, i (i)}
    <Dugum dugum={cocuk} {akista} {baglantiAc} />
  {/each}
{/snippet}

{#if dugum.type === 'text'}
  {dugum.value}
{:else if dugum.type === 'paragraph'}
  <p>{@render cocuklar(dugum)}</p>
{:else if dugum.type === 'heading'}
  <svelte:element this={`h${dugum.depth}`}>{@render cocuklar(dugum)}</svelte:element>
{:else if dugum.type === 'emphasis'}
  <em>{@render cocuklar(dugum)}</em>
{:else if dugum.type === 'strong'}
  <strong>{@render cocuklar(dugum)}</strong>
{:else if dugum.type === 'delete'}
  <del>{@render cocuklar(dugum)}</del>
{:else if dugum.type === 'inlineCode'}
  <code>{dugum.value}</code>
{:else if dugum.type === 'break'}
  <br />
{:else if dugum.type === 'thematicBreak'}
  <hr />
{:else if dugum.type === 'blockquote'}
  <blockquote>{@render cocuklar(dugum)}</blockquote>
{:else if dugum.type === 'list'}
  {#if dugum.ordered}
    <ol start={dugum.start ?? 1}>{@render cocuklar(dugum)}</ol>
  {:else}
    <ul>{@render cocuklar(dugum)}</ul>
  {/if}
{:else if dugum.type === 'listItem'}
  <li class={{ gorev: typeof dugum.checked === 'boolean' }}>
    {#if typeof dugum.checked === 'boolean'}
      <input type="checkbox" checked={dugum.checked} disabled />
    {/if}
    {@render cocuklar(dugum)}
  </li>
{:else if dugum.type === 'code'}
  <CodeBlock
    code={dugum.value}
    lang={dugum.lang ?? undefined}
    highlight={!akista}
    showLineNumbers={false}
  />
{:else if dugum.type === 'html'}
  <code class="ham-html">{dugum.value}</code>
{:else if dugum.type === 'link'}
  {@const adres = guvenliAdres(dugum.url)}
  {#if adres}
    <button type="button" class="baglanti" title={adres} onclick={() => baglantiAc(adres)}>
      {@render cocuklar(dugum)}
    </button>
  {:else}
    {@render cocuklar(dugum)}
  {/if}
{:else if dugum.type === 'linkReference'}
  {@render cocuklar(dugum)}
{:else if dugum.type === 'image' || dugum.type === 'imageReference'}
  <span class="gorsel">[{dugum.alt || m('markdown.gorsel')}]</span>
{:else if dugum.type === 'table'}
  <div class="tablo">
    <table>
      {#if dugum.children[0]}
        <thead>
          <tr>
            {#each dugum.children[0].children as hucre, j (j)}
              <th class={hiza(dugum.align?.[j])}>{@render cocuklar(hucre)}</th>
            {/each}
          </tr>
        </thead>
      {/if}
      <tbody>
        {#each dugum.children.slice(1) as satir, i (i)}
          <tr>
            {#each satir.children as hucre, j (j)}
              <td class={hiza(dugum.align?.[j])}>{@render cocuklar(hucre)}</td>
            {/each}
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{:else if dugum.type === 'footnoteReference'}
  <sup>[{dugum.label ?? dugum.identifier}]</sup>
{:else if dugum.type === 'footnoteDefinition'}
  <div class="dipnot">[{dugum.label ?? dugum.identifier}] {@render cocuklar(dugum)}</div>
{/if}
