<!--
  Konuşma sütunu (1c tasarımı §8.1, §9.3): başlık, uyarı şeritleri, geçmiş ve
  canlı olaylar, Composer. Yukarı kaydırınca eski turlar sayfa sayfa eklenir;
  kullanıcı en alttayken yeni içerik görünür kalır. "Bu oturumdaki izinler"
  paneli oturumun kısayollarını ve çalışma alanındaki eklentileri listeler;
  eklentiyi buradan da açıp kapatabilirsiniz (aşama 4 tasarımı §14).
-->
<script lang="ts">
  import { tick } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import Composer from '../bilesenler/Composer.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import { KIP_ADI, MODEL_PROFILI_ADI, kisayolCumlesi, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import { KIPLER, type Kip, type OturumEklentisi, type OturumId } from '../ipc/tipler';
  import { AltaYapisma, kaydirmaNiyetiMi } from './altaYapisma';
  import type { AnaDurum } from './anaDurum.svelte';
  import { gunAy, satirBasligi } from './bicim';
  import KonusmaOgesi from './KonusmaOgesi.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  const ekran = $derived(durum.aktif);
  const baslik = $derived.by(() => {
    const satir = durum.liste.find((s) => s.oturum === durum.secili);
    return satir ? satirBasligi(satir) : m('oturum.yeni');
  });
  const modeller = $derived(
    (durum.uygulama?.modeller ?? []).map((s) => ({
      value: s.kimlik,
      label: m(MODEL_PROFILI_ADI[s.profil]),
      hint: s.kimlik,
    })),
  );
  const kipler = KIPLER.map((k) => ({ value: k, label: m(KIP_ADI[k]) }));

  const kapaliNedeni = $derived.by(() => {
    if (!ekran) return null;
    const s = ekran.salt_okunur;
    if (s?.tip === 'CalismaAlaniYok') return m('hata.calisma_alani_yok', { yol: s.yol });
    if (s?.tip === 'CalismaAlaniKayitsiz') return m('hata.calisma_alani_kayitsiz');
    if (ekran.kapandi) return m('konusma.kapandi');
    return null;
  });

  let metin = $state('');
  let akis = $state<HTMLDivElement>();
  let icerik = $state<HTMLDivElement>();
  const yapisma = new AltaYapisma(48);

  // Oturum değişince en alttan başlanır.
  $effect(() => {
    void durum.secili;
    yapisma.sifirla();
  });

  // İçerik büyüdüğünde (yeni öğe, akan metin, ekrana giren öğenin gerçek
  // boyu) kullanıcı en alttaysa en altta kalınır.
  $effect(() => {
    void durum.secili;
    if (!akis) return;
    yapisma.kaydir(akis);
    if (!icerik || typeof ResizeObserver === 'undefined') return;
    const kutu = akis;
    const gozcu = new ResizeObserver(() => yapisma.kaydir(kutu));
    gozcu.observe(icerik);
    return () => gozcu.disconnect();
  });

  // Kullanıcının kaydırma niyeti (`kaydirmaNiyetiMi`). Yalnızca gözlenir; bu
  // yüzden şablonda değil, pasif dinleyicilerle bağlanır.
  $effect(() => {
    if (!akis) return;
    const kutu = akis;
    const niyet = (e: Event) => {
      if (kaydirmaNiyetiMi(e as Event & { key?: string }, kutu)) yapisma.niyet();
    };
    const olaylar = ['wheel', 'touchmove', 'pointerdown', 'keydown'] as const;
    for (const o of olaylar) kutu.addEventListener(o, niyet, { passive: true });
    return () => {
      for (const o of olaylar) kutu.removeEventListener(o, niyet);
    };
  });

  // "Bu oturumdaki izinler" panelindeki eklentiler (aşama 4 tasarımı §14).
  // Oturum kimliği düz metindir: ekran her olayda yenilenir ama kimlik
  // değişmedikçe aşağıdaki etki yeniden çalışmaz.
  let eklentiler = $state<OturumEklentisi[]>([]);
  /** Başlıktaki sayı verilmiş izinleri sayar: kapalı eklenti izin değildir. */
  const acikEklenti = $derived(eklentiler.filter((e) => e.acik).length);
  const eklentiOturumu = $derived(
    ekran && !ekran.salt_okunur && !ekran.kapandi ? ekran.oturum : null,
  );

  async function eklentileriYukle(oturum: OturumId) {
    try {
      const liste = await ipc.oturumEklentileri(oturum);
      if (eklentiOturumu === oturum) eklentiler = liste;
    } catch (hata) {
      durum.hata = komutHatasi(hata).mesaj;
    }
  }

  async function eklentiAyarla(oturum: OturumId, eklenti: OturumEklentisi, acik: boolean) {
    try {
      await ipc.eklentiAlaniAyarla(eklenti.ad, { tip: 'Oturum', oturum }, acik);
    } catch (hata) {
      durum.hata = komutHatasi(hata).mesaj;
    }
    await eklentileriYukle(oturum);
  }

  // Oturum seçilince ve açılınca; Eklentiler ekranından dönünce bileşen yeniden kurulur.
  $effect(() => {
    const oturum = eklentiOturumu;
    eklentiler = [];
    if (oturum) void eklentileriYukle(oturum);
  });

  function izinlerAcildi(e: Event & { currentTarget: HTMLDetailsElement }) {
    if (e.currentTarget.open && eklentiOturumu) void eklentileriYukle(eklentiOturumu);
  }

  function kaydirildi() {
    if (!akis) return;
    yapisma.olay(akis);
    if (akis.scrollTop < 40) void eskileriYukle();
  }

  /** Eski sayfa eklenince okunan yer kaymaz. */
  async function eskileriYukle() {
    if (!ekran?.daha_eski_var || !akis) return;
    const once = akis.scrollHeight;
    await durum.eskileriYukle(ekran.oturum);
    await tick();
    if (akis) akis.scrollTop += akis.scrollHeight - once;
  }
</script>

<section class="konusma" aria-label={m('konusma.ad')}>
  {#if durum.uygulama?.baglanti === 'Yok'}
    <div class="serit uyari" role="status">
      <Icon name="wifi-off" size={15} />
      <span>{m('hata.baglanti_yok')}</span>
    </div>
  {/if}
  {#if durum.hata}
    <div class="serit tehlike" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}

  {#if ekran}
    <header class="ust">
      <div class="baslik-blogu">
        <h2>{baslik}</h2>
        {#if ekran.calisma_alani}
          <span class="yol"><Icon name="folder" size={14} />{ekran.calisma_alani}</span>
        {/if}
      </div>
      <div class="eylemler">
        <Checkbox
          label={m('konusma.onizleme_ayari')}
          checked={ekran.ozet.onizleme_acik ?? false}
          onchange={(acik) => durum.onizlemeAyarla(ekran.oturum, acik)}
        />
        <IconButton
          icon="send"
          label={m('konusma.gunluk')}
          onclick={() => durum.gunluguAc(ekran.oturum)}
        />
        {#if ekran.kapandi}
          <Button size="sm" iconLeft="rotate-ccw" onclick={() => durum.yenidenAc(ekran.oturum)}>
            {m('konusma.yeniden_ac')}
          </Button>
        {/if}
        <IconButton icon="x" label={m('konusma.kapat')} onclick={() => durum.kapat(ekran.oturum)} />
      </div>
    </header>

    {#if ekran.salt_okunur}
      <div class="serit bilgi" role="status">
        <Icon name="lock" size={15} />
        <span>{kapaliNedeni}</span>
      </div>
    {/if}

    {#if ekran.kisayollar.length > 0 || eklentiler.length > 0}
      <details class="izinler" ontoggle={izinlerAcildi}>
        <summary>
          <Icon name="key-round" size={14} />
          {m('kisayol.liste', { sayi: ekran.kisayollar.length + acikEklenti })}
        </summary>
        {#if ekran.kisayollar.length > 0}
          <p class="izin-aciklama">{m('kisayol.aciklama')}</p>
          <ul>
            {#each ekran.kisayollar as k (k.kimlik)}
              <li>
                <span class="metin">{kisayolCumlesi(k.teklif)}</span>
                <IconButton
                  icon="x"
                  size="sm"
                  label={m('kisayol.kaldir')}
                  onclick={() => durum.kisayoluKaldir(ekran.oturum, k.kimlik)}
                />
              </li>
            {/each}
          </ul>
        {/if}
        {#if eklentiler.length > 0}
          <p class="izin-aciklama">{m('oturum.eklenti.aciklama')}</p>
          <ul aria-label={m('oturum.eklentiler')}>
            {#each eklentiler as e (`${e.yerlesik}:${e.ad}`)}
              <li>
                <div class="eklenti" role="group" aria-label={e.ad}>
                  <span class="eklenti-adi">{e.ad}</span>
                  <Checkbox
                    label={m('eklentiler.alan_acik')}
                    checked={e.acik}
                    disabled={e.kullanilamaz}
                    onchange={(acik) => void eklentiAyarla(ekran.oturum, e, acik)}
                  />
                  {#if e.kullanilamaz}
                    <span class="eklenti-notu kullanilamaz">
                      {m('oturum.eklenti.kullanilamaz')}
                    </span>
                  {/if}
                  <span class="eklenti-notu">
                    <span>{m('eklentiler.gonderim')}</span>
                    <span>
                      {e.gonderim_izni_ms === null
                        ? m('eklentiler.izin_yok')
                        : m('eklentiler.izin_var', { tarih: gunAy(e.gonderim_izni_ms) })}
                    </span>
                  </span>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </details>
    {/if}

    <div class="akis" bind:this={akis} onscroll={kaydirildi}>
      <div class="icerik" bind:this={icerik}>
        {#if ekran.daha_eski_var}
          <div class="eski">
            <Button variant="ghost" size="sm" iconLeft="chevron-down" onclick={eskileriYukle}>
              {m('konusma.eskiler')}
            </Button>
          </div>
        {/if}
        {#each ekran.ogeler as oge (oge.anahtar)}
          <div class="oge">
            <KonusmaOgesi {oge} {durum} oturum={ekran.oturum} />
          </div>
        {:else}
          <p class="bos">{m('konusma.bos')}</p>
        {/each}
      </div>
    </div>

    <div class="alt">
      <Composer
        bind:value={metin}
        models={modeller}
        model={ekran.ozet.model ?? durum.uygulama?.varsayilan_model ?? ''}
        modes={kipler}
        mode={ekran.ozet.kip ?? 'Siki'}
        running={ekran.anlik !== 'Bos'}
        disabled={kapaliNedeni !== null}
        placeholder={kapaliNedeni ?? undefined}
        onsend={(t) => durum.gonder(t)}
        oncancel={() => durum.iptal()}
        onmodelchange={(v) => durum.modelSec(v)}
        onmodechange={(v) => durum.kipSec(v as Kip)}
      />
    </div>
  {:else}
    <div class="karsilama">
      <IconTile icon="message-square" tone="accent" size={40} />
      <h2>{m('konusma.karsilama.baslik')}</h2>
      <p>{m('konusma.karsilama.metin')}</p>
      <Button variant="primary" iconLeft="folder-plus" onclick={() => durum.yeniOturum()}>
        {m('liste.yeni')}
      </Button>
    </div>
  {/if}
</section>

<style>
  .konusma {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  .serit {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    font: var(--type-small);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  /* Zemin durum renginden türetilir; iki temada da doğru kalır. */
  .uyari {
    color: var(--status-warn);
    background: color-mix(in srgb, var(--status-warn) 14%, transparent);
  }
  .tehlike {
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 14%, transparent);
  }
  .bilgi {
    color: var(--text-secondary);
    background: var(--surface-sunken);
  }
  .izinler {
    padding: 8px 22px;
    font: var(--type-small);
    color: var(--text-secondary);
    background: var(--surface-sunken);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .izinler summary {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .izin-aciklama {
    margin: 6px 0;
  }
  .izinler ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .izinler li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .izinler li .metin {
    flex: 1;
    color: var(--text-body);
  }
  .eklenti {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 16px;
    padding: 2px 0;
  }
  .eklenti-adi {
    font-family: var(--font-mono);
    color: var(--text-body);
  }
  .eklenti-notu {
    display: inline-flex;
    gap: 6px;
    color: var(--text-muted);
  }
  .eklenti-notu.kullanilamaz {
    color: var(--status-warn);
  }
  .ust {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 22px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .baslik-blogu {
    min-width: 0;
    flex: 1;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 20px / 1.2 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .yol {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .eylemler {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .akis {
    flex: 1;
    overflow-y: auto;
  }
  .icerik {
    min-height: 100%;
    box-sizing: border-box;
    padding: 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  /* Ekran dışındaki öğeler çizilmez (1c tasarımı §9.3). */
  .oge {
    content-visibility: auto;
    contain-intrinsic-size: auto 96px;
  }
  .eski {
    display: flex;
    justify-content: center;
  }
  .bos {
    margin: auto;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .alt {
    padding: 0 22px 18px;
  }
  .karsilama {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px;
    text-align: center;
  }
  .karsilama h2 {
    white-space: normal;
  }
  .karsilama p {
    max-width: 52ch;
    margin: 0 0 8px;
    font: var(--type-body);
    color: var(--text-secondary);
  }
</style>
