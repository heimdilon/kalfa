<!--
  Sağ panelin Ortam bölümü (masaüstü arayüzü tasarımı §6.2): çalışma alanı
  ve git dalı, izin kipi, yalıtım, model ve efor; son isteğin bağlam
  kullanımı; bu oturumda değişen dosyalar. Değişen dosyalar ve özetleme
  sayısı bütün kayıttan Rust'ta toplanır (yüklenen geçmiş sayfasından
  bağımsız); yeni tur özeti, geri alma ya da bağlam özeti gelince yenilenir.
  Dosyaya tıklamak Değişiklikler bölümünde açar.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import IsKarti, {
    type IsKartKipi,
    type ModelGirdiDurumu,
  } from '../bilesenler/IsKarti.svelte';
  import PlanListesi from '../bilesenler/PlanListesi.svelte';
  import { EFOR_ADI, KIP_ADI, m } from '../dil/tr';
  import * as ipc from '../ipc/ana';
  import { komutHatasi } from '../ipc/hata';
  import type { CalismaAlaniOzeti, IsKipi, IsOzeti, ProjeTalimatUyarisi } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { projeAdi, sayi } from './bicim';

  let { durum, ondosya }: { durum: AnaDurum; ondosya: (yol: string) => void } = $props();

  interface IsGorunumu {
    ozet: IsOzeti;
    cikti: string;
    imlec: number;
  }

  const ekran = $derived(durum.aktif);
  let ozet = $state<CalismaAlaniOzeti | null>(null);
  let hata = $state<string | null>(null);
  let isler = $state<IsGorunumu[]>([]);

  /** Toplamları değiştiren canlı öğelerin sayısı; değişince özet yeniden okunur. */
  const degisimler = $derived(
    ekran?.ogeler.filter(
      (o) => o.tip === 'TurOzeti' || o.tip === 'GeriAlma' || o.tip === 'OturumOzeti',
    ).length ?? 0,
  );

  /** Araç çağrısı veya bekleyen onay değiştiğinde arka plan işlerini yeniler. */
  const isTetigi = $derived(
    ekran?.ogeler.filter((o) => o.tip === 'AracCagrisi' || o.tip === 'OnayIstegi').length ?? 0,
  );

  const oturum = $derived(ekran?.oturum);

  // Yalnız oturum ya da toplamı değiştiren öğe sayısı değişince: her akış
  // parçasında bütün kaydı yeniden okutmaz.
  $effect(() => {
    void degisimler;
    if (!oturum) return;
    ipc.calismaAlaniOzeti(oturum).then(
      (o) => {
        ozet = o;
        hata = null;
      },
      (e) => (hata = komutHatasi(e).mesaj),
    );
  });

  async function isleriYenile(hedefOturum: string) {
    let liste: IsOzeti[];
    try {
      liste = (await ipc.oturumIsleri(hedefOturum)) ?? [];
    } catch {
      return;
    }
    if (oturum !== hedefOturum) return;
    const eskiTablo = new Map(
      isler.filter((g) => g.ozet.oturum === hedefOturum).map((g) => [g.ozet.is_id, g]),
    );
    const yeniListe: IsGorunumu[] = [];
    for (const isOzeti of liste) {
      const mevcut = eskiTablo.get(isOzeti.is_id);
      let cikti = mevcut?.cikti ?? '';
      let imlec = mevcut?.imlec ?? 0;
      const okunmali =
        !mevcut ||
        isOzeti.toplam_bayt !== mevcut.ozet.toplam_bayt ||
        isOzeti.durum.durum !== mevcut.ozet.durum.durum;
      if (okunmali && isOzeti.durum.durum !== 'sonucu_bilinmiyor') {
        try {
          const okuma = await ipc.isOku(hedefOturum, isOzeti.is_id, imlec);
          if (okuma) {
            cikti += okuma.cikti;
            imlec = okuma.yeni_imlec;
          }
        } catch {
          // Kurtarılmış veya çıktısı bellekte olmayan işlerde özet korunur.
        }
      }
      yeniListe.push({ ozet: isOzeti, cikti, imlec });
    }
    if (oturum === hedefOturum) {
      isler = yeniListe;
    }
  }

  $effect(() => {
    void isTetigi;
    if (!oturum) {
      isler = [];
      return;
    }
    void isleriYenile(oturum);
  });

  const etkinIsVar = $derived(
    isler.some(
      (g) => g.ozet.durum.durum === 'calisiyor' || g.ozet.durum.durum === 'durduruluyor',
    ),
  );

  $effect(() => {
    if (!oturum || !etkinIsVar) return;
    const hedef = oturum;
    const zamanlayici = setInterval(() => {
      void isleriYenile(hedef);
    }, 400);
    return () => clearInterval(zamanlayici);
  });

  function kipCevir(kip: IsKipi): IsKartKipi {
    if (kip.tur === 'terminal') {
      return { kip: 'terminal', boyut: kip.boyut };
    }
    return { kip: 'borulu' };
  }

  function modelGirdiBilgisi(isId: string): {
    durum: ModelGirdiDurumu;
    aciklama?: string;
  } {
    if (ekran) {
      for (let i = ekran.ogeler.length - 1; i >= 0; i--) {
        const oge = ekran.ogeler[i];
        if (oge?.tip === 'OnayIstegi' && oge.durum === 'Bekliyor') {
          const bekleyenOge = oge.ogeler.find(
            (o) => o.arac === 'komut_girdi' && o.aciklama.includes(isId),
          );
          if (bekleyenOge) {
            return { durum: 'onay_bekliyor', aciklama: bekleyenOge.aciklama };
          }
        }
        if (oge?.tip !== 'AracCagrisi' || oge.arac !== 'komut_girdi') continue;
        if (!oge.argumanlar) continue;
        try {
          const arg = JSON.parse(oge.argumanlar) as Record<string, unknown>;
          if (arg.is_id !== isId) continue;
        } catch {
          continue;
        }
        if (oge.durum === 'Reddedildi') {
          return { durum: 'reddedildi', aciklama: oge.aciklama ?? undefined };
        }
        if (oge.durum === 'Basarili') {
          return { durum: 'gonderildi', aciklama: oge.aciklama ?? undefined };
        }
      }
    }
    return { durum: 'yok' };
  }

  async function isiDurdur(isId: string) {
    if (!oturum) return;
    try {
      await ipc.isDurdur(oturum, isId);
    } finally {
      await isleriYenile(oturum);
    }
  }

  async function kullaniciGirdiGonder(isId: string, veri: string) {
    if (!oturum) return;
    try {
      await ipc.isGirdi(oturum, isId, veri, false);
    } finally {
      await isleriYenile(oturum);
    }
  }

  async function kullaniciEofGonder(isId: string) {
    if (!oturum) return;
    try {
      await ipc.isGirdi(oturum, isId, '', true);
    } finally {
      await isleriYenile(oturum);
    }
  }

  function talimatUyariMetni(u: ProjeTalimatUyarisi): string {
    switch (u.uyari.tur) {
      case 'dosya_boyutu_asildi':
        return m('proje_talimatlari.uyari.dosya_boyutu_asildi', {
          gercek: u.uyari.gercek,
          sinir: u.uyari.sinir,
        });
      case 'toplam_boyut_asildi':
        return m('proje_talimatlari.uyari.toplam_boyut_asildi', {
          gercek: u.uyari.gercek,
          sinir: u.uyari.sinir,
        });
      case 'kaynak_sayisi_asildi':
        return m('proje_talimatlari.uyari.kaynak_sayisi_asildi', {
          sinir: u.uyari.sinir,
        });
      case 'gecersiz_utf8':
        return m('proje_talimatlari.uyari.gecersiz_utf8');
      case 'sembolik_baglanti_reddedildi':
        return m('proje_talimatlari.uyari.sembolik_baglanti_reddedildi');
      case 'gizli_veya_korumali_reddedildi':
        return m('proje_talimatlari.uyari.gizli_veya_korumali_reddedildi');
      case 'okunamadi':
        return m('proje_talimatlari.uyari.okunamadi', { neden: u.uyari.neden });
    }
  }
</script>

{#if ekran}
  {@const yalitim = durum.uygulama?.yalitim}
  <div class="ortam">
    <section>
      <dl>
        <dt>{m('sag.calisma_alani')}</dt>
        <dd>
          <span class="ad">{projeAdi(ekran.calisma_alani)}</span>
          {#if ozet}<span class="yol" title={ekran.calisma_alani}>{ozet.kisa_yol}</span>{/if}
        </dd>
        <dt>{m('sag.git_dali')}</dt>
        <dd>
          {#if hata}
            <span class="soluk">{m('sag.okunamadi', { neden: hata })}</span>
          {:else if ozet?.git.tip === 'Dal'}
            <code>{ozet.git.ad}</code>
          {:else if ozet?.git.tip === 'AyrikHead'}
            {m('sag.git_ayrik')}
          {:else if ozet}
            <span class="soluk">{m('sag.git_yok')}</span>
          {/if}
        </dd>
        {#if ekran.ozet.kip}
          <dt>{m('sag.izin_kipi')}</dt>
          <dd>{m(KIP_ADI[ekran.ozet.kip])}</dd>
        {/if}
        {#if yalitim}
          <dt>{m('sag.yalitim')}</dt>
          <dd>
            {yalitim.tip === 'Hazir'
              ? m('durum.yalitimli_ipucu', { arka_uc: yalitim.arka_uc })
              : durum.uygulama?.yalitimsiz_izinli
                ? m('durum.yalitimsiz')
                : m('durum.komutlar_kapali')}
          </dd>
        {/if}
        {#if ekran.ozet.model}
          <dt>{m('sag.model')}</dt>
          <dd><code>{ekran.ozet.model}</code></dd>
        {/if}
        {#if ekran.ozet.efor}
          <dt>{m('sag.efor')}</dt>
          <dd>{m(EFOR_ADI[ekran.ozet.efor])}</dd>
        {/if}
      </dl>
    </section>

    <section>
      <h3>{m('sag.baglam')}</h3>
      {#if ekran.baglam === null || ekran.baglam.esik <= 0}
        <p class="soluk">{m('sag.baglam_bilinmiyor')}</p>
      {:else}
        <!-- Oturumun yaydığı tahmin ve compact eşiği (etkin pencerenin %90'ı). -->
        {@const { tahmin, esik } = ekran.baglam}
        <progress max={esik} value={Math.min(tahmin, esik)}></progress>
        <p>
          {m('sag.baglam_kullanim', {
            token: sayi(tahmin),
            esik: sayi(esik),
            yuzde: Math.min(100, Math.round((tahmin / esik) * 100)),
          })}
        </p>
      {/if}
      {#if ozet && ozet.ozetlenen > 0}
        <p class="soluk">{m('sag.baglam_ozetleme', { sayi: ozet.ozetlenen })}</p>
      {/if}
    </section>

    {#if ekran.plan && ekran.plan.adimlar.length > 0}
      <section>
        <PlanListesi plan={ekran.plan} />
      </section>
    {/if}

    {#if ekran.proje_talimatlari}
      {@const talimatlar = ekran.proje_talimatlari}
      <section>
        <div class="bolum-basligi">
          <h3>{m('proje_talimatlari.baslik')}</h3>
          {#if talimatlar.eksik_baglam}
            <Badge tone="warn">{m('proje_talimatlari.eksik_baglam')}</Badge>
          {/if}
        </div>
        {#if talimatlar.kaynaklar.length === 0}
          <p class="soluk">{m('proje_talimatlari.yok')}</p>
        {:else}
          <ul class="talimat-kaynaklari">
            {#each talimatlar.kaynaklar as kaynak (kaynak.goreli_yol)}
              {@const degisti = talimatlar.degisen_kaynaklar?.some(
                (d) => d.goreli_yol === kaynak.goreli_yol,
              )}
              <li class="talimat-satiri">
                <code class="yol-ad">{kaynak.goreli_yol}</code>
                <Badge tone="neutral">
                  {kaynak.kapsam.tur === 'kok'
                    ? m('proje_talimatlari.kapsam.kok')
                    : m('proje_talimatlari.kapsam.alt_dizin', {
                        dizin: kaynak.kapsam.deger.dizin,
                      })}
                </Badge>
                {#if degisti}
                  <Badge tone="info">{m('proje_talimatlari.degisti')}</Badge>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
        {#if talimatlar.uyarilar && talimatlar.uyarilar.length > 0}
          <ul class="talimat-uyarilari">
            {#each talimatlar.uyarilar as u (`${u.goreli_yol}:${u.uyari.tur}`)}
              <li class="talimat-uyari">
                <code>{u.goreli_yol}</code>: {talimatUyariMetni(u)}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}

    {#if isler.length > 0}
      <section>
        <h3>{m('sag.isler')}</h3>
        <div class="is-listesi">
          {#each isler as is (is.ozet.is_id)}
            {@const modelGirdi = modelGirdiBilgisi(is.ozet.is_id)}
            <IsKarti
              isId={is.ozet.is_id}
              gosterim={is.ozet.gosterim}
              kip={kipCevir(is.ozet.kip)}
              durum={is.ozet.durum}
              cikti={is.cikti}
              toplamBayt={is.ozet.toplam_bayt}
              tasti={is.ozet.tasti}
              uyarilar={is.ozet.uyarilar}
              modelGirdiDurumu={modelGirdi.durum}
              modelGirdiAciklama={modelGirdi.aciklama}
              onDurdur={isiDurdur}
              onKullaniciGirdi={kullaniciGirdiGonder}
              onKullaniciEof={kullaniciEofGonder}
            />
          {/each}
        </div>
      </section>
    {/if}

    <section>
      <h3>{m('sag.ciktilar')}</h3>
      {#if !ozet || ozet.ciktilar.length === 0}
        <p class="soluk">{m('sag.ciktilar_bos')}</p>
      {:else}
        <ul>
          {#each ozet.ciktilar as { yol, eklenen, silinen } (yol)}
            <li>
              <button type="button" onclick={() => ondosya(yol)}>
                <span class="yol-ad">{yol}</span>
                <span class="arti">+{eklenen}</span>
                <span class="eksi">−{silinen}</span>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}

<style>
  .ortam {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  section {
    padding: 12px 16px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  h3 {
    margin: 0 0 8px;
    font: var(--type-meta);
    font-weight: var(--fw-semibold);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 12px;
    margin: 0;
    font: var(--type-small);
  }
  dt {
    color: var(--text-muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    color: var(--text-body);
    overflow-wrap: anywhere;
  }
  .ad {
    font-weight: var(--fw-semibold);
  }
  .yol {
    display: block;
    font: var(--type-meta);
    font-family: var(--font-mono);
    color: var(--text-muted);
  }
  code {
    font: var(--type-code);
  }
  p {
    margin: 4px 0 0;
    font: var(--type-small);
    color: var(--text-body);
  }
  .soluk {
    color: var(--text-muted);
  }
  progress {
    width: 100%;
    height: 6px;
    accent-color: var(--copper-300);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 5px 6px;
    background: none;
    border: none;
    border-radius: var(--radius-sm);
    font: var(--type-small);
    text-align: left;
    color: var(--text-secondary);
    cursor: pointer;
  }
  li button:hover {
    background: var(--surface-raised);
    color: var(--text-body);
  }
  .yol-ad {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--text-micro);
  }
  .arti {
    color: var(--status-ok);
  }
  .eksi {
    color: var(--status-danger);
  }
  .bolum-basligi {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }
  .bolum-basligi h3 {
    margin: 0;
  }
  .talimat-kaynaklari {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .talimat-satiri {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }
  .talimat-uyarilari {
    margin-top: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .talimat-uyari {
    font: var(--type-meta);
    color: var(--status-warn);
  }
  .is-listesi {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
