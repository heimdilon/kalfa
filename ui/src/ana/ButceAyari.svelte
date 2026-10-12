<!--
  Ayarlar > "Yeni oturumların bütçesi" (5b tasarımı §9): yeni oturumların istek
  ve token sınırları. Açık ve kayıttan açılan oturumları etkilemez; bunu metin
  de söyler. Sınırlar burada ve Rust'ta aynı kurallarla doğrulanır.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Input from '../bilesenler/Input.svelte';
  import { m } from '../dil/tr';
  import type { ButceSinirlari } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { AYAR_ALT_SINIRI, butceGirdisiDogrula } from './butce';

  let { durum }: { durum: AnaDurum } = $props();

  const kimlik = 'ayar-butce';
  const istekKimligi = `${kimlik}-istek`;
  const tokenKimligi = `${kimlik}-token`;
  const hataKimligi = `${kimlik}-hata`;

  let istekMetni = $state('');
  let tokenMetni = $state('');
  let kaydediliyor = $state(false);
  let kaydedildi = $state(false);
  // Hata yalnızca kayıt denendikten sonra gösterilir; düzeltilince kalkar.
  let denendi = $state(false);
  const hatali = $derived(
    denendi && !butceGirdisiDogrula(istekMetni, tokenMetni, AYAR_ALT_SINIRI).gecerli,
  );
  let kaynak: ButceSinirlari | null = null;

  // Okunan (ya da kaydedilen) varsayılan her değiştiğinde girdiler onunla dolar.
  $effect(() => {
    const sinir = durum.butceVarsayilani;
    if (sinir !== kaynak) {
      kaynak = sinir;
      if (sinir) {
        istekMetni = String(sinir.istek);
        tokenMetni = String(sinir.token);
      }
    }
  });

  async function kaydet(): Promise<void> {
    const girdi = butceGirdisiDogrula(istekMetni, tokenMetni, AYAR_ALT_SINIRI);
    kaydedildi = false;
    denendi = true;
    if (!girdi.gecerli || kaydediliyor) return;
    kaydediliyor = true;
    try {
      kaydedildi = await durum.butceVarsayilaniniAyarla(girdi.sinirlar.istek, girdi.sinirlar.token);
    } finally {
      kaydediliyor = false;
    }
  }
</script>

<Card title={m('ayarlar.butce')} icon="coins">
  <p class="aciklama">{m('ayarlar.butce.aciklama')}</p>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      void kaydet();
    }}
  >
    <div class="girdiler">
      <div class="girdi-alani">
        <label class="etiket" for={istekKimligi}>{m('ayarlar.butce.istek')}</label>
        <Input
          id={istekKimligi}
          mono
          inputmode="numeric"
          autocomplete="off"
          invalid={hatali}
          aria-describedby={hatali ? hataKimligi : undefined}
          bind:value={istekMetni}        />
      </div>
      <div class="girdi-alani">
        <label class="etiket" for={tokenKimligi}>{m('ayarlar.butce.token')}</label>
        <Input
          id={tokenKimligi}
          mono
          inputmode="numeric"
          autocomplete="off"
          invalid={hatali}
          aria-describedby={hatali ? hataKimligi : undefined}
          bind:value={tokenMetni}        />
      </div>
    </div>
    {#if hatali}
      <p id={hataKimligi} class="hata" role="alert">{m('ayarlar.butce.hata')}</p>
    {/if}
    <div class="eylem">
      <Button type="submit" variant="primary" size="sm" loading={kaydediliyor}>
        {m('ayarlar.butce.kaydet')}
      </Button>
      {#if kaydedildi}
        <span class="tamam" role="status">{m('ayarlar.butce.kaydedildi')}</span>
      {/if}
    </div>
  </form>
</Card>

<style>
  .aciklama {
    margin: 0 0 12px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
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
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .eylem {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .tamam {
    font: var(--type-small);
    color: var(--status-ok);
  }
</style>
