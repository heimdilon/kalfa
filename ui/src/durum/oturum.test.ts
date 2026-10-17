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

describe('bağlam özeti', () => {
  it('not olarak eklenir', () => {
    const d = uygula(acilis(), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'BaglamOzetlendi', adet: 2, yer_tutucu: 0, once: 70000, sonra: 30000 }),
    });
    expect(d.ogeler).toMatchObject([
      { tip: 'BaglamOzetlendi', adet: 2, yer_tutucu: 0, once: 70000, sonra: 30000 },
    ]);
  });
});

describe('bütçe (5b tasarımı §7, §9)', () => {
  function butce(ustune: Partial<ButceGorunumu> = {}): ButceGorunumu {
    return {
      sinirlar: { istek: 200, token: 5_000_000 },
      baslatilan: 0,
      bilinen_token: 0,
      bilinmeyen: 0,
      onaylanan_bilinmeyen: 0,
      aktif: 0,
      engel: null,
      ...ustune,
    };
  }

  /** Kayıttan açılış: sayaç görünümü açılışla gelir (yeniden bağlanma). */
  function bagli(sayac: ButceGorunumu | null) {
    return acilistanEkran({
      oturum: OTURUM,
      calisma_alani: 'C:\proje',
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
      butce: sayac,
    });
  }

  it('açılış sayaçları görünümü taşır; kayıttan açılan oturumda bekleyen kart yeniden verilir', () => {
    const d0 = bagli(butce({ baslatilan: 2, bilinmeyen: 1, engel: 'bilinmeyen' }));
    expect(d0.butce).toMatchObject({ baslatilan: 2, bilinmeyen: 1 });
    expect(d0.ogeler).toEqual([]);

    const d = uygula(d0, {
      tip: 'Kanal',
      mesaj: paket(1, {
        tip: 'ButceKarariGerekli',
        durum: butce({ baslatilan: 2, bilinmeyen: 1, engel: 'bilinmeyen' }),
      }),
    });
    expect(d.anlik).toBe('OnayBekliyor');
    expect(d.ogeler.at(-1)).toMatchObject({
      tip: 'ButceKarari',
      durum: 'Bekliyor',
      gorunum: { engel: 'bilinmeyen', bilinmeyen: 1 },
    });
  });

  it('bekleyen karar yeni sayaçlarla yerinde yenilenir; ikinci kart açmaz', () => {
    const d1 = uygula(bagli(null), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'ButceKarariGerekli', durum: butce({ baslatilan: 200, engel: 'istek' }) }),
    });
    const d2 = uygula(d1, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'ButceKarariGerekli', durum: butce({ baslatilan: 200, engel: 'istek', bilinmeyen: 0, onaylanan_bilinmeyen: 0 }) }),
    });
    const kartlar = d2.ogeler.filter((o) => o.tip === 'ButceKarari');
    expect(kartlar).toHaveLength(1);
    expect(kartlar[0]).toMatchObject({ durum: 'Bekliyor' });
  });

  it('engelsiz sayaç bekleyen kartı yanıtlar; tur sürüyorsa akış devam eder', () => {
    let d = uygula(bagli(null), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'ButceKarariGerekli', durum: butce({ bilinmeyen: 1, engel: 'bilinmeyen' }) }),
    });
    expect(d.anlik).toBe('OnayBekliyor');
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'ButceDurumu', durum: butce({ bilinmeyen: 1, onaylanan_bilinmeyen: 1 }) }),
    });
    expect(d.ogeler.find((o) => o.tip === 'ButceKarari')).toMatchObject({ durum: 'Yanitlandi' });
    expect(d.anlik).toBe('Calisiyor');
    expect(d.butce).toMatchObject({ onaylanan_bilinmeyen: 1, engel: null });
  });

  it('karar kabul edilince (ButceYanitlandi) turda değilse oturum boşa döner', () => {
    let d = uygula(bagli(null), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'ButceKarariGerekli', durum: butce({ bilinmeyen: 1, engel: 'bilinmeyen' }) }),
    });
    d = uygula(d, { tip: 'ButceYanitlandi' });
    expect(d.ogeler.find((o) => o.tip === 'ButceKarari')).toMatchObject({ durum: 'Yanitlandi' });
    expect(d.anlik).toBe('Bos');
  });

  it('tur bitince bekleyen bütçe kartı geçersiz olur', () => {
    let d = uygula(bagli(null), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'TurBasladi' }, { tip: 'ButceKarariGerekli', durum: butce({ bilinmeyen: 1, engel: 'bilinmeyen' }) }),
    });
    d = uygula(d, {
      tip: 'Kanal',
      mesaj: paket(2, { tip: 'Tamamlandi', neden: { tip: 'Iptal' } }),
    });
    expect(d.ogeler.find((o) => o.tip === 'ButceKarari')).toMatchObject({ durum: 'Gecersiz' });
    expect(d.anlik).toBe('Bos');
  });

  it('ButceDurumu sayaçları günceller ve engel varken kartı yanıtlamaz', () => {
    const d = uygula(bagli(butce({ baslatilan: 1 })), {
      tip: 'Kanal',
      mesaj: paket(1, { tip: 'ButceDurumu', durum: butce({ baslatilan: 2, bilinmeyen: 1, engel: 'bilinmeyen' }) }),
    });
    expect(d.butce).toMatchObject({ baslatilan: 2, bilinmeyen: 1, engel: 'bilinmeyen' });
    expect(d.ogeler).toEqual([]);
  });
});
