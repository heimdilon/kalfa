<!--
  Turun özeti (aşama 3 tasarımı §5.5, §5.11): dosyalar, komutlar ve EVREN
  istekleri. "Turu geri al…" yalnızca en az bir dosya geri alınabiliyorsa
  görünür; önizlemeyi ister. Araç çağrısı olmayan turun özeti çizilmez.
-->
<script lang="ts">
  import TurnSummary from '../bilesenler/TurnSummary.svelte';
  import { m } from '../dil/tr';
  import type { TurOzeti } from '../ipc/tipler';
  import { turOzetiGorunumu } from './geriAlma';

  let {
    ozet,
    geriAlindi,
    ongeri,
  }: { ozet: TurOzeti; geriAlindi: boolean; ongeri?: () => void } = $props();

  const g = $derived(turOzetiGorunumu(ozet, geriAlindi));
</script>

{#if !g.bos}
  <TurnSummary
    class="tur-ozeti-blogu"
    title={m('tur_ozeti.baslik')}
    items={g.satirlar}
    note={g.notlar}
    onUndo={g.geriAlinabilir ? ongeri : undefined}
    undoLabel={m('tur_ozeti.geri_al')}
  />
{/if}
