// Üst belge §11.1 kabul kapısı (1c tasarımı §2, §11.4): gecikme, duraklayan
// arayüz, arayüz kopması ve uzun oturum. Ölçümler `sonuclar/<platform>.json`'a
// yazılır ve docs/GELISTIRME_DURUMU.md'ye aktarılır.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import {
  GECERLI_ANAHTAR,
  Kalfa,
  bekle,
  surecler,
  sureyleBekle,
  yuzdelik,
} from '../yardimci/kalfa.mjs';

const PARCA = 250;

const SENARYOLAR = {
  isinma: {
    yanitlar: [[{ tur: 'damgali', sayi: 100, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  'gecikme-duzenli': {
    yanitlar: [[{ tur: 'damgali', sayi: PARCA, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  'gecikme-yigin': {
    yanitlar: [
      [{ tur: 'damgali', sayi: PARCA, desen: { ad: 'yigin', en_az: 5, en_cok: 10, aralik_ms: 100 } }],
    ],
  },
  'gecikme-paralel': {
    yanitlar: [[{ tur: 'damgali', sayi: PARCA, desen: { ad: 'paralel', aralik_ms: 20 } }]],
  },
  kucuk: {
    yanitlar: [[{ tur: 'damgali', sayi: 150, desen: { ad: 'duzenli', aralik_ms: 20 } }]],
  },
  sel: { yanitlar: [[{ tur: 'sel', parca_bayt: 4096 }]] },
  uzun: {
    yanitlar: [[{ tur: 'damgali', sayi: 600, desen: { ad: 'duzenli', aralik_ms: 100 } }]],
  },
};

const AYARLAR = { sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR };

const sonuc = { platform: process.platform, tarih: new Date().toISOString() };
const yuvarla = (x) => Math.round(x * 10) / 10;
const MB = 1024 * 1024;
const bellek = () => Math.max(0, ...surecler().map((s) => s.bellek));

after(() => {
  const dizin = join(import.meta.dirname, '..', 'sonuclar');
  mkdirSync(dizin, { recursive: true });
  writeFileSync(join(dizin, `${process.platform}.json`), JSON.stringify(sonuc, null, 2));
  console.log(JSON.stringify(sonuc, null, 2));
});

/** Kayıttaki bitmiş tur sayısı ve sonuncusunun nedeni. */
function bitenler(k, oturum) {
  const t = k.kayit(oturum).filter((y) => y.tip === 'TurBitti');
  return { sayi: t.length, son: t.at(-1)?.neden.tip };
}

describe('kabul kapısı: aynı oturumda', () => {
  let k;
  let tur = 0;
  before(async () => {
    k = await Kalfa.ac(AYARLAR);
    await k.pencere('ana');
    await k.yeniOturum();
    // Kanca ilk dinlenen oturumla kurulur.
    await k.b.waitUntil(() => k.b.execute(() => Boolean(window.__kalfaE2e)), { timeout: 10000 });
    // Isınma turu ölçülmez: uygulamanın ilk turu webview'ın ilk çizimlerini ve
    // derleyici ısınmasını da içerir; kabul kapısı akışın sürekli durumdaki
    // gecikmesini sorar.
    await k.gonder('isinma');
    assert.equal(await k.turSonuBekle(++tur), 'Normal');
  });
  after(async () => {
    await k?.kapat();
  });

  for (const desen of ['duzenli', 'yigin', 'paralel']) {
    it(`gecikme (${desen}): p95 < 50 ms`, async () => {
      await k.b.execute(() => window.__kalfaE2e.sifirla());
      await k.gonder(`gecikme-${desen}`);
      assert.equal(await k.turSonuBekle(++tur), 'Normal');
      const beklenen = desen === 'paralel' ? 2 * PARCA : PARCA;
      await k.b.waitUntil(
        async () => (await k.b.execute(() => window.__kalfaE2e.gecikmeler().length)) >= beklenen,
        { timeout: 5000, timeoutMsg: 'bütün parçalar ölçülmedi' },
      );
      const g = await k.b.execute(() => window.__kalfaE2e.gecikmeler());
      const ozet = {
        parca: g.length,
        p50: yuvarla(yuzdelik(g, 50)),
        p95: yuvarla(yuzdelik(g, 95)),
        p99: yuvarla(yuzdelik(g, 99)),
        en_cok: yuvarla(Math.max(...g)),
      };
      (sonuc.gecikme ??= {})[desen] = ozet;
      assert.equal(g.length, beklenen);
      assert.ok(ozet.p95 < 50, `p95 ${ozet.p95} ms: ${JSON.stringify(ozet)}`);
    });
  }

  it('küçültülmüş pencerede paketler işlenmeye devam eder', async () => {
    await k.gonder('kucuk');
    await k.metniBekle('[[e2e:5:');
    await k.b.minimizeWindow();
    try {
      // Tur yaklaşık 3 saniye sürer; pencere küçükken biterse arayüz
      // paketleri işlemeyi sürdürmüştür.
      assert.equal(await k.turSonuBekle(++tur, 15000), 'Normal');
    } finally {
      await k.b.maximizeWindow();
    }
    assert.ok(!(await k.konusma()).includes('Arayüz uzun süre yanıt vermediği'));
  });

  it('duraklayan arayüz: yoldaki veri krediyi, kuyruk bütçeyi aşmaz; tur iptal edilir', async () => {
    const oturum = await k.seciliOturum();
    const once = bitenler(k, oturum).sayi;
    await k.invoke('e2e_olcumleri_sifirla');
    const bellekOnce = bellek();
    let bellekEnCok = bellekOnce;

    await k.b.execute(() => window.__kalfaE2e.duraklat());
    await k.gonder('sel');
    await sureyleBekle(
      () => {
        bellekEnCok = Math.max(bellekEnCok, bellek());
        return bitenler(k, oturum).sayi > once;
      },
      30000,
      'bütçe dolunca tur iptal edilmedi',
    );
    assert.equal(bitenler(k, oturum).son, 'Iptal');
    const bekleyen = await k.b.execute(() => window.__kalfaE2e.bekleyen());

    await k.b.execute(() => window.__kalfaE2e.surdur());
    await k.metniBekle('Arayüz uzun süre yanıt vermediği için oturum durduruldu.');
    const o = await k.invoke('e2e_olcumler');
    sonuc.duraklama = {
      ...o,
      duraklarken_gelen_paket: bekleyen,
      bellek_once_mb: yuvarla(bellekOnce / MB),
      bellek_en_cok_mb: yuvarla(bellekEnCok / MB),
    };
    assert.ok(bekleyen <= o.kredi_paket, `duraklarken ${bekleyen} paket geldi`);
    assert.ok(o.yolda_paket_en_cok <= o.kredi_paket, JSON.stringify(o));
    assert.ok(o.yolda_bayt_en_cok <= o.kredi_bayt, JSON.stringify(o));
    assert.ok(o.kuyruk_olay_en_cok <= o.butce_olay, JSON.stringify(o));
    assert.ok(o.kuyruk_bayt_en_cok <= o.butce_bayt, JSON.stringify(o));
    assert.ok(o.kuyruk_bayt_en_cok > o.butce_bayt / 2, `kuyruk dolmadı: ${JSON.stringify(o)}`);
    // Akıl sağlığı: sürecin belleği bütçenin birkaç katından fazla büyümez.
    assert.ok(bellekEnCok - bellekOnce < 256 * MB, JSON.stringify(sonuc.duraklama));
  });

  it('tur sürerken sayfa yeniden yüklenirse tur iptal edilir ve oturum kayıttan açılır', async () => {
    await k.yeniOturum();
    const oturum = await k.seciliOturum();
    await k.gonder('uzun');
    await k.metniBekle('[[e2e:3:');
    await k.b.refresh();
    await k.pencere('ana');
    // Yeni sayfa son oturumu kayıttan açar; kayıt kilidi bırakılmıştır.
    assert.equal(await k.turSonuBekle(1), 'Iptal');
    assert.deepEqual(bitenler(k, oturum), { sayi: 1, son: 'Iptal' });
    // İptal edilen akışın kullanımı bilinmez: yeni istekten önce bütçe kararı (5b).
    await k.butceyleDevam();
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(2), 'Normal');
  });
});

describe('kabul kapısı: pencere kapanması', () => {
  it('tur sürerken ana pencere kapanırsa tur iptal edilir', async () => {
    const k = await Kalfa.ac(AYARLAR);
    try {
      await k.pencere('ana');
      await k.yeniOturum();
      const oturum = await k.seciliOturum();
      await k.gonder('uzun');
      await k.metniBekle('[[e2e:3:');
      // Kapatma düğmesinin yolu: komutun yanıtı beklenmez, pencere kapanır.
      await k.b.execute(() => {
        window.__TAURI_INTERNALS__.invoke('e2e_ana_pencereyi_kapat');
      });
      await sureyleBekle(() => surecler().length === 0, 15000, 'Kalfa kapanmadı');
      assert.deepEqual(bitenler(k, oturum), { sayi: 1, son: 'Iptal' });
    } finally {
      await k.kapat();
    }
  });
});

describe('kabul kapısı: uzun oturum', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac(AYARLAR);
    await k.pencere('ana');
  });
  after(async () => {
    await k?.kapat();
  });

  /** Kaydın satırına tıklayıp son mesaj çizilene kadar geçen süre. */
  const ac = (tur) =>
    k.b.executeAsync((tur, bitti) => {
      const baslik = `(${tur} turluk kayıt)`;
      const satir = [...document.querySelectorAll('button.oturum-satiri')].find((b) =>
        b.textContent.includes(baslik),
      );
      const t0 = performance.now();
      satir.click();
      const bak = () => {
        const son = [...document.querySelectorAll('article.mesaj.kullanici .duz')].at(-1);
        if (son?.textContent === `${tur}. soru ${baslik}`) {
          requestAnimationFrame(() =>
            setTimeout(
              () =>
                bitti({
                  sure_ms: performance.now() - t0,
                  dom_dugumu: document.getElementsByTagName('*').length,
                  js_yigini_mb: performance.memory
                    ? performance.memory.usedJSHeapSize / 1048576
                    : null,
                }),
              0,
            ),
          );
        } else setTimeout(bak, 2);
      };
      bak();
    }, tur);

  it('500 turluk kayıt 20 turluk kayıtla aynı mertebede açılır', async () => {
    await k.invoke('e2e_kayit_uret', { tur: 20 });
    await k.invoke('e2e_kayit_uret', { tur: 500 });
    await k.b.refresh();
    await k.pencere('ana');
    await k.b.waitUntil(
      async () => (await k.b.$$('button.oturum-satiri')).length === 2,
      { timeout: 10000, timeoutMsg: 'kayıtlar listede yok' },
    );
    // Önce uzun kayıt açılır: ısınmanın avantajı kısa kayda kalır.
    const b500 = await ac(500);
    const r500 = { ...b500, bellek_mb: yuvarla(bellek() / MB) };
    await bekle(500);
    const b20 = await ac(20);
    const r20 = { ...b20, bellek_mb: yuvarla(bellek() / MB) };
    for (const r of [r20, r500]) {
      r.sure_ms = yuvarla(r.sure_ms);
      if (r.js_yigini_mb !== null) r.js_yigini_mb = yuvarla(r.js_yigini_mb);
    }
    sonuc.uzun_oturum = { tur_20: r20, tur_500: r500 };
    assert.ok(
      r500.sure_ms < Math.max(10 * r20.sure_ms, r20.sure_ms + 500),
      JSON.stringify(sonuc.uzun_oturum),
    );
    assert.ok(r500.dom_dugumu < 1.5 * r20.dom_dugumu, JSON.stringify(sonuc.uzun_oturum));
  });
});
