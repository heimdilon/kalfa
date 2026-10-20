<script lang="ts">
  import { m } from '../dil/tr';
  import type { OturumPlani, PlanAdimDurumu } from '../ipc/tipler';
  import Badge from './Badge.svelte';
  import Card from './Card.svelte';
  import Icon from './Icon.svelte';
  import type { IkonAdi } from './ikonlar';

  interface Props {
    plan: OturumPlani;
    class?: string;
  }

  let { plan, class: sinif }: Props = $props();

  const DURUM_IKONU: Record<PlanAdimDurumu, IkonAdi> = {
    Bekliyor: 'square',
    Suruyor: 'loader-circle',
    Tamam: 'circle-check',
  };

  const DURUM_TONU: Record<PlanAdimDurumu, 'neutral' | 'accent' | 'ok'> = {
    Bekliyor: 'neutral',
    Suruyor: 'accent',
    Tamam: 'ok',
  };

  const tamamSayisi = $derived(plan.adimlar.filter((a) => a.durum === 'Tamam').length);
  const etkinVar = $derived(plan.adimlar.some((a) => a.durum === 'Suruyor'));
</script>

<Card
  tone={etkinVar ? 'accent' : 'default'}
  icon="list"
  title={m('plan.baslik')}
  subtitle={m('plan.ilerleme', { tamam: tamamSayisi, toplam: plan.adimlar.length })}
  class={sinif}
>
  <ol class="adimlar">
    {#each plan.adimlar as adim (adim.kimlik)}
      <li class={['adim', `durum-${adim.durum}`]}>
        <span class="ikon">
          <Icon name={DURUM_IKONU[adim.durum]} size={14} />
        </span>
        <span class="baslik">{adim.baslik}</span>
        <Badge tone={DURUM_TONU[adim.durum]}>{m(`plan.durum.${adim.durum}`)}</Badge>
      </li>
    {/each}
  </ol>
</Card>

<style>
  .adimlar {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .adim {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font: var(--type-small);
    min-width: 0;
  }
  .adim.durum-Suruyor {
    border: var(--border-width) solid var(--border-accent);
    background: var(--surface-selected);
  }
  .ikon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    color: var(--text-muted);
  }
  .durum-Suruyor .ikon {
    color: var(--text-accent);
  }
  .durum-Tamam .ikon {
    color: var(--status-ok);
  }
  .baslik {
    flex: 1;
    min-width: 0;
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .durum-Tamam .baslik {
    color: var(--text-secondary);
  }
</style>
