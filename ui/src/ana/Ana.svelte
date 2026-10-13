<!--
  Ana pencere (1c tasarımı §8.1): menü, oturum listesi, konuşma ve en altta
  durum çubuğu. Onay sayfası konuşmanın üstünde açılır. Menüden Ayarlar ya da
  Eklentiler seçilince liste ve konuşmanın yerine o ekran gelir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle } from '../bilesenler/diff';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { m } from '../dil/tr';
  import { AnaDurum } from './anaDurum.svelte';
  import Ayarlar from './Ayarlar.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Eklentiler from './Eklentiler.svelte';
  import GeriAlmaSayfasi from './GeriAlmaSayfasi.svelte';
  import GonderimGunlugu from './GonderimGunlugu.svelte';
  import Konusma from './Konusma.svelte';
  import Menu from './Menu.svelte';
  import OnaySayfasi from './OnaySayfasi.svelte';
  import OturumListesi from './OturumListesi.svelte';

  let { durum = new AnaDurum() }: { durum?: AnaDurum } = $props();

  onMount(() => {
    void durum.baslat();
  });

  // Sol proje/oturum bölmesi: Codex'teki gibi dar sınırlar içinde (180px – 300px, 10px adımlarla).
  let solGenislik = $state(240);
  // Sağ diff/inceleme bölmesi: %28'den %50'ye (yarı yarıya) kadar çekilebilir (2'şer puan adımlarla).
  let sagOran = $state(38);
  let suruklenen = $state<'sol' | 'sag' | null>(null);
  let govdeKutu = $state<HTMLDivElement>();
  let seciliDiffIndeksi = $state(0);

  function solAdimSinifi(px: number): string {
    const sinirli = Math.min(300, Math.max(180, Math.round(px / 10) * 10));
    return `sol-genislik-${sinirli}`;
  }

  function sagOranSinifi(oran: number): string {
    const sinirli = Math.min(50, Math.max(28, Math.round(oran / 2) * 2));
    return `sag-oran-${sinirli}`;
  }

  function surukleBaslat(taraf: 'sol' | 'sag', e: PointerEvent) {
    e.preventDefault();
    suruklenen = taraf;
  }

  function surukleHareket(e: PointerEvent) {
    if (!suruklenen || !govdeKutu) return;
    const dikdortgen = govdeKutu.getBoundingClientRect();
    if (suruklenen === 'sol') {
      // 48px en soldaki ikon şeridi düşülür; 180px ile 300px arasında sınırlanır.
      const yeni = e.clientX - dikdortgen.left - 48;
      solGenislik = Math.min(300, Math.max(180, yeni));
    } else if (suruklenen === 'sag' && dikdortgen.width > 0) {
      const sagGenislik = dikdortgen.right - e.clientX;
      const yuzde = (sagGenislik / dikdortgen.width) * 100;
      sagOran = Math.min(50, Math.max(28, yuzde));
    }
  }

  function surukleBitir() {
    if (suruklenen) suruklenen = null;
  }

  function tutamacTus(taraf: 'sol' | 'sag', e: KeyboardEvent) {
    if (taraf === 'sol') {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        solGenislik = Math.max(180, solGenislik - 10);
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        solGenislik = Math.min(300, solGenislik + 10);
      }
    } else {
      if (e.key === 'ArrowLeft') {
        e.preventDefault();
        sagOran = Math.min(50, sagOran + 2);
      } else if (e.key === 'ArrowRight') {
        e.preventDefault();
        sagOran = Math.max(28, sagOran - 2);
      }
    }
  }

  const onayIstegi = $derived.by(() => {
    const acik = durum.onay;
    if (!acik) return null;
    const oge = durum.ekranlar
      .get(acik.oturum)
      ?.ogeler.find((o) => o.tip === 'OnayIstegi' && o.islem === acik.islem);
    return oge?.tip === 'OnayIstegi' && oge.durum === 'Bekliyor' ? oge : null;
  });

  /** Seçili oturumun açık geri alma planı ve turun ilk mesajı. */
  const geriAlma = $derived.by(() => {
    const ekran = durum.aktif;
    const onizleme = ekran?.geri_alma;
    if (!ekran || !onizleme) return null;
    const ilk = ekran.ogeler.find((o) => o.tip === 'KullaniciMesaji' && o.tur === onizleme.tur);
    return {
      oturum: ekran.oturum,
      onizleme,
      baslik: ilk?.tip === 'KullaniciMesaji' ? ilk.metin : null,
    };
  });

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
      if (oge.tip === 'OnayIstegi') {
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
          liste.push({
            yol: d.yol,
            eklenen: d.eklenen,
            silinen: d.silinen,
            diff: null,
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
</script>

<svelte:window onpointermove={surukleHareket} onpointerup={surukleBitir} />

<div class={['ana', { surukleniyor: suruklenen !== null }]}>
  <div bind:this={govdeKutu} class="govde">
    <Menu {durum} />
    {#if durum.gorunum === 'ayarlar'}
      <Ayarlar {durum} />
    {:else if durum.gorunum === 'eklentiler'}
      <Eklentiler {durum} />
    {:else}
      <div class={['sol-bolme-sarmal', solAdimSinifi(solGenislik)]}>
        <OturumListesi {durum} />
      </div>
      <button
        type="button"
        aria-label={m('konusma.bolme_boyutlandir')}
        class={['bolme-tutamaci', 'sol-tutamac', { etkin: suruklenen === 'sol' }]}
        onpointerdown={(e) => surukleBaslat('sol', e)}
        onkeydown={(e) => tutamacTus('sol', e)}
      ></button>

      <Konusma {durum} />

      {#if durum.sagBolmeAcik && durum.aktif}
        <button
          type="button"
          aria-label={m('konusma.bolme_boyutlandir')}
          class={['bolme-tutamaci', 'sag-tutamac', { etkin: suruklenen === 'sag' }]}
          onpointerdown={(e) => surukleBaslat('sag', e)}
          onkeydown={(e) => tutamacTus('sag', e)}
        ></button>

        <aside
          class={['sag-bolme', sagOranSinifi(sagOran)]}
          aria-label={m('konusma.denetci.degisiklikler')}
        >
          <header class="sag-ust">
            <div class="sag-baslik">
              <Icon name="code" size={15} />
              <span>{m('konusma.denetci.degisiklikler')}</span>
              <span class="sag-sayi">{sagBolmeDosyalari.length}</span>
            </div>
            <IconButton
              icon="x"
              size="sm"
              label={m('konusma.sag_bolme_kapat')}
              onclick={() => (durum.sagBolmeAcik = false)}
            />
          </header>

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
        </aside>
      {/if}
    {/if}
  </div>
  <DurumCubugu {durum} />
  {#if durum.onay && onayIstegi}
    {@const { oturum, islem } = durum.onay}
    <OnaySayfasi
      ogeler={onayIstegi.ogeler}
      onkarar={(kararlar) => durum.onayla(oturum, islem, kararlar)}
      onkapat={() => durum.onayiKapat()}
    />
  {/if}
  {#if geriAlma && !durum.onay}
    {@const { oturum, onizleme, baslik } = geriAlma}
    <GeriAlmaSayfasi
      {onizleme}
      {baslik}
      gonderiliyor={durum.geriAliniyor}
      ongeri={() => durum.turuGeriAl(oturum)}
      onkapat={() => durum.geriAlmayiKapat(oturum)}
    />
  {/if}
  {#if durum.gunlukOturum}
    <GonderimGunlugu
      oturum={durum.gunlukOturum}
      istekSayaci={durum.ekranlar.get(durum.gunlukOturum)?.istek_sayaci ?? 0}
      onkapat={() => durum.gunluguKapat()}
    />
  {/if}
  <div class="gizli-erisilebilir" aria-live="polite">{durum.duyuru}</div>
</div>

<style>
  .ana {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-app);
  }
  .ana.surukleniyor {
    user-select: none;
    cursor: col-resize;
  }
  .govde {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .sol-bolme-sarmal {
    flex: 0 0 auto;
    height: 100%;
    display: flex;
    min-width: 180px;
    max-width: 300px;
  }
  .sol-genislik-180 {
    width: 180px;
  }
  .sol-genislik-190 {
    width: 190px;
  }
  .sol-genislik-200 {
    width: 200px;
  }
  .sol-genislik-210 {
    width: 210px;
  }
  .sol-genislik-220 {
    width: 220px;
  }
  .sol-genislik-230 {
    width: 230px;
  }
  .sol-genislik-240 {
    width: 240px;
  }
  .sol-genislik-250 {
    width: 250px;
  }
  .sol-genislik-260 {
    width: 260px;
  }
  .sol-genislik-270 {
    width: 270px;
  }
  .sol-genislik-280 {
    width: 280px;
  }
  .sol-genislik-290 {
    width: 290px;
  }
  .sol-genislik-300 {
    width: 300px;
  }
  .bolme-tutamaci {
    position: relative;
    width: 5px;
    flex: 0 0 5px;
    padding: 0;
    border: none;
    cursor: col-resize;
    background: var(--border-subtle);
    transition: background 120ms ease;
    z-index: 15;
  }
  .bolme-tutamaci::after {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 3px;
    height: 28px;
    border-radius: var(--radius-pill);
    background: transparent;
    transition: background 120ms ease;
  }
  .bolme-tutamaci:hover,
  .bolme-tutamaci.etkin,
  .bolme-tutamaci:focus-visible {
    background: color-mix(in srgb, var(--copper-300) 45%, var(--border-subtle));
    outline: none;
  }
  .bolme-tutamaci:hover::after,
  .bolme-tutamaci.etkin::after,
  .bolme-tutamaci:focus-visible::after {
    background: var(--copper-200);
  }
  .sag-bolme {
    flex: 0 0 auto;
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    min-width: 260px;
    max-width: 50%;
    overflow: hidden;
  }
  .sag-oran-28 {
    width: 28%;
  }
  .sag-oran-30 {
    width: 30%;
  }
  .sag-oran-32 {
    width: 32%;
  }
  .sag-oran-34 {
    width: 34%;
  }
  .sag-oran-36 {
    width: 36%;
  }
  .sag-oran-38 {
    width: 38%;
  }
  .sag-oran-40 {
    width: 40%;
  }
  .sag-oran-42 {
    width: 42%;
  }
  .sag-oran-44 {
    width: 44%;
  }
  .sag-oran-46 {
    width: 46%;
  }
  .sag-oran-48 {
    width: 48%;
  }
  .sag-oran-50 {
    width: 50%;
  }
  .sag-ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 14px;
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
    box-shadow: var(--shadow-accent-edge);
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
