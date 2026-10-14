<!--
  Konuşma sütunu (1c tasarımı §8.1, §9.3): ince başlık, uyarı şeritleri,
  ortalanmış geçmiş ve canlı olaylar, altta yüzen Composer. Yukarı kaydırınca
  eski turlar sayfa sayfa eklenir; kullanıcı en alttayken yeni içerik görünür
  kalır. Başlıktaki "Bu oturumdaki izinler" açılır paneli oturumun
  kısayollarını ve çalışma alanındaki eklentileri listeler; eklentiyi buradan
  da açıp kapatabilirsiniz (aşama 4 tasarımı §14).
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
  import { gunAy, projeAdi, satirBasligi } from './bicim';
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

  const seciliModelKimligi = $derived(
    ekran?.ozet.model ?? durum.uygulama?.varsayilan_model ?? '',
  );
  const seciliModelBilgisi = $derived(
    (durum.uygulama?.modeller ?? []).find((s) => s.kimlik === seciliModelKimligi),
  );
  const seciliEfor = $derived(
    ekran?.ozet.efor ?? durum.uygulama?.varsayilan_efor ?? 'orta',
  );
  /** Geri alınmamış turlarda değişen dosyalar ve satır sayıları. */
  const degisenDosyaOzeti = $derived.by(() => {
    if (!ekran) return { sayi: 0, eklenen: 0, silinen: 0 };
    const dosyalar = new Set<string>();
    let eklenen = 0;
    let silinen = 0;
    for (const oge of ekran.ogeler) {
      if (oge.tip === 'TurOzeti' && !oge.geri_alindi) {
        for (const d of oge.ozet.dosyalar) {
          dosyalar.add(d.yol);
          eklenen += d.eklenen;
          silinen += d.silinen;
        }
      }
    }
    return { sayi: dosyalar.size, eklenen, silinen };
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
  {#if ekran}
    <header class="ust">
      {#if !durum.kenarAcik}
        <IconButton
          icon="panel-left"
          size="sm"
          label={m('kenar.goster')}
          onclick={() => (durum.kenarAcik = true)}
        />
      {/if}
      <h2>{baslik}</h2>
      {#if ekran.calisma_alani}
        <span class="cip" title={ekran.calisma_alani}>
          <Icon name="folder" size={13} />{projeAdi(ekran.calisma_alani)}
        </span>
      {/if}
      <span class="bosluk"></span>

      {#if ekran.kisayollar.length > 0 || eklentiler.length > 0}
        <details class="izinler" ontoggle={izinlerAcildi}>
          <summary>
            <Icon name="key-round" size={14} />
            {m('kisayol.liste', { sayi: ekran.kisayollar.length + acikEklenti })}
          </summary>
          <div class="izin-paneli">
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
          </div>
        </details>
      {/if}

      <span class="onizleme-ayari">
        <Checkbox
          label={m('konusma.onizleme_ayari')}
          checked={ekran.ozet.onizleme_acik ?? false}
          onchange={(acik) => durum.onizlemeAyarla(ekran.oturum, acik)}
        />
      </span>

      <button
        type="button"
        class={['degisiklik-dugme', { etkin: durum.sagBolmeAcik }]}
        aria-label={durum.sagBolmeAcik ? m('konusma.sag_bolme_kapat') : m('konusma.sag_bolme_ac')}
        aria-pressed={durum.sagBolmeAcik}
        onclick={() => durum.sagBolmeyiDegistir()}
      >
        <Icon name="code" size={14} />
        <span>{m('konusma.degisiklikler')}</span>
        {#if degisenDosyaOzeti.sayi > 0}
          <span class="arti">+{degisenDosyaOzeti.eklenen}</span>
          <span class="eksi">−{degisenDosyaOzeti.silinen}</span>
        {/if}
      </button>

      {#if ekran.kapandi}
        <Button size="sm" iconLeft="rotate-ccw" onclick={() => durum.yenidenAc(ekran.oturum)}>
          {m('konusma.yeniden_ac')}
        </Button>
      {/if}
      <IconButton
        icon="x"
        size="sm"
        label={m('konusma.kapat')}
        onclick={() => durum.kapat(ekran.oturum)}
      />
    </header>
  {/if}

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
    {#if ekran.salt_okunur}
      <div class="serit bilgi" role="status">
        <Icon name="lock" size={15} />
        <span>{kapaliNedeni}</span>
      </div>
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
        model={seciliModelKimligi}
        modes={kipler}
        mode={ekran.ozet.kip ?? 'Siki'}
        efor={seciliEfor}
        eforlar={seciliModelBilgisi?.eforlar}
        running={ekran.anlik !== 'Bos'}
        disabled={kapaliNedeni !== null}
        placeholder={kapaliNedeni ?? undefined}
        onsend={(t) => durum.gonder(t)}
        oncancel={() => durum.iptal()}
        onmodelchange={(v) => durum.modelSec(v)}
        onmodechange={(v) => durum.kipSec(v as Kip)}
        oneforchange={(v) => durum.eforSec(v)}
        onmodelyenile={() => durum.modelleriYenile()}
      />
    </div>
  {:else}
    <div class="karsilama">
      {#if !durum.kenarAcik}
        <span class="karsilama-kenar">
          <IconButton
            icon="panel-left"
            size="sm"
            label={m('kenar.goster')}
            onclick={() => (durum.kenarAcik = true)}
          />
        </span>
      {/if}
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
    position: relative;
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
    padding: 8px 20px;
    font: var(--type-small);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  /* Zemin durum renginden türetilir; iki temada da doğru kalır. */
  .uyari {
    color: var(--status-warn);
    background: color-mix(in srgb, var(--status-warn) 12%, transparent);
  }
  .tehlike {
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 12%, transparent);
  }
  .bilgi {
    color: var(--text-secondary);
    background: var(--surface-sunken);
  }

  /* İnce başlık: başlık, çalışma alanı, sağda eylemler. */
  .ust {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 46px;
    flex: 0 0 auto;
    padding: 0 12px 0 20px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h2 {
    margin: 0;
    min-width: 0;
    font: var(--fw-semibold) var(--text-body-size) / 1.2 var(--font-sans);
    color: var(--text-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cip {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-card);
    color: var(--text-secondary);
    font: var(--type-meta);
    white-space: nowrap;
  }
  .bosluk {
    flex: 1;
  }
  .onizleme-ayari {
    flex: none;
    font: var(--type-meta);
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .degisiklik-dugme {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    font: var(--type-small);
    white-space: nowrap;
    cursor: pointer;
    transition: var(--transition-control);
  }
  .degisiklik-dugme:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .degisiklik-dugme.etkin {
    background: var(--surface-selected);
    color: var(--text-title);
  }
  .arti {
    font: var(--type-meta);
    color: var(--status-ok);
  }
  .eksi {
    font: var(--type-meta);
    color: var(--status-danger);
  }

  /* "Bu oturumdaki izinler": başlıktan açılan panel. */
  .izinler {
    position: relative;
    flex: none;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .izinler summary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 10px;
    border-radius: var(--radius-md);
    list-style: none;
    white-space: nowrap;
    cursor: pointer;
    transition: var(--transition-control);
  }
  .izinler summary::-webkit-details-marker {
    display: none;
  }
  .izinler summary:hover,
  .izinler[open] summary {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .izin-paneli {
    position: absolute;
    top: calc(100% + 6px);
    right: 0;
    z-index: 30;
    width: 360px;
    padding: 10px 12px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-overlay);
  }
  .izin-aciklama {
    margin: 4px 0 6px;
    color: var(--text-muted);
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

  /* Akış: ortada okunur bir sütun. */
  .akis {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .icerik {
    min-height: 100%;
    max-width: 800px;
    margin: 0 auto;
    box-sizing: border-box;
    padding: 28px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  /* Ekran dışındaki öğeler çizilmez (1c tasarımı §9.3). */
  .oge {
    content-visibility: auto;
    contain-intrinsic-size: auto 48px;
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
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    box-sizing: border-box;
    padding: 0 24px 16px;
  }
  .karsilama {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px;
    text-align: center;
  }
  .karsilama-kenar {
    position: absolute;
    top: 10px;
    left: 12px;
  }
  .karsilama h2 {
    font: var(--type-title);
    white-space: normal;
  }
  .karsilama p {
    max-width: 52ch;
    margin: 0 0 8px;
    font: var(--type-body);
    color: var(--text-secondary);
  }
</style>
