import { Channel } from '@tauri-apps/api/core';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import * as ana from './ana';
import { komutHatasi } from './hata';
import * as sartlar from './sartlar';

afterEach(() => clearMocks());

function kaydet(): [string, Record<string, unknown> | undefined][] {
  const cagrilar: [string, Record<string, unknown> | undefined][] = [];
  mockIPC((komut, argumanlar) => {
    cagrilar.push([komut, argumanlar as Record<string, unknown> | undefined]);
    return null;
  });
  return cagrilar;
}

describe('ana pencere komutları (1c tasarımı §6.1)', () => {
  it('komut adları ve argüman adları Rust tarafıyla aynı', async () => {
    const c = kaydet();
    await ana.oturumBaslat(7, 'glm-5.3');
    await ana.oturumAc('o-1');
    await ana.paketiIsledim('o-1', 3);
    await ana.gecmisSayfasi('o-1', 120);
    await ana.mesajGonder('o-1', 'selam');
    await ana.turuIptalEt('o-1');
    await ana.aracOnayi('o-1', [{ cagri: 'c-1', muhur: 'm', karar: { tip: 'Onayla' } }]);
    await ana.modelDegistir('o-1', 'auto');
    await ana.izinKipiDegistir('o-1', 'Proje');
    await ana.oturumuKapat('o-1');
    await ana.baglantiAc('https://ornek.com/');
    expect(c).toEqual([
      ['oturum_baslat', { calisma_alani: 7, model: 'glm-5.3' }],
      ['oturum_ac', { oturum: 'o-1' }],
      ['paketi_isledim', { oturum: 'o-1', no: 3 }],
      ['gecmis_sayfasi', { oturum: 'o-1', once_sira: 120 }],
      ['mesaj_gonder', { oturum: 'o-1', metin: 'selam' }],
      ['turu_iptal_et', { oturum: 'o-1' }],
      ['arac_onayi', { oturum: 'o-1', kararlar: [{ cagri: 'c-1', muhur: 'm', karar: { tip: 'Onayla' } }] }],
      ['model_degistir', { oturum: 'o-1', model: 'auto' }],
      ['izin_kipi_degistir', { oturum: 'o-1', kip: 'Proje' }],
      ['oturumu_kapat', { oturum: 'o-1' }],
      ['baglanti_ac', { adres: 'https://ornek.com/' }],
    ]);
  });

  it('argümansız komutlar', async () => {
    const c = kaydet();
    await ana.uygulamaDurumu();
    await ana.oturumlariListele();
    await ana.calismaAlaniSec();
    await ana.anahtarPenceresiniAc();
    await ana.sartPenceresiniAc();
    expect(c.map(([komut]) => komut)).toEqual([
      'uygulama_durumu',
      'oturumlari_listele',
      'calisma_alani_sec',
      'anahtar_penceresini_ac',
      'sart_penceresini_ac',
    ]);
  });

  it('oturumu dinlerken bir kanal gönderir', async () => {
    const c = kaydet();
    await ana.oturumuDinle('o-1', () => {});
    expect(c[0][0]).toBe('oturumu_dinle');
    expect(c[0][1]?.oturum).toBe('o-1');
    expect(c[0][1]?.kanal).toBeInstanceOf(Channel);
  });
});

describe('MCP komutları (1.1a-2 Görev 10)', () => {
  it('komut adları ve argüman adları Rust tarafıyla aynı; yol alanı yok', async () => {
    const c = kaydet();
    const form = {
      ad: 'yerel',
      komut: { tur: 'secili_yurutulebilir' as const, kimlik: 'mcp-komut-1' },
      baslatma_zamani_asimi_sn: 30,
      cagri_zamani_asimi_sn: 120,
    };
    await ana.mcpDurumu();
    await ana.mcpSunucuAyrintisi('yerel');
    await ana.mcpKomutSec();
    await ana.mcpTaslakOlustur(null, form);
    await ana.mcpTaslakGuncelle('t-1', 'eski', form);
    await ana.mcpTaslakKaydet('t-1', 2, 'o-1');
    await ana.mcpIceAktarOnizle('{}');
    await ana.mcpIceAktarGizlilik('t-1', 'yerel', 'ANAHTAR', true);
    await ana.mcpIceAktarKaydet('t-1', 3, null);
    await ana.mcpEtkinAyarla('yerel', false);
    await ana.mcpSil('yerel');
    await ana.mcpYenidenBaslat('o-1', 'yerel');
    await ana.mcpBaslatmaKartiYanitla('k-1', true);
    await ana.mcpBaslatmaIzniGeriAl('alan-1', 'yerel');
    expect(c).toEqual([
      ['mcp_durumu', {}],
      ['mcp_sunucu_ayrintisi', { ad: 'yerel' }],
      ['mcp_komut_sec', {}],
      ['mcp_taslak_olustur', { eski_ad: null, form }],
      ['mcp_taslak_guncelle', { taslak_kimligi: 't-1', eski_ad: 'eski', form }],
      ['mcp_taslak_kaydet', { taslak_kimligi: 't-1', kusak: 2, oturum: 'o-1' }],
      ['mcp_ice_aktar_onizle', { json: '{}' }],
      [
        'mcp_ice_aktar_gizlilik',
        { taslak_kimligi: 't-1', sunucu: 'yerel', ortam: 'ANAHTAR', gizli: true },
      ],
      ['mcp_ice_aktar_kaydet', { taslak_kimligi: 't-1', kusak: 3, oturum: null }],
      ['mcp_etkin_ayarla', { ad: 'yerel', etkin: false }],
      ['mcp_sil', { ad: 'yerel' }],
      ['mcp_yeniden_baslat', { oturum: 'o-1', ad: 'yerel' }],
      ['mcp_baslatma_karti_yanitla', { istek_kimligi: 'k-1', kabul: true }],
      ['mcp_baslatma_izni_geri_al', { alan_kimligi: 'alan-1', ad: 'yerel' }],
    ]);
  });

  it('kanal eski revizyonlu görüntüyü yenisinin üstüne yazmaz', async () => {
    const c = kaydet();
    const gelenler: number[] = [];
    await ana.mcpDinle((durum) => gelenler.push(durum.revizyon));
    expect(c[0][0]).toBe('mcp_dinle');
    const kanal = c[0][1]?.kanal as Channel<{ revizyon: number }>;
    expect(kanal).toBeInstanceOf(Channel);
    const bos = { sunucular: [], baslatma_kartlari: [] };
    for (const revizyon of [2, 1, 2, 3]) {
      kanal.onmessage({ revizyon, ...bos } as never);
    }
    expect(gelenler).toEqual([2, 3]);
  });
});

describe('şart penceresi komutları (1c tasarımı §6.1)', () => {
  it('komut adları ve argüman adları', async () => {
    const c = kaydet();
    await sartlar.sartPenceresiDurumu();
    await sartlar.anahtariKaydet('evren_llm_deneme');
    await sartlar.sartlariKabulEt(3);
    await sartlar.sartlariReddet();
    await sartlar.sartMetniniGoster();
    await sartlar.baglantiAc('https://ornek.com/');
    expect(c).toEqual([
      ['sart_penceresi_durumu', {}],
      ['anahtari_kaydet', { anahtar: 'evren_llm_deneme' }],
      ['sartlari_kabul_et', { surum: 3 }],
      ['sartlari_reddet', {}],
      ['sart_metnini_goster', {}],
      ['baglanti_ac', { adres: 'https://ornek.com/' }],
    ]);
  });
});

describe('komut hatası', () => {
  it('komutun döndüğü hata aynen gelir', () => {
    expect(komutHatasi({ kod: 'KayitMesgul', mesaj: 'açık' })).toEqual({
      kod: 'KayitMesgul',
      mesaj: 'açık',
    });
  });

  it("Tauri'nin düz metin hatası iç hata sayılır", () => {
    expect(komutHatasi('Command x not allowed by ACL')).toEqual({
      kod: 'Ic',
      mesaj: 'Command x not allowed by ACL',
    });
    expect(komutHatasi({ kod: 'Bilinmeyen', mesaj: 'x' }).kod).toBe('Ic');
  });
});
