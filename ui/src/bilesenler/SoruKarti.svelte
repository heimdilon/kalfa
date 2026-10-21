<script lang="ts">
  import { m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { KullaniciSorusuYaniti, TurId } from '../ipc/tipler';
  import Badge from './Badge.svelte';
  import Button from './Button.svelte';
  import Card from './Card.svelte';
  import Input from './Input.svelte';

  type SoruOgesi = Extract<EkranOgesi, { tip: 'KullaniciSorusu' }>;

  interface Props {
    oge: SoruOgesi;
    disabled?: boolean;
    onyanitla?: (tur: TurId, soruKimligi: string, yanit: KullaniciSorusuYaniti) => void;
    oniptal?: () => void;
  }

  let { oge, disabled = false, onyanitla, oniptal }: Props = $props();

  let serbestMetin = $state('');

  const etkilesimli = $derived(oge.durum === 'Bekliyor' && !disabled);
  const secilenSecenekKimligi = $derived(
    oge.sonuc?.durum === 'Yanitlandi' && oge.sonuc.yanit.tur === 'Secenek'
      ? oge.sonuc.yanit.secenek_kimligi
      : null,
  );
  const secilenEtiket = $derived.by(() => {
    if (oge.sonuc?.durum !== 'Yanitlandi') return null;
    if (oge.sonuc.secilen_etiket) return oge.sonuc.secilen_etiket;
    if (oge.sonuc.yanit.tur === 'Secenek') {
      const kimlik = oge.sonuc.yanit.secenek_kimligi;
      return oge.soru.secenekler.find((s) => s.kimlik === kimlik)?.etiket ?? kimlik;
    }
    return null;
  });
  const serbestGecerli = $derived.by(() => {
    const temiz = serbestMetin.trim();
    return temiz.length > 0 && temiz.length <= 4000;
  });

  function secenekSec(secenekKimligi: string) {
    if (!etkilesimli) return;
    onyanitla?.(oge.soru.tur, oge.soru.soru_kimligi, {
      tur: 'Secenek',
      secenek_kimligi: secenekKimligi,
    });
  }

  function serbestGonder() {
    if (!etkilesimli || !oge.soru.serbest_yanit_izni || !serbestGecerli) return;
    const metin = serbestMetin.trim();
    serbestMetin = '';
    onyanitla?.(oge.soru.tur, oge.soru.soru_kimligi, {
      tur: 'SerbestMetin',
      metin,
    });
  }

  function serbestTus(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      serbestGonder();
    }
  }
</script>

<Card
  tone={oge.durum === 'Bekliyor' ? 'accent' : 'default'}
  icon="info"
  title={m('soru.kart.baslik')}
>
  {#snippet actions()}
    {#if oge.durum === 'Bekliyor'}
      <Badge tone="accent">{m('soru.kart.bekliyor')}</Badge>
      {#if etkilesimli && oniptal}
        <Button variant="ghost" size="sm" onclick={oniptal}>{m('soru.serbest.vazgec')}</Button>
      {/if}
    {:else if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('soru.kart.yanitlandi')}</Badge>
    {:else if oge.durum === 'KullaniciMesajiylaIptal'}
      <Badge tone="info">{m('soru.kart.yonlendirildi')}</Badge>
    {:else if oge.durum === 'KesintiyeUgradi'}
      <Badge tone="warn">{m('soru.kart.kesintiye_ugradi')}</Badge>
    {:else}
      <Badge tone="neutral">{m('soru.kart.iptal')}</Badge>
    {/if}
  {/snippet}

  <div class="govde">
    <p class="soru-metni">{oge.soru.baslik}</p>

    <div class="secenekler" role="group" aria-label={oge.soru.baslik}>
      {#each oge.soru.secenekler as secenek (secenek.kimlik)}
        <button
          type="button"
          class={['secenek', { secili: secilenSecenekKimligi === secenek.kimlik }]}
          disabled={!etkilesimli}
          onclick={() => secenekSec(secenek.kimlik)}
        >
          <span class="secenek-etiket">{secenek.etiket}</span>
          {#if secenek.aciklama}
            <span class="secenek-aciklama">{secenek.aciklama}</span>
          {/if}
        </button>
      {/each}
    </div>

    {#if etkilesimli && oge.soru.serbest_yanit_izni}
      <div class="serbest-satir">
        <Input
          bind:value={serbestMetin}
          placeholder={m('soru.serbest.yer_tutucu')}
          aria-label={m('soru.serbest.ac')}
          onkeydown={serbestTus}
          class="serbest-girdi"
        />
        <Button
          variant="primary"
          size="sm"
          disabled={!serbestGecerli}
          onclick={serbestGonder}
        >
          {m('soru.serbest.gonder')}
        </Button>
      </div>
    {/if}

    {#if oge.durum === 'Yanitlandi' && oge.sonuc?.durum === 'Yanitlandi'}
      <div class="sonuc-ozeti">
        {#if oge.sonuc.yanit.tur === 'Secenek'}
          {m('soru.yanit.secenek', { deger: secilenEtiket ?? oge.sonuc.yanit.secenek_kimligi })}
        {:else}
          {m('soru.yanit.serbest', { deger: oge.sonuc.yanit.metin })}
        {/if}
      </div>
    {:else if oge.durum === 'KullaniciMesajiylaIptal' && oge.sonuc?.durum === 'KullaniciMesajiylaIptal'}
      <div class="sonuc-ozeti">
        {m('soru.yanit.yeni_mesaj', { deger: oge.sonuc.yeni_mesaj })}
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
  .soru-metni {
    margin: 0;
    font: var(--fw-medium) var(--text-body-size) / 1.45 var(--font-sans);
    color: var(--text-title);
  }
  .secenekler {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .secenek {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    color: var(--text-body);
    font: var(--type-small);
    cursor: pointer;
    transition: var(--transition-control);
  }
  .secenek:hover:not(:disabled) {
    background: var(--surface-raised);
    border-color: var(--border-default);
  }
  .secenek.secili {
    border-color: var(--border-accent);
    background: var(--surface-selected);
  }
  .secenek:disabled {
    cursor: default;
    opacity: 0.8;
  }
  .secenek-etiket {
    font-weight: var(--fw-medium);
    color: var(--text-title);
  }
  .secenek-aciklama {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .serbest-satir {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .serbest-satir :global(.serbest-girdi) {
    flex: 1;
  }
  .sonuc-ozeti {
    font: var(--type-small);
    color: var(--text-secondary);
    padding-top: 4px;
    border-top: var(--border-width) solid var(--border-subtle);
  }
</style>
