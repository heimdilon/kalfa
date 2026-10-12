<!--
  Bütçe kararı kartı (5b tasarımı §7, §9): yeni istek, kullanıcı kararı
  verilmeden başlamaz. Kart nedeni, tüketimi ve eksik kullanımı gösterir; iki
  sonlu mutlak sınır girdisi sunar. "Bütçeyle devam et" kullanımı bilinmeyen
  istekleri kabul eder ama sayaçları sıfırlamaz. "Turu durdur" turu iptal eder.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Input from '../bilesenler/Input.svelte';
  import { m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { ButceKarari, ButceGorunumu } from '../ipc/tipler';
  import { sayi } from './bicim';
  import {
    butceGirdisiDogrula,
    kartAltSiniri,
    kartOnerisi,
    type ButceHataNedeni,
  } from './butce';

  let {
    oge,
    onkarar,
  }: {
    oge: Extract<EkranOgesi, { tip: 'ButceKarari' }>;
    onkarar: (karar: ButceKarari) => Promise<void>;
  } = $props();

  const kimlik = $derived(`butce-${oge.anahtar.replace(/[^A-Za-z0-9-]/g, '')}`);
  const istekKimligi = $derived(`${kimlik}-istek`);
  const tokenKimligi = $derived(`${kimlik}-token`);
  const hataKimligi = $derived(`${kimlik}-hata`);
  const ipucuKimligi = $derived(`${kimlik}-ipucu`);

  const g: ButceGorunumu = $derived(oge.gorunum);
  const acik = $derived(oge.durum === 'Bekliyor');
  const bilinmeyenKabul = $derived(g.bilinmeyen - g.onaylanan_bilinmeyen);

  // Girdiler her yeni karar gösteriminde mevcut limitlerle (ya da tüketimin
  // üstüne önerilen değerle) yeniden doldurulur.
  let istekMetni = $state('');
  let tokenMetni = $state('');
  let gonderiliyor = $state(false);
  let kaynak: ButceGorunumu | null = null;

  $effect(() => {
    if (oge.gorunum !== kaynak) {
      kaynak = oge.gorunum;
      const oneri = kartOnerisi(oge.gorunum);
      istekMetni = String(oneri.istek);
      tokenMetni = String(oneri.token);
    }
  });

  const girdi = $derived(butceGirdisiDogrula(istekMetni, tokenMetni, kartAltSiniri(g)));
  const hataNedeni: ButceHataNedeni | null = $derived(girdi.gecerli ? null : girdi.neden);

  const hataMetni = $derived.by(() => {
    if (hataNedeni === 'bicim') return m('butce.hata.bicim');
    if (hataNedeni === 'istek') return m('butce.hata.istek', { baslatilan: sayi(g.baslatilan) });
    if (hataNedeni === 'token') {
      return m('butce.hata.token', { bilinen: sayi(g.bilinen_token) });
    }
    return null;
  });

  const nedenMetni = $derived.by(() => {
    if (g.engel === 'bilinmeyen') return m('butce.neden.bilinmeyen', { sayi: sayi(g.bilinmeyen) });
    if (g.engel === 'token') return m('butce.neden.token');
    if (g.engel === 'istek') return m('butce.neden.istek');
    return '';
  });

  async function kararVer(karar: ButceKarari): Promise<void> {
    if (gonderiliyor) return;
    gonderiliyor = true;
    try {
      await onkarar(karar);
    } finally {
      gonderiliyor = false;
    }
  }

  function devamEt(): void {
    if (!girdi.gecerli) return;
    // Gözlenen sayaçlar kartın gösterdikleridir; değiştiyse Rust kararı reddeder.
    void kararVer({
      tip: 'DevamEt',
      sinirlar: girdi.sinirlar,
      gozlenen_baslatilan: g.baslatilan,
      gozlenen_bilinmeyen: g.bilinmeyen,
    });
  }
</script>

<Card
  tone={acik ? 'accent' : 'default'}
  icon="coins"
  title={m('butce.kart.baslik')}
>
  {#snippet actions()}
    {#if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('butce.kart.yanitlandi')}</Badge>
    {:else if oge.durum === 'Gecersiz'}
      <Badge>{m('butce.kart.gecersiz')}</Badge>
    {/if}
  {/snippet}

  {#if nedenMetni}
    <p class="neden">{nedenMetni}</p>
  {/if}

  <ul class="tuketim">
    <li>{m('butce.kart.istek', { baslatilan: sayi(g.baslatilan), sinir: sayi(g.sinirlar.istek) })}</li>
    <li>
      {m('butce.kart.token', { bilinen: sayi(g.bilinen_token), sinir: sayi(g.sinirlar.token) })}
    </li>
    {#if g.bilinmeyen > 0}
      <li class="bilinmeyen">{m('butce.kart.bilinmeyen', { sayi: sayi(g.bilinmeyen) })}</li>
    {/if}
  </ul>

  {#if acik}
    <form
      class="karar"
      onsubmit={(e) => {
        e.preventDefault();
        devamEt();
      }}
    >
      <div class="girdiler">
        <div class="girdi-alani">
          <label class="etiket" for={istekKimligi}>{m('butce.girdi.istek')}</label>
          <Input
            id={istekKimligi}
            mono
            inputmode="numeric"
            autocomplete="off"
            invalid={hataNedeni === 'istek' || hataNedeni === 'bicim'}
            aria-describedby={hataMetni ? `${hataKimligi} ${ipucuKimligi}` : ipucuKimligi}
            bind:value={istekMetni}
          />
        </div>
        <div class="girdi-alani">
          <label class="etiket" for={tokenKimligi}>{m('butce.girdi.token')}</label>
          <Input
            id={tokenKimligi}
            mono
            inputmode="numeric"
            autocomplete="off"
            invalid={hataNedeni === 'token' || hataNedeni === 'bicim'}
            aria-describedby={hataMetni ? `${hataKimligi} ${ipucuKimligi}` : ipucuKimligi}
            bind:value={tokenMetni}
          />
        </div>
      </div>
      <p id={ipucuKimligi} class="ipucu">{m('butce.girdi.ipucu')}</p>
      {#if hataMetni}
        <p id={hataKimligi} class="hata" role="alert">{hataMetni}</p>
      {/if}

      <div class="dugmeler">
        <Button
          type="submit"
          variant="primary"
          size="sm"
          disabled={!girdi.gecerli || gonderiliyor}
        >
          {m('butce.devam')}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          disabled={gonderiliyor}
          onclick={() => void kararVer({ tip: 'Durdur' })}
        >
          {m('butce.durdur')}
        </Button>
      </div>
      {#if bilinmeyenKabul > 0}
        <p class="aciklama">
          {m('butce.devam.bilinmeyen', { sayi: sayi(bilinmeyenKabul) })}
        </p>
      {/if}
    </form>
  {/if}
</Card>

<style>
  .neden {
    margin: 0 0 8px;
    font: var(--type-small);
    color: var(--text-body);
  }
  .tuketim {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .bilinmeyen {
    color: var(--status-warn);
  }
  .karar {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: 12px;
  }
  .girdiler {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
  }
  .girdi-alani {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 160px;
    flex: 1 1 160px;
  }
  .etiket {
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .ipucu,
  .aciklama {
    margin: 0;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .dugmeler {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
