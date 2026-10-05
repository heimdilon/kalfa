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
  import { m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { CagriId, IslemId } from '../ipc/tipler';

  let {
    oge,
    onkarar,
  }: {
    oge: Extract<EkranOgesi, { tip: 'GonderimOnizlemesi' }>;
    onkarar: (islem: IslemId, gonderilmeyecekler: CagriId[]) => void;
  } = $props();

  let secilenler = $state<Set<CagriId>>(new Set());

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
        onclick={() => onkarar(oge.islem, Array.from(secilenler))}
      >
        {m('onizleme.kart.gonder')}
      </Button>
      <Button
        variant="secondary"
        size="sm"
        disabled={secilenler.size === 0}
        onclick={() => onkarar(oge.islem, Array.from(secilenler))}
      >
        {m('onizleme.kart.secilenleri_gonderme')}
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
          <span class="denetim-uyarisi">{o.denetim}</span>
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
