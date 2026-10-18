<!--
  Yazan araç çağrısının dosya değişiklikleri (masaüstü arayüzü tasarımı §4).
  Dosya başına ad, "+N −M" ve diff'in ilk 20 satırı; fazlası "Tamamını
  göster" ile açılır. Diff çekirdekten süzülmüş gelir: gizli dosyada, çok
  dosyalı yazmada ve 64 KiB üstünde yoktur. Çağrı başarılı bitmediyse yazma
  yarıda kalmış olabilir: değişiklik "uygulanmış olabilir" diye işaretlenir
  (dosya kesilip yazılırken G/Ç hatası ya da yazma sonrası denetim hatası).
  Hunk'sız diff aynı içeriğin yazıldığı anlamına gelir.
-->
<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle, diffKirp } from '../bilesenler/diff';
  import { m } from '../dil/tr';
  import type { DosyaDegisikligi } from '../ipc/tipler';

  const ONIZLEME_SATIRI = 20;

  let {
    degisiklikler,
    belirsiz = false,
  }: { degisiklikler: DosyaDegisikligi[]; belirsiz?: boolean } = $props();

  const acik = new SvelteSet<number>();

  const dosyalar = $derived(
    degisiklikler.map((d) => {
      const parcalar = d.diff ? diffCozumle(d.diff) : [];
      const satir = parcalar.reduce((t, p) => t + p.satirlar.length, 0);
      return { ...d, parcalar, kisa: diffKirp(parcalar, ONIZLEME_SATIRI), satir };
    }),
  );
</script>

<div class="degisiklikler">
  {#each dosyalar as d, i (`${i}:${d.yol}`)}
    <div class="dosya">
      <div class="ust">
        <span class="yol" title={d.yol}>{d.yol}</span>
        <span class="arti">+{d.eklenen}</span>
        <span class="eksi">−{d.silinen}</span>
        {#if belirsiz}
          <span title={m('arac.diff.belirsiz_aciklama')}>
            <Badge tone="warn">{m('arac.diff.belirsiz')}</Badge>
          </span>
        {/if}
      </div>
      {#if d.parcalar.length > 0}
        {@const gosterilen = acik.has(i) ? d.parcalar : d.kisa.parcalar}
        {#each gosterilen as p, j (j)}
          <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
        {/each}
        {#if d.kisa.kirpildi}
          <Button
            size="sm"
            variant="ghost"
            onclick={() => (acik.has(i) ? acik.delete(i) : acik.add(i))}
          >
            {acik.has(i) ? m('arac.diff.daralt') : m('arac.diff.tamami', { sayi: d.satir })}
          </Button>
        {/if}
      {:else if d.diff != null}
        <p class="yok">{m('arac.diff.ayni')}</p>
      {:else}
        <p class="yok">{m('arac.diff.yok')}</p>
      {/if}
    </div>
  {/each}
</div>

<style>
  .degisiklikler {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .dosya {
    display: flex;
    flex-direction: column;
    gap: 6px;
    align-items: flex-start;
  }
  .dosya :global(.diff) {
    align-self: stretch;
  }
  .ust {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font: var(--type-small);
  }
  .yol {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    color: var(--text-body);
  }
  .arti {
    color: var(--status-ok);
  }
  .eksi {
    color: var(--status-danger);
  }
  .yok {
    margin: 0;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
