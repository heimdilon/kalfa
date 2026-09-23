<!--
  Anahtar formu (1c tasarımı §7.2). Anahtar gönderilince alan boşaltılır;
  anahtar arayüzde tutulmaz ve hiçbir komut onu geri döndürmez.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Input from '../bilesenler/Input.svelte';
  import { m, type MetinAnahtari } from '../dil/tr';
  import type { AnahtarNedeni } from '../ipc/tipler';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum, neden }: { durum: SartDurum; neden: AnahtarNedeni } = $props();

  const kimlik = $props.id();
  let anahtar = $state('');

  const ACIKLAMA: Record<AnahtarNedeni, MetinAnahtari> = {
    Yok: 'anahtar.aciklama.Yok',
    KayitliGecersiz: 'anahtar.aciklama.KayitliGecersiz',
    Degistir: 'anahtar.aciklama.Degistir',
  };

  async function gonder(e: SubmitEvent) {
    e.preventDefault();
    const girilen = anahtar.trim();
    if (!girilen || durum.mesgul) return;
    anahtar = '';
    await durum.anahtarKaydet(girilen);
  }
</script>

<form onsubmit={gonder} novalidate>
  <h1>{m('anahtar.baslik')}</h1>
  <p class="aciklama">{m(ACIKLAMA[neden])}</p>

  <label for="{kimlik}-anahtar">{m('anahtar.etiket')}</label>
  <Input
    id="{kimlik}-anahtar"
    type="password"
    mono
    size="lg"
    iconLeft="key-round"
    placeholder="evren_llm_…"
    autocomplete="off"
    spellcheck={false}
    invalid={durum.hata !== null}
    aria-describedby="{kimlik}-not"
    bind:value={anahtar}
  />
  {#if durum.hata}
    <p class="hata" role="alert">{durum.hata}</p>
  {/if}
  <p class="not" id="{kimlik}-not">{m('anahtar.not')}</p>

  <div class="dugmeler">
    <Button
      type="submit"
      variant="primary"
      size="lg"
      loading={durum.mesgul}
      disabled={!anahtar.trim() || durum.mesgul}
    >
      {m('anahtar.kaydet')}
    </Button>
  </div>
</form>

<style>
  h1 {
    margin: 0 0 8px;
    font: var(--type-display);
    color: var(--text-title);
  }
  .aciklama {
    margin: 0 0 24px;
    font: var(--type-subtitle);
    color: var(--text-secondary);
  }
  label {
    display: block;
    margin-bottom: 8px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .hata {
    margin: 10px 0 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .not {
    margin: 10px 0 0;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .dugmeler {
    display: flex;
    justify-content: flex-end;
    margin-top: 24px;
  }
</style>
