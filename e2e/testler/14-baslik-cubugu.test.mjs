// Kalfa'ya özel başlık çubuğu (masaüstü arayüzü tasarımı §5.4): Windows'ta
// ana pencere çerçevesizdir; küçült, büyüt/geri yükle ve kapat düğmeleri
// pencerenin kendi komutlarını çağırır. Kapat, bugünkü kapanış yolundan geçer
// ve süreç kapanır. Sürükleme elle denenir.
import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, surecler, sureyleBekle } from '../yardimci/kalfa.mjs';

const AYARLAR = { sart: 'kabul', anahtar: GECERLI_ANAHTAR };

/** `[simge durumunda mı, büyük mü]`. */
const durum = (k) => k.invoke('e2e_pencere_durumu');

async function hal(k, beklenen, mesaj) {
  await sureyleBekle(
    async () => JSON.stringify(await durum(k)) === JSON.stringify(beklenen),
    10000,
    `${mesaj}: ${JSON.stringify(await durum(k))}`,
  );
}

describe('başlık çubuğu', { skip: process.platform !== 'win32' }, () => {
  it('küçült, büyüt ve geri yükle pencereye uygulanır; kapat süreci kapatır', async () => {
    const k = await Kalfa.ac(AYARLAR);
    try {
      await k.pencere('ana');
      await k.yeniOturum();
      await k.gonder('merhaba');
      await k.metniBekle('Yanıt: merhaba');
      assert.deepEqual(await durum(k), [false, false]);

      await k.ikonDugmesi('Büyüt');
      await hal(k, [false, true], 'pencere büyümedi');
      await k.ikonDugmesi('Önceki boyutuna getir');
      await hal(k, [false, false], 'pencere geri yüklenmedi');

      await k.ikonDugmesi('Simge durumuna küçült');
      await hal(k, [true, false], 'pencere simge durumuna küçülmedi');
      // Simge durumundaki pencerenin düğmesine tıklanamaz; komutla açılır.
      assert.equal(await k.invoke('pencere_buyut_degistir'), true);
      await hal(k, [false, true], 'pencere simge durumundan açılmadı');

      const d = await k.b.$('button[aria-label="Pencereyi kapat"]');
      await d.waitForClickable({ timeout: 15000 });
      await d.click().catch((e) => {
        if (!/no such window|invalid session/.test(String(e))) throw e;
      });
      await sureyleBekle(() => surecler().length === 0, 15000, 'Kalfa kapanmadı');
    } finally {
      await k.kapat();
    }
  });
});
