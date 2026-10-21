<!--
  Arka plan işi ve etkileşimli terminal kartı (G2b / G2c).
  - Çıktıda yalnızca güvenli SGR renk/kalınlık stilleri (`ansiParcala`) ve görünmez
    Unicode kaçışları (`GorunurMetin`) çizilir; OSC 8 bağlantı, OSC 52 pano ve
    DCS/APC/PM/SOS dizileri tamamen ayıklanır; HTML olarak yorumlanmaz.
  - Modelin `komut_girdi` onay durumu ile kullanıcının yerel denetimleri (durdur,
    girdi gönder, EOF) birbirinden ayrı gösterilir.
-->
<script lang="ts" module>
  export type IsKartKipi =
    | { kip: 'borulu' }
    | { kip: 'terminal'; boyut: { sutun: number; satir: number } };

  export type IsKartDurumu =
    | { durum: 'calisiyor' }
    | { durum: 'durduruluyor' }
    | { durum: 'bitti'; cikis_kodu: number | null; sure_ms: number; zaman_asimi: boolean }
    | { durum: 'sonucu_bilinmiyor'; neden: string };

  export type ModelGirdiDurumu = 'yok' | 'onay_bekliyor' | 'reddedildi' | 'gonderildi';

  // OSC (ESC ] ... BEL/ST), DCS (ESC P ... ST), SOS (ESC X ... ST), PM (ESC ^ ... ST),
  // APC (ESC _ ... ST) kontrol dizilerini SGR ayrıştırmasından önce temizler.
  const TEHLIKELI_KONTROL_DIZILERI =
    /\x1b\][\s\S]*?(?:\x07|\x1b\\|$)|\x1b[PX^_][\s\S]*?(?:\x07|\x1b\\|$)/g;

  export function terminalMetniniTemizle(girdi: string): string {
    return girdi.replace(TEHLIKELI_KONTROL_DIZILERI, '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  }
</script>

<script lang="ts">
  import { ansiParcala } from '../ana/ansi';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  import Card from './Card.svelte';
  import GorunurMetin from './GorunurMetin.svelte';
  import Input from './Input.svelte';

  interface Props {
    isId: string;
    gosterim: string;
    kip: IsKartKipi;
    durum: IsKartDurumu;
    cikti?: string;
    toplamBayt?: number;
    tasti?: boolean;
    uyarilar?: string[];
    modelGirdiDurumu?: ModelGirdiDurumu;
    modelGirdiAciklama?: string;
    onDurdur?: (isId: string) => void;
    onKullaniciGirdi?: (isId: string, veri: string) => void;
    onKullaniciEof?: (isId: string) => void;
  }

  let {
    isId,
    gosterim,
    kip,
    durum,
    cikti = '',
    toplamBayt = 0,
    tasti = false,
    uyarilar = [],
    modelGirdiDurumu = 'yok',
    modelGirdiAciklama,
    onDurdur,
    onKullaniciGirdi,
    onKullaniciEof,
  }: Props = $props();

  let kullaniciGirdisi = $state('');

  const temizCikti = $derived(terminalMetniniTemizle(cikti));
  const satirlar = $derived(temizCikti ? temizCikti.split('\n') : []);
  const calisiyor = $derived(durum.durum === 'calisiyor');
  const etkin = $derived(durum.durum === 'calisiyor' || durum.durum === 'durduruluyor');

  const kipEtiketi = $derived(
    kip.kip === 'terminal'
      ? `Terminal (${kip.boyut.sutun}×${kip.boyut.satir})`
      : 'Arka plan işi (borulu)',
  );

  const durumBilgisi = $derived.by(() => {
    switch (durum.durum) {
      case 'calisiyor':
        return { etiket: 'Çalışıyor', ton: 'warn' as const };
      case 'durduruluyor':
        return { etiket: 'Durduruluyor…', ton: 'warn' as const };
      case 'bitti':
        if (durum.zaman_asimi) {
          return { etiket: 'Zaman aşımı', ton: 'danger' as const };
        }
        if (durum.cikis_kodu === 0) {
          return { etiket: 'Tamamlandı (0)', ton: 'ok' as const };
        }
        return {
          etiket:
            durum.cikis_kodu !== null
              ? `Çıkış kodu: ${durum.cikis_kodu}`
              : 'Durduruldu',
          ton: 'neutral' as const,
        };
      case 'sonucu_bilinmiyor':
        return { etiket: 'Sonucu bilinmiyor', ton: 'danger' as const };
    }
  });

  const altBaslik = $derived(`${isId} · ${kipEtiketi} · ${toplamBayt} bayt`);

  function kullaniciGirdiGonder() {
    if (!calisiyor || !onKullaniciGirdi || !kullaniciGirdisi) return;
    // PTY/ConPTY'de Enter CR'dır; borulu süreçler LF satır sonu alır.
    const son = kip.kip === 'terminal' ? '\r' : '\n';
    const veri = /[\r\n]$/.test(kullaniciGirdisi) ? kullaniciGirdisi : `${kullaniciGirdisi}${son}`;
    kullaniciGirdisi = '';
    onKullaniciGirdi(isId, veri);
  }
</script>

<Card
  title={gosterim}
  subtitle={altBaslik}
  icon="square-terminal"
  tone={durum.durum === 'sonucu_bilinmiyor' ? 'danger' : 'default'}
>
  {#snippet actions()}
    <Badge tone={durumBilgisi.ton} dot={etkin}>{durumBilgisi.etiket}</Badge>
    {#if onDurdur && calisiyor}
      <Button
        variant="danger"
        size="sm"
        iconLeft="circle-stop"
        onclick={() => onDurdur(isId)}
      >
        İşi durdur
      </Button>
    {/if}
  {/snippet}

  <div class="govde">
    {#if modelGirdiDurumu !== 'yok'}
      <div class={['model-durumu', `m-${modelGirdiDurumu}`]} role="status">
        <span class="model-etiket">
          {#if modelGirdiDurumu === 'onay_bekliyor'}
            Model standart girdi göndermek için onayınızı bekliyor
          {:else if modelGirdiDurumu === 'reddedildi'}
            Modelin standart girdi isteği reddedildi; iş çalışmaya devam ediyor
          {:else}
            Model standart girdi gönderdi
          {/if}
        </span>
        {#if modelGirdiAciklama}
          <code class="model-onizleme"><GorunurMetin metin={modelGirdiAciklama} /></code>
        {/if}
      </div>
    {/if}

    {#if durum.durum === 'sonucu_bilinmiyor'}
      <p class="belirsiz" role="alert">
        <GorunurMetin metin={durum.neden} />
      </p>
    {/if}

    {#if uyarilar.length > 0}
      <ul class="uyarilar" aria-label="Güvenlik uyarıları">
        {#each uyarilar as uyari, idx (idx)}
          <li><GorunurMetin metin={uyari} /></li>
        {/each}
      </ul>
    {/if}

    {#if tasti}
      <p class="tasti-notu">
        Çıktı tamponu 1 MiB sınırına ulaştı; yeni baytlar saklanmadan tüketiliyor.
      </p>
    {/if}

    <pre class="terminal" aria-label={`${isId} çıktısı`}>{#if satirlar.length === 0}<span
          class="bos">Henüz çıktı yok.</span
        >{:else}{#each satirlar as satir, i (i)}{#if i > 0}{'\n'}{/if}<span class="satir"
            >{#each ansiParcala(satir) as parca, j (j)}<span
                class={[parca.renk && `r-${parca.renk}`, { kalin: parca.kalin }]}
                ><GorunurMetin metin={parca.metin} /></span
              >{/each}</span
          >{/each}{/if}</pre>

    {#if calisiyor && (onKullaniciGirdi || onKullaniciEof)}
      <div class="kullanici-denetim" aria-label="Yerel kullanıcı standart girdi denetimleri">
        {#if onKullaniciGirdi}
          <div class="girdi-satiri">
            <Input
              bind:value={kullaniciGirdisi}
              placeholder="Standart girdiye satır gönderin…"
              aria-label="Standart girdi metni"
              mono
            />
            <Button
              variant="secondary"
              size="sm"
              disabled={!kullaniciGirdisi}
              onclick={kullaniciGirdiGonder}
            >
              Girdi gönder
            </Button>
          </div>
        {/if}
        {#if onKullaniciEof}
          <Button
            variant="ghost"
            size="sm"
            onclick={() => onKullaniciEof(isId)}
          >
            EOF gönder
          </Button>
        {/if}
      </div>
    {/if}
  </div>
</Card>

<style>
  .govde {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .model-durumu {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    border: var(--border-width) solid var(--border-subtle);
    font: var(--type-small);
    background: var(--surface-sunken);
    color: var(--text-secondary);
  }
  .m-onay_bekliyor {
    border-color: color-mix(in oklab, var(--amber-200) 42%, transparent);
    color: var(--status-warn);
  }
  .m-reddedildi {
    border-color: color-mix(in oklab, var(--red-400) 40%, transparent);
    color: var(--status-danger);
  }
  .model-onizleme {
    font-family: var(--font-mono);
    font-size: var(--text-micro);
    color: var(--text-body);
  }
  .belirsiz {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-md);
    background: color-mix(in oklab, var(--red-400) 14%, transparent);
    color: var(--status-danger);
    font: var(--type-small);
  }
  .uyarilar {
    margin: 0;
    padding-left: 18px;
    color: var(--status-warn);
    font: var(--type-small);
  }
  .tasti-notu {
    margin: 0;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .terminal {
    margin: 0;
    max-height: 280px;
    overflow: auto;
    padding: 10px 12px;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font: var(--text-small) / 1.5 var(--font-mono);
    color: var(--text-body);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .bos {
    color: var(--text-muted);
    font-style: italic;
  }
  .r-kirmizi {
    color: var(--status-danger);
  }
  .r-yesil {
    color: var(--status-ok);
  }
  .r-sari {
    color: var(--status-warn);
  }
  .r-mavi {
    color: var(--status-info);
  }
  .r-soluk {
    color: var(--text-muted);
  }
  .kalin {
    font-weight: var(--fw-semibold);
  }
  .kullanici-denetim {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .girdi-satiri {
    display: flex;
    flex: 1;
    min-width: 200px;
    align-items: center;
    gap: 8px;
  }
</style>
