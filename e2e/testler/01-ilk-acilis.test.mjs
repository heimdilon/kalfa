// İlk açılış (1c tasarımı §7, §11.4): anahtar → şart → kabul → ana pencere;
// ret → kilitli görünüm; geçersiz anahtar ve ulaşılamayan EVREN → form hatası.
// Anahtar ana pencereye hiç girmez.
import assert from 'node:assert/strict';
import { afterEach, describe, it } from 'node:test';
import { AGSIZ_ANAHTAR, GECERLI_ANAHTAR, GECERSIZ_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

describe('ilk açılış', () => {
  let k;
  afterEach(async () => {
    await k?.kapat();
    k = undefined;
  });

  it('anahtar, şart metni ve kabulden sonra ana pencere açılır', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);

    const kabul = await k.b.$('button=Kabul ediyorum');
    await kabul.waitForClickable({ timeout: 15000 });
    const metin = await k.b.$('body').getText();
    assert.match(metin, /EVREN kullanım şartları/);
    assert.match(metin, /Sahte kullanım şartları/);
    await k.kapatanDugme('Kabul ediyorum');

    await k.pencere('ana');
    await k.b.waitUntil(async () => (await k.pencereSayisi()) === 1, {
      timeout: 10000,
      timeoutMsg: 'şart penceresi kapanmadı',
    });
    await (await k.b.$('button[aria-label="Yeni oturum"]')).waitForDisplayed({ timeout: 15000 });
  });

  it('anahtar ana pencerenin DOM’una ve durumuna girmez', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);
    await k.kapatanDugme('Kabul ediyorum');
    await k.pencere('ana');
    await k.yeniOturum();
    await k.gonder('merhaba');
    await k.metniBekle('Yanıt: merhaba');

    const izler = await k.b.execute(() => {
      const depo = (s) => [...Array(s.length).keys()].map((i) => `${s.key(i)}=${s.getItem(s.key(i))}`);
      return [
        document.documentElement.outerHTML,
        ...depo(localStorage),
        ...depo(sessionStorage),
        document.cookie,
      ].join('\n');
    });
    assert.ok(!izler.includes(GECERLI_ANAHTAR), 'anahtar ana pencerenin DOM’unda ya da deposunda');
    const durum = JSON.stringify(await k.invoke('uygulama_durumu'));
    assert.ok(!durum.includes(GECERLI_ANAHTAR), 'anahtar uygulama durumunda');
  });

  it('şartlar reddedilince kilitli görünüm gelir, ana pencere açılmaz', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERLI_ANAHTAR);
    await k.dugme('Kabul etmiyorum');
    const baslik = await k.b.$('h1=Şartlar kabul edilmedi');
    await baslik.waitForDisplayed({ timeout: 15000 });
    await (await k.b.$('button=Metni yeniden göster')).waitForDisplayed();
    assert.equal(await k.pencereSayisi(), 1);
  });

  it('geçersiz anahtar formda hata verir ve saklanmaz', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(GECERSIZ_ANAHTAR);
    const uyari = await k.b.$('[role="alert"]');
    await uyari.waitForDisplayed({ timeout: 15000 });
    assert.match(await uyari.getText(), /Anahtar geçersiz ya da eksik/);
    await (await k.b.$('input[type=password]')).waitForDisplayed();
  });

  it('EVREN’e ulaşılamazsa formda hata verir', async () => {
    k = await Kalfa.ac({ sart: 'gerekli' });
    await k.pencere('sartlar');
    await k.anahtarGir(AGSIZ_ANAHTAR);
    const uyari = await k.b.$('[role="alert"]');
    await uyari.waitForDisplayed({ timeout: 15000 });
    assert.match(await uyari.getText(), /EVREN'e ulaşılamıyor/);
  });
});
