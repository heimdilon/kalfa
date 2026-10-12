// Eklenti (aşama 4 tasarımı §16.5): yerleşik kalfa-rag Eklentiler ekranında
// görünür; çalışma alanında açılır; sahte model aramayı çağırır; ilk gönderim
// kartı yerel onayla geçer; sonuç kaynaklarıyla gelir ve günlükte
// "Eklenti · kalfa-rag" satırı vardır; önizleme açıkken ikinci arama kartta
// bekler ve "Metni göster" isteğin gövdesini gösterir. Sahte EVREN gömme ve
// sıralamayı belirlenimli yanıtlar; yerel pencereleri `onay: true` onaylar.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

const ARAMA = [
  {
    tur: 'arac',
    ad: 'kalfa_rag__anlamsal_ara',
    argumanlar: { sorgu: 'kabuk yalıtım seccomp', sonuc_sayisi: 3 },
  },
];
const SENARYOLAR = {
  ara: { yanitlar: [ARAMA, [{ tur: 'arac_sonucu' }]] },
  'yeniden ara': { yanitlar: [ARAMA, [{ tur: 'metin', metin: 'Yeniden aradım.' }]] },
};

/** Seçicideki ilk öğenin görünen metni (yoksa boş). */
function metin(k, secici) {
  return k.b.execute((s) => document.querySelector(s)?.textContent ?? '', secici);
}

/** Metni içeren ilk düğmeye basar (simgeli düğmelerde metin tam eşleşmez). */
function dugmeyeBas(k, parca) {
  return k.b.execute((p) => {
    const d = [...document.querySelectorAll('button')].find(
      (b) => !b.disabled && b.textContent?.includes(p),
    );
    d?.click();
    return d !== undefined;
  }, parca);
}

describe('eklenti', () => {
  let k;
  let oturum;

  before(async () => {
    k = await Kalfa.ac({
      sart: 'kabul',
      anahtar: GECERLI_ANAHTAR,
      onay: true,
      senaryolar: SENARYOLAR,
    });
    writeFileSync(
      join(k.alan, 'yalitim.md'),
      '# Yalıtım\n\nKabuk komutları yalıtımda çalışır: Linux için bubblewrap ve seccomp.\n',
    );
    writeFileSync(join(k.alan, 'kahve.md'), '# Kahve\n\nTürk kahvesi cezvede pişirilir.\n');
    await k.pencere('ana');
    await k.yeniOturum();
    oturum = await k.seciliOturum();
  });

  after(async () => {
    await k?.kapat();
  });

  it('Eklentiler ekranı yerleşik kalfa-rag\'ı gösterir', async () => {
    const liste = await k.invoke('eklentileri_listele');
    const rag = liste.find((e) => e.ad === 'kalfa-rag');
    assert.ok(rag, `kalfa-rag listede yok: ${JSON.stringify(liste)}`);
    assert.equal(rag.yerlesik, true);
    assert.equal(rag.durum.tip, 'Yuklendi', JSON.stringify(rag.durum));
    assert.deepEqual(
      rag.araclar.map((a) => a.ad),
      ['kalfa_rag__anlamsal_ara'],
    );

    assert.ok(await dugmeyeBas(k, 'Eklentiler'), 'menüde Eklentiler yok');
    await sureyleBekle(
      async () => (await metin(k, 'section.eklentiler')).includes('kalfa-rag'),
      10000,
      'Eklentiler ekranı kalfa-rag\'ı göstermedi',
    );
    assert.match(await metin(k, 'section.eklentiler'), /Yerleşik/);
    assert.ok(await dugmeyeBas(k, 'Oturumlar'));
    await (await k.composer()).waitForEnabled({ timeout: 15000 });
  });

  it('ilk gönderim yerel onayla geçer; sonuç kaynaklarıyla gelir, günlükte görünür', async () => {
    // Varsayılan olarak hiçbir çalışma alanında açık değildir (K1).
    let satir = (await k.invoke('oturum_eklentileri', { oturum })).find(
      (e) => e.ad === 'kalfa-rag',
    );
    assert.equal(satir?.acik, false);
    await k.invoke('eklenti_alani_ayarla', {
      ad: 'kalfa-rag',
      hedef: { tip: 'Oturum', oturum },
      acik: true,
    });

    await k.gonder('ara');
    await k.onaySayfasiniAc();
    const sayfa = await metin(k, '[role="dialog"]');
    assert.match(sayfa, /EVREN'e ilk gönderim: kalfa-rag/);
    assert.match(sayfa, /Neden izin gerekiyor/);
    assert.match(sayfa, /ilk kez EVREN'e veri gönderecek/);
    await k.dugme('Tümünü onayla');

    assert.equal(await k.turSonuBekle(1), 'Normal');
    // Sahte model aracın sonucunu yazar: en ilgili parça kaynağıyla gelir.
    await k.metniBekle('dosya:yalitim.md:');

    satir = (await k.invoke('oturum_eklentileri', { oturum })).find((e) => e.ad === 'kalfa-rag');
    assert.equal(satir?.acik, true);
    assert.ok(satir?.gonderim_izni_ms, 'onaylanan ilk gönderim izni kalıcı olmalı');

    // İsteğin metni kayda girmez (karar 8); sonuç araç sonucu olarak girer.
    const istekler = k
      .kayit(oturum)
      .map((o) => JSON.stringify(o))
      .filter((o) => o.includes('EklentiIstegi'));
    assert.ok(istekler.length > 0, 'eklenti istekleri kayda girmeli');
    for (const istek of istekler) {
      assert.ok(!istek.includes('bubblewrap'), `isteğin metni kayda girmemeli: ${istek}`);
    }

    await k.ikonDugmesi('Gönderim günlüğü');
    await sureyleBekle(
      async () => (await metin(k, 'div[role="dialog"]')).includes('Eklenti · kalfa-rag'),
      10000,
      'günlükte eklenti satırı yok',
    );
    await k.ikonDugmesi('Kapat');
    await sureyleBekle(
      async () => k.b.execute(() => document.querySelector('div[role="dialog"]') === null),
      5000,
      'gönderim günlüğü kapanmadı',
    );
  });

  it('önizleme açıkken ikinci arama kartta bekler; "Metni göster" gövdeyi gösterir', async () => {
    await k.invoke('onizleme_ayarla', { oturum, acik: true });
    await k.gonder('yeniden ara');

    // İzin artık var: ilk gönderim kartı yok, önizleme kartı gelir.
    await sureyleBekle(
      async () => (await k.konusma()).includes("Eklenti EVREN'e veri göndermek istiyor"),
      20000,
      'eklenti önizleme kartı gelmedi',
    );
    assert.ok(await dugmeyeBas(k, 'Metni göster'), '"Metni göster" yok');
    await sureyleBekle(
      async () => (await k.konusma()).includes('qwen3-embedding-8b'),
      10000,
      'isteğin gövdesi gösterilmedi',
    );

    assert.ok(await dugmeyeBas(k, 'Bu araç çağrısında hep gönder'));
    // Aracın sonucu da modele gitmeden önce gösterilir.
    await sureyleBekle(
      async () =>
        k.b.execute(() =>
          [...document.querySelectorAll('button')].some(
            (b) => !b.disabled && b.textContent?.trim() === 'Gönder',
          ),
        ),
      20000,
      'sonucun önizlemesi gelmedi',
    );
    await k.dugme('Gönder');
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('Yeniden aradım.');
  });
});
