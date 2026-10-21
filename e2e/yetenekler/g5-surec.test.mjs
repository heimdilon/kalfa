// Gerçek Tauri + PTY/ConPTY: kullanıcı girdisi UI denetiminden geçer.
import assert from 'node:assert/strict';
import { after, before, describe, it } from 'node:test';
import { GECERLI_ANAHTAR, Kalfa, kalfaProfilSayisi, sureyleBekle } from '../yardimci/kalfa.mjs';

if (process.platform === 'win32' &&
    (process.env.GITHUB_ACTIONS !== 'true' || process.env.KALFA_G5_MASAUSTU_CI !== '1')) {
  throw new Error('Windows gerçek süreç kabulü yalnız geçici kurucu CI makinesinde çalıştırılır.');
}
const betik = process.platform === 'win32'
  ? "[Console]::WriteLine('G5-UI-GIRDI-BEKLIYOR'); $s=[Console]::ReadLine(); if($s -eq 'dogrula'){[Console]::WriteLine('G5-UI-ONAYLANDI'); exit 0}; exit 7"
  : 'echo G5-UI-GIRDI-BEKLIYOR; IFS= read -r satir; if [ "$satir" = dogrula ]; then echo G5-UI-ONAYLANDI; exit 0; fi; exit 7';

describe('G5 gerçek terminal kartı', () => {
  let k, profiller;
  before(async () => {
    profiller = kalfaProfilSayisi();
    k = await Kalfa.ac({ sart: 'kabul', anahtar: GECERLI_ANAHTAR, onay: true,
      senaryolar: { terminal: { yanitlar: [
        [{ tur: 'arac', ad: 'kabuk', argumanlar: { betik, arka_plan: true, terminal: true } }],
        [{ tur: 'metin', metin: 'Terminal kullanıcı girdisini bekliyor.' }],
      ] } } });
    await k.pencere('ana');
    await k.yeniOturum();
  });
  after(async () => {
    let temiz = false;
    try {
      await k?.hazirlananAlanlariKaldir();
      assert.equal(kalfaProfilSayisi(), profiller, 'Çalışma alanı profili geri alınmalı');
      temiz = true;
    } finally { await k?.kapat({ veriyiKoru: !temiz }); }
  });

  it('Girdi gönder düğmesi gerçek terminal satırını tamamlar ve doğal çıkış0 gösterir', async () => {
    await k.gonder('terminal');
    await k.dugme('Bir kez izin ver');
    assert.equal(await k.turSonuBekle(1), 'Normal');
    const oturum = await k.seciliOturum();
    const isler = await k.invoke('oturum_isleri', { oturum });
    assert.equal(isler.length, 1);
    const is_id = isler[0].is_id;
    await sureyleBekle(async () => {
      const o = await k.invoke('is_oku', { oturum, is_id, imlec: 0 });
      return JSON.stringify(o).includes('G5-UI-GIRDI-BEKLIYOR');
    }, 30000, 'Gerçek terminal girdi beklemeye başlamadı');
    const girdi = await k.b.$('input[aria-label="Standart girdi metni"]');
    await girdi.waitForDisplayed({ timeout: 15000 });
    await girdi.setValue('dogrula');
    await k.dugme('Girdi gönder');
    await sureyleBekle(async () => {
      const o = await k.invoke('is_oku', { oturum, is_id, imlec: 0 });
      return JSON.stringify(o).includes('G5-UI-ONAYLANDI');
    }, 30000, 'UI girdisi gerçek terminal satırını tamamlamadı');
    await sureyleBekle(async () => {
      const [is] = await k.invoke('oturum_isleri', { oturum });
      return is.durum.durum === 'bitti' && is.durum.cikis_kodu === 0;
    }, 15000, 'Terminal doğal çıkış0 ile bitmedi');
    await sureyleBekle(async () => (await k.b.$('body').getText()).includes('Tamamlandı (0)'),
      15000, 'Gerçek süreç sonucu terminal kartında görünmedi');
  });
});
