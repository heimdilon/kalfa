// Yerel MCP (1.1a tasarımı §11 "E2e"): test sunucusu Ayarlar'daki formla
// eklenir, sohbetteki başlatma kartıyla izin verilir, sahte model aracı
// çağırır, onay sayfasında onaylanır ve sonuç görünür. Tam erişimde formun
// kalıcı uyarısı, kartın yerel onay notu ve her çağrının yerel onayı
// görünür. Yerel pencereleri `onay: true` onaylar; dosya seçici
// `secili_dosya`'yı döner. Sunucu ikilisi çalışma alanına kopyalanır:
// yalıtımda yalnız orası görünür.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { copyFileSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, bekle, kalfaProfilSayisi, sureyleBekle } from '../yardimci/kalfa.mjs';

const WINDOWS = process.platform === 'win32';
const SUNUCU = join(
  dirname(process.env.KALFA_E2E_UYGULAMA),
  WINDOWS ? 'kalfa-mcp-test-sunucusu.exe' : 'kalfa-mcp-test-sunucusu',
);

function senaryolar(sunucu, okunur) {
  return {
    merhaba: { yanitlar: [[{ tur: 'metin', metin: 'Merhaba.' }]] },
    // Keşif turun içinde yayımlanır; tur gerçek bir model gibi sürer.
    bekle: { yanitlar: [[{ tur: 'bekle', ms: 3000 }, { tur: 'metin', metin: 'Hazır.' }]] },
    oku: {
      yanitlar: [
        [{ tur: 'arac', ad: `mcp__${sunucu}__dene`, argumanlar: { islem: 'oku', yol: okunur } }],
        [{ tur: 'arac_sonucu' }],
      ],
    },
  };
}

/** Seçicideki ilk öğenin görünen metni (yoksa boş). */
function metin(k, secici) {
  return k.b.execute((s) => document.querySelector(s)?.textContent ?? '', secici);
}

/** Metni içeren ilk düğmeye basar. */
function dugmeyeBas(k, parca, kapsam = 'body') {
  return k.b.execute(
    (p, s) => {
      const d = [...document.querySelector(s).querySelectorAll('button')].find(
        (b) => !b.disabled && b.textContent?.includes(p),
      );
      d?.click();
      return d !== undefined;
    },
    parca,
    kapsam,
  );
}

/** Etiketiyle bulunan alana yazar (Svelte girdisi `input` olayını dinler). */
async function alanaYaz(k, etiket, deger) {
  const ok = await k.b.execute(
    (e, d) => {
      const l = [...document.querySelectorAll('form label')].find(
        (x) => x.textContent?.trim().startsWith(e),
      );
      const alan = l?.htmlFor ? document.getElementById(l.htmlFor) : l?.querySelector('input, textarea');
      if (!alan) return false;
      const ayarla = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(alan), 'value').set;
      ayarla.call(alan, d);
      alan.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    },
    etiket,
    deger,
  );
  assert.ok(ok, `alan yok: ${etiket}`);
}

async function radyoSec(k, etiket) {
  const ok = await k.b.execute((e) => {
    const l = [...document.querySelectorAll('label')].find((x) => x.textContent?.trim() === e);
    l?.querySelector('input')?.click();
    return l !== undefined;
  }, etiket);
  assert.ok(ok, `seçenek yok: ${etiket}`);
}

/** Ayarlar → MCP sunucuları → formla sunucu ekler. */
async function sunucuEkle(k, ad, tamErisim) {
  assert.ok(await dugmeyeBas(k, 'Ayarlar', 'nav'), 'menüde Ayarlar yok');
  await sureyleBekle(
    async () => (await metin(k, 'section.ayarlar')).includes('MCP sunucuları'),
    10000,
    'MCP bölümü görünmedi',
  );
  assert.ok(await dugmeyeBas(k, 'Sunucu ekle'));
  await alanaYaz(k, 'Ad', ad);
  await radyoSec(k, 'Dosya seç');
  assert.ok(await dugmeyeBas(k, 'Gözat'));
  await sureyleBekle(
    async () => (await metin(k, 'form')).includes(WINDOWS ? 'sunucu.exe' : 'sunucu'),
    10000,
    'seçilen dosya formda görünmedi',
  );
  await alanaYaz(k, 'Argümanlar', 'sinama');
  if (tamErisim) {
    await radyoSec(k, 'Tam sistem erişimi');
    const form = await metin(k, 'form');
    assert.match(form, /geride çalışan süreçler kalabilir/);
    assert.match(form, /Sınırsız/);
  }
  assert.ok(await dugmeyeBas(k, 'Kaydet', 'form'));
  await sureyleBekle(
    async () =>
      k.b.execute((a) => document.querySelector(`[role="group"][aria-label="${a}"]`) !== null, ad),
    15000,
    'sunucu listede görünmedi',
  );
  assert.ok(await dugmeyeBas(k, 'Oturumlar', 'nav'));
  await (await k.composer()).waitForEnabled({ timeout: 15000 });
}

/** Başlatma kartına "Başlat" der; keşif yayımlanana kadar turlar sürer. */
async function baslatVeKesfet(k, ad) {
  await k.gonder('merhaba');
  await k.turSonuBekle(1);
  const baslik = `${ad} bu çalışma alanında başlatılsın mı?`;
  await sureyleBekle(
    async () => (await k.konusma()).includes(baslik),
    20000,
    'başlatma kartı gelmedi',
  );
  const kart = await metin(k, `[role="group"][aria-label="${baslik}"]`);
  assert.match(kart, /Bu kartı model açamaz ve yanıtlayamaz/);
  assert.match(kart, /araç çağrılmasa da çalışır/);
  assert.ok(await dugmeyeBas(k, 'Başlat', `[role="group"][aria-label="${baslik}"]`));
  let tur = 1;
  for (let i = 0; i < 6; i += 1) {
    await k.gonder('bekle');
    tur += 1;
    await k.turSonuBekle(tur, 60000);
    const durum = await k.invoke('mcp_durumu');
    const alan = durum.sunucular.find((s) => s.ozet.ad === ad)?.alanlar[0];
    if (alan?.arac_sayisi === 1) return { tur, kart };
    await bekle(500);
  }
  throw new Error(`keşif yayımlanmadı: ${JSON.stringify(await k.invoke('mcp_durumu'))}`);
}

/** Bu testin çalışma alanından başlamış süreç kaldı mı? */
function alandanSurecVar(alan) {
  if (WINDOWS) {
    const yol = alan.replaceAll("'", "''");
    return (
      execFileSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-Command',
          `@(Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -like '${yol}*' }).Count`,
        ],
        { encoding: 'utf8' },
      ).trim() !== '0'
    );
  }
  return readdirSync('/proc')
    .filter((d) => /^\d+$/.test(d))
    .some((d) => {
      try {
        return readFileSync(`/proc/${d}/cmdline`, 'utf8').includes(alan);
      } catch {
        return false;
      }
    });
}

for (const { ad, tamErisim } of [
  { ad: 'notlar', tamErisim: false },
  { ad: 'tam', tamErisim: true },
]) {
  describe(`MCP ${tamErisim ? 'tam erişim' : 'yalıtımlı'}`, () => {
    let k;
    let profiller;

    before(async () => {
      profiller = kalfaProfilSayisi();
      k = await Kalfa.ac((alan) => {
        const exe = join(alan, WINDOWS ? 'sunucu.exe' : 'sunucu');
        copyFileSync(SUNUCU, exe);
        const okunur = join(alan, 'okunur.txt');
        writeFileSync(okunur, 'ALAN-ICERIGI');
        return {
          sart: 'kabul',
          anahtar: GECERLI_ANAHTAR,
          onay: true,
          secili_dosya: exe,
          senaryolar: senaryolar(ad, okunur),
        };
      });
      await k.pencere('ana');
      await k.yeniOturum();
    });

    after(async () => {
      if (!k) return;
      const alan = k.alan;
      await k.hazirlananAlanlariKaldir().catch(() => {});
      await k.kapat();
      await sureyleBekle(async () => !alandanSurecVar(alan), 15000, 'MCP süreci kapanmadı');
      // MCP kendisi AppContainer profili bırakmaz (§11 Windows).
      assert.equal(kalfaProfilSayisi(), profiller);
    });

    it('formla eklenir, kartla başlar, çağrı onaylanır ve sonuç görünür', async () => {
      await sunucuEkle(k, ad, tamErisim);
      const { tur, kart } = await baslatVeKesfet(k, ad);
      if (tamErisim) {
        assert.match(kart, /Sınırsız/);
        assert.match(kart, /yerel bir pencerede ayrıca onay ister/);
      }

      await k.gonder('oku');
      await k.onaySayfasiniAc();
      const sayfa = await metin(k, '[role="dialog"]');
      assert.match(sayfa, new RegExp(`${ad} MCP sunucusu · dene`));
      assert.match(sayfa, tamErisim ? /Tam sistem erişimi/ : /Yalıtılmış süreç/);
      if (tamErisim) assert.match(sayfa, /Tam sistem erişimli MCP araçları/);
      await k.dugme('Tümünü onayla');
      assert.equal(await k.turSonuBekle(tur + 1, 60000), 'Normal');
      await k.metniBekle('TAMAM:ALAN-ICERIGI');
      assert.match(await k.konusma(), new RegExp(`${ad} · dene`));
    });
  });
}
