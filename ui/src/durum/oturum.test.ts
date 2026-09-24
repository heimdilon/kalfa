import { describe, expect, it, vi } from 'vitest';
import type { KanalMesaji, Olay, OturumAcilisi, Zarf } from '../ipc/tipler';
import { kanalIsleyicisi } from './dinleyici';
import { acilistanEkran, uygula, type EkranOgesi, type OturumEkrani } from './oturum';

const OTURUM = 'o-1';
const TUR = 't-1';

function acilis(gecmis: OturumAcilisi['gecmis'] = { ogeler: [], daha_eski_var: false }) {
  return acilistanEkran({
    oturum: OTURUM,
    calisma_alani: 'C:\\proje',
    gecmis,
    ozet: {
      model: 'glm-5.3',
      kip: 'Siki',
      yonlendirilen_model: null,
      son_kredi: null,
      istem: 0,
      tamamlama: 0,
      dusunce: 0,
    },
    salt_okunur: null,
  });
}

let sira = 100;
function zarf(yuk: Olay, tur: string | null = TUR): Zarf<Olay> {
  sira += 1;
  return { sema: 1, oturum: OTURUM, tur, sira, yuk };
}

function paket(no: number, ...yukler: Olay[]): KanalMesaji {
  return { tip: 'Olaylar', no, zarflar: yukler.map((y) => zarf(y)) };
}

function onayGerekiyor(islem: string, cagri: string): Extract<Olay, { tip: 'OnayGerekiyor' }> {
  return {
    tip: 'OnayGerekiyor',
    islem,
    ogeler: [
      {
        cagri,
        arac: 'dosya_yaz',
        aciklama: 'a.rs yazılır',
        hedefler: ['a.rs'],
        diff: null,
        uyarilar: [],
        muhur: 'm',
        kabuk: null,
        yerel_onay: false,
      },
    ],
  };
}

function tipler(d: OturumEkrani): EkranOgesi['tip'][] {
  return d.ogeler.map((o) => o.tip);
}

describe('kanal mesajı → oturum durumu (1c tasarımı §8.4)', () => {
  it('mesaj, tur ve bitişik metin parçaları tek yanıtta birleşir', () => {
    let d = uygula(acilis(), { tip: 'MesajGonderildi', metin: 'selam' });
    expect(d.anlik).toBe('Calisiyor');
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'TokenParcasi', metin: 'Mer' },
        { tip: 'TokenParcasi', metin: 'haba' },
      ),
    });
    expect(tipler(d)).toEqual(['KullaniciMesaji', 'Yanit']);
    expect(d.ogeler[0]).toMatchObject({ metin: 'selam', tur: TUR });
    expect(d.ogeler[1]).toMatchObject({ metin: 'Merhaba', akista: true });
    expect(d.aktif_tur).toBe(TUR);
    expect(d.beklenen_no).toBe(2);
  });

  it('düşünce ve metin aynı yanıtta ayrı tutulur; araç çağrısı akışı keser', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'DusunceParcasi', metin: 'düşün' },
        { tip: 'TokenParcasi', metin: 'yaz' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-1', arac: 'dosya_oku', aciklama: 'a.rs' },
        { tip: 'TokenParcasi', metin: 'sonra' },
      ),
    });
    expect(tipler(d)).toEqual(['Yanit', 'AracCagrisi', 'Yanit']);
    expect(d.ogeler[0]).toMatchObject({ dusunce: 'düşün', metin: 'yaz', akista: false });
    expect(d.ogeler[1]).toMatchObject({ cagri: 'c-1', durum: 'Suruyor' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, {
        tip: 'AracSonucu',
        cagri: 'c-1',
        arac: 'dosya_oku',
        durum: 'Basarili',
        icerik: 'içerik',
      }),
    });
    expect(d.ogeler[1]).toMatchObject({ durum: 'Basarili', sonuc: 'içerik' });
  });

  it('kabuk çıktısı kendi çağrısının terminaline işlenir (aşama 2 tasarımı §10.1)', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-1', arac: 'kabuk', aciklama: 'a' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-2', arac: 'kabuk', aciklama: 'b' },
        { tip: 'AracCiktisiParcasi', cagri: 'c-1', akis: 'Standart', metin: 'bir\n', atlanan_bayt: 0 },
        { tip: 'AracCiktisiParcasi', cagri: 'c-2', akis: 'Hata', metin: 'iki\n', atlanan_bayt: 5 },
        { tip: 'AracCiktisiParcasi', cagri: 'yok', akis: 'Standart', metin: 'x', atlanan_bayt: 0 },
      ),
    });
    const [a, b] = d.ogeler.filter((o) => o.tip === 'AracCagrisi');
    expect(a.tip === 'AracCagrisi' && a.terminal?.satirlar).toEqual([
      { akis: 'Standart', metin: 'bir' },
    ]);
    expect(b.tip === 'AracCagrisi' && b.terminal?.satirlar).toEqual([
      { akis: 'Bilgi', metin: '… 5 bayt gösterilmedi …' },
      { akis: 'Hata', metin: 'iki' },
    ]);
  });

  it('onay isteği kart olur; yanıtlanınca çalışmaya döner', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        {
          tip: 'OnayGerekiyor',
          islem: 'i-1',
          ogeler: [
            {
              cagri: 'c-1',
              arac: 'dosya_yaz',
              aciklama: 'a.rs yazılır',
              hedefler: ['a.rs'],
              diff: null,
              uyarilar: [],
              muhur: 'm',
              kabuk: null,
              yerel_onay: false,
            },
          ],
        },
      ),
    });
    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-1', durum: 'Bekliyor' });
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler.at(-1)).toMatchObject({ durum: 'Yanitlandi' });
  });

  it('önceki onayın gecikmiş yanıtı yeni bekleyen isteği temizlemez', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, onayGerekiyor('i-1', 'c-1')),
    });
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, onayGerekiyor('i-2', 'c-2')),
    });
    expect(d.anlik).toBe('OnayBekliyor');

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });

    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-2', durum: 'Bekliyor' });
  });

  it('eski onay kartı yanıtlanırken yeni bekleyen istek durumu korur', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, onayGerekiyor('i-1', 'c-1')),
    });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, onayGerekiyor('i-2', 'c-2')),
    });

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1' });

    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.find((o) => o.tip === 'OnayIstegi' && o.islem === 'i-1')).toMatchObject({
      durum: 'Yanitlandi',
    });
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'OnayIstegi', islem: 'i-2', durum: 'Bekliyor' });
  });

  it('tur sonu akışı dondurur, bekleyen onayı geçersiz kılar', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'TokenParcasi', metin: 'x' },
        { tip: 'OnayGerekiyor', islem: 'i-1', ogeler: [] },
        { tip: 'Tamamlandi', neden: { tip: 'Iptal' } },
      ),
    });
    expect(tipler(d)).toEqual(['Yanit', 'OnayIstegi', 'TurSonu']);
    expect(d.ogeler[0]).toMatchObject({ akista: false });
    expect(d.ogeler[1]).toMatchObject({ durum: 'Gecersiz' });
    expect(d.anlik).toBe('Bos');
    expect(d.aktif_tur).toBeNull();
  });

  it('şart beklerken gelen olay beklemeyi çözer', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'SartOnayiGerekiyor', surum: 3, metin: '…' }),
    });
    expect(d.anlik).toBe('SartBekliyor');
    d = uygula(d, { tip: 'Kanal', mesaj: paket(2, { tip: 'TokenParcasi', metin: 'devam' }) });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler.find((o) => o.tip === 'SartIstegi')).toMatchObject({ durum: 'Cozuldu' });
  });

  it('kullanım, kredi ve yönlendirilen model durum özetine işlenir', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'IstekBilgisi', istek_id: 'r-1', yonlendirilen_model: 'deepseek-v4-flash' },
        { tip: 'TokenKullanimi', istem: 10, tamamlama: 5, dusunce: 2 },
        { tip: 'TokenKullanimi', istem: 1, tamamlama: 1, dusunce: 0 },
        {
          tip: 'KrediGozlemi',
          gozlem: { kalan: '12.5', tutulan: null, istek_id: 'r-1', zaman_ms: 1 },
        },
      ),
    });
    expect(d.ozet).toMatchObject({
      yonlendirilen_model: 'deepseek-v4-flash',
      istem: 11,
      tamamlama: 6,
      dusunce: 2,
    });
    expect(d.ozet.son_kredi?.kalan).toBe('12.5');
  });

  it('aktarım durunca uyarı öğesi eklenir ve oturum boşa düşer', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' }),
    });
    expect(d.ogeler.at(-1)).toMatchObject({ tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' });
    expect(d.anlik).toBe('Bos');
  });

  it('paket numarasında boşluk hata olarak işaretlenir; sonrası uygulanmaz', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }) });
    d = uygula(d, { tip: 'Kanal', mesaj: paket(3, { tip: 'TokenParcasi', metin: 'x' }) });
    expect(d.bosluk).toBe(true);
    expect(tipler(d)).toEqual([]);
    const sonra = uygula(d, { tip: 'Kanal', mesaj: paket(4, { tip: 'TokenParcasi', metin: 'y' }) });
    expect(sonra).toBe(d);
  });

  it('kapanış mesajından sonra oturum kapalıdır', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }) });
    d = uygula(d, { tip: 'Kanal', mesaj: { tip: 'Kapandi', no: 2 } });
    expect(d.kapandi).toBe(true);
    expect(d.anlik).toBe('Bos');
  });

  it('eski durum değişmez', () => {
    const once = uygula(acilis(), { tip: 'MesajGonderildi', metin: 'a' });
    const kopya = structuredClone(once);
    uygula(once, { tip: 'Kanal', mesaj: paket(1, { tip: 'TokenParcasi', metin: 'b' }) });
    expect(once).toEqual(kopya);
  });
});

describe('geçmiş ve canlı olaylar (1c tasarımı §5.5)', () => {
  const gecmis: OturumAcilisi['gecmis'] = {
    ogeler: [
      { tip: 'KullaniciMesaji', sira: 10, tur: 't-0', metin: 'önce' },
      { tip: 'Yanit', sira: 12, tur: 't-0', metin: 'yanıt', dusunce: null },
      {
        tip: 'AracCagrisi',
        sira: 12,
        tur: 't-0',
        model_cagri_id: 'm-1',
        arac: 'dosya_yaz',
        argumanlar: '{}',
        durum: 'SonucuBilinmiyor',
        sonuc: null,
      },
      { tip: 'TurSonu', sira: 15, tur: 't-0', neden: { tip: 'Normal' } },
    ],
    daha_eski_var: true,
  };

  it('geçmiş ekran öğelerine çevrilir; anahtarlar benzersiz', () => {
    const d = acilis(gecmis);
    expect(tipler(d)).toEqual(['KullaniciMesaji', 'Yanit', 'AracCagrisi', 'TurSonu']);
    expect(new Set(d.ogeler.map((o) => o.anahtar)).size).toBe(4);
    expect(d.gecmis_son_sira).toBe(15);
    expect(d.daha_eski_var).toBe(true);
  });

  it('geçmişte zaten olan sıradaki canlı olay atlanır', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'Kanal',
      mesaj: {
        tip: 'Olaylar',
        no: 1,
        zarflar: [
          { sema: 1, oturum: OTURUM, tur: null, sira: 14, yuk: { tip: 'Hata', mesaj: 'eski' } },
          { sema: 1, oturum: OTURUM, tur: null, sira: 16, yuk: { tip: 'Hata', mesaj: 'yeni' } },
        ],
      },
    });
    expect(d.ogeler.filter((o) => o.tip === 'Hata')).toMatchObject([{ mesaj: 'yeni' }]);
  });

  it('eşi olmayan araç sonucu yok sayılır', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'AracSonucu',
        cagri: 'bilinmeyen',
        arac: 'dosya_yaz',
        durum: 'SonucuBilinmiyor',
        icerik: '[sonucu bilinmiyor]',
      }),
    });
    expect(d.ogeler).toHaveLength(4);
  });

  it('eski sayfa başa eklenir; tekrar eden öğe eklenmez', () => {
    const d = uygula(acilis(gecmis), {
      tip: 'EskiSayfa',
      sayfa: {
        ogeler: [
          { tip: 'KullaniciMesaji', sira: 2, tur: 't-a', metin: 'en eski' },
          { tip: 'KullaniciMesaji', sira: 10, tur: 't-0', metin: 'önce' },
        ],
        daha_eski_var: false,
      },
    });
    expect(d.ogeler.map((o) => o.sira)).toEqual([2, 10, 12, 12, 15]);
    expect(d.daha_eski_var).toBe(false);
  });
});

describe('kanal dinleyicisi', () => {
  it('"işledim"i uygular uygulamaz bildirir; çizimi beklemez (1c tasarımı §4.3)', () => {
    let durum = acilis();
    const sirasi: string[] = [];
    const isledim = vi.fn((no: number) => sirasi.push(`isledim ${no}`));
    const dinle = kanalIsleyicisi(
      () => durum,
      (d) => {
        sirasi.push('yaz');
        durum = d;
      },
      isledim,
    );
    dinle(paket(1, { tip: 'TurBasladi' }));
    expect(sirasi).toEqual(['yaz', 'isledim 1']);
  });

  it('boşlukta ve kapanışta "işledim" gönderilmez', () => {
    let durum = acilis();
    const isledim = vi.fn();
    const dinle = kanalIsleyicisi(
      () => durum,
      (d) => (durum = d),
      isledim,
    );
    dinle(paket(2, { tip: 'TurBasladi' }));
    expect(isledim).not.toHaveBeenCalled();
    durum = acilis();
    dinle({ tip: 'Kapandi', no: 1 });
    expect(isledim).not.toHaveBeenCalled();
  });
});
