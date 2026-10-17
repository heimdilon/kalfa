<!--
  MCP sunucusu ekleme/düzenleme formu (1.1a tasarımı §9). Yol arayüzden
  gitmez: komut ya çıplak program adıdır ya da "Gözat" yerel seçiciyi açar
  ve arka uç bir kimlik döner. Yeni sunucu yalıtımlıdır. Tam erişimde ağ
  alanı gizlenir, "sınırsız" görünür ve kalıcı uyarı çıkar. Gizli değer
  yalnız parola alanında yazılır ve arka uca gider; kayıtlı gizli değer geri
  gelmez. Kaydı Rust yerel pencerede tam komut satırıyla sorar.
-->
<script lang="ts">
  import { untrack } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import { gorunurMetin } from '../bilesenler/gorunmez';
  import GorunurMetin from '../bilesenler/GorunurMetin.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import Input from '../bilesenler/Input.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import { MCP_ROZETI, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type {
    McpKipi,
    McpOrtamFormGirdisi,
    McpSunucuAyari,
    McpSunucuFormu,
    OturumId,
  } from '../ipc/tipler';

  let {
    baslangic,
    oturum,
    onkapat,
    onkaydedildi,
  }: {
    baslangic: McpSunucuAyari | null;
    oturum: OturumId | null;
    onkapat: () => void;
    onkaydedildi: (ad: string) => void;
  } = $props();

  interface OrtamSatiri {
    anahtar: number;
    ad: string;
    deger: string;
    gizli: boolean;
    /** Kayıtlı gizli değerin kimliği; değer arayüze hiç gelmez. */
    korunan: string | null;
  }
  interface AgSatiri {
    anahtar: number;
    host: string;
    port: string;
    ozel: boolean;
  }

  const kimlik = $props.id();
  let sayac = 0;
  const ilk = untrack(() => baslangic);
  const baslik = ilk
    ? gorunurMetin(m('mcp.form.baslik_duzenle', { ad: ilk.ad }))
    : m('mcp.form.baslik_yeni');
  const ilkKomut = ilk?.komut;

  let ad = $state(ilk?.ad ?? '');
  let komutTuru = $state<'program_adi' | 'secili_yurutulebilir'>(ilkKomut?.tur ?? 'program_adi');
  let program = $state(ilkKomut?.tur === 'program_adi' ? ilkKomut.ad : '');
  let secili = $state<{ kimlik: string; ad: string } | null>(
    ilkKomut?.tur === 'secili_yurutulebilir'
      ? { kimlik: ilkKomut.kimlik, ad: m('mcp.form.secili_kayitli') }
      : null,
  );
  let argumanlar = $state((ilkKomut?.argumanlar ?? []).join('\n'));
  let ortam = $state<OrtamSatiri[]>(
    (ilk?.ortam ?? []).map((o) => ({
      anahtar: ++sayac,
      ad: o.ad,
      deger: o.deger_acik ?? '',
      gizli: o.gizli,
      korunan: o.gizli && o.gizli_kimlik ? o.gizli_kimlik : null,
    })),
  );
  let kip = $state<McpKipi>(ilk?.kip ?? 'yalitimli');
  let ag = $state<AgSatiri[]>(
    (ilk?.ag_izinleri ?? []).map((k) => ({
      anahtar: ++sayac,
      host: k.host,
      port: String(k.port),
      ozel: k.ozel_ag ?? false,
    })),
  );
  let baslatmaZamani = $state(String(ilk?.baslatma_zamani_asimi_sn ?? 30));
  let cagriZamani = $state(String(ilk?.cagri_zamani_asimi_sn ?? 120));
  let hata = $state<string | null>(null);
  let kaydediliyor = $state(false);
  let seciliyor = $state(false);

  async function gozat(): Promise<void> {
    seciliyor = true;
    try {
      const sonuc = await ipc.mcpKomutSec();
      if (sonuc) secili = { kimlik: sonuc.kimlik, ad: sonuc.gorunen_ad };
    } catch (e) {
      hata = komutHatasi(e).mesaj;
    } finally {
      seciliyor = false;
    }
  }

  function tamSayi(metin: string): number | null {
    const sayi = Number(metin.trim());
    return Number.isInteger(sayi) && sayi > 0 ? sayi : null;
  }

  /** Formu arka ucun sözleşmesine çevirir; eksikte kullanıcıya hata metni döner. */
  function formUret(): McpSunucuFormu | string {
    if (!ad.trim()) return m('mcp.form.hata.ad');
    const args = argumanlar.split('\n').filter((s) => s.trim() !== '');
    let komut: McpSunucuFormu['komut'];
    if (komutTuru === 'program_adi') {
      if (!program.trim()) return m('mcp.form.hata.komut');
      komut = { tur: 'program_adi', ad: program.trim(), argumanlar: args };
    } else {
      if (!secili) return m('mcp.form.hata.komut');
      komut = { tur: 'secili_yurutulebilir', kimlik: secili.kimlik, argumanlar: args };
    }
    if (ortam.some((o) => !o.ad.trim())) return m('mcp.form.hata.ortam');
    const ortamGirdileri: McpOrtamFormGirdisi[] = ortam.map((o) => {
      if (o.korunan) return { tur: 'korunan_gizli', ad: o.ad.trim(), kimlik: o.korunan };
      return o.gizli
        ? { tur: 'yeni_gizli', ad: o.ad.trim(), deger: o.deger }
        : { tur: 'duz', ad: o.ad.trim(), deger: o.deger };
    });
    const agIzinleri = [];
    if (kip === 'yalitimli') {
      for (const a of ag) {
        const port = tamSayi(a.port);
        if (!a.host.trim() || port === null || port > 65535) return m('mcp.form.hata.ag');
        agIzinleri.push({ host: a.host.trim(), port, ozel_ag: a.ozel });
      }
    }
    const baslatma = tamSayi(baslatmaZamani);
    const cagri = tamSayi(cagriZamani);
    if (baslatma === null || cagri === null) return m('mcp.form.hata.zaman');
    return {
      ad: ad.trim(),
      etkin: true,
      komut,
      ortam: ortamGirdileri,
      kip,
      ag_izinleri: agIzinleri,
      baslatma_zamani_asimi_sn: baslatma,
      cagri_zamani_asimi_sn: cagri,
    };
  }

  async function kaydet(e: SubmitEvent): Promise<void> {
    e.preventDefault();
    const form = formUret();
    if (typeof form === 'string') {
      hata = form;
      return;
    }
    hata = null;
    kaydediliyor = true;
    try {
      const taslak = await ipc.mcpTaslakOlustur(baslangic?.ad ?? null, form);
      const kaydedildi = await ipc.mcpTaslakKaydet(taslak.taslak_kimligi, taslak.kusak, oturum);
      if (kaydedildi) onkaydedildi(form.ad);
      else hata = m('mcp.reddedildi');
    } catch (e) {
      hata = komutHatasi(e).mesaj;
    } finally {
      kaydediliyor = false;
    }
  }
</script>

<Card icon="box" title={baslik}>
  <form class="form" onsubmit={kaydet} aria-label={baslik}>
    <label class="alan">
      <span class="etiket">{m('mcp.form.ad')}</span>
      <Input bind:value={ad} autocomplete="off" spellcheck={false} />
    </label>

    <fieldset class="alan">
      <legend class="etiket">{m('mcp.form.komut')}</legend>
      <div class="secenekler">
        <label class="secenek">
          <input type="radio" name="{kimlik}-komut" value="program_adi" bind:group={komutTuru} />
          {m('mcp.form.komut_program')}
        </label>
        <label class="secenek">
          <input
            type="radio"
            name="{kimlik}-komut"
            value="secili_yurutulebilir"
            bind:group={komutTuru}
          />
          {m('mcp.form.komut_secili')}
        </label>
      </div>
      {#if komutTuru === 'program_adi'}
        <Input
          bind:value={program}
          mono
          aria-label={m('mcp.form.komut_program')}
          autocomplete="off"
          spellcheck={false}
        />
        <p class="ipucu">{m('mcp.form.program_ipucu')}</p>
      {:else}
        <div class="satir">
          <Button size="sm" iconLeft="folder-open" loading={seciliyor} onclick={() => void gozat()}>
            {m('mcp.form.gozat')}
          </Button>
          <span class="secili">
            {#if secili}<GorunurMetin metin={secili.ad} />{:else}{m('mcp.form.secili_yok')}{/if}
          </span>
        </div>
      {/if}
    </fieldset>

    <div class="alan">
      <label class="etiket" for="{kimlik}-arg">{m('mcp.form.argumanlar')}</label>
      <Textarea
        id="{kimlik}-arg"
        bind:value={argumanlar}
        rows={3}
        spellcheck={false}
        aria-describedby="{kimlik}-arg-ipucu"
      />
      <span id="{kimlik}-arg-ipucu" class="ipucu">{m('mcp.form.argumanlar_ipucu')}</span>
    </div>

    <fieldset class="alan">
      <legend class="etiket">{m('mcp.form.ortam')}</legend>
      {#each ortam as o (o.anahtar)}
        <div class="satir">
          <Input
            bind:value={o.ad}
            mono
            aria-label={m('mcp.form.ortam_ad')}
            placeholder={m('mcp.form.ortam_ad')}
            autocomplete="off"
            spellcheck={false}
          />
          {#if o.korunan}
            <span class="korunan">{m('mcp.form.ortam_kayitli_gizli')}</span>
            <Button
              size="sm"
              variant="ghost"
              onclick={() => {
                o.korunan = null;
                o.deger = '';
              }}
            >
              {m('mcp.form.ortam_degistir')}
            </Button>
          {:else}
            <Input
              bind:value={o.deger}
              type={o.gizli ? 'password' : 'text'}
              mono
              aria-label={m('mcp.form.ortam_deger')}
              placeholder={m('mcp.form.ortam_deger')}
              autocomplete="off"
              spellcheck={false}
            />
            <Checkbox
              checked={o.gizli}
              label={m('mcp.form.ortam_gizli')}
              onchange={(v) => (o.gizli = v)}
            />
          {/if}
          <IconButton
            icon="x"
            size="sm"
            label={gorunurMetin(m('mcp.form.ortam_kaldir', { ad: o.ad || '…' }))}
            onclick={() => (ortam = ortam.filter((x) => x.anahtar !== o.anahtar))}
          />
        </div>
      {/each}
      <div>
        <Button
          size="sm"
          variant="ghost"
          iconLeft="plus"
          onclick={() =>
            ortam.push({ anahtar: ++sayac, ad: '', deger: '', gizli: false, korunan: null })}
        >
          {m('mcp.form.ortam_ekle')}
        </Button>
      </div>
    </fieldset>

    <fieldset class="alan">
      <legend class="etiket">{m('mcp.form.erisim')}</legend>
      <div class="secenekler">
        {#each ['yalitimli', 'tam_erisim'] as const as k (k)}
          <label class="secenek">
            <input type="radio" name="{kimlik}-kip" value={k} bind:group={kip} />
            {m(MCP_ROZETI[k])}
          </label>
        {/each}
      </div>
      {#if kip === 'tam_erisim'}
        <p class="uyari" role="note">{m('mcp.form.tam_uyari')}</p>
      {/if}
    </fieldset>

    <fieldset class="alan">
      <legend class="etiket">{m('mcp.form.ag')}</legend>
      {#if kip === 'tam_erisim'}
        <p class="sinirsiz">{m('mcp.form.ag_sinirsiz')}</p>
      {:else}
        {#each ag as a (a.anahtar)}
          <div class="satir">
            <Input
              bind:value={a.host}
              mono
              aria-label={m('mcp.form.ag_host')}
              placeholder={m('mcp.form.ag_host')}
              autocomplete="off"
              spellcheck={false}
            />
            <Input
              bind:value={a.port}
              mono
              inputmode="numeric"
              aria-label={m('mcp.form.ag_port')}
              placeholder={m('mcp.form.ag_port')}
              class="port"
            />
            <Checkbox
              checked={a.ozel}
              label={m('mcp.form.ag_ozel')}
              onchange={(v) => (a.ozel = v)}
            />
            <IconButton
              icon="x"
              size="sm"
              label={gorunurMetin(m('mcp.form.ag_kaldir', { host: a.host || '…' }))}
              onclick={() => (ag = ag.filter((x) => x.anahtar !== a.anahtar))}
            />
          </div>
        {:else}
          <p class="ipucu">{m('mcp.form.ag_ipucu')}</p>
        {/each}
        <div>
          <Button
            size="sm"
            variant="ghost"
            iconLeft="plus"
            onclick={() => ag.push({ anahtar: ++sayac, host: '', port: '', ozel: false })}
          >
            {m('mcp.form.ag_ekle')}
          </Button>
        </div>
      {/if}
    </fieldset>

    <div class="zamanlar">
      <label class="alan">
        <span class="etiket">{m('mcp.form.baslatma_zamani')}</span>
        <Input bind:value={baslatmaZamani} inputmode="numeric" />
      </label>
      <label class="alan">
        <span class="etiket">{m('mcp.form.cagri_zamani')}</span>
        <Input bind:value={cagriZamani} inputmode="numeric" />
      </label>
    </div>

    <p class="ipucu">{m('mcp.form.yerel_onay')}</p>
    {#if hata}
      <p class="hata" role="alert"><GorunurMetin metin={hata} /></p>
    {/if}
    <div class="dugmeler">
      <Button type="submit" size="sm" variant="primary" iconLeft="check" loading={kaydediliyor}>
        {m('mcp.form.kaydet')}
      </Button>
      <Button size="sm" disabled={kaydediliyor} onclick={onkapat}>{m('mcp.form.vazgec')}</Button>
    </div>
  </form>
</Card>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .alan {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }
  .etiket {
    padding: 0;
    font: var(--type-small);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .secenekler {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
  }
  .secenek {
    display: flex;
    align-items: center;
    gap: 6px;
    font: var(--type-small);
    color: var(--text-body);
  }
  .satir {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .satir > :global(.port) {
    max-width: 96px;
  }
  .secili,
  .korunan,
  .sinirsiz {
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .sinirsiz {
    margin: 0;
    color: var(--status-danger);
  }
  .ipucu {
    margin: 0;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .uyari {
    margin: 0;
    padding: 8px 10px;
    font: var(--type-small);
    color: var(--status-danger);
    background: color-mix(in srgb, var(--status-danger) 12%, transparent);
    border-radius: var(--radius-md);
  }
  .zamanlar {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
  }
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .dugmeler {
    display: flex;
    gap: 8px;
  }
</style>
