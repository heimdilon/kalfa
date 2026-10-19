// Compact ve bütçesiz oturum (plan Görev 7 e2e): `/compact` yazılır → bölücü
// görünür → sonraki mesaj gönderilir. Sahte arka uç özetleyiciyi yanıtlar.
import assert from 'node:assert/strict';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa } from '../yardimci/kalfa.mjs';

describe('compact: elle özetleme ve sonraki tur', () => {
  let k;
  let oturum;

  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
    });
    await k.pencere('ana');
    await k.yeniOturum();
    oturum = await k.seciliOturum();
  });

  after(async () => {
    await k?.kapat();
  });

  it('/compact yazılır, bölücü görünür ve sonraki mesaj gönderilir', async () => {
    await k.gonder('selam');
    await k.metniBekle('Yanıt: selam');
    assert.equal(await k.turSonuBekle(1), 'Normal');

    await k.gonder('/compact odak: testler');
    await k.metniBekle('Konuşma özetlendi');
    await k.metniBekle('Talimat: odak: testler');
    assert.equal(await k.turSonuBekle(2), 'Normal');

    await k.gonder('devam');
    await k.metniBekle('Yanıt: devam');
    assert.equal(await k.turSonuBekle(3), 'Normal');

    const gunluk = await k.invoke('gonderim_gunlugu', { oturum, once_sira: null });
    assert.equal(gunluk.satirlar.length, 3, 'ana, özetleme ve sonraki ana istek günlükte');
    assert.equal(gunluk.satirlar[1].amac, 'özetleme');
  });
});
