// Gerçek Tauri IPC'si: plan görünümü ve soru kartından kullanıcı yanıtı.
// Sahte sağlayıcı yalnız model yanıtını belirler; araçlar ve UI gerçektir.
import assert from 'node:assert/strict';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  'g5-plan': {
    yanitlar: [
      [{ tur: 'arac', ad: 'plan_guncelle', argumanlar: { adimlar: [
        { kimlik: 'hazirlik', baslik: 'Kaynağı okuyun', durum: 'Tamam' },
        { kimlik: 'kabul', baslik: 'Geçişi doğrulayın', durum: 'Suruyor' },
      ] } }],
      [{ tur: 'metin', metin: 'Kabul planı güncellendi.' }],
    ],
  },
  'g5-soru': {
    yanitlar: [
      [{ tur: 'arac', ad: 'kullaniciya_sor', argumanlar: {
        baslik: 'Hangi yapılandırmayı kullanalım?',
        secenekler: [{ kimlik: 'toml', etiket: 'TOML' }, { kimlik: 'json', etiket: 'JSON' }],
      } }],
      [{ tur: 'arac_sonucu' }],
    ],
  },
};

describe('G5 gerçek proje akışı', () => {
  let k;
  before(async () => {
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, senaryolar: SENARYOLAR });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => { await k?.kapat(); });

  it('plan_guncelle IPC olayı gerçek plan adımlarını gösterir', async () => {
    await k.gonder('g5-plan');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    const liste = await k.b.$('ol.adimlar');
    await liste.waitForDisplayed({ timeout: 15000 });
    const planMetni = await liste.getText();
    assert.match(planMetni, /Geçişi doğrulayın/);
    assert.match(planMetni, /Kaynağı okuyun/);
    const kayit = k.kayit(await k.seciliOturum());
    const guncelleme = kayit.findLast((oge) => oge.tip === 'PlanGuncellendi');
    assert.ok(guncelleme, 'Plan JSONL kaydında bulunmalı');
    assert.deepEqual(guncelleme.plan.adimlar.map((adim) => adim.kimlik), ['hazirlik', 'kabul']);
  });

  it('soru kartındaki seçenek doğru tura bağlanıp araç sonucuna döner', async () => {
    await k.gonder('g5-soru');
    await k.dugme('TOML');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('Araç sonucu:');
    assert.match(await k.konusma(), /toml/i);
    const kayit = k.kayit(await k.seciliOturum());
    const soru = kayit.findLast((oge) => oge.tip === 'KullaniciSorusuSoruldu');
    const yanit = kayit.findLast((oge) => oge.tip === 'KullaniciSorusuCozuldu');
    assert.ok(soru && yanit, 'Soru ve yanıtı JSONL kaydında bulunmalı');
    assert.equal(yanit.soru_kimligi, soru.soru.soru_kimligi);
    assert.equal(yanit.sonuc.durum, 'Yanitlandi');
    assert.deepEqual(yanit.sonuc.yanit, { tur: 'Secenek', secenek_kimligi: 'toml' });
  });
});
