// Gönderim günlüğü ve göndermeden önce gösterme (aşama 3 tasarımı §6, §11.5;
// 3b planı Task 9): Günlük satırları açılır ve denetim/istek bilgisi
// incelenir; önizleme açıkken araç sonucu göndermeden önce gösterilir ve
// çıkarılan öğe yer tutucuyla gider.
import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, sureyleBekle } from '../yardimci/kalfa.mjs';

const SENARYOLAR = {
  selam: {
    yanitlar: [[{ tur: 'metin', metin: 'Merhaba, Kalfa hazır.' }]],
  },
  'rapor oku': {
    yanitlar: [
      [
        { tur: 'arac', ad: 'dosya_oku', argumanlar: { yol: 'rapor.txt' } },
        { tur: 'arac', ad: 'dosya_oku', argumanlar: { yol: 'kamu.txt' } },
      ],
      [{ tur: 'metin', metin: 'Raporları okudum.' }],
    ],
  },
};

describe('gönderim günlüğü ve önizleme', () => {
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
  });

  after(async () => {
    await k?.kapat();
  });

  it('gönderim günlüğü tablosu ve ayrıntı paneli açılır', async () => {
    await k.gonder('selam');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    await k.metniBekle('Merhaba, Kalfa hazır.');

    // Başlıktaki "Gönderim günlüğü" butonuna basılır
    await k.ikonDugmesi('Gönderim günlüğü');
    await sureyleBekle(
      async () => k.b.execute(() => document.querySelector('div[role="dialog"]') !== null),
      5000,
      'gönderim günlüğü iletişim kutusu açılmadı',
    );

    // Günlük satırlarını denetle
    const satirSayisi = await k.b.execute(
      () => document.querySelectorAll('table.gunluk-tablosu tbody tr').length,
    );
    assert.ok(satirSayisi >= 1, 'en az bir istek satırı olmalı');

    const dialogMetni = await k.b.execute(
      () => document.querySelector('div[role="dialog"]')?.textContent ?? '',
    );
    assert.match(dialogMetni, /glm-5\.3/);
    assert.match(dialogMetni, /Başarılı/);

    // İlk satıra tıklayıp ayrıntıları gör
    await k.b.execute(() => {
      const ilkSatir = document.querySelector('table.gunluk-tablosu tbody tr');
      ilkSatir?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });

    // İletişim kutusunu kapat
    await k.ikonDugmesi('Kapat');
    await sureyleBekle(
      async () => k.b.execute(() => document.querySelector('div[role="dialog"]') === null),
      5000,
      'gönderim günlüğü kapanmadı',
    );
  });

  it('önizlemede bir öğe çıkarılır ve modele yer tutucu gider', async () => {
    writeFileSync(join(k.alan, 'rapor.txt'), 'gizli-veri-e2e-8f2a\n');
    writeFileSync(join(k.alan, 'kamu.txt'), 'kamu-veri-e2e-4b1c\n');

    // Göndermeden önce göster (önizleme) açılır
    await k.invoke('onizleme_ayarla', { oturum, acik: true });

    await k.gonder('rapor oku');

    // Önizleme kartının gelmesini bekle
    await sureyleBekle(
      async () =>
        k.b.execute(
          () =>
            [...document.querySelectorAll('.kart')].some((kart) =>
              kart.textContent?.includes('Gönderim önizlemesi'),
            ),
        ),
      15000,
      'önizleme kartı gelmedi',
    );

    // 'rapor.txt' öğesinin onay kutusunu işaretle
    await k.b.execute(() => {
      const satirlar = [...document.querySelectorAll('.oge-satiri')];
      const raporSatiri = satirlar.find((s) => s.textContent?.includes('rapor.txt'));
      const cb = raporSatiri?.querySelector('input[type="checkbox"]');
      if (cb && !cb.checked) cb.click();
    });

    // İşaretli öğe varken tek düğme işaretlileri çıkarıp gönderir (3c Görev 8)
    await k.dugme('İşaretlileri çıkarıp gönder');

    // Turun bitmesini bekle
    assert.equal(await k.turSonuBekle(2), 'Normal');
    await k.metniBekle('Raporları okudum.');

    // Önizleme kartında Yanıtlandı rozetini bekle
    await k.metniBekle('Yanıtlandı');

    // Kayıt denetimi: çıkarılan içerik kayıtta ve istekte yok, yer tutucu var
    const kayit = k.kayit(oturum);
    const kayitMetni = JSON.stringify(kayit);
    assert.ok(!kayitMetni.includes('gizli-veri-e2e-8f2a'), 'çıkarılan içerik kayda girmemeli');
    assert.ok(kayitMetni.includes('[kullanıcı bu içeriği göndermedi]'), 'yer tutucu kayıtta olmalı');
    assert.ok(kayitMetni.includes('kamu-veri-e2e-4b1c'), 'gönderilen içerik kayıtta olmalı');
  });
});
