<!--
  Gönderim önizleme kartı (tasarım §6.4, §6.5): dış kaynaklı araç sonuçları
  modele gönderilmeden önce kullanıcıya gösterilir. Kullanıcı isterse bazı
  öğeleri çıkarabilir. Güvenlik denetçisi (3c) şüpheli bulduğu öğeleri de
  bu kartla sunar.
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

  let {
    oge,
    onkarar,
  }: {
    oge: Extract<EkranOgesi, { tip: 'GonderimOnizlemesi' }>;
    onkarar: (islem: IslemId, gonderilmeyecekler: CagriId[]) => void;
  } = $props();

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

  const baslik = $derived(
    oge.neden === 'Denetim'
      ? m('onizleme.kart.denetim_baslik', { sayi: oge.ogeler.length })
      : m('onizleme.kart.baslik', { sayi: oge.ogeler.length }),
  );

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
      <Button
        variant="primary"
        size="sm"
        title={m('onizleme.kart.isaret_ipucu')}
        onclick={() => onkarar(oge.islem, Array.from(secilenler))}
      >
        {secilenler.size === 0
          ? m('onizleme.kart.gonder')
          : m('onizleme.kart.isaretlileri_cikar')}
      </Button>
    {:else if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('onizleme.kart.yanitlandi')}</Badge>
    {:else}
      <Badge>{m('onizleme.kart.gecersiz')}</Badge>
    {/if}
  {/snippet}

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
  .denetim-uyarisi {
    font: var(--type-small);
    color: var(--status-warn);
  }
</style>
