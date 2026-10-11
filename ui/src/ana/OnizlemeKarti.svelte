<!--
  Gönderim önizleme kartı (tasarım §6.4, §6.5): dış kaynaklı araç sonuçları
  modele gönderilmeden önce kullanıcıya gösterilir. Kullanıcı isterse bazı
  öğeleri çıkarabilir. Güvenlik denetçisi (3c) şüpheli bulduğu öğeleri de
  bu kartla sunar. Eklentinin EVREN isteğinde (aşama 4, §9.4) kart üç karar
  sunar: gönder, gönderme, bu araç çağrısında hep gönder.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import SourceTag from '../bilesenler/SourceTag.svelte';
  import { untrack } from 'svelte';
  import { denetimKategorisi, m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { CagriId, IslemId } from '../ipc/tipler';
  import EklentiAyrinti from './EklentiAyrinti.svelte';

  let {
    oge,
    onkarar,
  }: {
    oge: Extract<EkranOgesi, { tip: 'GonderimOnizlemesi' }>;
    onkarar: (
      islem: IslemId,
      gonderilmeyecekler: CagriId[],
      hepGonderilecekler: CagriId[],
    ) => void;
  } = $props();

  const eklenti = $derived(oge.neden === 'Eklenti');
  const cagrilar = $derived(oge.ogeler.map((o) => o.cagri));

  // İşaretin anlamı "gönderme"dir. Katı denetimde şüpheli ve denetlenemeyen
  // öğeler işaretli gelir; kullanıcı işareti kaldırırsa gider (3c Görev 6).
  let secilenler = $state<Set<CagriId>>(
    untrack(() =>
      oge.neden === 'Denetim'
        ? new Set(oge.ogeler.filter((o) => o.denetim).map((o) => o.cagri))
        : new Set(),
    ),
  );

  /** Çekirdeğin etiketi ("şüpheli: <kategori>", "denetlenemedi") okunur metne. */
  function denetimEtiketi(etiket: string): string {
    if (etiket.startsWith('şüpheli: ')) {
      return m('denetim.rozet.supheli', {
        kategori: denetimKategorisi(etiket.slice('şüpheli: '.length)),
      });
    }
    if (etiket === 'denetlenemedi') return m('denetim.rozet.bilinmiyor');
    return etiket;
  }

  const baslik = $derived.by(() => {
    if (eklenti) return m('onizleme.kart.eklenti_baslik');
    if (oge.neden === 'Denetim') {
      return m('onizleme.kart.denetim_baslik', { sayi: oge.ogeler.length });
    }
    return m('onizleme.kart.baslik', { sayi: oge.ogeler.length });
  });

  function secimDegistir(cagri: CagriId, secildi: boolean) {
    const sonraki = new Set(secilenler);
    if (secildi) sonraki.add(cagri);
    else sonraki.delete(cagri);
    secilenler = sonraki;
  }
</script>

<Card
  tone={oge.durum === 'Bekliyor' ? 'accent' : 'default'}
  icon="send"
  title={baslik}
>
  {#snippet actions()}
    {#if oge.durum === 'Bekliyor'}
      {#if !eklenti}
        <Button
          variant="primary"
          size="sm"
          title={m('onizleme.kart.isaret_ipucu')}
          onclick={() => onkarar(oge.islem, Array.from(secilenler), [])}
        >
          {secilenler.size === 0
            ? m('onizleme.kart.gonder')
            : m('onizleme.kart.isaretlileri_cikar')}
        </Button>
      {/if}
    {:else if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('onizleme.kart.yanitlandi')}</Badge>
    {:else}
      <Badge>{m('onizleme.kart.gecersiz')}</Badge>
    {/if}
  {/snippet}

  {#if eklenti}
    <ul class="ogeler">
      {#each oge.ogeler as o (o.cagri)}
        <li class="eklenti-ogesi">
          {#if o.eklenti}
            <span class="eklenti-adi">{m('eklenti.amac', { ad: o.eklenti.eklenti })}</span>
            <EklentiAyrinti ozet={o.eklenti} />
          {:else}
            <SourceTag kaynak={o.kaynak} bayt={o.bayt} />
          {/if}
        </li>
      {/each}
    </ul>
    {#if oge.durum === 'Bekliyor'}
      <div class="karar-dugmeleri">
        <Button variant="primary" size="sm" onclick={() => onkarar(oge.islem, [], [])}>
          {m('onizleme.kart.gonder')}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          onclick={() => onkarar(oge.islem, cagrilar, [])}
        >
          {m('onizleme.kart.eklenti.gonderme')}
        </Button>
        <Button variant="ghost" size="sm" onclick={() => onkarar(oge.islem, [], cagrilar)}>
          {m('onizleme.kart.eklenti.hep_gonder')}
        </Button>
      </div>
    {/if}
  {:else}
    <ul class="ogeler">
      {#each oge.ogeler as o (o.cagri)}
        <li class="oge-satiri">
          {#if oge.durum === 'Bekliyor'}
            <Checkbox
              checked={secilenler.has(o.cagri)}
              onchange={(dolu) => secimDegistir(o.cagri, dolu)}
            />
          {/if}
          <SourceTag kaynak={o.kaynak} bayt={o.bayt} />
          {#if o.denetim}
            <span class="denetim-uyarisi">{denetimEtiketi(o.denetim)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</Card>

<style>
  .ogeler {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .oge-satiri {
    display: flex;
    align-items: center;
    gap: 10px;
    font: var(--type-small);
    min-width: 0;
    flex-wrap: wrap;
  }
  .eklenti-ogesi {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .eklenti-adi {
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .karar-dugmeleri {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .denetim-uyarisi {
    font: var(--type-small);
    color: var(--status-warn);
  }
</style>
