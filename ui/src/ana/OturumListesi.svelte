<!--
  Oturum listesi (1c tasarımı §5.4, §8.1): kayıtlardan okunan oturumlar, gün
  gruplarıyla. Açık oturumun anlık hâli (çalışıyor, onay bekliyor, şart
  bekliyor) kayıttaki son durumdan önce gelir.
-->
<script lang="ts">
  import { SvelteSet } from 'svelte/reactivity';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import Input from '../bilesenler/Input.svelte';
  import SessionRow from '../bilesenler/SessionRow.svelte';
  import { m } from '../dil/tr';
  import type { OturumSatiri } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { GUN_GRUBU_ADI, gruplara, satirBasligi, satirDurumu, zamanEtiketi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  let arama = $state('');
  let tumunuGoster = $state(false);
  const kapaliProjeler = new SvelteSet<string>();

  function projeAdi(yol: string | null): string {
    if (!yol) return 'kalfa';
    const parcalar = yol.replace(/\\/g, '/').split('/').filter(Boolean);
    return parcalar.at(-1) ?? yol;
  }

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

  const projeler = $derived.by(() => {
    const aranan = arama.trim().toLocaleLowerCase('tr-TR');
    const suzulmus = aranan
      ? satirlar.filter((s) =>
          [satirBasligi(s), s.calisma_alani ?? ''].some((t) =>
            t.toLocaleLowerCase('tr-TR').includes(aranan),
          ),
        )
      : satirlar;
    const harita = new Map<string, { ad: string; yol: string | null; ogeler: OturumSatiri[] }>();
    for (const s of suzulmus) {
      const anahtar = s.calisma_alani ?? '__varsayilan__';
      const mevcut = harita.get(anahtar);
      if (mevcut) {
        mevcut.ogeler.push(s);
      } else {
        harita.set(anahtar, {
          ad: projeAdi(s.calisma_alani),
          yol: s.calisma_alani,
          ogeler: [s],
        });
      }
    }
    return [...harita.entries()].map(([anahtar, deger]) => ({ anahtar, ...deger }));
  });

  function projeAcKapat(anahtar: string) {
    if (kapaliProjeler.has(anahtar)) {
      kapaliProjeler.delete(anahtar);
    } else {
      kapaliProjeler.add(anahtar);
    }
  }
</script>

<aside class="liste" aria-label={m('liste.baslik')}>
  <div class="ust">
    <div class="marka-baslik">
      <span class="kalfa-etiket">{m('uygulama.adi')}</span>
      <h2>{m('liste.baslik')}</h2>
    </div>
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
    {#if projeler.length > 0}
      <div class="bolum-baslik">
        <span>{m('liste.projeler')}</span>
        {#if gruplar[0]}
          <span class="grup">{m(GUN_GRUBU_ADI[gruplar[0].grup])}</span>
        {/if}
      </div>

      {#each projeler as p (p.anahtar)}
        {@const kapali = kapaliProjeler.has(p.anahtar)}
        {@const gosterilenler = tumunuGoster ? p.ogeler : p.ogeler.slice(0, 8)}
        <div class="proje-grubu">
          <button
            type="button"
            class="proje-baslik"
            aria-expanded={!kapali}
            onclick={() => projeAcKapat(p.anahtar)}
          >
            <Icon name={kapali ? 'chevron-right' : 'folder-open'} size={14} />
            <span class="proje-adi">{p.ad}</span>
            <span class="proje-sayi">{p.ogeler.length}</span>
          </button>

          {#if !kapali}
            <div class="proje-icerik">
              {#each gosterilenler as s (s.dosya)}
                {@const anlik = s.oturum ? (durum.ekranlar.get(s.oturum)?.anlik ?? null) : null}
                {@const d = satirDurumu(s.durum, anlik)}
                <div class={['satir', { secili: s.oturum !== null && s.oturum === durum.secili }]}>
                  <SessionRow
                    title={satirBasligi(s)}
                    preview={s.durum.tip === 'Okunamadi' ? s.durum.neden : undefined}
                    time={zamanEtiketi(s.son_degisiklik_ms, Date.now())}
                    status={d.status}
                    statusLabel={d.etiket}
                    active={s.oturum !== null && s.oturum === durum.secili}
                    disabled={s.oturum === null}
                    onclick={() => s.oturum && durum.oturumSec(s.oturum)}
                  />
                  {#if s.oturum}
                    {@const oturum = s.oturum}
                    <span class="sil">
                      <IconButton
                        icon="trash-2"
                        size="sm"
                        label={m('liste.sil', { baslik: satirBasligi(s) })}
                        onclick={() => durum.oturumuSil(oturum)}
                      />
                    </span>
                  {/if}
                </div>
              {/each}
              {#if p.ogeler.length > 8 && !tumunuGoster}
                <button type="button" class="daha-fazla" onclick={() => (tumunuGoster = true)}>
                  {m('liste.daha_fazla')}
                </button>
              {/if}
            </div>
          {/if}
        </div>
      {/each}

      {#each gruplar.slice(1) as g (g.grup)}
        <span class="gizli-erisilebilir">{m(GUN_GRUBU_ADI[g.grup])}</span>
      {/each}
    {:else}
      <p class="bos">{arama.trim() ? m('liste.sonuc_yok') : m('liste.bos')}</p>
    {/if}
  </div>
</aside>

<style>
  .liste {
    width: 100%;
    height: 100%;
    flex: 1;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    overflow: hidden;
  }
  .ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px 8px;
  }
  .marka-baslik {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
  }
  .kalfa-etiket {
    font: var(--fw-semibold) 15px / 1 var(--font-sans);
    color: var(--text-title);
  }
  h2 {
    margin: 0;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .ara {
    padding: 0 12px 10px;
  }
  .satirlar {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
  }
  .bolum-baslik {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px 4px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .grup {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .proje-grubu {
    margin-bottom: 6px;
  }
  .proje-baslik {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 6px 8px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font: var(--fw-medium) var(--text-small) / 1.2 var(--font-sans);
    cursor: pointer;
    text-align: left;
  }
  .proje-baslik:hover {
    background: var(--surface-raised);
    color: var(--text-title);
  }
  .proje-adi {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proje-sayi {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .proje-icerik {
    padding-left: 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  /* Silme düğmesi satırın üstünde durur; seçili satırda, fare ya da odak
     gelince görünür. */
  .satir {
    position: relative;
  }
  .sil {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: var(--transition-control);
  }
  .satir:hover .sil,
  .satir:focus-within .sil,
  .secili .sil {
    opacity: 1;
  }
  .daha-fazla {
    padding: 6px 10px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font: var(--type-meta);
    text-align: left;
    cursor: pointer;
  }
  .daha-fazla:hover {
    color: var(--text-body);
  }
  .bos {
    margin: 16px 8px;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
