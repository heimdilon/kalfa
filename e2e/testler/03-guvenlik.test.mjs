// Güvenlik sınırları gerçek webview'da (1c tasarımı §2, §6, §9.1, §11.4):
// pencere yetkileri, CSP ve model çıktısındaki betik, görsel ve bağlantılar.
import assert from 'node:assert/strict';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const ZARARLI = [
  'Deneme <script>window.__xss = 1</script> sonu.',
  '',
  '<img src="x" onerror="window.__xss = 2">',
  '',
  '![uzak görsel](https://example.invalid/iz.png)',
  '',
  '[betik bağlantısı](javascript:window.__xss=3)',
  '',
  '[dosya bağlantısı](file:///etc/passwd)',
  '',
  '<iframe src="https://example.invalid/"></iframe>',
].join('\n');

const SENARYOLAR = {
  zararli: { yanitlar: [[{ tur: 'metin', metin: ZARARLI }]] },
};

/** Webview'ın yüklediği kaynakların ve IPC çağrılarının adresleri. */
const kaynaklar = (k) =>
  k.b.execute(() => performance.getEntriesByType('resource').map((e) => e.name));

describe('güvenlik', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('ana pencere şart kabul edemez, anahtar kaydedemez, pencere açamaz', async () => {
    for (const [komut, args] of [
      ['sartlari_kabul_et', { surum: 1 }],
      ['anahtari_kaydet', { anahtar: 'evren_llm_x' }],
      ['sart_penceresi_durumu', {}],
      ['plugin:webview|create_webview_window', {}],
      ['plugin:opener|open_url', { url: 'https://example.com' }],
      ['plugin:dialog|open', {}],
    ]) {
      await assert.rejects(k.invoke(komut, args), /not allowed/, `${komut} reddedilmedi`);
    }
  });

  // WebDriver'ın çalıştırdığı betik sayfanın değil sürücünün bağlamındadır;
  // Chromium orada `eval`'i ihlal bildirerek çalıştırır. Bu yüzden eval, sayfanın
  // kendi bağlamında çalışan dize zamanlayıcısıyla sınanır.
  it('CSP satır içi betiği, satır içi olay işleyicisini ve dizeden kodu engeller', async () => {
    const sonuc = await k.b.executeAsync((bitti) => {
      const ihlaller = [];
      document.addEventListener('securitypolicyviolation', (e) =>
        ihlaller.push({ yonerge: e.effectiveDirective || e.violatedDirective, adres: e.blockedURI }),
      );
      const betik = document.createElement('script');
      betik.textContent = 'window.__csp = 1';
      document.body.append(betik);
      const dugme = document.createElement('button');
      dugme.setAttribute('onclick', 'window.__csp = 2');
      document.body.append(dugme);
      dugme.click();
      setTimeout('window.__csp = 3', 0);
      betik.remove();
      dugme.remove();
      setTimeout(() => bitti({ csp: window.__csp ?? null, ihlaller }), 300);
    });
    assert.equal(sonuc.csp, null, 'satır içi kod çalıştı');
    const betikIhlali = sonuc.ihlaller.filter((i) => i.yonerge.startsWith('script-src'));
    assert.ok(betikIhlali.length >= 3, `CSP ihlalleri eksik: ${JSON.stringify(sonuc.ihlaller)}`);
    assert.ok(
      betikIhlali.some((i) => i.adres === 'eval'),
      `dizeden kod ihlali yok: ${JSON.stringify(sonuc.ihlaller)}`,
    );
  });

  it('model çıktısındaki betik çalışmaz; görsel, çerçeve ve tehlikeli bağlantı oluşmaz', async () => {
    const once = new Set(await kaynaklar(k));
    await k.gonder('zararli');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('uzak görsel');

    const dom = await k.b.execute(() => {
      const akis = document.querySelector('.akis');
      return {
        xss: window.__xss ?? null,
        tehlikeli: akis.querySelectorAll('script, img, iframe, object, embed, a[href], [onerror]')
          .length,
        metin: akis.textContent,
        dugmeler: [...akis.querySelectorAll('button')].map((b) => b.textContent.trim()),
      };
    });
    assert.equal(dom.xss, null, 'model çıktısındaki betik çalıştı');
    assert.equal(dom.tehlikeli, 0, 'konuşmada betik, görsel, çerçeve ya da bağlantı var');
    assert.match(dom.metin, /<script>window.__xss = 1<\/script>/, 'ham HTML metin olarak görünmüyor');
    assert.ok(!dom.dugmeler.includes('betik bağlantısı'), 'javascript: bağlantıya dönüştü');
    assert.ok(!dom.dugmeler.includes('dosya bağlantısı'), 'file: bağlantıya dönüştü');

    // Bağlantı metnine tıklamak bir şey yapmaz.
    const betikMetni = await k.b.$('//*[contains(@class,"akis")]//*[text()[contains(.,"betik bağlantısı")]]');
    if (await betikMetni.isExisting()) await betikMetni.click();

    const yeni = (await kaynaklar(k)).filter((a) => !once.has(a));
    assert.ok(
      !yeni.some((a) => a.includes('example.invalid')),
      `uzak kaynak yüklendi: ${yeni.join(', ')}`,
    );
    const ipc = yeni
      .filter((a) => /ipc/.test(a))
      .map((a) => decodeURIComponent(new URL(a).pathname.slice(1)));
    const beklenen = new Set(['mesaj_gonder', 'paketi_isledim', 'oturumlari_listele']);
    const beklenmeyen = ipc.filter((c) => !beklenen.has(c));
    assert.deepEqual(beklenmeyen, [], `beklenmeyen IPC çağrıları: ${beklenmeyen.join(', ')}`);
    assert.equal(await k.b.execute(() => window.__xss ?? null), null);
  });

  it('şart penceresi oturum komutlarını çağıramaz', async () => {
    await k.b.execute(() => {
      const d = [...document.querySelectorAll('button')].find(
        (b) => b.textContent.trim() === 'Anahtarı değiştir',
      );
      d.click();
    });
    await k.pencere('sartlar');
    for (const [komut, args] of [
      ['oturumlari_listele', {}],
      ['mesaj_gonder', { oturum: '00000000-0000-0000-0000-000000000000', metin: 'x' }],
      ['calisma_alani_sec', {}],
      ['e2e_olcumler', {}],
    ]) {
      await assert.rejects(k.invoke(komut, args), /not allowed/, `${komut} reddedilmedi`);
    }
  });
});
