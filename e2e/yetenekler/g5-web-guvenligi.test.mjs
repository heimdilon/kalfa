// Gerçek üretim web aracı özel IP'yi ağ bağlantısından önce reddetmeli.
import assert from 'node:assert/strict';
import { createServer } from 'node:net';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

describe('G5 gerçek web aracı masaüstü güvenliği', () => {
  let k, sunucu, baglanti = 0;
  before(async () => {
    sunucu = createServer(s => { baglanti++; s.destroy(); });
    await new Promise((coz, red) => { sunucu.once('error', red); sunucu.listen(0, '127.0.0.1', coz); });
    const url = `https://127.0.0.1:${sunucu.address().port}/gizli`;
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR,
      senaryolar: { web: { yanitlar: [
        [{ tur: 'arac', ad: 'web_getir', argumanlar: { url } }],
        [{ tur: 'arac_sonucu' }],
      ] } } });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => { await k?.kapat(); if (sunucu) await new Promise(coz => sunucu.close(coz)); });
  it('özel IP reddi gerçek model sonucunda ve JSONL’de görünür; TCP bağlantısı açılmaz', async () => {
    await k.gonder('web');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('Araç sonucu:');
    assert.match(await k.konusma(), /yasaklı|loopback/i);
    assert.equal(baglanti, 0, 'Özel IP sunucusuna bağlantı ulaşmamalı');
    const kayit = k.kayit(await k.seciliOturum());
    assert.ok(kayit.some(o => JSON.stringify(o).includes('web_getir')));
    assert.ok(kayit.some(o => /yasaklı|loopback/i.test(JSON.stringify(o))));
  });
});
