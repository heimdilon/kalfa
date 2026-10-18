<!--
  Sağ panel (masaüstü arayüzü tasarımı §6): Ortam, Değişiklikler ve Dosyalar
  bölümleri. Kabı (sürüklenebilir tutamaç ve genişlik) Ana'dadır. Ortam'daki
  bir çıktıya tıklamak Değişiklikler'de o dosyayı seçer.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle } from '../bilesenler/diff';
  import Icon from '../bilesenler/Icon.svelte';
  import type { IkonAdi } from '../bilesenler/ikonlar';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { m, type MetinAnahtari } from '../dil/tr';
  import type { AnaDurum, SagBolum } from './anaDurum.svelte';
  import DosyalarBolumu from './DosyalarBolumu.svelte';
  import OrtamBolumu from './OrtamBolumu.svelte';

  let { durum, bolum }: { durum: AnaDurum; bolum: SagBolum } = $props();

  const BASLIK: Record<SagBolum, [IkonAdi, MetinAnahtari]> = {
    ortam: ['info', 'sag.ortam'],
    degisiklikler: ['code', 'sag.degisiklikler'],
    dosyalar: ['folder', 'sag.dosyalar'],
  };

  let seciliDiffIndeksi = $state(0);

  /** Sağ diff bölmesinde gösterilecek dosya değişiklikleri. */
  const sagBolmeDosyalari = $derived.by(() => {
    const ekran = durum.aktif;
    if (!ekran) return [];
    const liste: {
      yol: string;
      eklenen: number;
      silinen: number;
      diff: string | null;
      tur: string | null;
      geriAlinabilir: boolean;
    }[] = [];
    for (const oge of ekran.ogeler) {
      if (oge.tip === 'OnayIstegi' && oge.durum === 'Bekliyor') {
        for (const o of oge.ogeler) {
          if (o.hedefler[0] || o.diff) {
            liste.push({
              yol: o.hedefler[0] ?? o.arac,
              eklenen: 0,
              silinen: 0,
              diff: o.diff ?? null,
              tur: null,
              geriAlinabilir: o.geri_alinabilir?.tip === 'Evet',
            });
          }
        }
      } else if (oge.tip === 'TurOzeti' && !oge.geri_alindi) {
        for (const d of oge.ozet.dosyalar) {
          const planDiff =
            ekran.geri_alma?.tur === oge.ozet.tur
              ? (ekran.geri_alma.plan.ogeler.find((p) => p.yol === d.yol)?.diff ?? null)
              : null;
          liste.push({
            yol: d.yol,
            eklenen: d.eklenen,
            silinen: d.silinen,
            diff: d.diff ?? planDiff ?? null,
            tur: oge.ozet.tur,
            geriAlinabilir: d.geri_alinabilir.tip === 'Evet',
          });
        }
      }
    }
    return liste;
  });

  const seciliSagDosya = $derived(
    sagBolmeDosyalari[seciliDiffIndeksi] ?? sagBolmeDosyalari[0] ?? null,
  );
  const seciliSagParcalar = $derived(
    seciliSagDosya?.diff ? diffCozumle(seciliSagDosya.diff) : [],
  );

  /** Ortam'daki çıktıdan gelinir: o dosyanın son değişikliği seçilir. */
  function dosyayiGoster(yol: string) {
    const i = sagBolmeDosyalari.findLastIndex((d) => d.yol === yol);
    if (i >= 0) seciliDiffIndeksi = i;
    durum.sagBolumuAc('degisiklikler');
  }
</script>

<header class="sag-ust">
  <div class="sag-baslik">
    <Icon name={BASLIK[bolum][0]} size={15} />
    <span>{m(BASLIK[bolum][1])}</span>
    {#if bolum === 'degisiklikler'}
      <span class="sag-sayi">{sagBolmeDosyalari.length}</span>
    {/if}
  </div>
  <IconButton
    icon="x"
    size="sm"
    label={m('konusma.sag_bolme_kapat')}
    onclick={() => durum.sagBolumuAc(null)}
  />
</header>

{#if bolum === 'ortam'}
  <OrtamBolumu {durum} ondosya={dosyayiGoster} />
{:else if bolum === 'dosyalar' && durum.aktif}
  <DosyalarBolumu oturum={durum.aktif.oturum} />
{:else if bolum === 'degisiklikler'}
    {#if sagBolmeDosyalari.length > 0}
      <div class="sag-dosyalar">
        {#each sagBolmeDosyalari as d, i (`${i}:${d.yol}`)}
          <button
            type="button"
            class={['sag-dosya', { secili: i === seciliDiffIndeksi }]}
            onclick={() => (seciliDiffIndeksi = i)}
          >
            <span class="sag-yol">{d.yol}</span>
            {#if d.eklenen > 0 || d.silinen > 0}
              <span class="sag-arti">+{d.eklenen}</span>
              <span class="sag-eksi">−{d.silinen}</span>
            {/if}
          </button>
        {/each}
      </div>

      {#if seciliSagDosya}
        <div class="sag-icerik">
          <div class="sag-dosya-ust">
            <span class="sag-dosya-ad">{seciliSagDosya.yol}</span>
            {#if seciliSagDosya.tur && seciliSagDosya.geriAlinabilir && durum.aktif}
              {@const aktifOturum = durum.aktif.oturum}
              {@const hedefTur = seciliSagDosya.tur}
              <Button
                size="sm"
                variant="ghost"
                iconLeft="rotate-ccw"
                onclick={() => durum.geriAlmaOnizle(aktifOturum, hedefTur)}
              >
                {m('tur_ozeti.geri_al')}
              </Button>
            {/if}
          </div>
          {#if seciliSagParcalar.length > 0}
            <div class="sag-diff">
              {#each seciliSagParcalar as p, i (i)}
                <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
              {/each}
            </div>
          {:else}
            <p class="sag-bos">{m('onay.sayfa.diff_yok')}</p>
          {/if}
        </div>
      {/if}
    {:else}
      <p class="sag-bos">{m('geri_alma.sayfa.yapilacak_yok')}</p>
    {/if}
{/if}

<style>
  .sag-ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 46px;
    flex: 0 0 auto;
    padding: 0 12px 0 16px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .sag-baslik {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font: var(--fw-semibold) var(--text-small) / 1.2 var(--font-sans);
    color: var(--text-title);
  }
  .sag-sayi {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .sag-dosyalar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px;
    border-bottom: var(--border-width) solid var(--border-subtle);
    max-height: 180px;
    overflow-y: auto;
  }
  .sag-dosya {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-secondary);
    font: var(--type-small);
    text-align: left;
    cursor: pointer;
  }
  .sag-dosya:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .sag-dosya.secili {
    background: var(--surface-selected);
    color: var(--text-title);
  }
  .sag-yol {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: var(--text-micro);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sag-arti {
    font: var(--type-meta);
    color: var(--status-ok);
  }
  .sag-eksi {
    font: var(--type-meta);
    color: var(--status-danger);
  }
  .sag-icerik {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .sag-dosya-ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 14px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .sag-dosya-ad {
    font-family: var(--font-mono);
    font-size: var(--text-micro);
    color: var(--text-body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sag-diff {
    flex: 1;
    overflow: auto;
  }
  .sag-bos {
    margin: 18px 14px;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
