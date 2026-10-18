<!--
  Kenar çubuğunun üstü (1c tasarımı §5.4, §8.1): yeni oturum, arama ve
  çalışma alanına göre gruplanan oturumlar. Açık oturumun anlık hâli
  (çalışıyor, onay bekliyor, şart bekliyor) kayıttaki son durumdan önce gelir.
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
  import {
    GUN_GRUBU_ADI,
    gruplara,
    projeAdi,
    satirBasligi,
    satirDurumu,
    zamanEtiketi,
  } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  let arama = $state('');
  let tumunuGoster = $state(false);
  const kapaliProjeler = new SvelteSet<string>();

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

  const suzulmus = $derived.by(() => {
    const aranan = arama.trim().toLocaleLowerCase('tr-TR');
    if (!aranan) return satirlar;
    return satirlar.filter((s) =>
      [satirBasligi(s), s.calisma_alani ?? ''].some((t) =>
        t.toLocaleLowerCase('tr-TR').includes(aranan),
      ),
    );
  });

  // Gün grupları ekranda gösterilmez; ekran okuyucu için okunur.
  const gruplar = $derived(gruplara(suzulmus, (s) => s.son_degisiklik_ms, Date.now()));

  const projeler = $derived.by(() => {
    const harita = new Map<string, { ad: string; yol: string | null; ogeler: OturumSatiri[] }>();
    for (const s of suzulmus) {
      const anahtar = s.calisma_alani ?? '__varsayilan__';
      const mevcut = harita.get(anahtar);
      if (mevcut) {
        mevcut.ogeler.push(s);
      } else {
        harita.set(anahtar, { ad: projeAdi(s.calisma_alani), yol: s.calisma_alani, ogeler: [s] });
      }
    }
    const ham = [...harita.entries()].map(([anahtar, deger]) => ({ anahtar, ...deger }));
    const adSayisi = new Map<string, number>();
    for (const p of ham) {
      adSayisi.set(p.ad, (adSayisi.get(p.ad) ?? 0) + 1);
    }
    return ham.map((p) => ({
      ...p,
      altYol: (adSayisi.get(p.ad) ?? 0) > 1 ? p.yol : null,
    }));
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
  <!-- Windows'ta marka ve kenar düğmesi başlık çubuğundadır. -->
  {#if !durum.ozelCerceve}
    <div class="marka">
      <button
        type="button"
        class="marka-dugme"
        title={`${m('uygulama.adi')} — ${m('uygulama.slogan')}`}
        onclick={() => durum.oturumlaraDon()}
      >
        <span class="amblem" aria-hidden="true"><Icon name="hexagon" size={13} /></span>
        <span>{m('uygulama.adi')}</span>
      </button>
      <IconButton
        icon="panel-left"
        size="sm"
        label={m('kenar.gizle')}
        onclick={() => (durum.kenarAcik = false)}
      />
    </div>
  {/if}

  <button
    type="button"
    class="yeni"
    aria-label={m('liste.yeni')}
    onclick={() => durum.yeniOturum()}
  >
    <Icon name="square-pen" size={16} />
    <span>{m('liste.yeni')}</span>
    <kbd aria-hidden="true">Ctrl N</kbd>
  </button>

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
      <div class="bolum-baslik">{m('liste.projeler')}</div>
      {#each gruplar as g (g.grup)}
        <span class="gizli-erisilebilir">{m(GUN_GRUBU_ADI[g.grup])}</span>
      {/each}

      {#each projeler as p (p.anahtar)}
        {@const kapali = kapaliProjeler.has(p.anahtar)}
        {@const gosterilenler = tumunuGoster ? p.ogeler : p.ogeler.slice(0, 8)}
        <div class="proje-grubu">
          <button
            type="button"
            class="proje-baslik"
            title={p.yol ?? undefined}
            aria-expanded={!kapali}
            onclick={() => projeAcKapat(p.anahtar)}
          >
            <Icon name={kapali ? 'folder' : 'folder-open'} size={14} />
            <span class="proje-metin">
              <span class="proje-adi">{p.ad}</span>
              {#if p.altYol}
                <span class="proje-yol">{p.altYol}</span>
              {/if}
            </span>
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
    {:else}
      <p class="bos">{arama.trim() ? m('liste.sonuc_yok') : m('liste.bos')}</p>
    {/if}
  </div>
</aside>

<style>
  .liste {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .marka {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 8px 8px 10px;
  }
  .marka-dugme {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 2px 4px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-title);
    font: var(--fw-semibold) var(--text-body-size) / 1 var(--font-sans);
    cursor: pointer;
  }
  .amblem {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: var(--radius-sm);
    background: var(--action-primary);
    color: var(--action-primary-fg);
  }
  .yeni {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    margin: 0 8px 6px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-title);
    font: var(--fw-medium) var(--text-small) / 1 var(--font-sans);
    text-align: left;
    cursor: pointer;
    transition: var(--transition-control);
  }
  .yeni:hover {
    background: var(--surface-raised);
  }
  .yeni span {
    flex: 1;
  }
  kbd {
    font: var(--text-nano) / 1 var(--font-sans);
    color: var(--text-muted);
  }
  .ara {
    padding: 0 8px 8px;
  }
  .satirlar {
    flex: 1;
    overflow-y: auto;
    padding: 0 8px 12px;
  }
  .bolum-baslik {
    padding: 10px 10px 4px;
    font: var(--fw-medium) var(--text-micro) / 1.3 var(--font-sans);
    color: var(--text-muted);
  }
  .proje-grubu {
    margin-bottom: 4px;
  }
  .proje-baslik {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 10px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font: var(--type-small);
    cursor: pointer;
    text-align: left;
  }
  .proje-baslik:hover {
    color: var(--text-title);
  }
  .proje-metin {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .proje-adi {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proje-yol {
    font: var(--type-meta);
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .proje-icerik {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  /* Silme düğmesi satırın sağında durur; fare ya da odak gelince saatin
     yerini alır. */
  .satir {
    position: relative;
  }
  .sil {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    opacity: 0;
    transition: opacity var(--dur-fast) var(--ease-standard);
  }
  .satir:hover .sil,
  .satir:focus-within .sil {
    opacity: 1;
  }
  .satir:hover :global(.zaman),
  .satir:focus-within :global(.zaman) {
    visibility: hidden;
  }
  .daha-fazla {
    padding: 6px 10px 6px 28px;
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
    margin: 16px 10px;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
