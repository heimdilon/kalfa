import { describe, expect, it } from 'vitest';
import type { EkranOgesi } from '../durum/oturum';
import { onayOgesi } from '../sahte/arka';
import {
  aracBasligi,
  aracHedefi,
  baglamNotu,
  kabukOzeti,
  aracKartDurumu,
  ciktiOnizlemesi,
  CANLI_DUSUNCE,
  dusunceKuyrugu,
  gruplara,
  gunAy,
  gunlukSinirMetni,
  kodParcalari,
  kredi,
  mbMetni,
  satirBasligi,
  satirDurumu,
  sayi,
  tarih,
  zamanEtiketi,
} from './bicim';
import { sonucGovdesi } from './bicim';
import { kararlariUret, kisaAd } from './onay';

const SIMDI = new Date(2026, 8, 23, 15, 0).getTime();
const saatOnce = (s: number) => SIMDI - s * 3600_000;
const gunOnce = (g: number) => new Date(2026, 8, 23 - g, 10, 0).getTime();

describe('zaman ve gün grupları (1c tasarımı §8.1)', () => {
  it('bugün saat, dün "Dün", bu hafta gün adı, daha eskisi gün ve ay', () => {
    expect(zamanEtiketi(new Date(2026, 8, 23, 9, 5).getTime(), SIMDI)).toBe('09:05');
    expect(zamanEtiketi(gunOnce(1), SIMDI)).toBe('Dün');
    expect(zamanEtiketi(gunOnce(3), SIMDI)).toBe('Paz');
    expect(zamanEtiketi(gunOnce(30), SIMDI)).toBe('24 Ağu');
  });

  it('sırayı bozmadan gruplar; boş grup yok', () => {
    const ogeler = [saatOnce(1), saatOnce(2), gunOnce(1), gunOnce(10)];
    expect(gruplara(ogeler, (x) => x, SIMDI).map((g) => [g.grup, g.ogeler.length])).toEqual([
      ['bugun', 2],
      ['dun', 1],
      ['eski', 1],
    ]);
  });
});

describe('oturum satırı', () => {
  it('açık oturumun anlık hâli kayıttaki durumdan önce gelir', () => {
    expect(satirDurumu({ tip: 'Tamamlandi' }, 'OnayBekliyor')).toEqual({
      status: 'waiting',
      etiket: 'Onay bekliyor',
    });
    expect(satirDurumu({ tip: 'Tamamlandi' }, 'SartBekliyor').etiket).toBe('Şart onayı bekliyor');
    expect(satirDurumu({ tip: 'YarimKaldi' }, 'Bos').status).toBe('partial');
    expect(satirDurumu({ tip: 'Okunamadi', neden: 'x' }, null)).toEqual({
      status: 'failed',
      etiket: 'Okunamadı',
    });
  });

  it('başlık yoksa "Yeni oturum", okunamayan kayıtta dosya adı', () => {
    const satir = {
      oturum: 'o',
      dosya: 'C:\\veri\\oturumlar\\bozuk.jsonl',
      baslik: null,
      calisma_alani: null,
      son_degisiklik_ms: 0,
    };
    expect(satirBasligi({ ...satir, durum: { tip: 'Bos' } })).toBe('Yeni oturum');
    expect(satirBasligi({ ...satir, durum: { tip: 'Okunamadi', neden: 'x' } })).toBe(
      'bozuk.jsonl',
    );
  });
});

describe('araç kartı', () => {
  const arac = (ek: Partial<Extract<EkranOgesi, { tip: 'AracCagrisi' }>>) =>
    ({
      tip: 'AracCagrisi',
      anahtar: 'a',
      sira: 1,
      tur: null,
      cagri: null,
      arac: 'dosya_oku',
      aciklama: null,
      argumanlar: null,
      durum: 'Basarili',
      sonuc: null,
      terminal: null,
      ...ek,
    }) as Extract<EkranOgesi, { tip: 'AracCagrisi' }>;

  it('başlık sonuca göre: sürüyor, bitti, yalın ad', () => {
    expect(aracBasligi(arac({ durum: 'Suruyor' }))).toBe('Dosya okunuyor');
    expect(aracBasligi(arac({ durum: 'Basarili' }))).toBe('Dosya okundu');
    expect(aracBasligi(arac({ arac: 'dosya_yaz', durum: 'Reddedildi' }))).toBe('Dosya yazma');
    expect(aracBasligi(arac({ arac: 'bilinmeyen' }))).toBe('bilinmeyen');
  });

  it('hedef: canlı çağrıda açıklama, kayıttan gelende argümandaki yol', () => {
    expect(aracHedefi(arac({ aciklama: 'src/main.rs' }))).toBe('src/main.rs');
    expect(aracHedefi(arac({ argumanlar: '{"yol":"src/lib.rs"}' }))).toBe('src/lib.rs');
    expect(aracHedefi(arac({ argumanlar: 'bozuk' }))).toBeUndefined();
  });

  it('kabuk: başlık, kayıttan gelen komut ve çıkış kodu özeti (aşama 2 tasarımı §10.1)', () => {
    expect(aracBasligi(arac({ arac: 'kabuk', durum: 'Suruyor' }))).toBe('Komut çalışıyor');
    expect(aracBasligi(arac({ arac: 'kabuk', durum: 'Basarisiz' }))).toBe('Komut');
    expect(
      aracHedefi(arac({ arac: 'kabuk', argumanlar: '{"program":"cargo","argumanlar":["test"]}' })),
    ).toBe('cargo test');
    expect(aracHedefi(arac({ arac: 'kabuk', argumanlar: '{"betik":"a | b\\nc"}' }))).toBe('a | b');
    expect(
      kabukOzeti(
        arac({
          arac: 'kabuk',
          sonuc:
            '<arac_sonucu kaynak="kabuk:cargo" sinir="0123456789abcdef0123456789abcdef">\nÇıkış kodu: 101\nSüre: 2 sn\n</arac_sonucu sinir="0123456789abcdef0123456789abcdef">',
        }),
      ),
    ).toBe('Çıkış kodu: 101');
    expect(kabukOzeti(arac({ sonuc: 'fn main() {}' }))).toBeUndefined();
  });

  it('başlamamış çağrı "Çalıştırılmadı", bilinmeyen sonuç "Sonucu bilinmiyor"', () => {
    expect(aracKartDurumu(null)).toEqual({ status: 'unknown', etiket: 'Çalıştırılmadı' });
    expect(aracKartDurumu('SonucuBilinmiyor').etiket).toBe('Sonucu bilinmiyor');
    expect(aracKartDurumu('IptalEdildi').status).toBe('denied');
  });

  it('uzun çıktı kırpılır', () => {
    const uzun = Array.from({ length: 50 }, (_, i) => `satır ${i}`).join('\n');
    const o = ciktiOnizlemesi(uzun, 40);
    expect(o.kirpildi).toBe(true);
    expect(o.kod.split('\n')).toHaveLength(40);
    expect(ciktiOnizlemesi('a\nb\n')).toEqual({ kod: 'a\nb', kirpildi: false });
  });
});

describe('sayılar', () => {
  it('Türkçe biçim', () => {
    expect(sayi(15980)).toBe('15.980');
    expect(kredi('1250.5')).toBe('1.250,50');
    expect(kredi('bilinmiyor')).toBe('bilinmiyor');
  });
});

describe('onay kararları (1c tasarımı §8.2)', () => {
  const ogeler = [
    onayOgesi({ cagri: 'c1', hedefler: ['C:\\p\\src\\a.rs'], muhur: 'm1' }),
    onayOgesi({ cagri: 'c2', hedefler: ['C:\\p\\src\\b.rs'], muhur: 'm2' }),
  ];

  it('seçilenler onaylanır, seçilmeyenler gerekçeyle reddedilir; mühür taşınır', () => {
    expect(kararlariUret(ogeler, 'secilenler', new Set(['c2']), '  a.rs olmasın ')).toEqual([
      {
        cagri: 'c1',
        muhur: 'm1',
        karar: { tip: 'Reddet', gerekce: 'a.rs olmasın' },
        kisayol: false,
      },
      { cagri: 'c2', muhur: 'm2', karar: { tip: 'Onayla' }, kisayol: false },
    ]);
  });

  it('tümü ve hiçbiri; boş gerekçe null', () => {
    expect(kararlariUret(ogeler, 'tumu', new Set(), '').map((k) => k.karar.tip)).toEqual([
      'Onayla',
      'Onayla',
    ]);
    expect(kararlariUret(ogeler, 'hicbiri', new Set(['c1', 'c2']), ' ')[0].karar).toEqual({
      tip: 'Reddet',
      gerekce: null,
    });
  });

  it('kısa ad hedefin son bileşeni', () => {
    expect(kisaAd(ogeler[0])).toBe('a.rs');
    expect(kisaAd({ ...ogeler[0], hedefler: [] })).toBe('dosya_yaz');
  });
});

describe('sonucGovdesi (güvenlik analizi Bulgu 5)', () => {
  const S = '0123456789abcdef0123456789abcdef';
  it('sarmalı ayıklar; içerikteki sahte kapanış ve güvenlik satırı kalır', () => {
    const sonuc = [
      `<arac_sonucu kaynak="dosya:a.rs" sinir="${S}">`,
      'bir',
      '</arac_sonucu sinir="x">',
      'iki',
      `</arac_sonucu sinir="${S}">`,
      '[güvenlik] uyarı',
    ].join('\n');
    expect(sonucGovdesi(sonuc)).toBe('bir\n</arac_sonucu sinir="x">\niki\n[güvenlik] uyarı');
  });

  it('eski kayıtlardaki kaynak satırını ve etiketsiz metni işler', () => {
    expect(sonucGovdesi('[kaynak: dosya:a.rs]\niçerik')).toBe('içerik');
    expect(sonucGovdesi('[hata] olmadı')).toBe('[hata] olmadı');
  });
});

describe('eklenti ekranının biçimleri (aşama 4 tasarımı §14)', () => {
  const IZIN = new Date(2026, 9, 20, 10, 5).getTime();

  it('izin günü "20 Ekim", kurulum tarihi "20 Ekim 2026"', () => {
    expect(gunAy(IZIN)).toBe('20 Ekim');
    expect(tarih(IZIN)).toBe('20 Ekim 2026');
  });

  it("ilk gönderim boyutu Rust'taki metinle aynı: ondalık MB, tek basamak", () => {
    expect(mbMetni(6_000_000)).toBe('6,0 MB');
    expect(mbMetni(5_950_000)).toBe('6,0 MB');
    expect(mbMetni(1_234_567)).toBe('1,2 MB');
    expect(mbMetni(0)).toBe('0,0 MB');
    // Tam yarıda Rust gibi çifte yuvarlanır.
    expect(mbMetni(250_000)).toBe('0,2 MB');
    expect(mbMetni(750_000)).toBe('0,8 MB');
  });

  it('ters tırnaklı parçalar kod olur; başka işaret yorumlanmaz', () => {
    expect(kodParcalari('a `b` c')).toEqual([
      { metin: 'a ', kod: false },
      { metin: 'b', kod: true },
      { metin: ' c', kod: false },
    ]);
    expect(kodParcalari('`**/*.md`, `docs/**`')).toEqual([
      { metin: '**/*.md', kod: true },
      { metin: ', ', kod: false },
      { metin: 'docs/**', kod: true },
    ]);
    expect(kodParcalari('**kalın** <b>x</b>')).toEqual([
      { metin: '**kalın** <b>x</b>', kod: false },
    ]);
    expect(kodParcalari('')).toEqual([]);
  });

  it('kapanmayan ters tırnak olduğu gibi kalır, hiçbir karakter düşmez', () => {
    expect(kodParcalari('a `b')).toEqual([{ metin: 'a `b', kod: false }]);
    expect(kodParcalari('`x` ve `y')).toEqual([
      { metin: 'x', kod: true },
      { metin: ' ve `y', kod: false },
    ]);
  });

  it('günlük sınır: sıfırlanma anı yerel saatle, geçersizse yalnızca ilk kısım', () => {
    const yerel = new Date(2026, 9, 12, 3, 0).toISOString();
    expect(gunlukSinirMetni({ sifirlanma: yerel })).toBe('Günlük sınır doldu · sıfırlanma 03:00');
    expect(gunlukSinirMetni({ sifirlanma: new Date(2026, 9, 12, 23, 45).toISOString() })).toBe(
      'Günlük sınır doldu · sıfırlanma 23:45',
    );
    for (const sifirlanma of [null, '', 'yarın', '12', '2026-13-45T25:61:00Z']) {
      expect(gunlukSinirMetni({ sifirlanma })).toBe('Günlük sınır doldu');
    }
  });
});

describe('baglamNotu', () => {
  it('özet sayısını ve token tahminini yazar', () => {
    expect(baglamNotu({ adet: 6, yer_tutucu: 0, once: 71234, sonra: 29010 })).toBe(
      'Eski 6 araç çıktısı özetlendi (≈71.234 → ≈29.010 token).',
    );
  });

  it('özetlenemeyenleri ayrıca söyler', () => {
    expect(baglamNotu({ adet: 4, yer_tutucu: 2, once: 80000, sonra: 30000 })).toBe(
      'Eski 6 araç çıktısı özetlendi (≈80.000 → ≈30.000 token). 2 tanesi özetlenemedi ve kaldırıldı.',
    );
  });
});

describe('dusunceKuyrugu', () => {
  it('kısa düşünceyi olduğu gibi, uzununu son kısmıyla verir; vekil çifti bölünmez', () => {
    expect(dusunceKuyrugu('kısa')).toBe('kısa');
    const uzun = 'a'.repeat(CANLI_DUSUNCE + 10);
    expect(dusunceKuyrugu(uzun)).toBe(`…${'a'.repeat(CANLI_DUSUNCE)}`);
    // Kesim noktası bir emojinin ikinci yarısına düşer: emoji tümüyle atlanır.
    const emojili = `x😀${'b'.repeat(CANLI_DUSUNCE - 1)}`;
    expect(dusunceKuyrugu(emojili)).toBe(`…${'b'.repeat(CANLI_DUSUNCE - 1)}`);
  });
});
