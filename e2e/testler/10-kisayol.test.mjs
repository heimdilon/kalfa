// "Bu oturumda izin ver" kısayolları ve gizli sayılmayanlar (aşama 3
// tasarımı §9.1, §9.2, §11.5; 3e planı Görev 6). Sıkı kipte bir yazma
// kısayolla onaylanır, aynı dizindeki sonraki yazma onaysız geçer; kip
// değişince kısayol kalkar. Gizli sayılmayanlara eklenen dosya onaysız okunur.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  'ilk yazma': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'src/a.txt', icerik: 'a\n' } }],
      [{ tur: 'metin', metin: 'a yazıldı.' }],
    ],
  },
  'ikinci yazma': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'src/b.txt', icerik: 'b\n' } }],
      [{ tur: 'metin', metin: 'b yazıldı.' }],
    ],
  },
  'üçüncü yazma': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_yaz', argumanlar: { yol: 'src/c.txt', icerik: 'c\n' } }],
      [{ tur: 'metin', metin: 'c yazıldı.' }],
    ],
  },
  'örneği oku': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_oku', argumanlar: { yol: '.env.example' } }],
      [{ tur: 'metin', metin: 'Örneği okudum.' }],
    ],
  },
};

/** Konuşmanın üstündeki "Bu oturumdaki izinler" panelinin başlığı; yoksa boş. */
async function izinler(k) {
  return k.b.execute(() => document.querySelector('details.izinler summary')?.textContent?.trim() ?? '');
}

/** Onay kartı açık mı? */
async function onayKartiVar(k) {
  return k.b.execute(() =>
    [...document.querySelectorAll('button')].some((b) => b.textContent?.trim() === 'İncele'),
  );
}

describe('kısayollar ve gizli sayılmayanlar', () => {
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
    assert.equal(await k.invoke('izin_kipi_degistir', { oturum, kip: 'Siki' }), 'Siki');
  });

  after(async () => {
    await k?.kapat();
  });

  it('kısayolla onaylanan yazmadan sonra aynı dizindeki yazma onaysız; kip değişince kalkar', async () => {
    await k.gonder('ilk yazma');
    await k.onaySayfasiniAc();
    const etiket = 'Bu oturumda src/ altındaki dosyaları dosya_yaz ile onaysız düzenleyebilir.';
    await sureyleBekle(
      async () => (await k.b.$(`label*=${etiket}`)).isExisting(),
      5000,
      'kapsam satırı gelmedi',
    );
    await (await k.b.$(`label*=${etiket}`)).click();
    await k.dugme('Seçilenleri onayla');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await sureyleBekle(
      async () => (await izinler(k)) === 'Bu oturumdaki izinler (1)',
      10000,
      'kısayol listesi gelmedi',
    );

    // Aynı dizinde ikinci yazma kart açmadan yapılır.
    await k.gonder('ikinci yazma');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    assert.equal(readFileSync(join(k.alan, 'src/b.txt'), 'utf8'), 'b\n');

    // Kip değişince kısayol silinir; sonraki yazma yine onay ister.
    assert.equal(await k.invoke('izin_kipi_degistir', { oturum, kip: 'Proje' }), 'Proje');
    assert.equal(await k.invoke('izin_kipi_degistir', { oturum, kip: 'Siki' }), 'Siki');
    await sureyleBekle(async () => (await izinler(k)) === '', 5000, 'kısayol listesi kalkmadı');
    await k.gonder('üçüncü yazma');
    await sureyleBekle(() => onayKartiVar(k), 15000, 'üçüncü yazmada onay sorulmadı');
    await k.onaySayfasiniAc();
    await k.dugme('Tümünü onayla');
    assert.equal(await k.turSonuBekle(3), 'Normal');
  });

  it('gizli sayılmayanlara eklenen dosya onaysız okunur', async () => {
    writeFileSync(join(k.alan, '.env.example'), 'ORNEK=1\n');
    // Eklemek yerel pencerede onaylanır; e2e'nin sahte penceresi onaylar.
    const g = await k.invoke('gizli_sayilmayan_ekle', { kalip: '.env.example' });
    assert.deepEqual(g.gizli_sayilmayanlar, ['.env.example']);

    await k.gonder('örneği oku');
    assert.equal(await k.turSonuBekle(4), 'Normal');
    await k.metniBekle('Örneği okudum.');
    const kayitMetni = JSON.stringify(k.kayit(oturum));
    assert.ok(kayitMetni.includes('ORNEK=1'), 'dosya onaysız okunup modele gitmeli');
  });
});
