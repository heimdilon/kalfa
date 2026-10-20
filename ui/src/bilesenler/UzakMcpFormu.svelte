<script lang="ts">
  import { m } from '../dil/tr';
  import type {
    UzakMcpSunucuAyari,
    UzakMcpSunucuFormu,
    UzakMcpYetkiEtiketi,
    UzakMcpYetkiFormGirdisi,
  } from '../ipc/tipler';

  interface Props {
    mevcut?: UzakMcpSunucuAyari | null;
    onKaydet: (eskiAd: string | null, form: UzakMcpSunucuFormu) => Promise<void> | void;
    onVazgec: () => void;
  }

  let { mevcut = null, onKaydet, onVazgec }: Props = $props();

  // svelte-ignore state_referenced_locally
  let ad = $state(mevcut?.ad ?? '');
  // svelte-ignore state_referenced_locally
  let etkin = $state(mevcut?.etkin ?? true);
  // svelte-ignore state_referenced_locally
  let ucUrl = $state(mevcut?.uc_url ?? '');
  // svelte-ignore state_referenced_locally
  let yetkiSecimi = $state<UzakMcpYetkiEtiketi>(mevcut?.yetki_turu ?? 'yetkisiz');
  // svelte-ignore state_referenced_locally
  let baslikAdi = $state(mevcut?.sabit_baslik_adi ?? 'Authorization');
  let baslikDegeri = $state('');
  // svelte-ignore state_referenced_locally
  let korunanBaslikKimligi = $state<string | null>(
    mevcut?.yetki_turu === 'sabit_baslik' ? (mevcut.gizli_kimlik ?? null) : null,
  );
  // svelte-ignore state_referenced_locally
  let oauthIstemciKimligi = $state(mevcut?.oauth_istemci_kimligi ?? '');
  // svelte-ignore state_referenced_locally
  let oauthKapsamlarMetni = $state((mevcut?.oauth_kapsamlar ?? []).join(' '));
  // svelte-ignore state_referenced_locally
  let oauthKaynakUrl = $state(mevcut?.oauth_kaynak_url ?? '');
  // svelte-ignore state_referenced_locally
  let baslatmaZamani = $state(mevcut?.baslatma_zamani_asimi_sn ?? 15);
  // svelte-ignore state_referenced_locally
  let cagriZamani = $state(mevcut?.cagri_zamani_asimi_sn ?? 30);
  let hataMesaji = $state<string | null>(null);

  function adGecerliMi(deger: string): boolean {
    const temiz = deger.trim();
    return temiz.length > 0 && temiz.length <= 64 && /^[a-z0-9-]+$/.test(temiz);
  }

  function httpsUrlGecerliMi(deger: string): boolean {
    try {
      const u = new URL(deger.trim());
      return u.protocol === 'https:' && u.hostname.length > 0 && !u.username && !u.password && !u.hash;
    } catch {
      return false;
    }
  }

  async function gonder(ev: SubmitEvent) {
    ev.preventDefault();
    hataMesaji = null;

    const temizAd = ad.trim();
    if (!adGecerliMi(temizAd)) {
      hataMesaji = m('uzak_mcp.form.hata.ad');
      return;
    }

    const temizUrl = ucUrl.trim();
    if (!httpsUrlGecerliMi(temizUrl)) {
      hataMesaji = m('uzak_mcp.form.hata.url');
      return;
    }

    const baslatmaSn = Number(baslatmaZamani);
    const cagriSn = Number(cagriZamani);
    if (!Number.isInteger(baslatmaSn) || baslatmaSn <= 0 || !Number.isInteger(cagriSn) || cagriSn <= 0) {
      hataMesaji = m('uzak_mcp.form.hata.zaman');
      return;
    }

    let yetki: UzakMcpYetkiFormGirdisi;
    if (yetkiSecimi === 'yetkisiz') {
      yetki = { tur: 'yetkisiz' };
    } else if (yetkiSecimi === 'sabit_baslik') {
      const temizBaslikAdi = baslikAdi.trim();
      if (!temizBaslikAdi) {
        hataMesaji = m('uzak_mcp.form.hata.baslik');
        return;
      }
      if (korunanBaslikKimligi && baslikDegeri.trim() === '') {
        yetki = {
          tur: 'korunan_sabit_baslik',
          baslik_adi: temizBaslikAdi,
          kimlik: korunanBaslikKimligi,
        };
      } else {
        if (baslikDegeri.trim() === '') {
          hataMesaji = m('uzak_mcp.form.hata.baslik');
          return;
        }
        yetki = {
          tur: 'yeni_sabit_baslik',
          baslik_adi: temizBaslikAdi,
          deger: baslikDegeri,
        };
      }
    } else {
      const kapsamlar = oauthKapsamlarMetni
        .split(/[\s,]+/)
        .map((k) => k.trim())
        .filter((k) => k.length > 0);
      yetki = {
        tur: 'oauth',
        istemci_kimligi: oauthIstemciKimligi.trim() || null,
        kapsamlar,
        kaynak_url: oauthKaynakUrl.trim() || null,
        korunan_yetki_kimligi: null,
      };
    }

    await onKaydet(mevcut?.ad ?? null, {
      ad: temizAd,
      etkin,
      uc_url: temizUrl,
      yetki,
      baslatma_zamani_asimi_sn: baslatmaSn,
      cagri_zamani_asimi_sn: cagriSn,
    });
  }
</script>

<form class="uzak-mcp-formu" onsubmit={gonder} aria-label={mevcut ? m('uzak_mcp.form.baslik_duzenle', { ad: mevcut.ad }) : m('uzak_mcp.form.baslik_yeni')}>
  <h4 class="uzak-mcp-formu-baslik">
    {mevcut ? m('uzak_mcp.form.baslik_duzenle', { ad: mevcut.ad }) : m('uzak_mcp.form.baslik_yeni')}
  </h4>

  {#if hataMesaji}
    <p class="uzak-mcp-form-hata" role="alert">{hataMesaji}</p>
  {/if}

  <label class="uzak-mcp-alan">
    <span>{m('uzak_mcp.form.ad')}</span>
    <input type="text" bind:value={ad} name="ad" autocomplete="off" />
  </label>

  <div class="uzak-mcp-alan-grubu">
    <label class="uzak-mcp-alan">
      <span>{m('uzak_mcp.form.uc_url')}</span>
      <input type="url" bind:value={ucUrl} name="uc_url" placeholder="https://mcp.ornek.com/mcp" autocomplete="off" />
    </label>
    <small>{m('uzak_mcp.form.uc_url_ipucu')}</small>
  </div>

  <label class="uzak-mcp-alan">
    <span>{m('uzak_mcp.form.yetki')}</span>
    <select bind:value={yetkiSecimi} name="yetki_turu">
      <option value="yetkisiz">{m('uzak_mcp.yetki.yetkisiz')}</option>
      <option value="sabit_baslik">{m('uzak_mcp.yetki.sabit_baslik')}</option>
      <option value="oauth">{m('uzak_mcp.yetki.oauth')}</option>
    </select>
  </label>

  {#if yetkiSecimi === 'sabit_baslik'}
    <label class="uzak-mcp-alan">
      <span>{m('uzak_mcp.form.baslik_adi')}</span>
      <input type="text" bind:value={baslikAdi} name="baslik_adi" autocomplete="off" />
    </label>

    {#if korunanBaslikKimligi}
      <div class="uzak-mcp-korunan-gizli">
        <span>{m('uzak_mcp.form.korunan_baslik')}</span>
        <button
          type="button"
          onclick={() => {
            korunanBaslikKimligi = null;
          }}
        >
          {m('uzak_mcp.form.baslik_degistir')}
        </button>
      </div>
    {:else}
      <label class="uzak-mcp-alan">
        <span>{m('uzak_mcp.form.baslik_degeri')}</span>
        <input type="password" bind:value={baslikDegeri} name="baslik_degeri" autocomplete="off" />
      </label>
    {/if}
  {:else if yetkiSecimi === 'oauth'}
    <label class="uzak-mcp-alan">
      <span>{m('uzak_mcp.form.oauth_istemci')}</span>
      <input type="text" bind:value={oauthIstemciKimligi} name="oauth_istemci" autocomplete="off" />
    </label>

    <label class="uzak-mcp-alan">
      <span>{m('uzak_mcp.form.oauth_kapsamlar')}</span>
      <input type="text" bind:value={oauthKapsamlarMetni} name="oauth_kapsamlar" autocomplete="off" />
    </label>

    <label class="uzak-mcp-alan">
      <span>{m('uzak_mcp.form.oauth_kaynak')}</span>
      <input type="url" bind:value={oauthKaynakUrl} name="oauth_kaynak" autocomplete="off" />
    </label>
  {/if}

  <div class="uzak-mcp-zamanlar">
    <label class="uzak-mcp-alan">
      <span>{m('mcp.form.baslatma_zamani')}</span>
      <input type="number" min="1" bind:value={baslatmaZamani} name="baslatma_zamani" />
    </label>

    <label class="uzak-mcp-alan">
      <span>{m('mcp.form.cagri_zamani')}</span>
      <input type="number" min="1" bind:value={cagriZamani} name="cagri_zamani" />
    </label>
  </div>

  <p class="uzak-mcp-yerel-onay">{m('uzak_mcp.form.yerel_onay')}</p>

  <div class="uzak-mcp-dugmeler">
    <button type="submit">{m('mcp.form.kaydet')}</button>
    <button type="button" onclick={onVazgec}>{m('mcp.form.vazgec')}</button>
  </div>
</form>
