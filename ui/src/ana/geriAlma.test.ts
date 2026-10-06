import { describe, expect, it } from 'vitest';
import type { DosyaOzeti, TurOzeti } from '../ipc/tipler';
import {
  boyutMetni,
  eylemMetni,
  geriAlinabilirlikMetni,
  geriAlmaSonucMetni,
  turOzetiGorunumu,
} from './geriAlma';

function dosya(ek: Partial<DosyaOzeti> & { yol: string }): DosyaOzeti {
  return {
    islem: 'Degisti',
    eklenen: 2,
    silinen: 1,
    sonucu_bilinmiyor: false,
    geri_alinabilir: { tip: 'Evet' },
    ...ek,
  };
}

function ozet(ek: Partial<TurOzeti> = {}): TurOzeti {
  return {
    tur: 't',
    dosyalar: [],
    komutlar: [],
    istekler: { sayi: 0, istem: 0, tamamlama: 0, dusunce: 0 },
    guvenlik_uyarisi: 0,
    ...ek,
  };
}

describe('tur özeti görünümü (aşama 3 tasarımı §5.11)', () => {
  it('dosyalar, komutlar, istekler ve notlar', () => {
    const g = turOzetiGorunumu(
      ozet({
        dosyalar: [
          dosya({ yol: 'src/a.rs' }),
          dosya({ yol: 'yeni.rs', islem: 'Olusturuldu', eklenen: 3, silinen: 0 }),
          dosya({ yol: 'buyuk.bin', geri_alinabilir: { tip: 'Hayir', neden: 'Buyuk' } }),
        ],
        komutlar: [
          {
            cagri: 'c1',
            komut: 'cargo build\n--release',
            durum: 'Basarili',
            degisiklikler: { dosyalar: [], toplam: 1204, eksik: true },
          },
          { cagri: 'c2', komut: 'ls', durum: 'Basarili' },
        ],
        istekler: { sayi: 3, istem: 10000, tamamlama: 2400, dusunce: 0 },
        guvenlik_uyarisi: 1,
      }),
      false,
    );
    expect(g.satirlar).toEqual([
      'src/a.rs +2 −1',
      'yeni.rs +3 −0 · yeni',
      'buyuk.bin +2 −1 · geri alınamaz',
      'cargo build: 1204 dosya değişti',
      'ls: dosya değişmedi',
      '3 EVREN isteği, 12.400 token',
      '1 güvenlik uyarısı',
    ]);
    expect(g.notlar).toEqual([
      'Kabukla yapılan değişiklikler geri alınamaz.',
      'Liste eksik olabilir.',
    ]);
    expect(g.geriAlinabilir).toBe(true);
    expect(g.bos).toBe(false);
  });

  it('ağ özeti komutların altında görünür', () => {
    const g = turOzetiGorunumu(
      ozet({
        komutlar: [
          {
            cagri: 'c1',
            komut: 'cargo build',
            durum: 'Basarili',
            ag: {
              hostlar: [
                {
                  host: 'index.crates.io',
                  port: 443,
                  baglanti_sayisi: 2,
                  giden_bayt: 120,
                  gelen_bayt: 4500,
                  reddedilenler: 0,
                },
                {
                  host: 'evil.com',
                  port: 443,
                  baglanti_sayisi: 1,
                  giden_bayt: 0,
                  gelen_bayt: 0,
                  reddedilenler: 1,
                },
              ],
              toplam_giden: 120,
              toplam_gelen: 4500,
              reddedilen_sayisi: 1,
            },
          },
        ],
      }),
      false,
    );
    expect(g.satirlar).toEqual([
      'cargo build: dosya değişmedi',
      'Ağ (index.crates.io:443): 2 bağlantı, 4,5 KiB',
      'Ağ (evil.com:443): 1 bağlantı reddedildi',
    ]);
  });

  it('geri alınabilir dosyası olmayan turda düğme yok; araçsız tur boş', () => {
    const g = turOzetiGorunumu(
      ozet({
        dosyalar: [dosya({ yol: 'a', geri_alinabilir: { tip: 'Hayir', neden: 'Kayitsiz' } })],
      }),
      true,
    );
    expect(g.geriAlinabilir).toBe(false);
    expect(g.notlar).toEqual(['Bu tur geri alındı.']);
    expect(turOzetiGorunumu(ozet(), false).bos).toBe(true);
  });
});

describe('geri alma metinleri', () => {
  it('eylem ve nedenleri', () => {
    expect(eylemMetni({ tip: 'Sil' })).toEqual({
      ad: 'Silinir; bu turda oluşturulmuştu',
      neden: null,
    });
    expect(eylemMetni({ tip: 'Cakisma', neden: { tip: 'SonrakiTur', tur: 't2' } }).neden).toBe(
      'Bu dosya sonraki bir turda da değişti; önce o turu geri alın.',
    );
    expect(eylemMetni({ tip: 'Cakisma', neden: { tip: 'Okunamadi', hata: 'erişim' } }).neden).toBe(
      'Dosyanın şimdiki hâli okunamadı: erişim',
    );
    expect(eylemMetni({ tip: 'GeriAlinamaz', neden: 'VeriSilindi' }).neden).toBe(
      'Önceki hâli depo sınırı yüzünden silindi.',
    );
  });

  it('onay kartının satırı', () => {
    expect(geriAlinabilirlikMetni({ tip: 'Evet' })).toBe('Evet: turu geri alarak');
    expect(geriAlinabilirlikMetni({ tip: 'Hayir', neden: 'Kabuk' })).toBe(
      'Hayır: komutların etkileri geri alınamaz',
    );
  });

  it('konuşmadaki sonuç satırı dokunulmayanları sayar', () => {
    expect(
      geriAlmaSonucMetni([
        { yol: 'a', eylem: { tip: 'GeriYukle', ozet: 'x' } },
        { yol: 'b', eylem: { tip: 'Sil' }, hata: 'yazılamadı' },
        { yol: 'c', eylem: { tip: 'Cakisma', neden: { tip: 'Elle' } } },
        { yol: 'd', eylem: { tip: 'ZatenOnceki' } },
      ]),
    ).toBe('Tur geri alındı: 1 dosya önceki hâline döndü. 2 dosyaya dokunulmadı.');
  });

  it('boyut', () => {
    expect(boyutMetni(0)).toBe('0 bayt');
    expect(boyutMetni(1536)).toBe('1,5 KiB');
    expect(boyutMetni(12.3 * 1024 * 1024)).toBe('12,3 MiB');
  });
});
