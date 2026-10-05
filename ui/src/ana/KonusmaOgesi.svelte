<!-- Konuşmadaki tek öğe (1c tasarımı §5.5, §8.1, §8.2, §10). -->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import type { IkonAdi } from '../bilesenler/ikonlar';
  import Message from '../bilesenler/Message.svelte';
  import ThoughtPanel from '../bilesenler/ThoughtPanel.svelte';
  import ToolCallCard from '../bilesenler/ToolCallCard.svelte';
  import { AKTARIM_DURMA_METNI, denetimKategorisi, m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { BitisNedeni, OturumId } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';
  import type { AnaDurum } from './anaDurum.svelte';
  import {
    aracBasligi,
    aracHedefi,
    aracKartDurumu,
    ciktiOnizlemesi,
    kabukOzeti,
    sonucGovdesi,
  } from './bicim';
  import { geriAlmaSonucMetni } from './geriAlma';
  import OnayKarti from './OnayKarti.svelte';
  import OnizlemeKarti from './OnizlemeKarti.svelte';
  import TerminalGorunumu from './TerminalGorunumu.svelte';
  import TurOzeti from './TurOzeti.svelte';

  let { oge, durum, oturum }: { oge: EkranOgesi; durum: AnaDurum; oturum: OturumId } = $props();

  const ARAC_IKONU: Record<string, IkonAdi> = {
    dosya_oku: 'file-text',
    dosya_listele: 'folder-open',
    icerik_ara: 'file-search',
    dosya_yaz: 'file-pen',
    dosya_duzenle: 'file-pen',
    kabuk: 'square-terminal',
  };

  function turSonu(neden: BitisNedeni): string {
    if (neden.tip === 'Sinir') return m('tur.bitti.Sinir', { aciklama: neden.aciklama });
    return m(`tur.bitti.${neden.tip}`);
  }
</script>

{#if oge.tip === 'KullaniciMesaji'}
  <Message author="user">
    <p class="duz">{oge.metin}</p>
  </Message>
{:else if oge.tip === 'Yanit'}
  <Message streaming={oge.akista}>
    {#if oge.metin}
      <Markdown metin={oge.metin} akista={oge.akista} baglantiAc={durum.baglantiAc} />
    {/if}
    {#if oge.dusunce}
      <div class="dusunce">
        <ThoughtPanel>
          <Markdown metin={oge.dusunce} akista={oge.akista} baglantiAc={durum.baglantiAc} />
        </ThoughtPanel>
      </div>
    {/if}
  </Message>
{:else if oge.tip === 'AracCagrisi'}
  {@const d = aracKartDurumu(oge.durum)}
  <ToolCallCard
    tool={aracBasligi(oge)}
    target={aracHedefi(oge)}
    meta={kabukOzeti(oge)}
    status={d.status}
    statusLabel={d.etiket}
    icon={ARAC_IKONU[oge.arac] ?? 'box'}
  >
    {#snippet rozet()}
      {#if oge.denetim?.tip === 'Suruyor'}
        <Badge tone="info" icon="loader-circle">{m('denetim.rozet.suruyor')}</Badge>
      {:else if oge.denetim?.tip === 'Supheli'}
        <span title={oge.denetim.gerekce}>
          <Badge tone="danger" icon="shield-alert">
            {m('denetim.rozet.supheli', { kategori: denetimKategorisi(oge.denetim.kategori) })}
          </Badge>
        </span>
      {:else if oge.denetim?.tip === 'Bilinmiyor'}
        <span title={oge.denetim.neden}>
          <Badge tone="warn" icon="circle-help">{m('denetim.rozet.bilinmiyor')}</Badge>
        </span>
      {:else if oge.denetim?.tip === 'Atlandi'}
        <Badge tone="neutral" icon="shield-check">{m('denetim.rozet.atlandi')}</Badge>
      {/if}
    {/snippet}
    {#if oge.terminal}
      <TerminalGorunumu terminal={oge.terminal} />
    {:else if oge.sonuc}
      {@const o = ciktiOnizlemesi(sonucGovdesi(oge.sonuc))}
      <CodeBlock code={o.kod} truncated={o.kirpildi} maxHeight="320px" />
    {/if}
  </ToolCallCard>
{:else if oge.tip === 'OnayIstegi'}
  <OnayKarti {oge} onincele={() => durum.onayiAc(oturum, oge.islem)} />
{:else if oge.tip === 'GonderimOnizlemesi'}
  <OnizlemeKarti
    {oge}
    onkarar={(islem, gonderilmeyecekler) =>
      durum.gonderimKarari(oturum, islem, gonderilmeyecekler)}
  />
{:else if oge.tip === 'SartIstegi'}
  <Card tone="accent" icon="shield" title={m('sart.kart.baslik', { surum: oge.surum })}>
    {#snippet actions()}
      {#if oge.durum === 'Bekliyor'}
        <Button size="sm" onclick={() => durum.sartPenceresiniGoster()}>
          {m('sart.kart.ac')}
        </Button>
      {/if}
    {/snippet}
    <p class="aciklama">
      {oge.durum === 'Bekliyor' ? m('sart.kart.bekliyor') : m('sart.kart.cozuldu')}
    </p>
  </Card>
{:else if oge.tip === 'Hata'}
  <Card tone="danger" icon="triangle-alert" title={m('hata.kart.baslik')}>
    {#snippet actions()}
      {#if oge.mesaj === m('hata.anahtar_gecersiz')}
        <Button size="sm" iconLeft="key-round" onclick={() => durum.anahtariDegistir()}>
          {m('menu.anahtar')}
        </Button>
      {/if}
    {/snippet}
    <p class="aciklama">{oge.mesaj}</p>
  </Card>
{:else if oge.tip === 'GuvenlikUyarisi'}
  <Card tone="danger" icon="shield-alert" title={m('guvenlik.kart.baslik')}>
    <p class="aciklama">{oge.mesaj}</p>
  </Card>
{:else if oge.tip === 'TurSonu'}
  <div class={['tur-sonu', `n-${oge.neden.tip}`]}>
    <span>{turSonu(oge.neden)}</span>
  </div>
{:else if oge.tip === 'GeriAlma'}
  <div class="bilgi-satiri">
    <span>
      {oge.yarim ? m('geri_alma.yarim') : geriAlmaSonucMetni(oge.sonuc)}
    </span>
  </div>
{:else if oge.tip === 'TurOzeti'}
  <TurOzeti
    ozet={oge.ozet}
    geriAlindi={oge.geri_alindi}
    ongeri={() => durum.geriAlmaOnizle(oturum, oge.ozet.tur)}
    ongunluk={() => durum.gunluguAc(oturum)}
  />
{:else if oge.tip === 'KalfaNotu'}
  <div class="bilgi-satiri">
    <span>{m('geri_alma.not', { metin: oge.metin })}</span>
  </div>
{:else if oge.tip === 'AktarimDurdu'}
  <Card tone="danger" icon="triangle-alert" title={m('aktarim.baslik')}>
    {#snippet actions()}
      <Button size="sm" iconLeft="rotate-ccw" onclick={() => durum.yenidenAc(oturum)}>
        {m('konusma.yeniden_ac')}
      </Button>
    {/snippet}
    <p class="aciklama">{m(AKTARIM_DURMA_METNI[oge.neden])}</p>
  </Card>
{/if}

<style>
  .duz {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .dusunce {
    margin-top: 10px;
  }
  .aciklama {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .tur-sonu {
    display: flex;
    align-items: center;
    gap: 12px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .tur-sonu::before,
  .tur-sonu::after {
    content: '';
    flex: 1;
    height: var(--border-width);
    background: var(--border-subtle);
  }
  .n-Hata {
    color: var(--status-danger);
  }
  /* Geri alma ve Kalfa notu. Tur sonu çizgisinden ayrı sınıftır: uçtan uca
     testler tur sonlarını `.tur-sonu` ile sayar. */
  .bilgi-satiri {
    display: flex;
    align-items: center;
    gap: 12px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
