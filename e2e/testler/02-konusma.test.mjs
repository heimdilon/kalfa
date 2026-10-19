// Konuşma (1c tasarımı §1, §8, §11.4): mesaj → akış → onay sayfası → yazma;
// ret gerekçesinin modele gitmesi; iptal; oturumu kapatıp kayıttan açma ve
// geçmişin doğru sırayla gelmesi.
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  yaz: {
    yanitlar: [
      [
        { tur: 'metin', metin: 'Notu yazıyorum.' },
        { tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'notlar.txt', icerik: 'merhaba\n' } },
      ],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  'yaz-ret': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'ret.txt', icerik: 'x\n' } }],
      [{ tur: 'arac_sonucu' }],
    ],
  },
  uzun: {
    yanitlar: [[{ tur: 'damgali', sayi: 600, desen: { ad: 'duzenli', aralik_ms: 100 } }]],
  },
};

describe('konuşma', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    await k?.kapat();
  });

  it('akan yanıttan sonra karttan onaylanan değişiklik dosyaya yazılır', async () => {
    await k.gonder('yaz');
    await k.metniBekle('Notu yazıyorum.');
    // Yazma alanının üstündeki karttan, sayfa açmadan (masaüstü arayüzü §3).
    await k.dugme('Bir kez izin ver');
    assert.equal(await k.b.execute(() => document.querySelector('[role="dialog"]') !== null), false);
    assert.equal(await k.turSonuBekle(1), 'Normal');
    assert.equal(readFileSync(join(k.alan, 'notlar.txt'), 'utf8'), 'merhaba\n');
    await k.metniBekle('Araç sonucu:');
  });

  it('reddedilen değişikliğin gerekçesi modele gider, dosya yazılmaz', async () => {
    await k.gonder('yaz-ret');
    await k.dugme('İncele');
    const gerekce = await k.b.$('textarea[placeholder^="Reddetme nedeninizi"]');
    await gerekce.waitForDisplayed({ timeout: 15000 });
    await gerekce.setValue('başka bir ad kullan');
    await k.dugme('Reddet');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('[kullanıcı reddetti] başka bir ad kullan');
    assert.ok(!existsSync(join(k.alan, 'ret.txt')));
  });

  it('süren tur iptal edilir', async () => {
    await k.gonder('uzun');
    await k.metniBekle('[[e2e:3:');
    await k.ikonDugmesi('Turu durdur');
    assert.equal(await k.turSonuBekle(3), 'Iptal');
  });

  it('kapatılan oturum kayıttan açılır; geçmiş sırasıyla gelir', async () => {
    const oturum = await k.seciliOturum();
    await k.ikonDugmesi('Oturumu kapat');
    await (await k.composer()).waitForExist({ reverse: true, timeout: 10000 });

    const satir = await k.b.$('button.oturum-satiri');
    await satir.waitForClickable({ timeout: 10000 });
    await satir.click();
    await k.turSonuBekle(3);

    const kullanici = await k.b.execute(() =>
      [...document.querySelectorAll('article.mesaj.kullanici .duz')].map((e) => e.textContent),
    );
    assert.deepEqual(kullanici, ['yaz', 'yaz-ret', 'uzun']);
    assert.deepEqual(await k.turSonlari(), ['Normal', 'Normal', 'Iptal']);
    const bitenler = k
      .kayit(oturum)
      .filter((y) => y.tip === 'TurBitti')
      .map((y) => y.neden.tip);
    assert.deepEqual(bitenler, ['Normal', 'Normal', 'Iptal']);

    // Kayıttan açılan oturum yazılabilir: kayıt kilidi yeniden alındı.
    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(4), 'Normal');
  });
});
