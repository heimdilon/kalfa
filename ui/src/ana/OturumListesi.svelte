<!--
  Oturum listesi (1c tasarımı §5.4, §8.1): kayıtlardan okunan oturumlar, gün
  gruplarıyla. Açık oturumun anlık hâli (çalışıyor, onay bekliyor, şart
  bekliyor) kayıttaki son durumdan önce gelir.
-->
<script lang="ts">
  import IconButton from '../bilesenler/IconButton.svelte';
  import Input from '../bilesenler/Input.svelte';
  import SessionRow from '../bilesenler/SessionRow.svelte';
  import { m } from '../dil/tr';
  import type { OturumSatiri } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { GUN_GRUBU_ADI, gruplara, satirBasligi, satirDurumu, zamanEtiketi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  let arama = $state('');

  /** Listeye henüz düşmemiş yeni oturumlar en üstte görünür. */
  const satirlar = $derived.by((): OturumSatiri[] => {
    const listede = new Set(durum.liste.map((s) => s.oturum));
    const yeniler: OturumSatiri[] = [];
    for (const [oturum, ekran] of durum.ekranlar) {
      if (listede.has(oturum)) continue;
      yeniler.push({
        oturum,
        dosya: oturum,
        baslik: null,
        calisma_alani: ekran.calisma_alani,
        son_degisiklik_ms: Date.now(),
        durum: { tip: 'Bos' },
      });
    }
    return [...yeniler, ...durum.liste];
  });

  const gruplar = $derived.by(() => {
    const aranan = arama.trim().toLocaleLowerCase('tr-TR');
    const suzulmus = aranan
      ? satirlar.filter((s) =>
          [satirBasligi(s), s.calisma_alani ?? ''].some((t) =>
            t.toLocaleLowerCase('tr-TR').includes(aranan),
          ),
        )
      : satirlar;
    return gruplara(suzulmus, (s) => s.son_degisiklik_ms, Date.now());
  });
</script>

<aside class="liste" aria-label={m('liste.baslik')}>
  <div class="ust">
    <h2>{m('liste.baslik')}</h2>
    <IconButton icon="plus" label={m('liste.yeni')} onclick={() => durum.yeniOturum()} />
  </div>
  <div class="ara">
    <Input
      type="search"
      iconLeft="search"
      placeholder={m('liste.ara')}
      aria-label={m('liste.ara')}
      bind:value={arama}
    />
  </div>
  <div class="satirlar">
    {#each gruplar as g (g.grup)}
      <div class="grup">{m(GUN_GRUBU_ADI[g.grup])}</div>
      {#each g.ogeler as s (s.dosya)}
        {@const anlik = s.oturum ? (durum.ekranlar.get(s.oturum)?.anlik ?? null) : null}
        {@const d = satirDurumu(s.durum, anlik)}
        <SessionRow
          title={satirBasligi(s)}
          preview={s.durum.tip === 'Okunamadi' ? s.durum.neden : (s.calisma_alani ?? undefined)}
          time={zamanEtiketi(s.son_degisiklik_ms, Date.now())}
          status={d.status}
          statusLabel={d.etiket}
          active={s.oturum !== null && s.oturum === durum.secili}
          disabled={s.oturum === null}
          onclick={() => s.oturum && durum.oturumSec(s.oturum)}
        />
      {/each}
    {:else}
      <p class="bos">{arama.trim() ? m('liste.sonuc_yok') : m('liste.bos')}</p>
    {/each}
  </div>
</aside>

<style>
  .liste {
    width: var(--list-width);
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px 10px;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 19px / 1 var(--font-sans);
    color: var(--text-title);
  }
  .ara {
    padding: 0 16px 12px;
  }
  .satirlar {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 12px;
  }
  .grup {
    padding: 10px 8px 6px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .bos {
    margin: 16px 8px;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
