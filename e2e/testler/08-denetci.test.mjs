// Güvenlik denetçisi, katı kip (aşama 3 tasarımı §7, §11.5; 3c planı Görev 9):
// denetçinin şüpheli bulduğu araç sonucu kartsız modele gitmez. Kart öğeyi
// gönderilmemek üzere işaretli getirir; kullanıcı öyle gönderirse içerik
// hiçbir isteğe ve kayda girmez, araç kartında "Şüpheli" rozeti görünür.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

// Sahte denetçi bu işareti taşıyan içeriği şüpheli bulur (e2e/saglayici.rs).
const ISARET = 'e2e-supheli';
const ICERIK = `${ISARET} gizli-olmayan-ama-telkinli-7c4d`;

const SENARYOLAR = {
  'notu oku': {
    yanitlar: [
      [{ tur: 'arac', ad: 'dosya_oku', argumanlar: { yol: 'not.txt' } }],
      [{ tur: 'metin', metin: 'Notu okudum.' }],
    ],
  },
};

describe('katı denetçi', () => {
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
    assert.equal(await k.invoke('izin_kipi_degistir', { oturum, kip: 'Proje' }), 'Proje');
    // Kipi yükseltmek onay istemez.
    const durum = await k.invoke('denetci_kipini_ayarla', { kip: 'Kati' });
    assert.equal(durum.kip, 'Kati');
  });

  after(async () => {
    await k?.kapat();
  });

  it('şüpheli sonuç kartta işaretli gelir ve gönderilmez', async () => {
    writeFileSync(join(k.alan, 'not.txt'), `${ICERIK}\n`);
    await k.gonder('notu oku');

    await sureyleBekle(
      async () =>
        k.b.execute(() =>
          [...document.querySelectorAll('.kart')].some((kart) =>
            kart.textContent?.includes('Güvenlik denetimi'),
          ),
        ),
      15000,
      'katı denetimin kartı gelmedi',
    );
    const isaretli = await k.b.execute(() => {
      const kutu = document.querySelector('.oge-satiri input[type="checkbox"]');
      return kutu?.checked ?? null;
    });
    assert.equal(isaretli, true, 'şüpheli öğe gönderilmemek üzere işaretli gelmeli');
    await k.metniBekle('Şüpheli: talimat enjeksiyonu');

    await k.dugme('İşaretlileri çıkarıp gönder');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('Notu okudum.');

    const kayitMetni = JSON.stringify(k.kayit(oturum));
    assert.ok(!kayitMetni.includes('telkinli-7c4d'), 'gönderilmeyen içerik kayda girmemeli');
    assert.ok(kayitMetni.includes('[kullanıcı bu içeriği göndermedi]'));
    assert.ok(kayitMetni.includes('DenetimKarari'), 'kullanıcının kararı kayda yazılmalı');
  });
});
