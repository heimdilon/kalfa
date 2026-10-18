// Konuşmadaki onay kartı (masaüstü arayüzü tasarımı §3): kararlar sayfanın
// mühürlü `kararlariUret`'iyle aynıdır; riskli öğe karttan onaylanamaz.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import type { OnayOgesi } from '../ipc/tipler';
import { onayOgesi } from '../sahte/arka';
import BekleyenOnay from './BekleyenOnay.svelte';
import { kararlariUret, kartKarariUret, kartKararlari } from './onay';

const KISAYOL = { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' } as const;

function kabuk(ag: boolean): OnayOgesi {
  return onayOgesi({
    cagri: 'k1',
    arac: 'kabuk',
    aciklama: 'testleri çalıştırır',
    muhur: 'mk',
    kabuk: {
      komut: 'python -m unittest -v',
      bicim: { tip: 'Program' },
      dizin: '.',
      ag,
      ag_hostlari: ag ? ['pypi.org:443'] : [],
      yalitim: { tip: 'Yalitimli', arka_uc: 'windows AppContainer', profil: 'p' },
      ilk_cagri: false,
    } as OnayOgesi['kabuk'],
  });
}

const duzenleme = onayOgesi({
  cagri: 'd1',
  arac: 'dosya_duzenle',
  hedefler: ['C:\\proje\\src\\gorevler.py'],
  diff: '@@ -1 +1 @@\n-eski\n+yeni\n',
  muhur: 'md',
  kisayol: KISAYOL,
});

describe('kart kararları', () => {
  it('riskli öğe varsa yalnız ret; kısayol yalnız teklifi olan tek öğede', () => {
    expect(kartKararlari([kabuk(true)])).toEqual(['reddet']);
    expect(kartKararlari([duzenleme])).toEqual(['reddet', 'oturum', 'bir_kez']);
    expect(kartKararlari([kabuk(false)])).toEqual(['reddet', 'bir_kez']);
    expect(kartKararlari([duzenleme, kabuk(false)])).toEqual(['reddet', 'bir_kez']);
  });

  it('kararlar sayfanınkiyle aynı mühürlü girdilerdir', () => {
    const ogeler = [duzenleme];
    const hepsi = new Set(['d1']);
    expect(kartKarariUret(ogeler, 'bir_kez')).toEqual(kararlariUret(ogeler, 'tumu', new Set(), ''));
    expect(kartKarariUret(ogeler, 'oturum')).toEqual(
      kararlariUret(ogeler, 'secilenler', hepsi, '', hepsi),
    );
    expect(kartKarariUret(ogeler, 'oturum')?.[0]).toMatchObject({
      muhur: 'md',
      karar: { tip: 'Onayla' },
      kisayol: true,
    });
    expect(kartKarariUret(ogeler, 'reddet')?.[0]).toMatchObject({
      karar: { tip: 'Reddet', gerekce: null },
      kisayol: false,
    });
  });

  it('kartta verilemeyen karar üretilmez', () => {
    expect(kartKarariUret([kabuk(true)], 'bir_kez')).toBeNull();
    expect(kartKarariUret([kabuk(false)], 'oturum')).toBeNull();
  });
});

describe('BekleyenOnay', () => {
  function ciz(ogeler: OnayOgesi[]) {
    const onkarar = vi.fn(async () => {});
    const onincele = vi.fn();
    render(BekleyenOnay, { props: { ogeler, onkarar, onincele } });
    return { onkarar, onincele };
  }

  it('komut, dizin ve yalıtım görünür; "Bir kez izin ver" mühürle onaylar', async () => {
    const { onkarar } = ciz([kabuk(false)]);
    expect(screen.getByText('python -m unittest -v')).toBeTruthy();
    expect(screen.getByText('Yalıtımlı (windows AppContainer)')).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Bu oturumda izin ver' })).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'Bir kez izin ver' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'k1', muhur: 'mk', karar: { tip: 'Onayla' }, kisayol: false },
    ]);
  });

  it('düzenlemede diff görünür; "Bu oturumda izin ver" kısayolu ister', async () => {
    const { onkarar } = ciz([duzenleme]);
    expect(screen.getByText('gorevler.py')).toBeTruthy();
    expect(screen.getByText('yeni')).toBeTruthy();
    expect(screen.getByText(/src\/ altındaki dosyaları/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Bu oturumda izin ver' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'd1', muhur: 'md', karar: { tip: 'Onayla' }, kisayol: true },
    ]);
  });

  it('ağlı komut karttan onaylanamaz; yalnız ret ve İncele', async () => {
    const { onkarar, onincele } = ciz([kabuk(true)]);
    expect(screen.queryByRole('button', { name: 'Bir kez izin ver' })).toBeNull();
    expect(screen.getByText(/onayı İncele'den verin/)).toBeTruthy();
    expect(screen.getByText('Ağ: pypi.org:443')).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'İncele' }));
    expect(onincele).toHaveBeenCalled();
    await fireEvent.click(screen.getByRole('button', { name: 'Reddet' }));
    expect(onkarar).toHaveBeenCalledWith([
      { cagri: 'k1', muhur: 'mk', karar: { tip: 'Reddet', gerekce: null }, kisayol: false },
    ]);
  });

  it('birden çok öğede toplu düğmeler', () => {
    ciz([duzenleme, kabuk(false)]);
    expect(screen.getByRole('button', { name: 'Tümünü onayla' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Tümünü reddet' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Bu oturumda izin ver' })).toBeNull();
  });
});
