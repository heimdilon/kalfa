// Sağ panel (masaüstü arayüzü tasarımı §6.4): Dosyalar'dan proje dosyası
// salt okur açılır; gizli dosyanın içeriği gösterilmez; Ortam git dalını
// gösterir. Arayüz yol göndermez: kimlik başka oturumda geçersizdir.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

function proje(alan) {
  writeFileSync(join(alan, 'gorevler.py'), 'def tamamla(gorevler, numara):\n    pass\n');
  writeFileSync(join(alan, '.env'), 'EVREN_API_KEY=e2e-gizli-deger\n');
  mkdirSync(join(alan, '.git'));
  writeFileSync(join(alan, '.git', 'HEAD'), 'ref: refs/heads/deneme\n');
  return { sart: 'kabul', anahtar: GECERLI_ANAHTAR };
}

/** Paneldeki önizleme alanının metni. */
const onizleme = (k) =>
  k.b.execute(() => document.querySelector('aside.sag-bolme .onizleme')?.textContent ?? '');

async function paneldeDugme(k, metin) {
  const d = await k.b.$('aside.sag-bolme').$(`button=${metin}`);
  await d.waitForClickable({ timeout: 15000 });
  await d.click();
}

describe('sağ panel', () => {
  it('Dosyalar proje dosyasını açar, .env gizli kalır; Ortam git dalını gösterir', async () => {
    const k = await Kalfa.ac(proje);
    try {
      await k.pencere('ana');
      await k.yeniOturum();

      await k.ikonDugmesi('Dosyalar');
      await paneldeDugme(k, 'gorevler.py');
      await sureyleBekle(
        async () => (await onizleme(k)).includes('def tamamla(gorevler, numara):'),
        10000,
        'gorevler.py önizlenmedi',
      );

      await paneldeDugme(k, '.env');
      await sureyleBekle(
        async () => (await onizleme(k)).includes('Gizli dosya; içerik gösterilmez.'),
        10000,
        '.env gizli gösterilmedi',
      );
      const sayfa = await k.b.execute(() => document.documentElement.outerHTML);
      assert.ok(!sayfa.includes('e2e-gizli-deger'), 'gizli içerik sayfaya girdi');
      // Dosya listesinde .git görünmez.
      const adlar = await k.b.execute(() =>
        [...document.querySelectorAll('aside.sag-bolme [role="tree"] li')].map((l) => l.textContent.trim()),
      );
      assert.ok(!adlar.some((a) => a.startsWith('.git')), adlar.join(', '));

      await k.ikonDugmesi('Ortam');
      await sureyleBekle(
        async () =>
          k.b.execute(() => document.querySelector('aside.sag-bolme')?.textContent?.includes('deneme')),
        10000,
        'git dalı görünmedi',
      );

      // Arayüz yol gönderemez: komut yalnız kimlik alır, bilinmeyen kimlik reddedilir.
      const oturum = await k.seciliOturum();
      await assert.rejects(k.invoke('calisma_alani_dosyasi', { oturum, kimlik: 999999 }), /bilinmiyor/);
    } finally {
      await k.kapat();
    }
  });
});
