<!--
  Web kaynak kartı (Yetenek-05 Görev 3): `web_getir` aracının okuduğu dış
  sayfanın kaynak URL'sini, yönlendirme sonrası son URL'sini, başlığını,
  kırpma/imleç durumunu ve düz metin önizlemesini gösterir.
  Güvenlik sınırları:
  - Ham HTML yönergesi asla kullanılmaz; tüm metin düğümleri düz metin olarak işlenir.
  - Şablonda satır içi stil özniteliği kullanılmaz (sıkı CSP uyumu).
  - Harici bağlantı kullanıcı açıkça onaylamadan açılmaz; yalnız `http:` ve
    `https:` şemalarına izin verilir.
-->
<script lang="ts">
  import { boyutMetni } from '../ana/geriAlma';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import SourceTag from './SourceTag.svelte';

  interface Props {
    istenenUrl: string;
    sonUrl: string;
    baslik?: string | null;
    icerikTuru?: string;
    acilmisBayt: number;
    ciktiBayt: number;
    kirpildi?: boolean;
    devamImleci?: string | null;
    govdeOnizleme?: string;
    onBaglantiAc?: (url: string) => void;
    class?: string;
  }

  let {
    istenenUrl,
    sonUrl,
    baslik = null,
    icerikTuru = 'text/html',
    acilmisBayt,
    ciktiBayt,
    kirpildi = false,
    devamImleci = null,
    govdeOnizleme = '',
    onBaglantiAc,
    class: sinif,
  }: Props = $props();

  let onayAcik = $state(false);

  const yonlendirildi = $derived(istenenUrl !== sonUrl);
  const kaynakEtiketi = $derived(`web:${sonUrl}`);
  const gorunenBaslik = $derived(
    baslik && baslik.trim().length > 0 ? baslik.trim() : 'Başlıksız web sayfası',
  );
  const ciktiBoyutMetni = $derived(boyutMetni(ciktiBayt));
  const acilmisBoyutMetni = $derived(boyutMetni(acilmisBayt));

  const baglantiGuvenli = $derived.by((): boolean => {
    try {
      const parsed = new URL(sonUrl);
      return (
        (parsed.protocol === 'https:' || parsed.protocol === 'http:') &&
        parsed.username === '' &&
        parsed.password === ''
      );
    } catch {
      return false;
    }
  });

  function onayIste() {
    if (!baglantiGuvenli || !onBaglantiAc) return;
    onayAcik = true;
  }

  function onayVazgec() {
    onayAcik = false;
  }

  function onayliBaglantiAc() {
    if (!baglantiGuvenli || !onBaglantiAc) return;
    onayAcik = false;
    onBaglantiAc(sonUrl);
  }
</script>

<section class={['web-kaynak-karti', sinif]} aria-label={gorunenBaslik}>
  <header class="ust">
    <div class="baslik-satiri">
      <Icon name="external-link" size={15} />
      <h4 class="baslik">{gorunenBaslik}</h4>
      <Badge tone="warn">Dış web kaynağı (güvenilmez içerik)</Badge>
      {#if kirpildi}
        <Badge tone="accent">Kırpıldı</Badge>
      {:else}
        <Badge tone="neutral">Tam metin</Badge>
      {/if}
    </div>
    <div class="etiket-satiri">
      <SourceTag kaynak={kaynakEtiketi} bayt={ciktiBayt} />
      <span class="meta-etiket">{icerikTuru}</span>
      <span class="meta-etiket">Çıktı: {ciktiBoyutMetni} / Kaynak: {acilmisBoyutMetni}</span>
    </div>
  </header>

  <dl class="url-listesi">
    <div class="url-satiri">
      <dt>Kaynak URL</dt>
      <dd class="mono">{istenenUrl}</dd>
    </div>
    {#if yonlendirildi}
      <div class="url-satiri yonlendirildi">
        <dt>Son URL (yönlendirildi)</dt>
        <dd class="mono">{sonUrl}</dd>
      </div>
    {:else}
      <div class="url-satiri">
        <dt>Son URL</dt>
        <dd class="mono">{sonUrl}</dd>
      </div>
    {/if}
    {#if kirpildi && devamImleci}
      <div class="url-satiri">
        <dt>Devam imleci</dt>
        <dd class="mono">{devamImleci}</dd>
      </div>
    {/if}
  </dl>

  {#if govdeOnizleme}
    <pre class="onizleme">{govdeOnizleme}</pre>
  {/if}

  {#if onBaglantiAc}
    <footer class="alt">
      {#if !baglantiGuvenli}
        <span class="guvensiz-uyari" role="status">
          Güvensiz bağlantı şeması engellendi
        </span>
      {:else if onayAcik}
        <div class="baglanti-onay" role="group" aria-label="Harici bağlantı onayı">
          <span class="onay-metin">
            Bu harici bağlantıyı tarayıcıda açmak istiyor musunuz: <code class="mono">{sonUrl}</code>
          </span>
          <div class="onay-dugmeler">
            <Button variant="secondary" size="sm" onclick={onayliBaglantiAc}>
              Tarayıcıda aç
            </Button>
            <Button variant="ghost" size="sm" onclick={onayVazgec}>
              Vazgeç
            </Button>
          </div>
        </div>
      {:else}
        <Button variant="ghost" size="sm" onclick={onayIste}>
          Harici bağlantıyı aç
        </Button>
      {/if}
    </footer>
  {/if}
</section>

<style>
  .web-kaynak-karti {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .ust {
    display: flex;
    flex-direction: column;
    gap: var(--space-1-5);
  }

  .baslik-satiri {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    color: var(--fg-primary);
  }

  .baslik {
    margin: 0;
    font-size: var(--text-sm);
    font-weight: 600;
    color: var(--fg-primary);
  }

  .etiket-satiri {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }

  .meta-etiket {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--fg-muted);
  }

  .url-listesi {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: var(--space-2);
    background: var(--bg-sunken);
    border-radius: var(--radius-sm);
  }

  .url-satiri {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: var(--space-2);
    font-size: var(--text-xs);
  }

  .url-satiri dt {
    color: var(--fg-muted);
    font-weight: 500;
  }

  .url-satiri dd {
    margin: 0;
    color: var(--fg-secondary);
    word-break: break-all;
  }

  .url-satiri.yonlendirildi dd {
    color: var(--warning-fg);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .onizleme {
    margin: 0;
    padding: var(--space-2);
    max-height: 220px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-relaxed);
    color: var(--fg-secondary);
    background: var(--bg-sunken);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-sm);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .alt {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding-top: var(--space-1);
    border-top: 1px solid var(--border-subtle);
  }

  .baglanti-onay {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
  }

  .onay-metin {
    font-size: var(--text-xs);
    color: var(--fg-secondary);
    word-break: break-all;
  }

  .onay-dugmeler {
    display: flex;
    align-items: center;
    gap: var(--space-1-5);
  }

  .guvensiz-uyari {
    font-size: var(--text-xs);
    color: var(--danger-fg);
  }
</style>
