<!--
  Ayarlar'ın aşama 3 bölümleri (3e Görev 5): gizli dosyalar (ek kalıplar ve
  gizli sayılmayanlar, aşama 3 tasarımı §9.2), ağ izin listesi (§8.3) ve
  güvenlik denetçisinin kipi (§7.6). Gizliliği azaltan her değişikliği Rust
  yerel pencerede sorar; buradaki düğme yalnızca isteği gönderir.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import Input from '../bilesenler/Input.svelte';
  import Select from '../bilesenler/Select.svelte';
  import { DENETCI_KIPI_ADI, m } from '../dil/tr';
  import { komutHatasi } from '../ipc/hata';
  import * as ipc from '../ipc/ana';
  import type { AgHostu, DenetimKipi, GizliAyarGorunumu } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  let gizli = $state<GizliAyarGorunumu | null>(null);
  let ag = $state<AgHostu[]>([]);
  let ekKalip = $state('');
  let istisna = $state('');
  let host = $state('');
  let ozelAg = $state(false);
  let mesgul = $state(false);

  const kipler: DenetimKipi[] = ['Kapali', 'Uyar', 'Kati'];
  const kipSecenekleri = kipler.map((k) => ({ value: k, label: m(DENETCI_KIPI_ADI[k]) }));

  async function calistir<T>(is: () => Promise<T>): Promise<T | undefined> {
    if (mesgul) return undefined;
    mesgul = true;
    try {
      return await is();
    } catch (hata) {
      durum.hata = komutHatasi(hata).mesaj;
      return undefined;
    } finally {
      mesgul = false;
    }
  }

  async function gizliDegistir(is: () => Promise<GizliAyarGorunumu>, temizle?: () => void) {
    const g = await calistir(is);
    if (g) {
      gizli = g;
      temizle?.();
    }
  }

  async function agDegistir(is: () => Promise<AgHostu[]>, temizle?: () => void) {
    const l = await calistir(is);
    if (l) {
      ag = l;
      temizle?.();
    }
  }

  async function kipAyarla(kip: string) {
    const d = await calistir(() => ipc.denetciKipiniAyarla(kip as DenetimKipi));
    if (d) durum.denetci = d;
  }

  // Bir kez, açılışta. `$effect` `mesgul`'u okuyup yazdığı için kendini
  // yeniden tetiklerdi.
  onMount(() => {
    void calistir(async () => {
      gizli = await ipc.gizliAyarlari();
      ag = await ipc.agIzinListesi();
    });
  });
</script>

<Card title={m('ayarlar.gizli')} icon="lock">
  <p class="aciklama">{m('ayarlar.gizli.aciklama')}</p>
  {#if gizli?.ayar_hatasi}
    <p class="aciklama alt uyari-metni" role="status">{gizli.ayar_hatasi}</p>
  {/if}
  {#if gizli}
    <details class="varsayilan">
      <summary>{m('ayarlar.gizli.varsayilanlar', { sayi: gizli.varsayilanlar.length })}</summary>
      <ul class="kaliplar salt">
        {#each gizli.varsayilanlar as k (k)}<li><span class="kalip">{k}</span></li>{/each}
      </ul>
    </details>

    <div class="alt-bolum">
      <h3>{m('ayarlar.gizli.ek')}</h3>
      {#if gizli.ek_kaliplar.length > 0}
        <ul class="kaliplar" aria-label={m('ayarlar.gizli.ek')}>
          {#each gizli.ek_kaliplar as k (k)}
            <li>
              <span class="kalip">{k}</span>
              <IconButton
                icon="x"
                size="sm"
                disabled={mesgul}
                label={m('ayarlar.kaldir', { ad: k })}
                onclick={() => gizliDegistir(() => ipc.ekGizliKalipKaldir(k))}
              />
            </li>
          {/each}
        </ul>
      {:else}
        <p class="bos">{m('ayarlar.gizli.ek_bos')}</p>
      {/if}
      <form
        class="ekle"
        onsubmit={(e) => {
          e.preventDefault();
          void gizliDegistir(
            () => ipc.ekGizliKalipEkle(ekKalip),
            () => (ekKalip = ''),
          );
        }}
      >
        <Input
          bind:value={ekKalip}
          mono
          placeholder={m('ayarlar.gizli.kalip_yer')}
          aria-label={m('ayarlar.gizli.ek')}
        />
        <Button size="sm" type="submit" disabled={mesgul || !ekKalip.trim()}>
          {m('ayarlar.ekle')}
        </Button>
      </form>
    </div>

    <div class="alt-bolum">
      <h3>{m('ayarlar.gizli.istisna')}</h3>
      <p class="aciklama">{m('ayarlar.gizli.istisna_aciklama')}</p>
      {#if gizli.gizli_sayilmayanlar.length > 0}
        <ul class="kaliplar" aria-label={m('ayarlar.gizli.istisna')}>
          {#each gizli.gizli_sayilmayanlar as k (k)}
            <li>
              <span class="kalip">{k}</span>
              <IconButton
                icon="x"
                size="sm"
                disabled={mesgul}
                label={m('ayarlar.kaldir', { ad: k })}
                onclick={() => gizliDegistir(() => ipc.gizliSayilmayanKaldir(k))}
              />
            </li>
          {/each}
        </ul>
      {:else}
        <p class="bos">{m('ayarlar.gizli.istisna_bos')}</p>
      {/if}
      <form
        class="ekle"
        onsubmit={(e) => {
          e.preventDefault();
          void gizliDegistir(
            () => ipc.gizliSayilmayanEkle(istisna),
            () => (istisna = ''),
          );
        }}
      >
        <Input
          bind:value={istisna}
          mono
          placeholder={m('ayarlar.gizli.kalip_yer')}
          aria-label={m('ayarlar.gizli.istisna')}
        />
        <Button size="sm" type="submit" disabled={mesgul || !istisna.trim()}>
          {m('ayarlar.ekle')}
        </Button>
      </form>
    </div>
  {/if}
</Card>

<Card title={m('ayarlar.ag')} icon="list">
  <p class="aciklama">{m('ayarlar.ag.aciklama')}</p>
  {#if ag.length > 0}
    <ul class="kaliplar" aria-label={m('ayarlar.ag')}>
      {#each ag as h (`${h.host}:${h.port}`)}
        {@const ad = h.port === 443 ? h.host : `${h.host}:${h.port}`}
        <li>
          <span class="kalip">{ad}</span>
          {#if h.ozel_ag}<span class="isaret">{m('ayarlar.ag.ozel_ag_etiket')}</span>{/if}
          <IconButton
            icon="x"
            size="sm"
            disabled={mesgul}
            label={m('ayarlar.kaldir', { ad })}
            onclick={() => agDegistir(() => ipc.agHostuKaldir(ad))}
          />
        </li>
      {/each}
    </ul>
  {:else}
    <p class="bos">{m('ayarlar.ag.bos')}</p>
  {/if}
  <form
    class="ekle"
    onsubmit={(e) => {
      e.preventDefault();
      void agDegistir(
        () => ipc.agHostuEkle(host, ozelAg),
        () => {
          host = '';
          ozelAg = false;
        },
      );
    }}
  >
    <Input
      bind:value={host}
      mono
      placeholder={m('ayarlar.ag.host_yer')}
      aria-label={m('ayarlar.ag')}
    />
    <Button size="sm" type="submit" disabled={mesgul || !host.trim()}>
      {m('ayarlar.ekle')}
    </Button>
  </form>
  <Checkbox checked={ozelAg} label={m('ayarlar.ag.ozel_ag')} onchange={(v) => (ozelAg = v)} />
</Card>

<Card title={m('ayarlar.denetci')} icon="shield-check">
  <p class="aciklama">{m('ayarlar.denetci.aciklama')}</p>
  {#if durum.denetci?.ayar_hatasi}
    <p class="aciklama alt uyari-metni" role="status">{durum.denetci.ayar_hatasi}</p>
  {/if}
  <div class="alt">
    <Select
      value={durum.denetci?.kip ?? 'Kapali'}
      options={kipSecenekleri}
      label={m('ayarlar.denetci.kip')}
      size="sm"
      disabled={mesgul}
      onchange={(k) => void kipAyarla(k)}
    />
  </div>
</Card>

<style>
  .aciklama,
  .bos {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .bos {
    margin-top: 10px;
    color: var(--text-muted);
  }
  .alt {
    margin-top: 8px;
  }
  .uyari-metni {
    color: var(--status-warn);
  }
  .alt-bolum {
    margin-top: 16px;
    padding-top: 14px;
    border-top: var(--border-width) solid var(--border-subtle);
  }
  h3 {
    margin: 0 0 6px;
    font: var(--type-body);
    font-weight: var(--fw-semibold);
    color: var(--text-title);
  }
  .varsayilan {
    margin-top: 12px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .varsayilan summary {
    cursor: pointer;
  }
  .kaliplar {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .kaliplar li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 4px 4px 10px;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
  }
  .kaliplar.salt li {
    padding: 4px 10px;
  }
  .kalip {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--type-meta);
    color: var(--text-body);
  }
  .isaret {
    font: var(--type-small);
    color: var(--status-warn);
  }
  .ekle {
    display: flex;
    gap: 8px;
    margin: 10px 0 8px;
  }
  .ekle :global(.girdi) {
    flex: 1;
  }
</style>
