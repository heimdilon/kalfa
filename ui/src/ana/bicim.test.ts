import { describe, expect, it } from 'vitest';
import type { EkranOgesi } from '../durum/oturum';
import { onayOgesi } from '../sahte/arka';
import {
  aracBasligi,
  aracHedefi,
  kabukOzeti,
  aracKartDurumu,
  ciktiOnizlemesi,
  gruplara,
  kredi,
  satirBasligi,
  satirDurumu,
  sayi,
  zamanEtiketi,
} from './bicim';
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
      kabukOzeti(arac({ arac: 'kabuk', sonuc: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 2 sn' })),
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
      { cagri: 'c1', muhur: 'm1', karar: { tip: 'Reddet', gerekce: 'a.rs olmasın' } },
      { cagri: 'c2', muhur: 'm2', karar: { tip: 'Onayla' } },
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
