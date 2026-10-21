import { describe, expect, it, vi } from 'vitest';
import type { ButceGorunumu, KanalMesaji, Olay, OturumAcilisi, Zarf } from '../ipc/tipler';
import { kanalIsleyicisi } from './dinleyici';
import { acilistanEkran, uygula, type EkranOgesi, type OturumEkrani } from './oturum';

const OTURUM = 'o-1';
const TUR = 't-1';

function acilis(gecmis: OturumAcilisi['gecmis'] = { ogeler: [], daha_eski_var: false }) {
  return acilistanEkran({
    oturum: OTURUM,
    calisma_alani: 'C:\\proje',
    calisma_alani_kimligi: null,
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
    butce: null,
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

describe('güvenlik uyarısı', () => {
  it('canlı olaydan ve geçmişten konuşma öğesi olur', () => {
    const d = acilis({
      ogeler: [{ tip: 'GuvenlikUyarisi', sira: 3, tur: 't0', mesaj: 'eski uyarı' }],
      daha_eski_var: false,
    });
    expect(d.ogeler).toMatchObject([{ tip: 'GuvenlikUyarisi', mesaj: 'eski uyarı' }]);
    const sonra = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'GuvenlikUyarisi', cagri: 'c-1', mesaj: 'yeni uyarı' }),
    });
    expect(sonra.ogeler.at(-1)).toMatchObject({ tip: 'GuvenlikUyarisi', mesaj: 'yeni uyarı' });
  });
});

describe('otomatik karar', () => {
  it('canlı olaydan ve geçmişten konuşma öğesi olur', () => {
    const d = acilis({
      ogeler: [
        {
          tip: 'OtomatikKarar',
          sira: 3,
          tur: 't0',
          cagri: 'c-0',
          arac: 'kabuk',
          aciklama: '`cargo test` çalıştır',
          sonuc: 'Izin',
          risk: null,
          olasilik: 0.12,
        },
      ],
      daha_eski_var: false,
    });
    expect(d.ogeler).toMatchObject([
      {
        tip: 'OtomatikKarar',
        cagri: 'c-0',
        arac: 'kabuk',
        aciklama: '`cargo test` çalıştır',
        sonuc: 'Izin',
        risk: null,
        olasilik: 0.12,
      },
    ]);
    const sonra = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'OtomatikKarar',
        cagri: 'c-1',
        arac: 'kabuk',
        aciklama: '`rm -rf` çalıştır',
        sonuc: 'Ret',
        risk: 'yikici',
        olasilik: 0.91,
      }),
    });
    expect(sonra.ogeler.at(-1)).toMatchObject({
      tip: 'OtomatikKarar',
      cagri: 'c-1',
      arac: 'kabuk',
      aciklama: '`rm -rf` çalıştır',
      sonuc: 'Ret',
      risk: 'yikici',
      olasilik: 0.91,
    });
  });
});

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

  it('dosya değişiklikleri kendi çağrısına bağlanır (masaüstü arayüzü §4)', () => {
    const degisiklik = { yol: 'a.py', islem: 'Degisti' as const, eklenen: 1, silinen: 1, diff: '@@ -1 +1 @@\n-x\n+y\n' };
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'AracCagrisiBasladi', cagri: 'c-1', arac: 'dosya_oku', aciklama: 'a' },
        { tip: 'AracCagrisiBasladi', cagri: 'c-2', arac: 'dosya_duzenle', aciklama: 'b' },
        { tip: 'DosyaDegisiklikleri', cagri: 'c-2', dosyalar: [degisiklik] },
      ),
    });
    const [a, b] = d.ogeler.filter((o) => o.tip === 'AracCagrisi');
    expect(a.tip === 'AracCagrisi' && a.degisiklikler).toEqual([]);
    expect(b.tip === 'AracCagrisi' && b.degisiklikler).toEqual([degisiklik]);
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
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1', karar: 'Onaylandi' });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler.at(-1)).toMatchObject({ durum: 'Yanitlandi', karar: 'Onaylandi' });
  });

  it('önceki onayın gecikmiş yanıtı yeni bekleyen isteği temizlemez', () => {
    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, onayGerekiyor('i-1', 'c-1')),
    });
    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1', karar: 'Onaylandi' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, onayGerekiyor('i-2', 'c-2')),
    });
    expect(d.anlik).toBe('OnayBekliyor');

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1', karar: 'Onaylandi' });

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

    d = uygula(d, { tip: 'OnayYanitlandi', islem: 'i-1', karar: 'Onaylandi' });

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
        degisiklikler: [{ yol: 'a.py', islem: 'Olusturuldu', eklenen: 2, silinen: 0 }],
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
    expect(d.ogeler[2]).toMatchObject({
      degisiklikler: [{ yol: 'a.py', islem: 'Olusturuldu', eklenen: 2, silinen: 0 }],
    });
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

describe('geri alma (aşama 3 tasarımı §5.11)', () => {
  it('geçmişteki geri alma ve Kalfa notu ekran öğesi olur', () => {
    const d = acilis({
      ogeler: [
        { tip: 'KullaniciMesaji', sira: 1, tur: 't-0', metin: 'yaz' },
        { tip: 'TurSonu', sira: 5, tur: 't-0', neden: { tip: 'Normal' } },
        {
          tip: 'GeriAlma',
          sira: 7,
          tur: null,
          geri_alinan_tur: 't-0',
          sonuc: [{ yol: 'a.txt', eylem: { tip: 'Sil' } }],
          yarim: false,
        },
        { tip: 'KalfaNotu', sira: 9, tur: null, metin: '[Kalfa] not' },
      ],
      daha_eski_var: false,
    });
    expect(tipler(d)).toEqual(['KullaniciMesaji', 'TurSonu', 'GeriAlma', 'KalfaNotu']);
    expect(new Set(d.ogeler.map((o) => o.anahtar)).size).toBe(4);
  });

  it('TurGeriAlindi canlı bir geri alma öğesi ekler; önizleme ve silme öğe eklemez', () => {
    let d = acilis();
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(
        1,
        {
          tip: 'GeriAlmaPlani',
          geri_alinan_tur: 't-0',
          plan: { ogeler: [], veri_silindi: false },
          muhur: 'm',
        },
        { tip: 'GeriAlmaVerisiSilindi', turlar: ['t-0'] },
        { tip: 'TurGeriAlindi', geri_alinan_tur: 't-0', sonuc: [] },
      ),
    });
    expect(tipler(d)).toEqual(['GeriAlma']);
    const oge = d.ogeler[0] as Extract<EkranOgesi, { tip: 'GeriAlma' }>;
    expect(oge.geri_alinan_tur).toBe('t-0');
    expect(oge.yarim).toBe(false);
  });
});

describe('tur özeti (aşama 3 tasarımı §5.5)', () => {
  const ozet = {
    tur: 't-0',
    dosyalar: [
      {
        yol: 'a.txt',
        islem: 'Degisti' as const,
        eklenen: 1,
        silinen: 1,
        sonucu_bilinmiyor: false,
        geri_alinabilir: { tip: 'Evet' as const },
      },
      {
        yol: 'yeni.txt',
        islem: 'Olusturuldu' as const,
        eklenen: 1,
        silinen: 0,
        sonucu_bilinmiyor: false,
        geri_alinabilir: { tip: 'Evet' as const },
      },
    ],
    komutlar: [],
    istekler: { sayi: 2, istem: 10, tamamlama: 5, dusunce: 0 },
    guvenlik_uyarisi: 0,
  };

  it('özet olayı ekran öğesi olur; veri silinince değişen dosya geri alınamaz olur', () => {
    let d = acilis();
    d = uygula(d, { tip: 'Kanal', mesaj: paket(1, { tip: 'TurOzeti', ozet }) });
    expect(tipler(d)).toEqual(['TurOzeti']);
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'GeriAlmaVerisiSilindi', turlar: ['t-0'] }),
    });
    const oge = d.ogeler[0] as Extract<EkranOgesi, { tip: 'TurOzeti' }>;
    expect(oge.ozet.dosyalar.map((f) => f.geri_alinabilir)).toEqual([
      { tip: 'Hayir', neden: 'VeriSilindi' },
      { tip: 'Evet' },
    ]);
  });

  it('geçmişteki özet ekran öğesi olur', () => {
    const d = acilis({
      ogeler: [{ tip: 'TurOzeti', sira: 3, tur: 't-0', ozet }],
      daha_eski_var: false,
    });
    expect(tipler(d)).toEqual(['TurOzeti']);
  });
});

describe('geri alma sayfası (aşama 3 tasarımı §5.6)', () => {
  const plan = {
    ogeler: [
      {
        yol: 'a.txt',
        eylem: { tip: 'GeriYukle' as const, ozet: 'x' },
        simdiki: 'y',
        uyarilar: [],
        diff: null,
      },
    ],
    veri_silindi: false,
  };
  const ozet = {
    tur: 't-0',
    dosyalar: [],
    komutlar: [],
    istekler: { sayi: 1, istem: 1, tamamlama: 1, dusunce: 0 },
    guvenlik_uyarisi: 0,
  };

  it('plan sayfayı açar; yeni plan eskisinin yerine geçer; kapatınca kapanır', () => {
    let d = acilis();
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'GeriAlmaPlani', geri_alinan_tur: 't-0', plan, muhur: 'm1' }),
    });
    expect(d.geri_alma).toEqual({ tur: 't-0', plan, muhur: 'm1' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'GeriAlmaPlani', geri_alinan_tur: 't-0', plan, muhur: 'm2' }),
    });
    expect(d.geri_alma?.muhur).toBe('m2');
    d = uygula(d, { tip: 'GeriAlmaKapandi' });
    expect(d.geri_alma).toBeNull();
  });

  it('TurGeriAlindi sayfayı kapatır ve turun özetini "geri alındı" işaretler', () => {
    let d = acilis();
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurOzeti', ozet },
        { tip: 'GeriAlmaPlani', geri_alinan_tur: 't-0', plan, muhur: 'm' },
      ),
    });
    const once = d.ogeler[0] as Extract<EkranOgesi, { tip: 'TurOzeti' }>;
    expect(once.geri_alindi).toBe(false);
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'TurGeriAlindi', geri_alinan_tur: 't-0', sonuc: [] }),
    });
    expect(d.geri_alma).toBeNull();
    const sonra = d.ogeler[0] as Extract<EkranOgesi, { tip: 'TurOzeti' }>;
    expect(sonra.geri_alindi).toBe(true);
  });

  it('geçmişten açılışta geri alınmış turun özeti işaretlidir; yarım geri alma işaretlemez', () => {
    const d = acilis({
      ogeler: [
        { tip: 'TurOzeti', sira: 3, tur: 't-0', ozet },
        { tip: 'TurOzeti', sira: 6, tur: 't-1', ozet: { ...ozet, tur: 't-1' } },
        { tip: 'GeriAlma', sira: 8, tur: null, geri_alinan_tur: 't-0', sonuc: [], yarim: false },
        { tip: 'GeriAlma', sira: 9, tur: null, geri_alinan_tur: 't-1', sonuc: [], yarim: true },
      ],
      daha_eski_var: false,
    });
    const ozetler = d.ogeler.filter(
      (o): o is Extract<EkranOgesi, { tip: 'TurOzeti' }> => o.tip === 'TurOzeti',
    );
    expect(ozetler.map((o) => o.geri_alindi)).toEqual([true, false]);
    expect(d.geri_alma).toBeNull();
  });
});

describe('gönderim önizlemesi (aşama 3b)', () => {
  const onizleme = (): Olay => ({
    tip: 'GonderimOnizlemesi',
    islem: 'i-1',
    ogeler: [{ cagri: 'c-1', kaynak: 'dosya:a.txt', bayt: 10 }],
    neden: 'Onizleme',
  });

  it('karar verilen kart, IPC dönmeden biten turda yanıtlanmış kalır', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }, onizleme()) });
    d = uygula(d, { tip: 'OnizlemeYanitlandi', islem: 'i-1' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'Tamamlandi', neden: { tip: 'Normal' } }),
    });
    expect(d.ogeler.find((o) => o.tip === 'GonderimOnizlemesi')).toMatchObject({
      durum: 'Yanitlandi',
    });
  });

  it('kararı arka uca ulaşmayan kart yeniden karar bekler', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }, onizleme()) });
    d = uygula(d, { tip: 'OnizlemeYanitlandi', islem: 'i-1' });
    d = uygula(d, { tip: 'OnizlemeGonderilemedi', islem: 'i-1' });
    expect(d.ogeler.find((o) => o.tip === 'GonderimOnizlemesi')).toMatchObject({
      durum: 'Bekliyor',
    });
    expect(d.anlik).toBe('OnayBekliyor');
  });
});

describe('güvenlik denetçisinin rozeti (aşama 3c, Görev 8)', () => {
  const basladi = (): Olay => ({
    tip: 'AracCagrisiBasladi',
    cagri: 'c-1',
    arac: 'dosya_oku',
    aciklama: 'a.rs',
  });

  function rozet(d: OturumEkrani) {
    const o = d.ogeler.find((x) => x.tip === 'AracCagrisi');
    return o?.tip === 'AracCagrisi' ? o.denetim : undefined;
  }

  it('denetleniyor → şüpheli', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }, basladi()) });
    expect(rozet(d)).toBeNull();
    d = uygula(d, { tip: 'Kanal', mesaj: paket(2, { tip: 'DenetimBasladi', cagri: 'c-1' }) });
    expect(rozet(d)).toEqual({ tip: 'Suruyor' });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(3, {
        tip: 'DenetimSonucu',
        cagri: 'c-1',
        sonuc: { tip: 'Supheli', kategori: 'veri_sizdirma', gerekce: 'g' },
      }),
    });
    expect(rozet(d)).toEqual({ tip: 'Supheli', kategori: 'veri_sizdirma', gerekce: 'g' });
  });

  it('gizli okuma denetlenmedi diye işaretlenir; başka çağrının rozeti değişmez', () => {
    let d = uygula(acilis(), { tip: 'Kanal', mesaj: paket(1, { tip: 'TurBasladi' }, basladi()) });
    d = uygula(d, { tip: 'Kanal', mesaj: paket(2, { tip: 'DenetimAtlandi', cagri: 'c-1' }) });
    expect(rozet(d)).toEqual({ tip: 'Atlandi' });
    d = uygula(d, { tip: 'Kanal', mesaj: paket(3, { tip: 'DenetimBasladi', cagri: 'yok' }) });
    expect(rozet(d)).toEqual({ tip: 'Atlandi' });
  });

  it('geçmişteki denetim rozete gelir; eski kayıtta rozet yok', () => {
    const cagri = (denetim?: { tip: 'Bilinmiyor'; neden: string }) => ({
      tip: 'AracCagrisi' as const,
      sira: 3,
      tur: 't-0',
      model_cagri_id: 'm-1',
      arac: 'dosya_oku',
      argumanlar: '{}',
      durum: 'Basarili' as const,
      sonuc: 'x',
      ...(denetim ? { denetim } : {}),
    });
    const ile = acilis({ ogeler: [cagri({ tip: 'Bilinmiyor', neden: 'zaman aşımı' })], daha_eski_var: false });
    expect(rozet(ile)).toEqual({ tip: 'Bilinmiyor', neden: 'zaman aşımı' });
    const eski = acilis({ ogeler: [cagri()], daha_eski_var: false });
    expect(rozet(eski)).toBeNull();
  });
});

describe('oturum özeti ve bağlam durumu', () => {
  it('OturumOzetlendi olayı bir OturumOzeti öğesi ekler', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'OturumOzetlendi',
        tetik: 'Elle',
        once: 70000,
        sonra: 3000,
        talimat: 'odak: testler',
        ozet: 'kısa özet',
      }),
    });
    expect(d.ogeler).toMatchObject([
      {
        tip: 'OturumOzeti',
        tetik: 'Elle',
        once: 70000,
        sonra: 3000,
        talimat: 'odak: testler',
        ozet: 'kısa özet',
      },
    ]);
  });

  it('BaglamDurumu olayı baglam alanını ayarlar', () => {
    const d0 = acilis();
    expect(d0.baglam).toBeNull();
    const d = uygula(d0, {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'BaglamDurumu', tahmin: 90, esik: 100 }),
    });
    expect(d.baglam).toEqual({ tahmin: 90, esik: 100 });
  });

  it('geçmişten gelen OturumOzeti görünüm öğesi aynı öğeye döner', () => {
    const d = acilis({
      ogeler: [
        {
          tip: 'OturumOzeti',
          sira: 8,
          tur: 't-0',
          yontem: 'Otomatik',
          once: 180000,
          sonra: 6000,
          talimat: null,
          ozet: 'geçmiş özeti',
        },
      ],
      daha_eski_var: false,
    });
    expect(d.ogeler).toMatchObject([
      {
        tip: 'OturumOzeti',
        sira: 8,
        tur: 't-0',
        tetik: 'Otomatik',
        once: 180000,
        sonra: 6000,
        talimat: null,
        ozet: 'geçmiş özeti',
      },
    ]);
  });
});

describe('deneme defteri sayaçları', () => {
  function butce(ustune: Partial<ButceGorunumu> = {}): ButceGorunumu {
    return {
      baslatilan: 0,
      bilinen_token: 0,
      bilinmeyen: 0,
      aktif: 0,
      ...ustune,
    };
  }

  it('açılış sayaçlarını taşır ve ButceDurumu ile günceller', () => {
    const d0 = acilistanEkran({
      oturum: OTURUM,
      calisma_alani: 'C:\\proje',
      calisma_alani_kimligi: null,
      gecmis: { ogeler: [], daha_eski_var: false },
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
      butce: butce({ baslatilan: 2, bilinmeyen: 1 }),
    });
    expect(d0.butce).toMatchObject({ baslatilan: 2, bilinmeyen: 1 });

    const d = uygula(d0, {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'ButceDurumu',
        durum: butce({ baslatilan: 3, bilinen_token: 1200, bilinmeyen: 1 }),
      }),
    });
    expect(d.butce).toMatchObject({ baslatilan: 3, bilinen_token: 1200, bilinmeyen: 1 });
    expect(d.ogeler).toEqual([]);
  });
});


describe('proje akışı: plan, proje talimatları ve kullanıcı sorusu', () => {
  it('PlanGuncellendi ve ProjeTalimatlari olayları ekran durumunu günceller ve açılışta korunur', () => {
    const plan = {
      guncellendigi_tur: TUR,
      adimlar: [
        { kimlik: 'a1', baslik: 'AGENTS.md oku', durum: 'Tamam' as const },
        { kimlik: 'a2', baslik: 'Planı uygula', durum: 'Suruyor' as const },
        { kimlik: 'a3', baslik: 'Testleri koş', durum: 'Bekliyor' as const },
      ],
    };
    const goruntu = {
      tur: 1,
      kaynaklar: [
        {
          goreli_yol: 'AGENTS.md',
          kapsam: { tur: 'kok' as const },
          bayt: 120,
          ozet: 'abc123',
        },
      ],
      uyarilar: [],
      degisen_kaynaklar: [],
      eksik_baglam: false,
    };

    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(
        1,
        { tip: 'TurBasladi' },
        { tip: 'ProjeTalimatlari', goruntu },
        { tip: 'PlanGuncellendi', plan },
      ),
    });

    expect(d.plan).toEqual(plan);
    expect(d.proje_talimatlari).toEqual(goruntu);
    expect(d.ozet.plan).toEqual(plan);
    expect(d.ozet.proje_talimatlari).toEqual(goruntu);

    const kurtarilan = acilistanEkran({
      oturum: OTURUM,
      calisma_alani: 'C:\\proje',
      calisma_alani_kimligi: null,
      gecmis: { ogeler: [], daha_eski_var: false },
      ozet: {
        model: 'glm-5.3',
        kip: 'Siki',
        yonlendirilen_model: null,
        son_kredi: null,
        istem: 0,
        tamamlama: 0,
        dusunce: 0,
        plan,
        proje_talimatlari: goruntu,
      },
      salt_okunur: null,
      butce: null,
    });
    expect(kurtarilan.plan).toEqual(plan);
    expect(kurtarilan.proje_talimatlari).toEqual(goruntu);
  });

  it('kullanıcı sorusu SoruBekliyor durumuna geçer, yanıtlanınca veya yeni mesajla çözülür', () => {
    const soru = {
      soru_kimligi: 'soru-t-1-c-1',
      tur: TUR,
      cagri: 'c-1',
      baslik: 'Hangi yaklaşımı kullanalım?',
      secenekler: [
        { kimlik: 's1', etiket: 'Kısa yol', aciklama: 'Hızlı çözüm' },
        { kimlik: 's2', etiket: 'Kapsamlı yol', aciklama: null },
      ],
      serbest_yanit_izni: true,
    };

    let d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'KullaniciSorusuGerekiyor', soru }),
    });
    expect(d.anlik).toBe('SoruBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({
      tip: 'KullaniciSorusu',
      durum: 'Bekliyor',
      soru,
    });

    // IPC hatasında SoruGonderilemedi geri alır:
    d = uygula(d, {
      tip: 'SoruYanitlandi',
      soru_kimligi: soru.soru_kimligi,
      sonuc: {
        durum: 'Yanitlandi',
        yanit: { tur: 'Secenek', secenek_kimligi: 's1' },
      },
    });
    expect(d.anlik).toBe('Calisiyor');
    d = uygula(d, { tip: 'SoruGonderilemedi', soru_kimligi: soru.soru_kimligi });
    expect(d.anlik).toBe('SoruBekliyor');

    // Kullanıcı normal mesaj gönderirse bekleyen soru KullaniciMesajiylaIptal olur:
    d = uygula(d, { tip: 'MesajGonderildi', metin: 'Bunun yerine şunu yap' });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.ogeler[0]).toMatchObject({
      tip: 'KullaniciSorusu',
      durum: 'KullaniciMesajiylaIptal',
      sonuc: { durum: 'KullaniciMesajiylaIptal', yeni_mesaj: 'Bunun yerine şunu yap' },
    });
  });

  it('geçmişteki yanıtlanmamış soru KesintiyeUgradi olarak yüklenir', () => {
    const soru = {
      soru_kimligi: 'soru-t-0-c-0',
      tur: 't-0',
      cagri: 'c-0',
      baslik: 'Devam edilsin mi?',
      secenekler: [
        { kimlik: 'evet', etiket: 'Evet' },
        { kimlik: 'hayir', etiket: 'Hayır' },
      ],
      serbest_yanit_izni: false,
    };
    const d = acilis({
      ogeler: [{ tip: 'KullaniciSorusu', sira: 4, tur: 't-0', soru, sonuc: null }],
      daha_eski_var: false,
    });
    expect(d.ogeler).toMatchObject([
      {
        tip: 'KullaniciSorusu',
        durum: 'KesintiyeUgradi',
        sonuc: null,
      },
    ]);
  });
});