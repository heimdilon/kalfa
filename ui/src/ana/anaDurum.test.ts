import { clearMocks } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { onayOgesi, SahteArka } from '../sahte/arka';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

/** Dinlenen bir oturumla açılmış ana pencere durumu. */
async function acilmis(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle({ oturum: 'o1' }, { baslik: 'İlk oturum' });
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  await durum.baslat();
  return { arka, durum };
}

describe('açılış (1c tasarımı §5.6)', () => {
  it('uygulama durumu, liste ve son açık oturum; oturum dinlenir', async () => {
    const { arka, durum } = await acilmis();
    expect(durum.uygulama?.varsayilan_model).toBe('glm-5.3');
    expect(durum.liste.map((s) => s.baslik)).toEqual(['İlk oturum']);
    expect(durum.secili).toBe('o1');
    expect(arka.dinleniyor('o1')).toBe(true);
  });

  it('salt okunur oturum dinlenmez', async () => {
    const { arka, durum } = await acilmis((a) =>
      a.oturumEkle({ oturum: 'o1', salt_okunur: { tip: 'CalismaAlaniKayitsiz' } }),
    );
    expect(durum.aktif?.salt_okunur).toEqual({ tip: 'CalismaAlaniKayitsiz' });
    expect(arka.dinleniyor('o1')).toBe(false);
  });

  it('açılamayan oturum seçilmez; hata gösterilir', async () => {
    const { durum } = await acilmis();
    await durum.oturumSec('yok');
    expect(durum.secili).toBe('o1');
    expect(durum.hata).toBe('Bu oturumun kaydı bulunamadı.');
  });
});

describe('kanal', () => {
  it('olaylar ekrana uygulanır, "işledim" hemen gider', async () => {
    const { arka, durum } = await acilmis();
    arka.olaylar('o1', [{ tip: 'TurBasladi' }, { tip: 'TokenParcasi', metin: 'Merhaba' }]);
    expect(durum.aktif?.anlik).toBe('Calisiyor');
    expect(durum.aktif?.ogeler.at(-1)).toMatchObject({ tip: 'Yanit', metin: 'Merhaba' });
    expect(arka.komutlar('paketi_isledim')).toEqual([{ oturum: 'o1', no: 1 }]);
  });

  it('paket numarasında boşlukta oturum kayıttan yeniden açılır (§4.4)', async () => {
    const { arka, durum } = await acilmis();
    arka.paket('o1', { tip: 'Olaylar', no: 5, zarflar: [] });
    await new Promise((r) => setTimeout(r, 0));
    expect(arka.komutlar('oturumu_kapat')).toEqual([{ oturum: 'o1' }]);
    expect(arka.komutlar('oturum_ac')).toHaveLength(2);
    expect(arka.komutlar('oturumu_dinle')).toHaveLength(2);
    expect(durum.aktif?.bosluk).toBe(false);
    expect(arka.komutlar('paketi_isledim')).toEqual([]);
  });

  it('onay isteği ve tur sonu ekran okuyucuya duyurulur', async () => {
    const { arka, durum } = await acilmis();
    arka.olaylar('o1', [
      {
        tip: 'OnayGerekiyor',
        islem: 'i1',
        ogeler: [onayOgesi({ cagri: 'c1', muhur: 'm' })],
      },
    ]);
    expect(durum.duyuru).toBe('Onay isteği: 1 değişiklik kararınızı bekliyor.');
    arka.olaylar('o1', [{ tip: 'Tamamlandi', neden: { tip: 'Normal' } }]);
    expect(durum.duyuru).toBe('Tur bitti.');
  });

  it('tur bitince liste yenilenir', async () => {
    const { arka } = await acilmis();
    const once = arka.komutlar('oturumlari_listele').length;
    arka.olaylar('o1', [{ tip: 'Tamamlandi', neden: { tip: 'Normal' } }]);
    await Promise.resolve();
    expect(arka.komutlar('oturumlari_listele').length).toBe(once + 1);
  });
});

describe('komutlar', () => {
  it('mesaj önce ekrana eklenir, sonra gönderilir', async () => {
    const { arka, durum } = await acilmis();
    await durum.gonder('selam');
    expect(durum.aktif?.ogeler.at(-1)).toMatchObject({
      tip: 'KullaniciMesaji',
      metin: 'selam',
    });
    expect(arka.komutlar('mesaj_gonder')).toEqual([{ oturum: 'o1', metin: 'selam' }]);
  });

  it('onay kararları gönderilir, kart yanıtlandı olur, sayfa kapanır', async () => {
    const { arka, durum } = await acilmis();
    arka.olaylar('o1', [
      {
        tip: 'OnayGerekiyor',
        islem: 'i1',
        ogeler: [onayOgesi({ cagri: 'c1', muhur: 'm' })],
      },
    ]);
    durum.onayiAc('o1', 'i1');
    const kararlar = [{ cagri: 'c1', muhur: 'm', karar: { tip: 'Onayla' as const } }];
    await durum.onayla('o1', 'i1', kararlar);
    expect(arka.komutlar('arac_onayi')).toEqual([{ oturum: 'o1', kararlar }]);
    expect(durum.aktif?.ogeler.at(-1)).toMatchObject({
      tip: 'OnayIstegi',
      durum: 'Yanitlandi',
    });
    expect(durum.onay).toBeNull();
  });

  it('karar gönderilemezse kart bekler, sayfa açık kalır', async () => {
    const { arka, durum } = await acilmis((a) =>
      a.hatalar.set('arac_onayi', { kod: 'OturumYok', mesaj: 'Bu oturum açık değil.' }),
    );
    durum.onayiAc('o1', 'i1');
    await durum.onayla('o1', 'i1', []);
    expect(durum.onay).not.toBeNull();
    expect(durum.hata).toBe('Bu oturum açık değil.');
    expect(arka.komutlar('arac_onayi')).toHaveLength(1);
  });

  it('kip yükseltmesi yerel kutuda reddedilirse eski kip kalır', async () => {
    const { durum } = await acilmis((a) => (a.yukseltmeOnayi = false));
    await durum.kipSec('Serbest');
    expect(durum.aktif?.ozet.kip).toBe('Siki');
    await durum.kipSec('Inceleme');
    expect(durum.aktif?.ozet.kip).toBe('Inceleme');
  });

  it('yeni oturum: klasör seçilmezse hiçbir şey başlatılmaz', async () => {
    const { arka, durum } = await acilmis((a) => (a.klasor = null));
    await durum.yeniOturum();
    expect(arka.komutlar('oturum_baslat')).toEqual([]);
    expect(durum.secili).toBe('o1');
  });

  it('yeni oturum varsayılan modelle başlar, seçilir ve dinlenir', async () => {
    const { arka, durum } = await acilmis();
    await durum.yeniOturum();
    expect(arka.komutlar('oturum_baslat')).toEqual([{ calisma_alani: 1, model: 'glm-5.3' }]);
    expect(durum.secili).toBe('oturum-yeni-1');
    expect(arka.dinleniyor('oturum-yeni-1')).toBe(true);
  });

  it('eski sayfa en eski öğeden öncesini ister ve başa ekler', async () => {
    const { arka, durum } = await acilmis((a) =>
      a.oturumEkle(
        {
          oturum: 'o1',
          gecmis: {
            ogeler: [{ tip: 'KullaniciMesaji', sira: 50, tur: 't2', metin: 'yeni' }],
            daha_eski_var: true,
          },
        },
        {},
        {
          ogeler: [{ tip: 'KullaniciMesaji', sira: 10, tur: 't1', metin: 'eski' }],
          daha_eski_var: false,
        },
      ),
    );
    await durum.eskileriYukle('o1');
    expect(arka.komutlar('gecmis_sayfasi')).toEqual([{ oturum: 'o1', once_sira: 50 }]);
    expect(durum.aktif?.ogeler.map((o) => o.sira)).toEqual([10, 50]);
    expect(durum.aktif?.daha_eski_var).toBe(false);
  });

  it('kapatılan oturum ekrandan çıkar', async () => {
    const { arka, durum } = await acilmis();
    await durum.kapat('o1');
    expect(arka.komutlar('oturumu_kapat')).toEqual([{ oturum: 'o1' }]);
    expect(durum.secili).toBeNull();
    expect(durum.ekranlar.has('o1')).toBe(false);
  });
});
