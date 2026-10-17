// Kalfa'yı `tauri-driver` üzerinden açan ve süren yardımcılar (1c tasarımı
// §11.4). Her `Kalfa.ac` kendi geçici veri dizini, çalışma alanı ve ayar
// dosyasıyla yeni bir süreç başlatır; `kapat` süreci bitirir ve dizini siler.
import { execFileSync, spawn } from 'node:child_process';
import {
  closeSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  readlinkSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { remote } from 'webdriverio';

/** Sahte EVREN'in geçerli saydığı herhangi bir anahtar. */
export const GECERLI_ANAHTAR = 'evren_llm_e2e_gizli_anahtar_7f3a';
/** Sahte EVREN'in 401 döndüğü anahtar (`kalfa_uygulama::e2e`). */
export const GECERSIZ_ANAHTAR = 'evren_llm_gecersiz';
/** Sahte EVREN'e ulaşılamayan anahtar. */
export const AGSIZ_ANAHTAR = 'evren_llm_agsiz';

const SURUCU_PORTU = 4444;
const YEREL_PORT = 4445;
const WINDOWS = process.platform === 'win32';

const uygulama = process.env.KALFA_E2E_UYGULAMA;
if (!uygulama) throw new Error('KALFA_E2E_UYGULAMA yok; testleri `npm test` ile koşun.');

export const bekle = (ms) => new Promise((r) => setTimeout(r, ms));

export async function sureyleBekle(kosul, sure, mesaj) {
  const son = Date.now() + sure;
  while (Date.now() < son) {
    if (await kosul()) return;
    await bekle(100);
  }
  throw new Error(mesaj);
}

function portAcik(port) {
  return new Promise((coz) => {
    const s = connect(port, '127.0.0.1');
    s.once('connect', () => {
      s.destroy();
      coz(true);
    });
    s.once('error', () => coz(false));
  });
}

/** Bu testin derlediği Kalfa'nın süreçleri: kimlik ve yerleşik bellek (bayt). */
export function surecler() {
  if (WINDOWS) {
    const yol = uygulama.replaceAll("'", "''");
    const cikti = execFileSync(
      'powershell.exe',
      [
        '-NoProfile',
        '-Command',
        `Get-Process kalfa -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq '${yol}' } | ForEach-Object { "$($_.Id) $($_.WorkingSet64)" }; exit 0`,
      ],
      { encoding: 'utf8' },
    );
    return cikti
      .split(/\r?\n/)
      .filter(Boolean)
      .map((s) => {
        const [pid, bellek] = s.trim().split(' ').map(Number);
        return { pid, bellek };
      });
  }
  const bulunan = [];
  for (const ad of readdirSync('/proc')) {
    if (!/^\d+$/.test(ad)) continue;
    try {
      if (readlinkSync(`/proc/${ad}/exe`) !== uygulama) continue;
      const durum = readFileSync(`/proc/${ad}/status`, 'utf8');
      const kb = Number(/VmRSS:\s+(\d+)/.exec(durum)?.[1] ?? 0);
      bulunan.push({ pid: Number(ad), bellek: kb * 1024 });
    } catch {
      // Süreç bu arada bitti ya da başka kullanıcınınki.
    }
  }
  return bulunan;
}

/** Geçerli kullanıcının Kalfa çalışma alanı AppContainer profilleri. */
export function kalfaProfilSayisi() {
  if (!WINDOWS) return 0;
  const betik = `
$kok = 'Registry::HKEY_CURRENT_USER\\Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\CurrentVersion\\AppContainer\\Mappings'
if (-not (Test-Path -LiteralPath $kok)) { '0'; exit 0 }
try {
  $profiller = @(Get-ChildItem -LiteralPath $kok -ErrorAction Stop | Where-Object {
    $kayit = Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction Stop
    $kayit.Moniker -like 'Kalfa.Alan.*'
  })
  $profiller.Count
} catch [System.Security.SecurityException] {
  'ERISIM_YOK'
} catch [System.UnauthorizedAccessException] {
  'ERISIM_YOK'
}
`;
  const cikti = execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', betik], {
    encoding: 'utf8',
  }).trim();
  if (cikti === 'ERISIM_YOK') return null;
  const sayi = Number(cikti);
  if (!Number.isSafeInteger(sayi) || sayi < 0) {
    throw new Error(`Kalfa AppContainer profilleri sayılamadı: ${cikti}`);
  }
  return sayi;
}

/** Önceki testten kalan süreç yenisinin tek süreç kilidine takılır. */
async function sureclerBitsin(sure = 10000) {
  try {
    await sureyleBekle(() => surecler().length === 0, sure, 'Kalfa süreci kapanmadı');
  } catch {
    for (const { pid } of surecler()) {
      try {
        process.kill(pid, 'SIGKILL');
      } catch {
        // Zaten bitti.
      }
    }
    await sureyleBekle(() => surecler().length === 0, 5000, 'Kalfa süreci öldürülemedi');
  }
}

export class Kalfa {
  /**
   * Kalfa'yı açar. `ayarlar` sahte EVREN'in ayarlarıdır
   * (`kalfa_uygulama::e2e::E2eAyarlari`); çalışma alanı verilmezse geçici
   * dizindeki `alan` kullanılır. Ayarlar alanın yoluna bağlıysa `ayarlar`
   * bir işlevdir: uygulama açılmadan önce alanla çağrılır.
   */
  static async ac(ayarlar = {}) {
    await sureclerBitsin();
    const kok = mkdtempSync(join(tmpdir(), 'kalfa-e2e-'));
    const veri = join(kok, 'veri');
    const alan = join(kok, 'alan');
    mkdirSync(veri);
    mkdirSync(alan);
    if (typeof ayarlar === 'function') ayarlar = ayarlar(alan);
    // Linux'ta kabuk yalıtımda çalışır (aşama 2 tasarımı §12.5). Testlerin
    // `node`'u sistem dizinlerinde değilse (örneğin /opt/node) yalıtıma açılır;
    // kullanıcı bunu Ayarlar'dan yapar.
    // Yalıtım kurulu olmayan makinede kabuk testleri yalıtımsız koşar; bunun
    // için Ayarlar'daki yalıtımsız çalıştırma anahtarı açık verilir (§9.8).
    const nodeKoku = dirname(dirname(process.execPath));
    const yalitim = { yalitimsiz_izinli: true };
    if (process.platform === 'linux' && !nodeKoku.startsWith('/usr')) {
      yalitim.arac_zinciri_dizinleri = [nodeKoku];
    }
    writeFileSync(join(veri, 'yalitim.json'), JSON.stringify(yalitim));
    const ayarDosyasi = join(kok, 'e2e.json');
    writeFileSync(ayarDosyasi, JSON.stringify({ calisma_alani: alan, ...ayarlar }));

    const args = ['--port', String(SURUCU_PORTU), '--native-port', String(YEREL_PORT)];
    if (WINDOWS && process.env.KALFA_E2E_EDGEDRIVER) {
      args.push('--native-driver', process.env.KALFA_E2E_EDGEDRIVER);
    }
    const gunluk = openSync(join(kok, 'surucu.log'), 'a');
    const surucu = spawn('tauri-driver', args, {
      env: {
        ...process.env,
        KALFA_VERI_DIZINI: veri,
        KALFA_ANAHTAR_DEPOSU: 'bellek',
        KALFA_E2E: ayarDosyasi,
        // Xvfb'de GPU yok: WebKitGTK'nın GL birleştiricisi yazılımla
        // (llvmpipe) çalışır ve görünür her değişiklikte bir sonraki kare
        // ~25 ms gecikir. Gecikme ölçümü birleştiricinin değil arayüzün
        // gecikmesini sormalı; yazılım çiziminde birleştirme kapatılır.
        ...(process.platform === 'linux' ? { WEBKIT_DISABLE_COMPOSITING_MODE: '1' } : {}),
      },
      stdio: ['ignore', gunluk, gunluk],
    });
    closeSync(gunluk);
    try {
      await sureyleBekle(() => portAcik(SURUCU_PORTU), 15000, 'tauri-driver başlamadı');
      const b = await remote({
        hostname: '127.0.0.1',
        port: SURUCU_PORTU,
        logLevel: 'silent',
        capabilities: {
          'tauri:options': { application: uygulama },
          'wdio:enforceWebDriverClassic': true,
        },
      });
      return new Kalfa(b, surucu, { kok, veri, alan });
    } catch (hata) {
      // Oturum oluşmadan başarısız olursa `kapat` çağrılamaz. Yalnızca bu
      // denemenin başlattığı sürücü ağacını durdurup tanı günlüğünü koruruz.
      if (WINDOWS && surucu.pid) {
        try {
          execFileSync('taskkill.exe', ['/PID', String(surucu.pid), '/T', '/F'], {
            stdio: 'ignore',
          });
        } catch {
          surucu.kill();
        }
      } else {
        surucu.kill();
      }
      await sureclerBitsin();
      const gunlukMetni = readFileSync(join(kok, 'surucu.log'), 'utf8').trim();
      throw new Error(`${hata.message}\nSürücü günlüğü: ${gunlukMetni || '(boş)'}\nGeçici alan: ${kok}`);
    }
  }

  constructor(b, surucu, dizinler) {
    this.b = b;
    this.surucu = surucu;
    this.kok = dizinler.kok;
    this.veri = dizinler.veri;
    this.alan = dizinler.alan;
  }

  async kapat({ veriyiKoru = false } = {}) {
    await this.b.deleteSession().catch(() => {});
    this.surucu.kill();
    await sureclerBitsin();
    if (!veriyiKoru) {
      rmSync(this.kok, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 });
    }
  }

  /** Bu testin ACE kaydındaki alanları yerel onayla geri alır. */
  async hazirlananAlanlariKaldir() {
    const alanlar = await this.invoke('yalitim_calisma_alanlari');
    if (alanlar.length === 0) return;
    const oturum = await this.seciliOturum();
    if (oturum) await this.invoke('oturumu_kapat', { oturum });
    for (const { kimlik } of alanlar) {
      await this.invoke('yalitim_calisma_alanini_kaldir', { kimlik });
    }
    const kalan = await this.invoke('yalitim_calisma_alanlari');
    if (kalan.length > 0) {
      throw new Error(`Kalfa çalışma alanları kaldırılamadı: ${kalan.map((a) => a.kimlik).join(', ')}`);
    }
  }

  /** Pencereye geçer: `ana` ya da `sartlar`. Açılmasını bekler. */
  async pencere(ad, sure = 15000) {
    const yol = ad === 'sartlar' ? /\/sartlar\.html$/ : /^\/?(index\.html)?$/;
    await this.b.waitUntil(
      async () => {
        for (const h of await this.b.getWindowHandles().catch(() => [])) {
          try {
            await this.b.switchToWindow(h);
            if (yol.test(await this.b.execute(() => location.pathname))) return true;
          } catch {
            // Pencere bu arada kapandı ya da yeniden yükleniyor.
          }
        }
        return false;
      },
      { timeout: sure, interval: 200, timeoutMsg: `${ad} penceresi açılmadı` },
    );
  }

  async pencereSayisi() {
    return (await this.b.getWindowHandles()).length;
  }

  /** Bulunulan pencereden bir komut çağırır; hata metniyle reddedilir. */
  async invoke(komut, args = {}) {
    const [tamam, deger] = await this.b.executeAsync(
      (k, a, bitti) => {
        window.__TAURI_INTERNALS__.invoke(k, a).then(
          (d) => bitti([true, d ?? null]),
          (e) => bitti([false, typeof e === 'string' ? e : JSON.stringify(e)]),
        );
      },
      komut,
      args,
    );
    if (!tamam) throw new Error(deger);
    return deger;
  }

  /** Oturumun kayıt öğeleri (`yuk` alanları), yazılış sırasıyla. */
  kayit(oturum) {
    return readFileSync(join(this.veri, 'oturumlar', `${oturum}.jsonl`), 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((s) => JSON.parse(s).yuk);
  }

  /** Kayıt dizinindeki oturum kimlikleri. */
  oturumlar() {
    return readdirSync(join(this.veri, 'oturumlar'))
      .filter((d) => d.endsWith('.jsonl'))
      .map((d) => d.slice(0, -'.jsonl'.length));
  }

  // --- Şart penceresi ---

  async anahtarGir(anahtar) {
    const girdi = await this.b.$('input[type=password]');
    await girdi.waitForDisplayed({ timeout: 15000 });
    await girdi.setValue(anahtar);
    await (await this.b.$('button[type=submit]')).click();
  }

  // --- Ana pencere ---

  async dugme(metin) {
    const d = await this.b.$(`button=${metin}`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click();
  }

  /**
   * Onay kartındaki "İncele" ile onay sayfasını açar. İçerik o sırada büyüyüp
   * kayıyorsa (araç kartı adım adım çizilirken) tıklama düğmenin eski konumuna
   * düşebilir; sayfa açılana kadar yeniden basılır.
   */
  async onaySayfasiniAc() {
    await sureyleBekle(
      async () => {
        await this.dugme('İncele');
        return this.b.execute(() => document.querySelector('[role="dialog"]') !== null);
      },
      20000,
      'onay sayfası açılmadı',
    );
  }

  /**
   * Pencereyi kapatan düğmeye basar ("Kabul ediyorum"). Pencere tıklamanın
   * yanıtından önce kapanabilir; sürücünün "no such window" hatası o zaman
   * beklenen sonuçtur.
   */
  async kapatanDugme(metin) {
    const d = await this.b.$(`button=${metin}`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click().catch((e) => {
      if (!/no such window/.test(String(e))) throw e;
    });
  }

  async ikonDugmesi(etiket) {
    const d = await this.b.$(`button[aria-label="${etiket}"]`);
    await d.waitForClickable({ timeout: 15000 });
    await d.click();
  }

  /**
   * Bütçe kararı kartında "Bütçeyle devam et" (5b): kullanımı bilinmeyen bir
   * istekten (ör. iptal edilen akış) sonra yeni istekten önce karar gerekir.
   */
  async butceyleDevam() {
    const d = await this.b.$(
      '//section[contains(., "Bütçe kararı")]//button[normalize-space()="Bütçeyle devam et"]',
    );
    await d.waitForClickable({ timeout: 15000 });
    await d.click();
  }

  async yeniOturum() {
    await this.ikonDugmesi('Yeni oturum');
    await (await this.composer()).waitForEnabled({ timeout: 15000 });
  }

  composer() {
    return this.b.$('textarea[aria-label^="Bir şey sorun"]');
  }

  async gonder(metin) {
    const c = await this.composer();
    await c.waitForEnabled({ timeout: 15000 });
    await c.setValue(metin);
    await this.ikonDugmesi('Gönder');
  }

  /** Konuşma sütununun görünen metni. */
  async konusma() {
    return this.b.execute(() => document.querySelector('.akis')?.textContent ?? '');
  }

  async metniBekle(parca, sure = 20000) {
    await this.b.waitUntil(async () => (await this.konusma()).includes(parca), {
      timeout: sure,
      interval: 100,
      timeoutMsg: `konuşmada görünmedi: ${parca}`,
    });
  }

  /** Tur sonu çizgilerinin nedenleri (`Normal`, `Iptal`, …), sırayla. */
  async turSonlari() {
    return this.b.execute(() =>
      [...document.querySelectorAll('.tur-sonu')].map(
        (e) => [...e.classList].find((c) => c.startsWith('n-'))?.slice(2) ?? '',
      ),
    );
  }

  async turSonuBekle(sayi, sure = 30000) {
    await this.b.waitUntil(async () => (await this.turSonlari()).length >= sayi, {
      timeout: sure,
      interval: 100,
      timeoutMsg: `${sayi}. tur bitmedi`,
    });
    return (await this.turSonlari())[sayi - 1];
  }

  /** Seçili oturumun kimliği (uygulama durumundaki son oturum). */
  async seciliOturum() {
    return (await this.invoke('uygulama_durumu')).son_oturum;
  }
}

/** Konuşmadaki son kabuk çağrısının terminali. */
export async function terminalMetni(k) {
  return k.b.execute(
    () => [...document.querySelectorAll('[aria-label="Komut çıktısı"]')].at(-1)?.textContent ?? '',
  );
}

/** Seçiciye uyan son öğenin metni (örneğin son tur özeti). */
export async function sayfaMetni(k, secici) {
  return k.b.execute(
    (s) => [...document.querySelectorAll(s)].at(-1)?.textContent ?? '',
    secici,
  );
}

export function yuzdelik(degerler, p) {
  const sirali = [...degerler].sort((a, b) => a - b);
  const i = Math.min(sirali.length - 1, Math.ceil((p / 100) * sirali.length) - 1);
  return sirali[Math.max(0, i)];
}
