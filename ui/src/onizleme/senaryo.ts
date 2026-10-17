// Önizlemenin sahte verisi ve senaryolu turu. Yalnızca `onizleme.html`
// kullanır; uygulama paketine girmez. Metinler örnektir, EVREN'den gelmez.
import type { EklentiIstekOzeti, EklentiTarifi, Kisayol, Olay, OturumId } from '../ipc/tipler';
import { bosOzet, onayOgesi, sahteAlanKimligi, SahteArka } from '../sahte/arka';

const SAAT = 3600_000;
const GUN = 24 * SAAT;

/** `kalfa-rag`ın bir çalışma alanındaki ilk EVREN gönderimi (aşama 4 tasarımı §8.4). */
const RAG_TARIFI: EklentiTarifi = {
  ad: 'kalfa-rag',
  surum: '0.1.0',
  ozet: '9f2c4e7a1b8d03a65c4f7e2b90d1a3c8e5f6b7a4d2c1908e3f4a5b6c7d8e9f01',
  globlar: ['**/*.md', '**/*.rs'],
  evren_uclari: ['embeddings', 'rerank'],
  ilk_gonderim: true,
  dosya_sayisi: 412,
  bayt: 6_000_000,
};

const RAG_ACIKLAMA = [
  'kalfa-rag eklentisinin `anlamsal_ara` aracını çalıştırmak istiyor.',
  "**kalfa-rag** bu çalışma alanında ilk kez EVREN'e veri gönderecek.",
  "Okuyabileceği dosyalar: `**/*.md`, `**/*.rs` ile eşleşen **412 dosya (6,0 MB)**. Bu dosyalardan seçtiği parçalar gömme ve yeniden sıralama için EVREN'e gidecek. Gizli dosyalar gönderilmez.",
  'İzin verirseniz bu çalışma alanında bir daha sorulmaz. İzni Eklentiler ekranından geri alabilirsiniz.',
].join('\n\n');

/** "Göndermeden önce göster" kartının eklenti isteği (aşama 4 tasarımı §9.4). */
const RAG_ISTEK: EklentiIstekOzeti = {
  eklenti: 'kalfa-rag',
  uc: 'Gomme',
  model: 'qwen3-embedding-8b',
  metin_sayisi: 14,
  bayt: 412_000,
  ozet: '5d41402abc4b2a76b9719d911017c592e4d7a3b1f0c8e92a6b3d5f7a1c4e8b20',
  dosyalar: ['docs/README.md', 'src/auth.rs', 'src/depo.rs'],
  dosya_fazlasi: 409,
};

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

/**
 * `windows`: Ayarlar Windows'taki gibi görünür (`onizleme.html#windows`).
 * `gunlukSinir`: durum çubuğunda EVREN'in günlük sınırı dolu görünür
 * (`onizleme.html#sinir`).
 */
export function anaSenaryosu(windows = false, gunlukSinir = false): SahteArka {
  const arka = new SahteArka();
  const simdi = Date.now();
  arka.uygulama.son_oturum = 'o1';
  if (gunlukSinir) arka.gunlukSinir = { sifirlanma: new Date(simdi + 5 * SAAT).toISOString() };
  // "Bu oturumdaki izinler" paneli (aşama 4 tasarımı §14): Eklentiler ekranındaki
  // örnek verinin bu çalışma alanındaki hâli.
  arka.oturumEklentileri = [
    {
      ad: 'kalfa-rag',
      yerlesik: true,
      acik: true,
      kullanilamaz: false,
      gonderim_izni_ms: simdi - 2 * GUN,
    },
    { ad: 'ornek-notlar', yerlesik: false, acik: false, kullanilamaz: false, gonderim_izni_ms: null },
    { ad: 'bozuk-eklenti', yerlesik: false, acik: false, kullanilamaz: true, gonderim_izni_ms: null },
  ];
  // Yerel MCP sunucuları (1.1a tasarımı §9) ve `o1`'in alanında bekleyen
  // başlatma kartı (§5.1).
  const mcpAlani = sahteAlanKimligi('C:\\projeler\\kalfa-api');
  arka.mcp.sunucular = [
    {
      ozet: { ad: 'notlar', etkin: true, kip: 'yalitimli', gizli_ortam_sayisi: 1 },
      alanlar: [
        {
          alan_kimligi: mcpAlani,
          durum: 'calisiyor',
          kusak: 2,
          hata: null,
          izin: 'yalitimli',
          arac_sayisi: 3,
        },
      ],
    },
    {
      ozet: { ad: 'yerel-kabuk', etkin: false, kip: 'tam_erisim', gizli_ortam_sayisi: 0 },
      alanlar: [],
    },
  ];
  arka.mcp.baslatma_kartlari = [
    {
      istek_kimligi: 'mcp-istek-1',
      alan_kimligi: mcpAlani,
      sunucu: 'takvim',
      kip: 'yalitimli',
      erisim_kipi: 'Yalıtılmış süreç',
      komut_ozeti: 'npx -y @ornek/takvim-mcp@1.4.0',
      ag_listesi: ['api.takvim.ornek:443'],
      uyari:
        'Sunucu başladığı andan kapanana kadar, araç çağrılmasa da çalışır; çalışma alanına yazabilir ve izinli ağa çıkabilir.',
      wde_kodlu: false,
    },
  ];
  // Ayarlar (aşama 2 tasarımı §10.4): "Yeniden dene" yalıtımı bulur.
  if (windows) {
    arka.uygulama.platform = 'windows';
    arka.aracZinciri = ['C:\\Program Files\\nodejs'];
    // Yükseltmeden sonra dizinin izni gitmiş: Gelişmiş'te Onar görünür.
    arka.aracOnarimGerekli = true;
    arka.sonrakiYalitim = { tip: 'Hazir', arka_uc: 'windows AppContainer', surum: '10.0.26200' };
  } else {
    arka.aracZinciri = ['/opt/node'];
    arka.sonrakiYalitim = { tip: 'Hazir', arka_uc: 'bwrap', surum: '0.9.0' };
  }
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
    // "eklenti" geçen mesaj bir eklenti aracı çağırır (aşama 4): önce ilk
    // gönderim onayı, onaydan sonra "Göndermeden önce göster" kartı.
    if (String(a.metin).includes('eklenti')) {
      gecikmeli([
        [200, () => gonder([{ tip: 'TurBasladi' }])],
        [
          300,
          () => gonder([{ tip: 'TokenParcasi', metin: 'Anlamsal arama için eklentiyi çalıştırıyorum.' }]),
        ],
        [
          400,
          () =>
            gonder([
              {
                tip: 'OnayGerekiyor',
                islem: `islem-${tur}`,
                ogeler: [
                  onayOgesi({
                    cagri: `e${tur}`,
                    arac: 'kalfa_rag__anlamsal_ara',
                    aciklama: RAG_ACIKLAMA,
                    yerel_onay: true,
                    veri_cikisi: 'EklentiCiktisi',
                    eklenti: RAG_TARIFI,
                  }),
                ],
              },
            ]),
        ],
      ]);
      return null;
    }
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
                      alan_hazirligi: false,
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
                  kisayol: { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' },
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

  // "Bu oturumda izin ver" kısayolları (3e): kabul edilen teklif listeye girer.
  let kisayollar: Kisayol[] = [];
  arka.ozel.set('kisayolu_kaldir', (a) => {
    kisayollar = kisayollar.filter((k) => k.kimlik !== a.kimlik);
    arka.olaylar(a.oturum as OturumId, [{ tip: 'Kisayollar', liste: kisayollar }]);
    return null;
  });

  arka.ozel.set('arac_onayi', (a) => {
    const oturum = a.oturum as OturumId;
    const kararlar = a.kararlar as { cagri: string; karar: { tip: string }; kisayol?: boolean }[];
    const t = `t${tur}`;
    const gonder = (olaylar: Olay[]) => arka.olaylar(oturum, olaylar, t);
    if (kararlar.some((k) => k.kisayol)) {
      kisayollar = [
        { kimlik: 1, teklif: { tip: 'Dosya', arac: 'dosya_duzenle', dizin: 'src' } },
      ];
      gonder([{ tip: 'Kisayollar', liste: kisayollar }]);
    }
    const eklenti = kararlar.find((k) => k.cagri.startsWith('e'));
    if (eklenti) {
      const cagri = eklenti.cagri;
      if (eklenti.karar.tip !== 'Onayla') {
        gecikmeli([
          [
            200,
            () =>
              gonder([
                {
                  tip: 'AracSonucu',
                  cagri,
                  arac: 'kalfa_rag__anlamsal_ara',
                  durum: 'Reddedildi',
                  icerik: 'Kullanıcı eklentinin gönderimini reddetti.',
                },
                { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
              ]),
          ],
        ]);
        return null;
      }
      gecikmeli([
        [
          200,
          () =>
            gonder([
              {
                tip: 'AracCagrisiBasladi',
                cagri,
                arac: 'kalfa_rag__anlamsal_ara',
                aciklama: 'kalfa-rag eklentisinin `anlamsal_ara` aracını çalıştırmak istiyor.',
              },
            ]),
        ],
        [
          400,
          () =>
            gonder([
              {
                tip: 'GonderimOnizlemesi',
                islem: `onizleme-${tur}`,
                neden: 'Eklenti',
                ogeler: [
                  { cagri: `${cagri}-gomme`, kaynak: 'eklenti:kalfa-rag:gomme', bayt: 412_000, eklenti: RAG_ISTEK },
                ],
              },
            ]),
        ],
      ]);
      return null;
    }
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
            // Tur özeti ve "Turu geri al…" (aşama 3 tasarımı §5.11).
            {
              tip: 'TurOzeti',
              ozet: {
                tur: t,
                dosyalar: kararlar
                  .filter((k) => k.karar.tip === 'Onayla')
                  .map((_, i) => ({
                    yol: i === 0 ? 'src/auth.rs' : 'src/depo.rs',
                    islem: 'Degisti' as const,
                    eklenen: i === 0 ? 1 : 2,
                    silinen: i === 0 ? 1 : 2,
                    sonucu_bilinmiyor: false,
                    geri_alinabilir: { tip: 'Evet' as const },
                  })),
                komutlar: [],
                istekler: { sayi: 2, istem: 3120, tamamlama: 640, dusunce: 210 },
                guvenlik_uyarisi: 0,
              },
            },
          ]),
      ],
    ]);
    return null;
  });
  // Eklenti önizlemesinin kararı: gönderilirse arama sonuç verir, gönderilmezse eklenti hata alır.
  arka.ozel.set('gonderim_karari', (a) => {
    const oturum = a.oturum as OturumId;
    const gonderilmedi = (a.gonderilmeyecekler as string[]).length > 0;
    gecikmeli([
      [
        300,
        () =>
          arka.olaylar(
            oturum,
            [
              {
                tip: 'AracSonucu',
                cagri: `e${tur}`,
                arac: 'kalfa_rag__anlamsal_ara',
                durum: gonderilmedi ? 'Basarisiz' : 'Basarili',
                icerik: gonderilmedi
                  ? 'Eklenti hatası (kalfa-rag): izin-yok: kullanıcı göndermedi'
                  : 'dosya:src/auth.rs:12-40\nconst TOKEN_SURESI: Duration = Duration::from_secs(24 * 3600);',
              },
              {
                tip: 'TokenParcasi',
                metin: gonderilmedi
                  ? 'Anlamsal arama yapılamadı; gönderim onaylanmadı.'
                  : 'En yakın sonuç `src/auth.rs` içinde.',
              },
              { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
            ],
            `t${tur}`,
          ),
      ],
    ]);
    return null;
  });
  // Geri alma sayfasının önizlemesi: bir dosya geri döner, biri çakışır.
  arka.geriAlmaPlani = {
    ogeler: [
      {
        yol: 'src/auth.rs',
        eylem: { tip: 'GeriYukle', ozet: 'onceki' },
        simdiki: 'simdiki',
        uyarilar: [],
        diff: [
          '--- b/src/auth.rs',
          '+++ a/src/auth.rs',
          '@@ -3,1 +3,1 @@',
          '-const TOKEN_SURESI: Duration = Duration::from_secs(15 * 60);',
          '+const TOKEN_SURESI: Duration = Duration::from_secs(7 * 24 * 3600);',
        ].join('\n'),
      },
      {
        yol: 'src/depo.rs',
        eylem: { tip: 'Cakisma', neden: { tip: 'Elle' } },
        simdiki: 'elle',
        uyarilar: [],
        diff: null,
      },
    ],
    veri_silindi: false,
  };
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
