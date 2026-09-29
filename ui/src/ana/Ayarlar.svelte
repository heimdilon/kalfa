<!--
  Ayarlar (aşama 2 tasarımı §10.4): yalıtımın durumu; yalıtım yoksa sade
  açıklama, BT için kopyalanan teknik rapor (§9.8) ve algılamayı yeniden
  çalıştırma; Linux'ta yalıtıma açılan araç zinciri dizinleri; Windows'ta
  hazırlanmış çalışma alanları. Dizin yerel klasör seçiciyle eklenir; arayüz
  yol göndermez. Gelişmiş bölümde yalıtımsız çalıştırma anahtarı (§9.8); açmayı
  Rust yerel pencerede sorar. Windows'ta araç zinciri dizinleri de Gelişmiş'tedir
  (2d-3): Kalfa kendi Python ve Node'unu getirir; yöneticiye ait dizin için
  onayı ve UAC'yi Rust ister.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { YALITIM_NEDENI, YALITIM_SADE, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { YalitimCalismaAlani } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { boyutMetni } from './geriAlma';

  let { durum }: { durum: AnaDurum } = $props();

  const yalitim = $derived(durum.uygulama?.yalitim ?? null);
  const linux = $derived(durum.uygulama?.platform === 'linux');
  const windows = $derived(durum.uygulama?.platform === 'windows');

  let hazirAlanlar = $state<YalitimCalismaAlani[]>([]);
  let alanlarYukleniyor = $state(false);
  let alanlarYuklenemedi = $state(false);
  let kaldirilanAlan = $state<string | null>(null);
  let alanYuklemeNesli = 0;

  async function alanlariYukle(): Promise<void> {
    const nesil = ++alanYuklemeNesli;
    alanlarYukleniyor = true;
    try {
      const alanlar = await ipc.yalitimCalismaAlanlari();
      if (nesil === alanYuklemeNesli) {
        hazirAlanlar = alanlar;
        alanlarYuklenemedi = false;
      }
    } catch (hata) {
      if (nesil === alanYuklemeNesli) {
        alanlarYuklenemedi = true;
        durum.hata = komutHatasi(hata).mesaj;
      }
    } finally {
      if (nesil === alanYuklemeNesli) alanlarYukleniyor = false;
    }
  }

  async function calismaAlaniniKaldir(alan: YalitimCalismaAlani): Promise<void> {
    if (kaldirilanAlan) return;
    kaldirilanAlan = alan.kimlik;
    try {
      const kaldirildi = await ipc.yalitimCalismaAlaniniKaldir(alan.kimlik);
      if (kaldirildi) await alanlariYukle();
    } catch (hata) {
      durum.hata = komutHatasi(hata).mesaj;
    } finally {
      kaldirilanAlan = null;
    }
  }

  $effect(() => {
    if (windows) void alanlariYukle();
  });
</script>

<section class="ayarlar" aria-label={m('ayarlar.baslik')}>
  {#if durum.hata}
    <div class="serit" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}
  <header class="ust">
    <h2>{m('ayarlar.baslik')}</h2>
  </header>
  <div class="govde">
    <Card title={m('ayarlar.yalitim')} icon="shield">
      {#snippet actions()}
        <Button
          size="sm"
          iconLeft="rotate-ccw"
          loading={durum.algilaniyor}
          onclick={() => durum.yalitimiAlgila()}
        >
          {m('ayarlar.yalitim.yeniden_dene')}
        </Button>
      {/snippet}
      {#if yalitim?.tip === 'Hazir'}
        <p class="durum ok">
          <Icon name="shield-check" size={16} />
          {m('ayarlar.yalitim.hazir', { arka_uc: yalitim.arka_uc, surum: yalitim.surum })}
        </p>
        <p class="aciklama">{m('ayarlar.yalitim.hazir_aciklama')}</p>
        {#if durum.onarim?.tip === 'Tamam'}
          <p class="aciklama alt" role="status">{m('ayarlar.yalitim.onarildi')}</p>
        {:else if durum.onarim?.tip === 'Basarisiz'}
          <p class="aciklama alt uyari-metni" role="status">
            {m('ayarlar.yalitim.onarim_basarisiz', { mesaj: durum.onarim.mesaj })}
          </p>
        {/if}
      {:else if yalitim}
        {@const acik = durum.uygulama?.yalitimsiz_izinli ?? false}
        <p class="durum uyari">
          <Icon name="shield-alert" size={16} />
          {m(acik ? 'ayarlar.yalitim.yok' : 'ayarlar.yalitim.kapali', {
            neden: m(YALITIM_NEDENI[yalitim.neden]),
          })}
        </p>
        <p class="aciklama">{m(YALITIM_SADE[yalitim.neden])}</p>
        <p class="aciklama alt">
          {m(acik ? 'ayarlar.yalitim.sade.acik' : 'ayarlar.yalitim.sade.kapali')}
        </p>
        {#if windows}
          <p class="aciklama alt">{m('ayarlar.yalitim.onar_ipucu')}</p>
        {/if}
        <div class="dugmeler">
          {#if windows}
            <Button
              size="sm"
              variant="primary"
              iconLeft="shield-check"
              loading={durum.onariliyor}
              onclick={() => durum.yalitimiOnar()}
            >
              {m('ayarlar.yalitim.onar')}
            </Button>
          {/if}
          <Button size="sm" iconLeft="copy" onclick={() => durum.teknikRaporuKopyala()}>
            {m('ayarlar.yalitim.kopyala')}
          </Button>
        </div>
        {#if durum.onarim?.tip === 'Tamam'}
          <p class="aciklama alt uyari-metni" role="status">
            {m('ayarlar.yalitim.onarim_yetmedi')}
          </p>
        {:else if durum.onarim?.tip === 'Basarisiz'}
          <p class="aciklama alt uyari-metni" role="status">
            {m('ayarlar.yalitim.onarim_basarisiz', { mesaj: durum.onarim.mesaj })}
          </p>
        {:else if durum.onarim?.tip === 'KuruluDegil'}
          <p class="aciklama alt" role="status">{durum.onarim.talimat}</p>
        {/if}
        {#if durum.kopyalama}
          <p class="aciklama alt" role="status">
            {m(
              durum.kopyalama === 'kopyalandi'
                ? 'ayarlar.yalitim.kopyalandi'
                : 'ayarlar.yalitim.kopyalanamadi',
            )}
          </p>
        {/if}
        <details class="teknik" open={durum.kopyalama === 'kopyalanamadi'}>
          <summary>{m('ayarlar.yalitim.teknik')}</summary>
          <pre>{durum.teknikRapor ?? `${yalitim.ayrinti}\n\n${yalitim.talimat}`}</pre>
        </details>
      {/if}
    </Card>

    {#snippet dizinEkle()}
      <Button size="sm" iconLeft="folder-plus" onclick={() => durum.aracZinciriEkle()}>
        {m('ayarlar.arac_zinciri.ekle')}
      </Button>
    {/snippet}

    {#snippet dizinListesi(baslik: string)}
      {#if durum.aracZinciri.length > 0}
        <ul class="dizinler" aria-label={baslik}>
          {#each durum.aracZinciri as yol (yol)}
            <li>
              <span class="yol">{yol}</span>
              <IconButton
                icon="x"
                size="sm"
                label={m('ayarlar.arac_zinciri.kaldir', { yol })}
                onclick={() => durum.aracZinciriKaldir(yol)}
              />
            </li>
          {/each}
        </ul>
      {:else}
        <p class="bos">{m('ayarlar.arac_zinciri.bos')}</p>
      {/if}
    {/snippet}

    {#if linux}
      <Card title={m('ayarlar.arac_zinciri')} icon="folder" actions={dizinEkle}>
        <p class="aciklama">{m('ayarlar.arac_zinciri.aciklama')}</p>
        {@render dizinListesi(m('ayarlar.arac_zinciri'))}
      </Card>
    {/if}

    {#if windows}
      <Card title={m('ayarlar.yalitim.calisma_alanlari')} icon="folder">
        <p class="aciklama">{m('ayarlar.yalitim.calisma_alanlari.aciklama')}</p>
        {#if hazirAlanlar.length > 0}
          <ul class="dizinler" aria-label={m('ayarlar.yalitim.calisma_alanlari')}>
            {#each hazirAlanlar as alan (alan.kimlik)}
              <li>
                <span class="yol">{alan.yol}</span>
                <IconButton
                  icon="x"
                  size="sm"
                  disabled={kaldirilanAlan !== null || alanlarYukleniyor}
                  label={m('ayarlar.yalitim.calisma_alanlari.kaldir', { yol: alan.yol })}
                  onclick={() => void calismaAlaniniKaldir(alan)}
                />
              </li>
            {/each}
          </ul>
        {:else if alanlarYukleniyor}
          <p class="bos" role="status">{m('ayarlar.yalitim.calisma_alanlari.yukleniyor')}</p>
        {:else if alanlarYuklenemedi}
          <p class="bos" role="status">{m('ayarlar.yalitim.calisma_alanlari.yuklenemedi')}</p>
        {:else}
          <p class="bos">{m('ayarlar.yalitim.calisma_alanlari.bos')}</p>
        {/if}
      </Card>
    {/if}

    <Card title={m('ayarlar.geri_alma')} icon="rotate-ccw">
      <p class="aciklama">{m('ayarlar.geri_alma.aciklama')}</p>
      {#if durum.geriAlmaBoyutu !== null}
        <p class="aciklama alt">
          {m('ayarlar.geri_alma.boyut', { boyut: boyutMetni(durum.geriAlmaBoyutu) })}
        </p>
      {/if}
    </Card>

    <Card title={m('ayarlar.gelismis')} icon="settings">
      <Checkbox
        checked={durum.uygulama?.yalitimsiz_izinli ?? false}
        label={m('ayarlar.yalitimsiz')}
        onchange={(acik) => durum.yalitimsiziAyarla(acik)}
      />
      <p class="aciklama alt">{m('ayarlar.yalitimsiz.aciklama')}</p>
      {#if windows}
        <div class="alt-bolum">
          <div class="alt-baslik">
            <h3>{m('ayarlar.arac_zinciri.windows')}</h3>
            {@render dizinEkle()}
          </div>
          <p class="aciklama">{m('ayarlar.arac_zinciri.windows_aciklama')}</p>
          {#if durum.aracOnarimGerekli}
            <p class="aciklama alt uyari-metni" role="status">
              {m('ayarlar.arac_zinciri.onarim_gerekli')}
            </p>
            <div class="dugmeler">
              <Button
                size="sm"
                variant="primary"
                iconLeft="shield-check"
                loading={durum.onariliyor}
                onclick={() => durum.yalitimiOnar()}
              >
                {m('ayarlar.yalitim.onar')}
              </Button>
            </div>
          {/if}
          {@render dizinListesi(m('ayarlar.arac_zinciri.windows'))}
        </div>
      {/if}
    </Card>
  </div>
</section>

<style>
  .ayarlar {
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
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 14%, transparent);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  .ust {
    padding: 14px 22px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 20px / 1.2 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
  }
  .govde {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px 22px;
    max-width: 760px;
  }
  /* Kartlar küçülmez; kısa pencerede içerik kırpılmaz, gövde kayar. */
  .govde > :global(.kart) {
    flex-shrink: 0;
  }
  .durum {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 8px;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
  }
  .ok {
    color: var(--status-ok);
  }
  .uyari {
    color: var(--status-warn);
  }
  .uyari-metni {
    color: var(--status-warn);
  }
  .alt {
    margin-top: 8px;
  }
  .dugmeler {
    display: flex;
    gap: 8px;
    margin-top: 12px;
  }
  .alt-bolum {
    margin-top: 16px;
    padding-top: 14px;
    border-top: var(--border-width) solid var(--border-subtle);
  }
  .alt-baslik {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 6px;
  }
  h3 {
    margin: 0;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .teknik {
    margin-top: 12px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .teknik summary {
    cursor: pointer;
  }
  .teknik pre {
    margin: 8px 0 0;
    padding: 10px 12px;
    max-height: 240px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
    font: var(--type-meta);
    color: var(--text-body);
    background: var(--surface-sunken);
    border-radius: var(--radius-md);
  }
  .aciklama,
  .bos {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    margin-top: 10px;
    color: var(--text-muted);
  }
  .dizinler {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .yol {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--type-meta);
    color: var(--text-body);
  }
</style>
