// Önizlemenin sahte verisi ve senaryolu turu. Yalnızca `onizleme.html`
// kullanır; uygulama paketine girmez. Metinler örnektir, EVREN'den gelmez.
import type { Olay, OturumId } from '../ipc/tipler';
import { bosOzet, onayOgesi, SahteArka } from '../sahte/arka';

const SAAT = 3600_000;
const GUN = 24 * SAAT;

const YANIT = [
  'Kimlik doğrulama akışını inceledim. Genel yapı sağlam; iki yerde düzeltme öneriyorum:\n\n',
  '1. `src/auth.rs` içinde token süresi **7 gün** olarak ayarlanmış; 24 saat yeterli.\n',
  '2. Yenileme token’ları kayıtta düz metin tutuluyor.\n\n',
  '```rust\nconst TOKEN_SURESI: Duration = Duration::from_secs(24 * 3600);\n```\n\n',
  'Bu iki değişikliği yapmak için onayınızı isteyeceğim.',
];

const DIFF_AUTH = [
  '@@ -12,6 +12,6 @@ use std::time::Duration;',
  ' ',
  ' /// Erişim token’ının geçerlilik süresi.',
  '-const TOKEN_SURESI: Duration = Duration::from_secs(7 * 24 * 3600);',
  '+const TOKEN_SURESI: Duration = Duration::from_secs(24 * 3600);',
  ' ',
  ' pub fn token_uret(kullanici: &Kullanici) -> Token {',
].join('\n');

const DIFF_DEPO = [
  '@@ -40,7 +40,8 @@ impl TokenDeposu {',
  '     pub fn kaydet(&self, token: &YenilemeTokeni) -> Result<()> {',
  '-        self.db.ekle(token.deger.clone())',
  '+        let ozet = sha256(token.deger.as_bytes());',
  '+        self.db.ekle(hex::encode(ozet))',
  '     }',
].join('\n');

/** Kabuk çağrısının örnek çıktısı (aşama 2 tasarımı §10.1): renkli satırlar, stderr. */
const KABUK_CIKTISI: [string, 'Standart' | 'Hata'][] = [
  ['   Compiling kalfa-api v0.1.0 (C:\\projeler\\kalfa-api)\n', 'Hata'],
  ['    Finished `test` profile [unoptimized] target(s) in 4.21s\n', 'Hata'],
  ['     Running unittests src\\lib.rs\n', 'Hata'],
  ['\nrunning 3 tests\n', 'Standart'],
  ['test auth::token_suresi_24_saat ... \x1b[32mok\x1b[0m\n', 'Standart'],
  ['test depo::token_ozetlenir ... \x1b[32mok\x1b[0m\n', 'Standart'],
  ['test depo::eski_kayit_okunur ... \x1b[1;31mFAILED\x1b[0m\n', 'Standart'],
  ['\ntest result: \x1b[31mFAILED\x1b[0m. 2 passed; 1 failed\n', 'Standart'],
];

function gecikmeli(adimlar: [number, () => void][]) {
  let toplam = 0;
  for (const [ms, is] of adimlar) {
    toplam += ms;
    setTimeout(is, toplam);
  }
}

export function anaSenaryosu(): SahteArka {
  const arka = new SahteArka();
  const simdi = Date.now();
  arka.uygulama.son_oturum = 'o1';
  // Ayarlar (aşama 2 tasarımı §10.4): "Yeniden dene" yalıtımı bulur.
  arka.aracZinciri = ['/opt/node'];
  arka.sonrakiYalitim = { tip: 'Hazir', arka_uc: 'bwrap', surum: '0.9.0' };
  arka.oturumEkle(
    {
      oturum: 'o1',
      calisma_alani: 'C:\\projeler\\kalfa-api',
      gecmis: {
        ogeler: [
          {
            tip: 'KullaniciMesaji',
            sira: 1,
            tur: 't1',
            metin: 'Kimlik doğrulama akışını gözden geçirir misin? Özellikle token sürelerine bak.',
          },
          {
            tip: 'Yanit',
            sira: 2,
            tur: 't1',
            metin: 'Önce ilgili dosyayı okuyorum.',
            dusunce: 'Token süresi `src/auth.rs` içinde tanımlı olmalı; önce onu okuyacağım.',
          },
          {
            tip: 'AracCagrisi',
            sira: 3,
            tur: 't1',
            model_cagri_id: 'call_1',
            arac: 'dosya_oku',
            argumanlar: '{"yol":"src/auth.rs"}',
            durum: 'Basarili',
            sonuc:
              'use std::time::Duration;\n\n/// Erişim token’ının geçerlilik süresi.\n' +
              'const TOKEN_SURESI: Duration = Duration::from_secs(7 * 24 * 3600);\n',
          },
          { tip: 'TurSonu', sira: 4, tur: 't1', neden: { tip: 'Normal' } },
        ],
        daha_eski_var: false,
      },
      ozet: {
        ...bosOzet('glm-5.3', 'Siki'),
        yonlendirilen_model: 'glm-5.3',
        son_kredi: { kalan: '1250.00', tutulan: null, istek_id: 'req_1', zaman_ms: simdi - 5 * 60_000 },
        istem: 12840,
        tamamlama: 2210,
        dusunce: 930,
      },
    },
    {
      baslik: 'Kimlik doğrulama akışını gözden geçirir misin? Özellikle token sürelerine bak.',
      durum: { tip: 'Tamamlandi' },
      son_degisiklik_ms: simdi - 10 * 60_000,
    },
  );
  arka.oturumEkle(
    { oturum: 'o2', calisma_alani: 'C:\\projeler\\veritabani' },
    { baslik: 'Veritabanı şemasını düzenle', durum: { tip: 'YarimKaldi' }, son_degisiklik_ms: simdi - 3 * SAAT },
  );
  arka.oturumEkle(
    {
      oturum: 'o3',
      calisma_alani: 'C:\\eski\\proje',
      salt_okunur: { tip: 'CalismaAlaniYok', yol: 'C:\\eski\\proje' },
    },
    { baslik: 'Hata analizi: 500 yanıtları', durum: { tip: 'Hata' }, son_degisiklik_ms: simdi - GUN },
  );
  arka.oturumEkle(
    { oturum: 'o4', calisma_alani: 'C:\\projeler\\belgeler' },
    { baslik: 'Kurulum belgesini güncelle', durum: { tip: 'Tamamlandi' }, son_degisiklik_ms: simdi - 3 * GUN },
  );
  arka.satirEkle({
    oturum: null,
    dosya: 'C:\\veri\\oturumlar\\bozuk-kayit.jsonl',
    baslik: null,
    calisma_alani: null,
    son_degisiklik_ms: simdi - 20 * GUN,
    durum: { tip: 'Okunamadi', neden: '3. satır çözülemedi' },
  });

  let tur = 1;
  arka.ozel.set('mesaj_gonder', (a) => {
    const oturum = a.oturum as OturumId;
    tur += 1;
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    // "test" geçen mesaj kabuk çağrısı ister (aşama 2 tasarımı §10).
    if (String(a.metin).includes('test')) {
      gecikmeli([
        [200, () => gonder([{ tip: 'TurBasladi' }])],
        [300, () => gonder([{ tip: 'TokenParcasi', metin: 'Testleri çalıştırıyorum.' }])],
        [
          400,
          () =>
            gonder([
              {
                tip: 'OnayGerekiyor',
                islem: `islem-${tur}`,
                ogeler: [
                  onayOgesi({
                    cagri: `k${tur}`,
                    arac: 'kabuk',
                    aciklama: '`cargo test --lib` komutunu çalıştırmak istiyor.',
                    uyarilar: [
                      'Yalıtımsız: komut senin bütün yetkilerinle çalışacak; ev dizinindeki dosyaları okuyabilir ve ağa erişebilir.',
                    ],
                    kabuk: {
                      komut: 'cargo test --lib',
                      bicim: { tip: 'Program' },
                      dizin: '.',
                      ag: false,
                      yalitim: { tip: 'Yalitimsiz', talimat: arka.uygulama.yalitim.tip === 'Yok' ? arka.uygulama.yalitim.talimat : '' },
                      ilk_cagri: false,
                    },
                    yerel_onay: true,
                  }),
                ],
              },
            ]),
        ],
      ]);
      return null;
    }
    gecikmeli([
      [200, () => gonder([{ tip: 'TurBasladi' }])],
      [300, () => gonder([{ tip: 'DusunceParcasi', metin: 'Token süresi ve depolama ' }])],
      [300, () => gonder([{ tip: 'DusunceParcasi', metin: 'iki ayrı dosyada; ikisini de düzeltmeliyim.' }])],
      ...YANIT.map((p): [number, () => void] => [350, () => gonder([{ tip: 'TokenParcasi', metin: p }])]),
      [
        400,
        () =>
          gonder([
            {
              tip: 'OnayGerekiyor',
              islem: `islem-${tur}`,
              ogeler: [
                onayOgesi({
                  cagri: `c${tur}-1`,
                  arac: 'dosya_duzenle',
                  aciklama: 'Erişim token’ının süresini 7 günden 24 saate indirir.',
                  hedefler: ['C:\\projeler\\kalfa-api\\src\\auth.rs'],
                  diff: DIFF_AUTH,
                }),
                onayOgesi({
                  cagri: `c${tur}-2`,
                  arac: 'dosya_duzenle',
                  aciklama: 'Yenileme token’larını kayda yazmadan önce özetler.',
                  hedefler: ['C:\\projeler\\kalfa-api\\src\\depo.rs'],
                  diff: DIFF_DEPO,
                  uyarilar: ['Mevcut kayıtlı token’lar geçersiz olur; kullanıcılar yeniden giriş yapar.'],
                }),
              ],
            },
          ]),
      ],
    ]);
    return null;
  });

  arka.ozel.set('arac_onayi', (a) => {
    const oturum = a.oturum as OturumId;
    const kararlar = a.kararlar as { cagri: string; karar: { tip: string } }[];
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    const kabuk = kararlar.find((k) => k.cagri.startsWith('k'));
    if (kabuk) {
      const cagri = kabuk.cagri;
      gecikmeli([
        [
          200,
          () =>
            gonder([
              {
                tip: 'AracCagrisiBasladi',
                cagri,
                arac: 'kabuk',
                aciklama: '`cargo test --lib` komutunu çalıştırmak istiyor.',
              },
            ]),
        ],
        ...KABUK_CIKTISI.map(([metin, akis]): [number, () => void] => [
          350,
          () => gonder([{ tip: 'AracCiktisiParcasi', cagri, akis, metin, atlanan_bayt: 0 }]),
        ]),
        [
          300,
          () =>
            gonder([
              {
                tip: 'AracSonucu',
                cagri,
                arac: 'kabuk',
                durum: 'Basarisiz',
                icerik: '[kaynak: kabuk:cargo]\nÇıkış kodu: 101\nSüre: 5,3 sn\n--- çıktı ---\n…',
              },
              { tip: 'TokenParcasi', metin: 'Bir test başarısız: `depo::eski_kayit_okunur`.' },
              { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
            ]),
        ],
      ]);
      return null;
    }
    gecikmeli([
      [
        300,
        () =>
          gonder(
            kararlar.map((k, i) => ({
              tip: 'AracSonucu',
              cagri: k.cagri,
              arac: 'dosya_duzenle',
              durum: k.karar.tip === 'Onayla' ? 'Basarili' : 'Reddedildi',
              icerik: i === 0 ? 'src/auth.rs: 1 satır değişti' : 'src/depo.rs: 2 satır değişti',
            })),
          ),
      ],
      [400, () => gonder([{ tip: 'TokenParcasi', metin: 'Değişiklikler uygulandı.' }])],
      [
        300,
        () =>
          gonder([
            { tip: 'TokenKullanimi', istem: 3120, tamamlama: 640, dusunce: 210 },
            {
              tip: 'KrediGozlemi',
              gozlem: { kalan: '1248.75', tutulan: null, istek_id: 'req_2', zaman_ms: Date.now() },
            },
            { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
          ]),
      ],
    ]);
    return null;
  });
  return arka;
}

const SART_METNI = [
  '# EVREN Kullanım Şartları',
  '',
  'Bu metin önizleme için yazılmış örnek bir metindir.',
  '',
  '## 1. Hizmetin kapsamı',
  '',
  'EVREN, dil modellerine tek bir uç üzerinden erişim sağlar. İstekler hesabınıza bağlı anahtarla gönderilir.',
  '',
  '## 2. Kullanım koşulları',
  '',
  '- Anahtarınızı başkalarıyla paylaşmayın.',
  '- Kişisel veri içeren içeriği göndermeden önce gerekli izinleri alın.',
  '- Kotalar ve kredi kullanımı portalda gösterilir.',
  '',
  '![Durum rozeti](https://img.shields.io/badge/durum-aktif-green)',
  '',
  '## 3. Değişiklikler',
  '',
  'Şartlar değiştiğinde yeni sürüm kabul edilene kadar istekler yanıtlanmaz. Ayrıntılar için [EVREN portalı](https://evren.example/sartlar).',
].join('\n');

export function sartSenaryosu(durum: string): SahteArka {
  const arka = new SahteArka();
  arka.sartMetni = { surum: 3, esasli: true, metin: SART_METNI };
  if (durum === 'sartlar-anahtar') arka.sart = { tip: 'AnahtarGerekli', neden: 'Yok' };
  else if (durum === 'sartlar-kilitli') arka.sart = { tip: 'Kilitli' };
  else arka.sart = { tip: 'SartGerekli', metin: arka.sartMetni };
  return arka;
}
