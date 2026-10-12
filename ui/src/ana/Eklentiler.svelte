<!--
  Eklentiler (aşama 4 tasarımı §13, §14): yerleşik ve kurulu eklentiler,
  durumları ve yetenekleri; klasörden kurma, güncelleme, kaldırma, devre dışı
  bırakma; çalışma alanında açma, EVREN gönderim izni ve depo. Klasör
  seçiciyi, kaldırma ve veri silme onaylarını Rust yerel pencerede sorar;
  arayüz yol göndermez, çalışma alanları kimlikle gider. Eklentinin kendi
  yazdığı metinler (açıklama, yetenek cümlesi, yüklenememe nedeni) görünmez
  karakterler kaçışlanarak gösterilir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import { EKLENTI_DURUMU_ADI, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { EklentiSatiri, KurulumSonucu } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { gunAy, kodParcalari, tarih } from './bicim';
  import { boyutMetni } from './geriAlma';

  let { durum }: { durum: AnaDurum } = $props();

  const DURUM_TONU = { Yuklendi: 'ok', DevreDisi: 'neutral', Yuklenemedi: 'danger' } as const;

  let satirlar = $state<EklentiSatiri[]>([]);
  let yukleniyor = $state(true);
  /** Bir eylem sürüyor; eylemler art arda çalışır, liste her birinin sonunda yenilenir. */
  let mesgul = $state(false);
  let kuruluyor = $state(false);
  /** Eklenti adına göre son komut hatası; satırda gösterilir. */
  let hatalar = $state<Record<string, string>>({});
  /** "Klasörden kur"un sonucu (vazgeçilince yok). */
  let kurulum = $state<{ metin: string; not: string | null; hata: boolean } | null>(null);

  async function yukle(): Promise<void> {
    try {
      satirlar = await ipc.eklentileriListele();
    } catch (hata) {
      durum.hata = komutHatasi(hata).mesaj;
    } finally {
      yukleniyor = false;
    }
  }

  onMount(() => {
    void yukle();
  });

  /**
   * Bir eylemi çalıştırır, sonra listeyi yeniler. Hata `ad` verilmişse o
   * eklentinin satırında, verilmemişse ekranın üstünde gösterilir.
   */
  async function calistir(ad: string | null, is: () => Promise<unknown>): Promise<void> {
    if (mesgul) return;
    mesgul = true;
    if (ad) delete hatalar[ad];
    try {
      await is();
    } catch (hata) {
      const ileti = komutHatasi(hata).mesaj;
      if (ad) hatalar[ad] = ileti;
      else durum.hata = ileti;
    }
    await yukle();
    mesgul = false;
  }

  function kurulumSonucu(s: KurulumSonucu): void {
    if (s.tip === 'Vazgecildi') return;
    kurulum =
      s.tip === 'Basarisiz'
        ? { metin: m('eklentiler.kurulamadi', { mesaj: s.mesaj }), not: null, hata: true }
        : {
            metin: m(s.guncelleme ? 'eklentiler.guncellendi' : 'eklentiler.kuruldu', {
              ad: s.ad,
              surum: s.surum,
            }),
            not: s.izinler_silindi ? m('eklentiler.izinler_silindi') : null,
            hata: false,
          };
    durum.duyuru = [kurulum.metin, kurulum.not].filter(Boolean).join(' ');
  }

  /** "Klasörden kur" ve "Güncelle" aynı akıştır (§13.2). */
  function kur(ad: string | null): Promise<void> {
    return calistir(ad, async () => {
      kuruluyor = true;
      kurulum = null;
      try {
        kurulumSonucu(await ipc.eklentiKlasordenKur());
      } finally {
        kuruluyor = false;
      }
    });
  }
</script>

<section class="eklentiler" aria-label={m('eklentiler.baslik')}>
  {#if durum.hata}
    <div class="serit" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}
  <header class="ust">
    <h2>{m('eklentiler.baslik')}</h2>
    <Button
      size="sm"
      variant="primary"
      iconLeft="folder-plus"
      loading={kuruluyor}
      disabled={mesgul}
      onclick={() => void kur(null)}
    >
      {m('eklentiler.kur')}
    </Button>
  </header>
  <div class="govde">
    <p class="aciklama">{m('eklentiler.aciklama')}</p>
    {#if kurulum}
      <div class={['kurulum', { hata: kurulum.hata }]}>
        <p><GorunurMetin metin={kurulum.metin} /></p>
        {#if kurulum.not}<p>{kurulum.not}</p>{/if}
      </div>
    {/if}
    {#if satirlar.length > 0}
      <ul class="liste" aria-label={m('eklentiler.liste')}>
        {#each satirlar as e (`${e.yerlesik}:${e.ad}`)}
          <li>
            <div role="group" aria-label={e.ad}>
              <Card icon="puzzle" title={e.ad} subtitle={e.surum}>
                {#snippet actions()}
                  <Badge tone={e.yerlesik ? 'accent' : 'neutral'}>
                    {e.yerlesik ? m('eklentiler.yerlesik') : m('eklentiler.kurulu')}
                  </Badge>
                  <Badge tone={DURUM_TONU[e.durum.tip]}>{m(EKLENTI_DURUMU_ADI[e.durum.tip])}</Badge>
                {/snippet}
                <div class="icerik">
                  {#if e.durum.tip === 'Yuklenemedi'}
                    <p class="neden">
                      <GorunurMetin metin={m('eklentiler.neden', { neden: e.durum.neden })} />
                    </p>
                  {/if}
                  {#if e.araclar.length > 0}
                    <div class="satir">
                      <span class="etiket">{m('eklentiler.araclar')}</span>
                      <ul class="araclar">
                        {#each e.araclar as a (a.ad)}
                          <li class="arac" title={gorunurMetin(a.aciklama)}>{a.ad}</li>
                        {/each}
                      </ul>
                    </div>
                  {/if}
                  <details class="ayrinti">
                    <summary>{m('eklentiler.ayrinti')}</summary>
                    <dl>
                      <dt>{m('eklentiler.ayrinti.aciklama')}</dt>
                      <dd><GorunurMetin metin={e.aciklama} /></dd>
                      {#if e.yetenek}
                        <dt>{m('eklentiler.ayrinti.yetenek')}</dt>
                        <dd>
                          {#each kodParcalari(e.yetenek) as p, i (i)}{#if p.kod}<code
                                ><GorunurMetin metin={p.metin} /></code
                              >{:else}<GorunurMetin metin={p.metin} />{/if}{/each}
                        </dd>
                      {/if}
                      {#if e.sha256}
                        <dt>{m('eklentiler.ayrinti.sha256')}</dt>
                        <dd class="mono">{e.sha256}</dd>
                      {/if}
                      {#if e.kaynak}
                        <dt>{m('eklentiler.ayrinti.kaynak')}</dt>
                        <dd class="mono"><GorunurMetin metin={e.kaynak} /></dd>
                      {/if}
                      {#if e.kurulum_ms !== null}
                        <dt>{m('eklentiler.ayrinti.kurulum')}</dt>
                        <dd>{tarih(e.kurulum_ms)}</dd>
                      {/if}
                    </dl>
                  </details>
                  <div class="alanlar">
                    <h3>{m('eklentiler.alanlar')}</h3>
                    {#if e.alanlar.length === 0}
                      <p class="bos">{m('eklentiler.alan_yok')}</p>
                    {:else}
                      <ul class="alan-listesi">
                        {#each e.alanlar as a (a.kimlik)}
                          <li>
                            <div class="alan" role="group" aria-label={gorunurMetin(a.yol)}>
                              <div class="alan-ust">
                                <Icon name="folder" size={14} />
                                <span class="yol" title={gorunurMetin(a.yol)}>
                                  <GorunurMetin metin={a.yol} />
                                </span>
                                <Checkbox
                                  checked={a.acik}
                                  disabled={mesgul}
                                  label={m('eklentiler.alan_acik')}
                                  onchange={(acik) =>
                                    void calistir(e.ad, () =>
                                      ipc.eklentiAlaniAyarla(
                                        e.ad,
                                        { tip: 'Kimlik', kimlik: a.kimlik },
                                        acik,
                                      ),
                                    )}
                                />
                              </div>
                              <div class="alan-alt">
                                <span class="ikili">
                                  <span class="etiket">{m('eklentiler.gonderim')}</span>
                                  {#if a.gonderim_izni_ms !== null}
                                    <span>{m('eklentiler.izin_var', { tarih: gunAy(a.gonderim_izni_ms) })}</span>
                                    <Button
                                      size="sm"
                                      variant="ghost"
                                      disabled={mesgul}
                                      onclick={() =>
                                        void calistir(e.ad, () =>
                                          ipc.gonderimIzniniKaldir(e.ad, a.kimlik),
                                        )}
                                    >
                                      {m('eklentiler.izni_kaldir')}
                                    </Button>
                                  {:else}
                                    <span>{m('eklentiler.izin_yok')}</span>
                                  {/if}
                                </span>
                                <span class="ikili">
                                  <span class="etiket">{m('eklentiler.depo')}</span>
                                  <span>{boyutMetni(a.depo_bayt)}</span>
                                  <Button
                                    size="sm"
                                    variant="ghost"
                                    disabled={mesgul}
                                    onclick={() =>
                                      void calistir(e.ad, () =>
                                        ipc.eklentiVerisiniSil(e.ad, a.kimlik),
                                      )}
                                  >
                                    {m('eklentiler.veriyi_sil')}
                                  </Button>
                                </span>
                              </div>
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                  {#if hatalar[e.ad]}
                    <p class="satir-hata" role="alert">{hatalar[e.ad]}</p>
                  {/if}
                  <div class="dugmeler">
                    {#if e.durum.tip !== 'Yuklenemedi'}
                      <Button
                        size="sm"
                        disabled={mesgul}
                        onclick={() =>
                          void calistir(e.ad, () =>
                            ipc.eklentiEtkinAyarla(e.ad, e.durum.tip === 'DevreDisi'),
                          )}
                      >
                        {e.durum.tip === 'DevreDisi'
                          ? m('eklentiler.etkinlestir')
                          : m('eklentiler.devre_disi')}
                      </Button>
                    {/if}
                    {#if !e.yerlesik}
                      <Button
                        size="sm"
                        iconLeft="rotate-ccw"
                        disabled={mesgul}
                        onclick={() => void kur(e.ad)}
                      >
                        {m('eklentiler.guncelle')}
                      </Button>
                      <Button
                        size="sm"
                        iconLeft="trash-2"
                        disabled={mesgul}
                        onclick={() => void calistir(e.ad, () => ipc.eklentiKaldir(e.ad))}
                      >
                        {m('eklentiler.kaldir')}
                      </Button>
                    {/if}
                  </div>
                </div>
              </Card>
            </div>
          </li>
        {/each}
      </ul>
    {:else if yukleniyor}
      <p class="bos" role="status">{m('eklentiler.yukleniyor')}</p>
    {:else}
      <p class="bos">{m('eklentiler.bos')}</p>
    {/if}
  </div>
</section>

<style>
  .eklentiler {
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
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
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
  /* Kısa pencerede içerik kırpılmaz, gövde kayar. */
  .govde > * {
    flex-shrink: 0;
  }
  .aciklama,
  .bos {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    color: var(--text-muted);
  }
  .kurulum {
    padding: 10px 14px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font: var(--type-small);
    color: var(--text-body);
  }
  .kurulum p {
    margin: 0;
  }
  .kurulum p + p {
    margin-top: 4px;
    color: var(--status-warn);
  }
  .kurulum.hata {
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 14%, transparent);
  }
  .liste,
  .alan-listesi,
  .araclar {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .liste {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .icerik {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .neden,
  .satir-hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .satir {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px 10px;
    font: var(--type-small);
  }
  .etiket {
    color: var(--text-muted);
  }
  .araclar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .arac {
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
    font: var(--type-meta);
    color: var(--text-body);
  }
  .ayrinti {
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .ayrinti summary {
    cursor: pointer;
  }
  .ayrinti dl {
    display: grid;
    grid-template-columns: 120px 1fr;
    gap: 8px 12px;
    margin: 10px 0 0;
  }
  .ayrinti dt {
    color: var(--text-muted);
  }
  .ayrinti dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--text-body);
  }
  .mono {
    font: var(--type-meta);
  }
  code {
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--surface-sunken);
    font: var(--type-meta);
  }
  h3 {
    margin: 0 0 8px;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .alan-listesi {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .alan {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .alan-ust {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
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
  .alan-alt {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 18px;
    font: var(--type-small);
    color: var(--text-body);
  }
  .ikili {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .dugmeler {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
