// Tur özeti ve geri alma (aşama 3 tasarımı §5; 3a planı Task 11): Proje
// kipinde yazılan tur "Turu geri al…" ile geri alınır; sonradan değişen dosya
// çakışma olarak gösterilir ve ona dokunulmaz; kayıttan açılan oturumda da
// geri alma çalışır; oturumu silmek kaydı ve geri alma verisini götürür.
// Yerel pencereleri (kip yükseltme, oturum silme) sahte iletişim kutusu
// `onay: true` ile onaylar.
import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

const yaz = (yol, icerik) => ({ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol, icerik } });

const SENARYOLAR = {
  'iki dosya': {
    yanitlar: [
      [{ tur: 'metin', metin: 'Dosyaları yazıyorum.' }, yaz('a.txt', 'yeni\n'), yaz('yeni/b.txt', 'b\n')],
      [{ tur: 'metin', metin: 'İki dosya yazıldı.' }],
    ],
  },
  'cakisacak dosya': {
    yanitlar: [[yaz('c.txt', 'kalfa\n')], [{ tur: 'metin', metin: 'c yazıldı.' }]],
  },
  'kayittan dosya': {
    yanitlar: [[yaz('d.txt', 'kalfa\n')], [{ tur: 'metin', metin: 'd yazıldı.' }]],
  },
};

const oku = (k, yol) => readFileSync(join(k.alan, yol), 'utf8');

/**
 * Konuşmadaki son tur özetinin "Turu geri al…" düğmesine basar ve planın
 * gelip sayfanın açılmasını bekler. Her basış bir önizleme komutudur; sayfa
 * açılmadıysa en çok iki kez yeniden basılır.
 */
async function sonTurunuGeriAl(k) {
  for (let deneme = 0; deneme < 3; deneme++) {
    await k.b.execute(() => {
      const dugmeler = [
        ...document.querySelectorAll('section[aria-label="Tur özeti"] button'),
      ].filter((d) => d.textContent?.includes('Turu geri al'));
      dugmeler.at(-1)?.click();
    });
    try {
      await sureyleBekle(
        async () => k.b.execute(() => document.querySelector('[role="dialog"]') !== null),
        5000,
        'geri alma sayfası açılmadı',
      );
      return;
    } catch (hata) {
      if (deneme === 2) throw hata;
    }
  }
}

async function sayfaMetni(k) {
  return k.b.execute(() => document.querySelector('[role="dialog"]')?.textContent ?? '');
}

async function sayfaKapansin(k) {
  await sureyleBekle(
    async () => k.b.execute(() => document.querySelector('[role="dialog"]') === null),
    20000,
    'geri alma sayfası kapanmadı',
  );
}

describe('geri alma', () => {
  let k;
  let oturum;
  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    await k.pencere('ana');
    await k.yeniOturum();
    oturum = await k.seciliOturum();
    // Proje kipinde yazmalar onaysızdır; yükseltmenin yerel penceresini sahte
    // iletişim kutusu onaylar.
    assert.equal(await k.invoke('izin_kipi_degistir', { oturum, kip: 'Proje' }), 'Proje');
  });
  after(async () => {
    await k?.kapat();
  });

  it('Proje kipinde yazılan tur özetlenir ve geri alınır', async () => {
    writeFileSync(join(k.alan, 'a.txt'), 'eski\n');
    await k.gonder('iki dosya');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    assert.equal(oku(k, 'a.txt'), 'yeni\n');
    await k.metniBekle('a.txt +1 −1');
    await k.metniBekle('yeni/b.txt +1 −0 · yeni');

    await sonTurunuGeriAl(k);
    const metin = await sayfaMetni(k);
    assert.match(metin, /Önceki hâline döner/);
    assert.match(metin, /Silinir; bu turda oluşturulmuştu/);
    await k.dugme('Geri al');
    await sayfaKapansin(k);
    await k.metniBekle('Tur geri alındı: 2 dosya önceki hâline döndü.');

    assert.equal(oku(k, 'a.txt'), 'eski\n');
    assert.ok(!existsSync(join(k.alan, 'yeni', 'b.txt')));
    assert.ok(!existsSync(join(k.alan, 'yeni')), 'turun oluşturduğu boş dizin kalkar');
    const not = k.kayit(oturum).find((y) => y.tip === 'KalfaNotu');
    assert.match(not?.metin ?? '', /iki dosya/);
  });

  it('sonradan değişen dosya çakışmadır; ona dokunulmaz', async () => {
    await k.gonder('cakisacak dosya');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    writeFileSync(join(k.alan, 'c.txt'), 'kullanici\n');

    await sonTurunuGeriAl(k);
    const metin = await sayfaMetni(k);
    assert.match(metin, /Çakışma; dosyaya dokunulmaz/);
    assert.match(metin, /Dosya Kalfa'dan sonra değişti\./);
    assert.match(metin, /Geri alınacak bir değişiklik yok\./);
    const geriAlKapali = await k.b.execute(
      () =>
        [...document.querySelectorAll('[role="dialog"] button')].find(
          (d) => d.textContent?.trim() === 'Geri al',
        )?.disabled,
    );
    assert.equal(geriAlKapali, true);
    await k.dugme('Vazgeç');
    await sayfaKapansin(k);
    assert.equal(oku(k, 'c.txt'), 'kullanici\n');
  });

  it('kayıttan açılan oturumda geri alma çalışır; önceki geri almanın notu görünür', async () => {
    await k.gonder('kayittan dosya');
    assert.equal(await k.turSonuBekle(3), 'Normal');
    assert.equal(oku(k, 'd.txt'), 'kalfa\n');

    await k.ikonDugmesi('Oturumu kapat');
    await (await k.composer()).waitForExist({ reverse: true, timeout: 10000 });
    const satir = await k.b.$('button.oturum-satiri');
    await satir.waitForClickable({ timeout: 10000 });
    await satir.click();
    await k.turSonuBekle(3);
    await k.metniBekle('Modele bildirildi: [Kalfa] Kullanıcı');

    await sonTurunuGeriAl(k);
    assert.match(await sayfaMetni(k), /Silinir; bu turda oluşturulmuştu/);
    await k.dugme('Geri al');
    await sayfaKapansin(k);
    await k.metniBekle('Tur geri alındı: 1 dosya önceki hâline döndü.');
    assert.ok(!existsSync(join(k.alan, 'd.txt')));
  });

  it('oturumu silmek kaydı ve geri alma verisini siler', async () => {
    const kayit = join(k.veri, 'oturumlar', `${oturum}.jsonl`);
    const depo = join(k.veri, 'geri-alma', oturum);
    assert.ok(existsSync(kayit));
    assert.ok(existsSync(depo), 'önceki hâller oturumun deposunda');
    await k.ikonDugmesi('iki dosya oturumunu sil');
    await sureyleBekle(
      async () => !existsSync(kayit) && !existsSync(depo),
      20000,
      'oturum silinmedi',
    );
    assert.ok(!existsSync(`${kayit}.kilit`));
    await (await k.composer()).waitForExist({ reverse: true, timeout: 10000 });
  });
});
