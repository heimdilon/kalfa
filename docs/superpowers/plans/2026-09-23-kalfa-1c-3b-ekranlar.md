# Kalfa 1c-3b — Ekranlar Uygulama Planı

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** İki pencerenin ekranlarını yazıp komutlara bağlamak: ana pencere (menü, oturum listesi, konuşma, onay kartı ve onay sayfası, durum çubuğu) ve şart penceresi (anahtar formu, EVREN şart metni, kilitli görünüm).

**Architecture:** Her pencerenin bir durum sınıfı vardır (`ana/anaDurum.svelte.ts`, `sartlar/sartDurum.svelte.ts`; Svelte 5 runes). Sınıf komutları çağırır, kanalları dinler ve hataları toplar; bileşenler yalnızca onun alanlarını çizer ve yöntemlerini çağırır. Her açık oturumun ekranı 1c-3a'daki saf `uygula` işleviyle güncellenir. Sahte bir arka uç (`sahte/arka.ts`), `mockIPC` ile Rust komutlarını ve kanalları taklit eder; testler ve yalnızca geliştirmede açılan ekran önizlemesi (`onizleme.html`) onu kullanır. Uygulama paketine girmez.

**Tech Stack:** Svelte 5.57 (runes, `svelte/reactivity`) · TypeScript 6.0 · Vite 8.3 · Vitest 5 + @testing-library/svelte + jsdom · @tauri-apps/api 2.11 (`mockIPC`) · 1c-3a'nın bileşenleri, durum işlevi ve Markdown çizicisi

**Spec:** `docs/superpowers/specs/2026-09-22-kalfa-1c-design.md` (1c tasarımı) — §5.4–§5.6, §6.1, §7, §8, §9.3–§9.5, §10, §11.3, §11.5.

**Kod hakkında:** Bu plandaki kodun tamamı plan yazılırken ayrı bir kopyada derlendi ve doğrulandı (2026-09-23).
- **Windows 11, Node 24.20:** plan commit'inden başlanarak her görevin sonunda `npm run check` (0 hata, 0 uyarı), `npm test` ve `npm run build` (paket denetimi dahil) geçti. Son durumda 14 dosya, 167 test.
- **Linux (Ubuntu 24.04, WSL2, Node 24.20):** son durumda aynı sonuç.
- **Tarayıcıda (ekran önizlemesi):** senaryolu tur oynatıldı ve tasarım sisteminin görselleriyle karşılaştırıldı. Tur akan yanıt, düşünce, onay isteği, onay sayfası ve karar adımlarından geçti. Şart penceresinin üç hâli de ayrıca görüldü.

Kodu olduğu gibi kullan. Bir şey yine de derlenmezse testin sınadığı davranışı değiştirmeden düzelt; davranış değişmesi gerekiyorsa dur ve PR'a yorum yaz.

**Bu planın kapsamı dışında:** Rust tarafı (1c-2) · gerçek pencerede uçtan uca testler ve kabul ölçümleri (1c-4) · sonraki aşamaların ekranları (dosya paneli, gönderim günlüğü, geri alma, eklentiler).

## PR ile yürütme

Kullanıcı kararı (2026-09-23): kodu Codex yazar, Claude inceler.

- **Dal ve PR:** çalışma dalı `1c-3b/ekranlar`, PR'ın hedefi `kalfa-asama-1`.
- **Hazır gelenler (plan commit'i):** plan, tasarım belgesi §6.1 ve `ui/src/ipc/{tipler,sartlar}.ts`'deki sözleşme düzeltmesi (`KabulYaniti`'ne `Kilitli`; `sart_metnini_goster` → `SartPenceresiDurumu`). Düzeltme 1c-2 PR'ındakiyle birebir aynıdır; iki dal çakışmadan birleşir. Bu dosyaları değiştirme.
- **Ortam:**
  - **Node:** `^22.22.2 || >=24.15.0`, npm 10 ya da üstü. `npm ci` için ağ erişimi gerekir.
  - **Rust gerekmez:** `cargo` komutları bu PR'da koşturulmaz.
  - **Bağımlılık:** yeni bağımlılık yok; `package.json` ve `package-lock.json` değişmez. Biçimlendirici (Prettier ve benzeri) projede yok. Koşturma; kod plandaki biçimiyle yazılır (tek tırnak, iki boşluk girinti).
- **Sıra:** görevler sırayla. Her görev planda yazan mesajla kendi commit'ini alır; her commit'ten sonra push'la. Geçmiş yeniden yazılmaz, force-push yapılmaz.
- **Soru ve sapma:** plandan sapman gerekirse dur ve PR'a yorum yaz; kendi başına karar verme.
- **İnceleme:** Claude PR'a yorum yazar, Windows'ta ve Linux'ta doğrular, önizlemeyi tarayıcıda açıp tasarımla karşılaştırır; inceleme temizse birleştirir.

## Global Constraints

- **Araçlar:** Node `^22.22.2 || >=24.15.0`; npm; kurulum `npm ci`. `package.json` ve `package-lock.json` değişmez.
- **Şartlar (üst belge §11.8):** Kalfa şartları kullanıcı adına kabul etmez. `sartlari_kabul_et` yalnızca `SartDurum.kabul`'den, yani "Kabul ediyorum" düğmesinden çağrılır. Testler, bu düğmeye basılmadan kabul gönderilmediğini sınar.
- **Anahtar (1c tasarımı §7.2):** anahtar gönderilince form alanı boşaltılır. Anahtar hiçbir durum alanında, günlükte ya da komut yanıtında tutulmaz.
- **Paket ayrımı (1c tasarımı §6):** şart penceresinin kodu yalnızca `ipc/sartlar`'ı içe aktarır. `scripts/paket-denetimi.mjs` şart penceresinin paketinde ana pencere komut adı arar. Bu yüzden dil kataloğundaki anahtar ve metinler `oturum_ac`, `sart_penceresini_ac` gibi komut adlarını alt dizgi olarak içermez.
- **CSP (1c-3a):** şablonda sabit `style="…"` yok; `{@html}`, `innerHTML`, `eval` yok. `src/csp.test.ts` yeni dosyaları da tarar.
- **Dil (1c tasarımı §9.4):** kullanıcıya görünen her metin `src/dil/tr.ts`'de; "siz" dili; ünlem ve emoji yok; biten iş "Tamamlandı". `src/dil/dil.test.ts` kodda geçen her `m('…')` anahtarının katalogda olduğunu denetler.
- **Onay sayfası (1c tasarımı §8.2):**
  - "Bu değişiklikler geri alınabilir" cümlesi yazılmaz.
  - Veri çıkışı, yalıtım, geri alınabilirlik ve kapsam alanları 1c'de yok.
  - Seçilmeyen çağrılar gerekçeyle reddedilir.
- **Klavye ve ekran okuyucu (1c tasarımı §9.5):**
  - Enter gönderir, Shift+Enter yeni satır açar.
  - Onay sayfasında odak sayfanın içinde kalır; Escape karar vermeden kapatır.
  - Onay istekleri ve tur sonları `aria-live` ile duyurulur.
- **Tokenlar:** yalnızca `src/stiller/`'deki tokenlar (ör. `--amber-tint`, `--red-tint`, `--blue-tint`); olmayan token adı kullanılmaz.
- **Her görevin sonunda (`ui/` içinde):** `npm run check` 0 hata ve 0 uyarı; `npm test` yeşil; `npm run build` (paket denetimi dahil) temiz.
- **Commit:** mesajlar Türkçe, `ui: …` biçiminde. Codex'in commit'lerine Claude satırı eklenmez.
- **Dokunulmayanlar:** `ui/src/ipc/` (plan commit'inde hazır), `ui/src/durum/`, `ui/src/markdown/`, `ui/package*.json`, `design/`, `crates/`, `.env`, `Screenshot*.png`, kök dizindeki `tasks.png`. `ui/src/bilesenler/` içinde yalnızca 2. görevdeki iki odak düzeltmesi yapılır.

## Dosya Yapısı

```
ui/src/
  dil/tr.ts                        yeni metinler (değişir)
  sahte/arka.ts                    Rust tarafının sahtesi (mockIPC); testler ve önizleme
  ana/
    bicim.ts                       zaman, gün grubu, oturum durumu, araç başlığı, sayılar
    onay.ts                        onay kararları, kısa ad
    anaDurum.svelte.ts             ana pencerenin durumu ve komutları
    Ana.svelte                     yerleşim (yer tutucunun yerine)
    Menu.svelte, OturumListesi.svelte, Konusma.svelte, KonusmaOgesi.svelte
    OnayKarti.svelte, OnaySayfasi.svelte, DurumCubugu.svelte
    bicim.test.ts, anaDurum.test.ts, ekranlar.test.ts
  sartlar/
    sartDurum.svelte.ts            şart penceresinin durumu ve komutları
    Sartlar.svelte                 çerçeve (yer tutucunun yerine)
    AnahtarFormu.svelte, SartMetniEkrani.svelte, KilitliEkran.svelte
    sartlar.test.ts
  bilesenler/Input.svelte, Textarea.svelte   iç odak halkası kaldırılır (değişir)
  onizleme/giris.ts, senaryo.ts    ekran önizlemesi (yalnızca geliştirmede)
ui/onizleme.html                   önizleme sayfası
ui/vite.config.ts                  açıklama (değişir)
docs/GELISTIRME_DURUMU.md          1c-3b sonuçları (değişir)
```

---

### Task 1: Ana pencerenin durumu, biçim yardımcıları, sahte arka uç ve dil kataloğu

Ekranların dayandığı Tauri'siz katman: ana pencere durum sınıfı, onay kararları, biçim yardımcıları, testlerin ve önizlemenin kullandığı sahte arka uç ve iki pencerenin bütün yeni metinleri.

**Files:**
- Modify: `ui/src/dil/tr.ts`
- Create: `ui/src/ana/bicim.ts`, `ui/src/ana/onay.ts`, `ui/src/ana/anaDurum.svelte.ts`, `ui/src/sahte/arka.ts`
- Test: `ui/src/ana/bicim.test.ts`, `ui/src/ana/anaDurum.test.ts`

**Interfaces:**
- Consumes (1c-3a):
  - `durum/oturum.ts`: `acilistanEkran`, `uygula`, `Girdi`, `OturumEkrani`, `EkranOgesi`, `Anlik`; `durum/dinleyici.ts`: `kanalIsleyicisi`.
  - `ipc/ana.ts` (bütün çağrılar); `ipc/hata.ts`: `komutHatasi`; `ipc/tipler.ts`.
  - `dil/tr.ts`: `m`, `MetinAnahtari`, `ARAC_DURUMU_ADI`.
- Produces:
  - **`AnaDurum`:**
    - Alanlar: `uygulama`, `liste`, `ekranlar` (SvelteMap), `secili`, `hata`, `onay`, `duyuru`, `aktif`.
    - Yöntemler: `baslat()`, `listeyiYenile()`, `oturumSec(id)`, `yeniOturum()`, `gonder(metin)`, `iptal()`, `onayla(oturum, islem, kararlar)`, `modelSec(model)`, `kipSec(kip)`, `eskileriYukle(oturum)`, `kapat(oturum)`, `yenidenAc(oturum)`, `onayiAc(oturum, islem)`, `onayiKapat()`, `baglantiAc(adres)`, `anahtariDegistir()`, `sartPenceresiniGoster()`.
  - **`bicim.ts`:** `saat`, `zamanEtiketi`, `GUN_GRUBU_ADI`, `gunGrubu`, `gruplara`, `satirDurumu`, `satirBasligi`, `aracBasligi`, `aracHedefi`, `aracKartDurumu`, `ciktiOnizlemesi`, `sayi`, `kredi`.
  - **`onay.ts`:** `kararlariUret(ogeler, secim, secilenler, gerekce)`, `kisaAd(oge)`, `OnaySecimi`.
  - **`sahte/arka.ts`:**
    - `SahteArka`: `kur()`, `oturumEkle`, `satirEkle`, `olaylar(oturum, olaylar, tur?)`, `paket`, `siradakiNo`, `dinleniyor`, `komutlar(ad)`.
    - `SahteArka` alanları: `uygulama`, `klasor`, `yukseltmeOnayi`, `sart`, `sartMetni`, `hatalar`, `ozel`.
    - Yardımcılar: `onayOgesi(ek)`, `bosOzet(model?, kip?)`, `GECERSIZ_ANAHTAR`, `AGSIZ_ANAHTAR`.

**Davranış notları (testler sınar):**
- **Açılış (§5.6):** uygulama durumu, liste ve `son_oturum` açılır; yazılabilir oturum dinlenir, salt okunur oturum dinlenmez.
- **Kanal:** "işledim" çizimi beklemez. Paket numarasında boşluk görülürse oturum kapatılıp kayıttan yeniden açılır (§4.4).
- **Duyuru ve liste:** onay isteği, şart isteği ve tur sonu `duyuru`ya yazılır. Tur bitince liste yenilenir.
- **Hatalar:** komut hatası `hata`da gösterilir. Karar gönderilemezse onay sayfası açık kalır.
- **Kip:** yükseltme yerel kutuda reddedilirse Rust'ın döndürdüğü eski kip kalır.

- [ ] **Step 1: Başarısız testleri yaz**

`ui/src/ana/bicim.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import type { EkranOgesi } from '../durum/oturum';
import { onayOgesi } from '../sahte/arka';
import {
  aracBasligi,
  aracHedefi,
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
```

`ui/src/ana/anaDurum.test.ts`:

```ts
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
```

- [ ] **Step 2: Testlerin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/ana`
Expected: FAIL — `Failed to resolve import "./bicim"`, `"../sahte/arka"` ve benzerleri.

- [ ] **Step 3: Dil kataloğunu genişlet**

`ui/src/dil/tr.ts`'de `'kod.kirpildi': 'Çıktının devamı gösterilmiyor',` satırının hemen altına, `} as const;`'tan önce şunları ekle:

```ts
  'uygulama.slogan': 'Daha iyi yazılım, daha aydınlık yarınlar.',
  'uygulama.alt_not': 'Küçük adımlar, büyük işler.',
  'genel.kapat': 'Kapat',
  'zaman.dun': 'Dün',

  'menu.ad': 'Menü',
  'menu.oturumlar': 'Oturumlar',
  'menu.anahtar': 'Anahtarı değiştir',

  'liste.baslik': 'Oturumlar',
  'liste.yeni': 'Yeni oturum',
  'liste.ara': 'Oturumlarda ara…',
  'liste.bos': 'Henüz oturum yok. Yeni bir oturumla başlayın.',
  'liste.sonuc_yok': 'Aramaya uyan oturum yok.',
  'liste.grup.bugun': 'Bugün',
  'liste.grup.dun': 'Dün',
  'liste.grup.hafta': 'Bu hafta',
  'liste.grup.eski': 'Daha eski',

  'oturum.yeni': 'Yeni oturum',
  'oturum.durum.calisiyor': 'Çalışıyor',
  'oturum.durum.onay': 'Onay bekliyor',
  'oturum.durum.sart': 'Şart onayı bekliyor',
  'oturum.durum.tamamlandi': 'Tamamlandı',
  'oturum.durum.iptal': 'İptal edildi',
  'oturum.durum.hata': 'Hatayla bitti',
  'oturum.durum.yarim': 'Yarım kaldı',
  'oturum.durum.okunamadi': 'Okunamadı',
  'oturum.durum.bos': 'Boş',

  'konusma.ad': 'Konuşma',
  'konusma.kapat': 'Oturumu kapat',
  'konusma.yeniden_ac': 'Yeniden aç',
  'konusma.eskiler': 'Daha eski mesajları göster',
  'konusma.bos': 'Bu oturumda henüz mesaj yok.',
  'konusma.kapandi': 'Oturum kapandı. Devam etmek için yeniden açın.',
  'konusma.karsilama.baslik': 'Bir oturum seçin ya da yeni bir oturum başlatın.',
  'konusma.karsilama.metin':
    'Kalfa seçtiğiniz klasörde çalışır. Dosya yazmadan önce onayınızı ister.',

  'arac.dosya_oku.suruyor': 'Dosya okunuyor',
  'arac.dosya_oku.bitti': 'Dosya okundu',
  'arac.dosya_oku.ad': 'Dosya okuma',
  'arac.dosya_listele.suruyor': 'Dizin listeleniyor',
  'arac.dosya_listele.bitti': 'Dizin listelendi',
  'arac.dosya_listele.ad': 'Dizin listeleme',
  'arac.icerik_ara.suruyor': 'İçerik aranıyor',
  'arac.icerik_ara.bitti': 'İçerik arandı',
  'arac.icerik_ara.ad': 'İçerik arama',
  'arac.dosya_yaz.suruyor': 'Dosya yazılıyor',
  'arac.dosya_yaz.bitti': 'Dosya yazıldı',
  'arac.dosya_yaz.ad': 'Dosya yazma',
  'arac.dosya_duzenle.suruyor': 'Dosya düzenleniyor',
  'arac.dosya_duzenle.bitti': 'Dosya düzenlendi',
  'arac.dosya_duzenle.ad': 'Dosya düzenleme',

  'onay.kart.baslik': 'Onay isteği: {sayi} değişiklik',
  'onay.kart.incele': 'İncele',
  'onay.kart.yanitlandi': 'Yanıtlandı',
  'onay.kart.gecersiz': 'Tur bitti; karar gerekmiyor',
  'onay.kart.uyari': '{sayi} uyarı',
  'onay.sayfa.baslik': 'Kalfa {sayi} değişiklik yapmak istiyor.',
  'onay.sayfa.alt':
    'Devam etmeden önce değişiklikleri inceleyin. Seçmediğiniz değişiklikler gerekçenizle reddedilir.',
  'onay.sayfa.tumunu_sec': 'Tümünü seç',
  'onay.sayfa.secili': '{secili} / {toplam} seçili',
  'onay.sayfa.goster': '{ad} değişikliğini göster',
  'onay.sayfa.diff_yok': 'Bu değişiklik için önizleme yok.',
  'onay.sayfa.ne': 'Ne',
  'onay.sayfa.nerede': 'Nerede',
  'onay.sayfa.uyarilar': 'Uyarılar',
  'onay.sayfa.gerekce': 'Reddetme nedeni (isteğe bağlı)',
  'onay.sayfa.gerekce_yer': 'Reddetme nedeninizi buraya yazabilirsiniz…',
  'onay.sayfa.tumunu_onayla': 'Tümünü onayla',
  'onay.sayfa.secilenleri_onayla': 'Seçilenleri onayla',
  'onay.sayfa.reddet': 'Reddet',

  'sart.kart.baslik': 'EVREN şartları (sürüm {surum})',
  'sart.kart.bekliyor':
    'EVREN, şartlar kabul edilene kadar isteği yanıtlamıyor. Tur, şart penceresindeki kararınızı bekliyor.',
  'sart.kart.cozuldu': 'Şart kararı verildi.',
  'sart.kart.ac': 'Şart penceresini aç',

  'hata.kart.baslik': 'Tur hatayla bitti',
  'aktarim.baslik': 'Aktarım durdu',

  'durum.ad': 'Durum çubuğu',
  'durum.hazir': 'Hazır',
  'durum.calisiyor': 'Çalışıyor',
  'durum.baglanti_yok': 'Bağlantı yok',
  'durum.anahtar_bellekte': 'Anahtar bellekte',
  'durum.bakiye': '{kalan} kredi · {saat}',
  'durum.bakiye_ipucu': 'Son gözlenen bakiye ve gözlem zamanı',
  'durum.token': '{sayi} token',
  'durum.token_ipucu': 'İstem {istem} · Tamamlama {tamamlama} · Düşünce {dusunce}',
  'durum.model_ipucu': 'Yönlendirilen model',

  'duyuru.onay': 'Onay isteği: {sayi} değişiklik kararınızı bekliyor.',
  'duyuru.sart': 'EVREN şartlarının onayı bekleniyor.',
  'duyuru.tur_sonu': 'Tur bitti.',

  'anahtar.baslik': 'EVREN API anahtarı',
  'anahtar.aciklama.Yok':
    "Kalfa EVREN'e sizin anahtarınızla bağlanır. Anahtarınızı EVREN portalından alabilirsiniz.",
  'anahtar.aciklama.KayitliGecersiz':
    'Kayıtlı anahtar artık geçerli değil. EVREN portalından yeni anahtarınızı alıp girin.',
  'anahtar.aciklama.Degistir': 'Yeni anahtarınızı girin. Açık oturumlar yeni anahtarı kullanır.',
  'anahtar.etiket': 'Anahtar',
  'anahtar.not':
    'Anahtar işletim sisteminin kimlik deposunda saklanır. Kalfa onu arayüze geri göndermez.',
  'anahtar.kaydet': 'Kaydet',

  'sart.baslik': 'EVREN kullanım şartları',
  'sart.surum': 'Sürüm {surum}',
  'sart.esasli': 'Esaslı değişiklik',
  'sart.esasli_degil': 'Küçük değişiklik',
  'sart.aciklama':
    "Kalfa'yı kullanmak için EVREN'in şartlarını kabul etmeniz gerekir. Metin EVREN'den geldiği gibi gösterilir.",
  'sart.kabul': 'Kabul ediyorum',
  'sart.reddet': 'Kabul etmiyorum',

  'kilitli.baslik': 'Şartlar kabul edilmedi',
  'kilitli.metin':
    "EVREN, şartlar kabul edilmeden hiçbir isteği yanıtlamıyor. Bekleyen turlar iptal edildi; oturumlarınız kayıtlı. Anahtarınız saklı kalır; Kalfa'yı bir sonraki açışınızda şartlar yeniden gösterilir.",
  'kilitli.goster': 'Metni yeniden göster',

  'sartlar.tamam': 'Tamamlandı. Kalfa açılıyor.',
  'sartlar.alt_not': 'Kalfa dikkatli çalışır. Kontrol her zaman sizdedir.',
```

- [ ] **Step 4: Modülleri yaz**

`ui/src/ana/bicim.ts`:

```ts
// Ana pencerede gösterilen değerlerin biçimi: zaman, gün grubu, oturum durumu,
// araç kartı başlığı, sayılar. Saf işlevler; DOM'a dokunmaz.
import { ARAC_DURUMU_ADI, m, type MetinAnahtari } from '../dil/tr';
import type { AracDurumu, OturumDurumu, OturumSatiri } from '../ipc/tipler';
import type { Anlik, EkranOgesi } from '../durum/oturum';

const GUN_MS = 24 * 60 * 60 * 1000;

function gunBasi(ms: number): number {
  const t = new Date(ms);
  return new Date(t.getFullYear(), t.getMonth(), t.getDate()).getTime();
}

/** Kaç takvim günü önce: 0 bugün, 1 dün. */
function gunFarki(ms: number, simdi: number): number {
  return Math.round((gunBasi(simdi) - gunBasi(ms)) / GUN_MS);
}

const SAAT = new Intl.DateTimeFormat('tr-TR', { hour: '2-digit', minute: '2-digit' });
const HAFTA_GUNU = new Intl.DateTimeFormat('tr-TR', { weekday: 'short' });
const GUN_AY = new Intl.DateTimeFormat('tr-TR', { day: 'numeric', month: 'short' });

export function saat(ms: number): string {
  return SAAT.format(ms);
}

/**
 * Oturum satırının sağındaki zaman: bugün saat, dün "Dün", bu hafta gün adı,
 * daha eskisi gün ve ay.
 */
export function zamanEtiketi(ms: number, simdi: number): string {
  const fark = gunFarki(ms, simdi);
  if (fark <= 0) return saat(ms);
  if (fark === 1) return m('zaman.dun');
  if (fark < 7) return HAFTA_GUNU.format(ms);
  return GUN_AY.format(ms);
}

export type GunGrubu = 'bugun' | 'dun' | 'hafta' | 'eski';

export const GUN_GRUBU_ADI: Record<GunGrubu, MetinAnahtari> = {
  bugun: 'liste.grup.bugun',
  dun: 'liste.grup.dun',
  hafta: 'liste.grup.hafta',
  eski: 'liste.grup.eski',
};

export function gunGrubu(ms: number, simdi: number): GunGrubu {
  const fark = gunFarki(ms, simdi);
  if (fark <= 0) return 'bugun';
  if (fark === 1) return 'dun';
  if (fark < 7) return 'hafta';
  return 'eski';
}

/** Satırları sırasını bozmadan gün gruplarına ayırır; boş grup dönmez. */
export function gruplara<T>(
  ogeler: T[],
  zaman: (oge: T) => number,
  simdi: number,
): { grup: GunGrubu; ogeler: T[] }[] {
  const gruplar: { grup: GunGrubu; ogeler: T[] }[] = [];
  for (const oge of ogeler) {
    const grup = gunGrubu(zaman(oge), simdi);
    const son = gruplar.at(-1);
    if (son?.grup === grup) son.ogeler.push(oge);
    else gruplar.push({ grup, ogeler: [oge] });
  }
  return gruplar;
}

export type SatirDurumu = 'running' | 'waiting' | 'done' | 'failed' | 'partial' | 'idle';

/**
 * Oturum satırının durumu (1c tasarımı §8.1). Açık oturumun anlık hâli kayıttaki
 * son durumdan önce gelir: çalışıyor, onay bekliyor, şart bekliyor.
 */
export function satirDurumu(
  durum: OturumDurumu,
  anlik: Anlik | null,
): { status: SatirDurumu; etiket: string } {
  switch (anlik) {
    case 'Calisiyor':
      return { status: 'running', etiket: m('oturum.durum.calisiyor') };
    case 'OnayBekliyor':
      return { status: 'waiting', etiket: m('oturum.durum.onay') };
    case 'SartBekliyor':
      return { status: 'waiting', etiket: m('oturum.durum.sart') };
  }
  switch (durum.tip) {
    case 'Tamamlandi':
      return { status: 'done', etiket: m('oturum.durum.tamamlandi') };
    case 'IptalEdildi':
      return { status: 'idle', etiket: m('oturum.durum.iptal') };
    case 'Hata':
      return { status: 'failed', etiket: m('oturum.durum.hata') };
    case 'YarimKaldi':
      return { status: 'partial', etiket: m('oturum.durum.yarim') };
    case 'Okunamadi':
      return { status: 'failed', etiket: m('oturum.durum.okunamadi') };
    case 'Bos':
      return { status: 'idle', etiket: m('oturum.durum.bos') };
  }
}

/** Listede gösterilen başlık: ilk kullanıcı mesajı ya da dosya adı. */
export function satirBasligi(satir: OturumSatiri): string {
  if (satir.baslik) return satir.baslik;
  if (satir.durum.tip === 'Okunamadi') return satir.dosya.split(/[\\/]/).at(-1) ?? satir.dosya;
  return m('oturum.yeni');
}

type AracOgesi = Extract<EkranOgesi, { tip: 'AracCagrisi' }>;
type KartDurumu = 'running' | 'done' | 'failed' | 'denied' | 'unknown';

/** Her aracın başlığı: sürerken, başarıyla bitince, diğer sonuçlarda. */
const ARAC_BASLIGI: Record<string, [MetinAnahtari, MetinAnahtari, MetinAnahtari]> = {
  dosya_oku: ['arac.dosya_oku.suruyor', 'arac.dosya_oku.bitti', 'arac.dosya_oku.ad'],
  dosya_listele: [
    'arac.dosya_listele.suruyor',
    'arac.dosya_listele.bitti',
    'arac.dosya_listele.ad',
  ],
  icerik_ara: ['arac.icerik_ara.suruyor', 'arac.icerik_ara.bitti', 'arac.icerik_ara.ad'],
  dosya_yaz: ['arac.dosya_yaz.suruyor', 'arac.dosya_yaz.bitti', 'arac.dosya_yaz.ad'],
  dosya_duzenle: [
    'arac.dosya_duzenle.suruyor',
    'arac.dosya_duzenle.bitti',
    'arac.dosya_duzenle.ad',
  ],
};

/**
 * Araç kartının başlığı: sürerken şimdiki zamanla, başarıyla bitince geçmiş
 * zamanla ("Dosya okundu"); reddedilen ya da başarısız çağrıda yalın adla.
 */
export function aracBasligi(oge: AracOgesi): string {
  const basliklar = ARAC_BASLIGI[oge.arac];
  if (!basliklar) return oge.arac;
  if (oge.durum === 'Suruyor') return m(basliklar[0]);
  if (oge.durum === 'Basarili') return m(basliklar[1]);
  return m(basliklar[2]);
}

/** Kartın hedefi: canlı çağrıda açıklama, kayıttan gelende argümandaki yol ya da desen. */
export function aracHedefi(oge: AracOgesi): string | undefined {
  if (oge.aciklama) return oge.aciklama;
  if (!oge.argumanlar) return undefined;
  try {
    const a = JSON.parse(oge.argumanlar) as Record<string, unknown>;
    for (const alan of ['yol', 'desen', 'dizin']) {
      if (typeof a[alan] === 'string') return a[alan];
    }
  } catch {
    // Çözülemeyen argüman gösterilmez.
  }
  return undefined;
}

export function aracKartDurumu(durum: AracDurumu | 'Suruyor' | null): {
  status: KartDurumu;
  etiket: string;
} {
  switch (durum) {
    case 'Suruyor':
      return { status: 'running', etiket: m(ARAC_DURUMU_ADI.Suruyor) };
    case 'Basarili':
      return { status: 'done', etiket: m(ARAC_DURUMU_ADI.Basarili) };
    case 'Basarisiz':
      return { status: 'failed', etiket: m(ARAC_DURUMU_ADI.Basarisiz) };
    case 'Reddedildi':
      return { status: 'denied', etiket: m(ARAC_DURUMU_ADI.Reddedildi) };
    case 'IptalEdildi':
      return { status: 'denied', etiket: m(ARAC_DURUMU_ADI.IptalEdildi) };
    case 'SonucuBilinmiyor':
      return {
        status: 'unknown',
        etiket: m(ARAC_DURUMU_ADI.SonucuBilinmiyor),
      };
    case null:
      return { status: 'unknown', etiket: m(ARAC_DURUMU_ADI.Baslamadi) };
  }
}

/** Araç çıktısının kartta gösterilen kısmı. */
export function ciktiOnizlemesi(
  metin: string,
  azamiSatir = 40,
): { kod: string; kirpildi: boolean } {
  const satirlar = metin.replace(/\n$/, '').split('\n');
  if (satirlar.length <= azamiSatir) return { kod: satirlar.join('\n'), kirpildi: false };
  return { kod: satirlar.slice(0, azamiSatir).join('\n'), kirpildi: true };
}

const TAM_SAYI = new Intl.NumberFormat('tr-TR');
const KREDI = new Intl.NumberFormat('tr-TR', {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export function sayi(n: number): string {
  return TAM_SAYI.format(n);
}

/** EVREN'in ondalık metni; çözülemezse olduğu gibi. */
export function kredi(ondalik: string): string {
  const n = Number(ondalik);
  return Number.isFinite(n) ? KREDI.format(n) : ondalik;
}
```

`ui/src/ana/onay.ts`:

```ts
// Onay sayfasının kararları (1c tasarımı §8.2). Seçilen çağrılar onaylanır;
// seçilmeyenler gerekçeyle reddedilir. Her karar çağrının mührünü taşır.
import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';

export type OnaySecimi = 'tumu' | 'secilenler' | 'hicbiri';

export function kararlariUret(
  ogeler: OnayOgesi[],
  secim: OnaySecimi,
  secilenler: ReadonlySet<CagriId>,
  gerekce: string,
): OnayGirdisi[] {
  const temizGerekce = gerekce.trim() || null;
  return ogeler.map((o) => {
    const onay = secim === 'tumu' || (secim === 'secilenler' && secilenler.has(o.cagri));
    return {
      cagri: o.cagri,
      muhur: o.muhur,
      karar: onay ? { tip: 'Onayla' } : { tip: 'Reddet', gerekce: temizGerekce },
    };
  });
}

/** Dosya listesinde gösterilen kısa ad: hedefin son bileşeni. */
export function kisaAd(oge: OnayOgesi): string {
  const hedef = oge.hedefler[0];
  if (!hedef) return oge.arac;
  return hedef.split(/[\\/]/).filter(Boolean).at(-1) ?? hedef;
}
```

`ui/src/sahte/arka.ts`:

```ts
// Rust tarafının bellekteki sahtesi: testler ve geliştirme önizlemesi
// (`onizleme.html`) kullanır; uygulama paketine girmez. Komutlar `mockIPC` ile
// yakalanır; kanal mesajları dinleyen `Channel`'a doğrudan verilir.
import type { Channel } from '@tauri-apps/api/core';
import { mockIPC } from '@tauri-apps/api/mocks';
import { KIPLER } from '../ipc/tipler';
import type {
  CagriId,
  CalismaAlani,
  GecmisSayfasi,
  KanalMesaji,
  Kip,
  KomutHatasi,
  Olay,
  OnayOgesi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  SartMetni,
  SartPenceresiDurumu,
  TurId,
  UygulamaDurumu,
} from '../ipc/tipler';

type Argumanlar = Record<string, unknown>;

export const GECERSIZ_ANAHTAR = 'evren_llm_gecersiz';
export const AGSIZ_ANAHTAR = 'evren_llm_agsiz';

interface SahteOturum {
  acilis: OturumAcilisi;
  satir: OturumSatiri;
  eskiSayfa: GecmisSayfasi;
}

export function bosOzet(model = 'glm-5.3', kip: Kip = 'Siki'): OturumAcilisi['ozet'] {
  return {
    model,
    kip,
    yonlendirilen_model: null,
    son_kredi: null,
    istem: 0,
    tamamlama: 0,
    dusunce: 0,
  };
}

/** Onay isteğindeki bir çağrı; verilmeyen alanlar boş. */
export function onayOgesi(ek: Partial<OnayOgesi> & { cagri: CagriId }): OnayOgesi {
  return {
    arac: 'dosya_yaz',
    aciklama: '',
    hedefler: [],
    diff: null,
    uyarilar: [],
    muhur: `muhur-${ek.cagri}`,
    ...ek,
  };
}

export class SahteArka {
  /** Gelen her komut, sırayla. */
  cagrilar: { komut: string; argumanlar: Argumanlar }[] = [];
  uygulama: UygulamaDurumu = {
    baglanti: 'Var',
    anahtar_deposu: 'Kalici',
    modeller: [
      { kimlik: 'qwen3.8-flash-next', profil: 'Hizli' },
      { kimlik: 'deepseek-v4-flash', profil: 'Dengeli' },
      { kimlik: 'glm-5.3', profil: 'Derin' },
      { kimlik: 'auto', profil: 'Otomatik' },
    ],
    varsayilan_model: 'glm-5.3',
    son_oturum: null,
  };
  /** `calisma_alani_sec`'in döneceği klasör; `null` vazgeçmektir. */
  klasor: CalismaAlani | null = { kimlik: 1, yol: 'C:\\projeler\\kalfa' };
  /** İzin kipi yükseltmesinde yerel iletişim kutusunun yanıtı. */
  yukseltmeOnayi = true;
  /** Şart penceresinin durumu ve gösterilecek metin. */
  sart: SartPenceresiDurumu = { tip: 'Tamam' };
  sartMetni: SartMetni = {
    surum: 1,
    esasli: true,
    metin: '# Kullanım şartları\n\nDeneme metni.',
  };
  /** Bu komutlar verilen hatayla reddedilir. */
  hatalar = new Map<string, KomutHatasi>();
  /** Komutun yerine geçen davranış (önizlemenin senaryosu kullanır). */
  ozel = new Map<string, (a: Argumanlar) => unknown>();

  #oturumlar = new Map<OturumId, SahteOturum>();
  #kanallar = new Map<OturumId, Channel<KanalMesaji>>();
  #no = new Map<OturumId, number>();
  #sira = new Map<OturumId, number>();
  #sayac = 0;

  kur(): this {
    mockIPC((komut, argumanlar) => this.#isle(komut, (argumanlar ?? {}) as Argumanlar));
    return this;
  }

  /** Kayıtlı bir oturum ekler (listede görünür, açılabilir). */
  oturumEkle(
    acilis: Partial<OturumAcilisi> & { oturum: OturumId },
    satir: Partial<OturumSatiri> = {},
    eskiSayfa: GecmisSayfasi = { ogeler: [], daha_eski_var: false },
  ): OturumId {
    const tam: OturumAcilisi = {
      calisma_alani: 'C:\\projeler\\kalfa',
      gecmis: { ogeler: [], daha_eski_var: false },
      ozet: bosOzet(),
      salt_okunur: null,
      ...acilis,
    };
    this.#oturumlar.set(tam.oturum, {
      acilis: tam,
      satir: {
        oturum: tam.oturum,
        dosya: `C:\\veri\\oturumlar\\${tam.oturum}.jsonl`,
        baslik: null,
        calisma_alani: tam.calisma_alani,
        son_degisiklik_ms: Date.now(),
        durum: { tip: 'Bos' },
        ...satir,
      },
      eskiSayfa,
    });
    this.#sira.set(tam.oturum, Math.max(0, ...tam.gecmis.ogeler.map((o) => o.sira)));
    return tam.oturum;
  }

  /** Listede yalnızca satırı olan (okunamayan) kayıt. */
  satirEkle(satir: OturumSatiri): void {
    this.#oturumlar.set(satir.dosya, {
      acilis: {
        oturum: satir.dosya,
        calisma_alani: null,
        gecmis: { ogeler: [], daha_eski_var: false },
        ozet: bosOzet(),
        salt_okunur: null,
      },
      satir,
      eskiSayfa: { ogeler: [], daha_eski_var: false },
    });
  }

  dinleniyor(oturum: OturumId): boolean {
    return this.#kanallar.has(oturum);
  }

  /** Dinlenen oturuma olayları tek pakette gönderir. */
  olaylar(oturum: OturumId, olaylar: Olay[], tur: TurId | null = 'tur-1'): void {
    const sira = this.#sira.get(oturum) ?? 0;
    const zarflar = olaylar.map((yuk, i) => ({ sema: 1, oturum, tur, sira: sira + i + 1, yuk }));
    this.#sira.set(oturum, sira + olaylar.length);
    this.paket(oturum, { tip: 'Olaylar', no: this.#sonrakiNo(oturum), zarflar });
  }

  /** Numarayı elle veren ham paket (boşluk testleri). */
  paket(oturum: OturumId, mesaj: KanalMesaji): void {
    const kanal = this.#kanallar.get(oturum);
    if (!kanal) throw new Error(`${oturum} dinlenmiyor`);
    kanal.onmessage(mesaj);
  }

  /** Sıradaki paket numarasını harcamadan verir. */
  siradakiNo(oturum: OturumId): number {
    return this.#no.get(oturum) ?? 1;
  }

  komutlar(ad: string): Argumanlar[] {
    return this.cagrilar.filter((c) => c.komut === ad).map((c) => c.argumanlar);
  }

  #sonrakiNo(oturum: OturumId): number {
    const no = this.siradakiNo(oturum);
    this.#no.set(oturum, no + 1);
    return no;
  }

  #isle(komut: string, a: Argumanlar): unknown {
    this.cagrilar.push({ komut, argumanlar: a });
    const hata = this.hatalar.get(komut);
    if (hata) throw hata;
    const ozel = this.ozel.get(komut);
    if (ozel) return ozel(a);
    const oturum = a.oturum as OturumId;
    switch (komut) {
      case 'uygulama_durumu':
        return this.uygulama;
      case 'oturumlari_listele':
        return [...this.#oturumlar.values()]
          .map((o) => o.satir)
          .sort((x, y) => y.son_degisiklik_ms - x.son_degisiklik_ms);
      case 'calisma_alani_sec':
        return this.klasor;
      case 'oturum_baslat': {
        this.#sayac += 1;
        const yeni = `oturum-yeni-${this.#sayac}`;
        this.oturumEkle({
          oturum: yeni,
          calisma_alani: this.klasor?.yol ?? null,
          ozet: bosOzet(a.model as string),
        });
        return this.#oturumlar.get(yeni)?.acilis;
      }
      case 'oturum_ac': {
        const o = this.#oturumlar.get(oturum);
        if (!o) throw { kod: 'OturumYok', mesaj: 'Bu oturumun kaydı bulunamadı.' };
        return o.acilis;
      }
      case 'oturumu_dinle':
        this.#kanallar.set(oturum, a.kanal as Channel<KanalMesaji>);
        this.#no.set(oturum, 1);
        return null;
      case 'gecmis_sayfasi':
        return this.#oturumlar.get(oturum)?.eskiSayfa;
      case 'izin_kipi_degistir': {
        const o = this.#oturumlar.get(oturum);
        const eski = o?.acilis.ozet.kip ?? 'Siki';
        const yeni = a.kip as Kip;
        const yukseltme = KIPLER.indexOf(yeni) > KIPLER.indexOf(eski);
        const sonuc = yukseltme && !this.yukseltmeOnayi ? eski : yeni;
        if (o) o.acilis.ozet.kip = sonuc;
        return sonuc;
      }
      case 'oturumu_kapat':
        this.#kanallar.delete(oturum);
        return null;
      case 'sart_penceresi_durumu':
        return this.sart;
      case 'anahtari_kaydet': {
        const anahtar = String(a.anahtar).trim();
        if (!anahtar || anahtar === GECERSIZ_ANAHTAR) return { tip: 'Gecersiz' };
        if (anahtar === AGSIZ_ANAHTAR) return { tip: 'Ulasilamadi' };
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return { tip: 'Kaydedildi', depo: this.uygulama.anahtar_deposu, sonraki: this.sart };
      }
      case 'sartlari_kabul_et': {
        if (this.sart.tip === 'Kilitli') return { tip: 'Kilitli' };
        if (this.sart.tip !== 'SartGerekli') {
          throw { kod: 'GecersizGirdi', mesaj: 'Onay bekleyen şart metni yok.' };
        }
        if (a.surum !== this.sartMetni.surum) {
          this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
          return { tip: 'YeniMetin', metin: this.sartMetni };
        }
        this.sart = { tip: 'Tamam' };
        return { tip: 'Kabul' };
      }
      case 'sartlari_reddet':
        if (this.sart.tip === 'SartGerekli') this.sart = { tip: 'Kilitli' };
        return null;
      case 'sart_metnini_goster':
        this.sart = { tip: 'SartGerekli', metin: this.sartMetni };
        return this.sart;
      default:
        // mesaj_gonder, turu_iptal_et, arac_onayi, model_degistir, paketi_isledim,
        // baglanti_ac, anahtar_penceresini_ac, sart_penceresini_ac
        return null;
    }
  }
}
```

`ui/src/ana/anaDurum.svelte.ts`:

```ts
// Ana pencerenin durumu ve komutları (1c tasarımı §5, §8). Ekranlar yalnızca
// bu sınıfın alanlarını çizer ve yöntemlerini çağırır. Her açık oturumun ekran
// durumu `durum/oturum.ts`'deki saf işlevle güncellenir.
import { SvelteMap } from 'svelte/reactivity';
import { m } from '../dil/tr';
import { acilistanEkran, uygula, type Girdi, type OturumEkrani } from '../durum/oturum';
import { kanalIsleyicisi } from '../durum/dinleyici';
import * as ipc from '../ipc/ana';
import { komutHatasi } from '../ipc/hata';
import type {
  IslemId,
  Kip,
  OnayGirdisi,
  OturumAcilisi,
  OturumId,
  OturumSatiri,
  UygulamaDurumu,
} from '../ipc/tipler';

export class AnaDurum {
  uygulama = $state<UygulamaDurumu | null>(null);
  liste = $state<OturumSatiri[]>([]);
  /** Açık oturumların ekranları; oturum onay beklerken diğerine geçilebilir. */
  ekranlar = new SvelteMap<OturumId, OturumEkrani>();
  secili = $state<OturumId | null>(null);
  /** Konuşmanın üstünde gösterilen son komut hatası. */
  hata = $state<string | null>(null);
  /** Açık onay sayfası. */
  onay = $state<{ oturum: OturumId; islem: IslemId } | null>(null);
  /** Ekran okuyucuya okunan son duyuru (`aria-live`, 1c tasarımı §9.5). */
  duyuru = $state('');
  /** Eski sayfası yüklenen oturumlar; aynı sayfa iki kez istenmez. */
  #eskiYukleniyor = new Set<OturumId>();
  #yenidenAciliyor = new Set<OturumId>();

  aktif = $derived(this.secili ? (this.ekranlar.get(this.secili) ?? null) : null);

  /** Pencere açılınca: uygulama durumu, liste ve son açık oturum (1c tasarımı §5.6). */
  async baslat(): Promise<void> {
    await this.#dene(async () => {
      this.uygulama = await ipc.uygulamaDurumu();
    });
    await this.listeyiYenile();
    const son = this.uygulama?.son_oturum;
    if (son) await this.oturumSec(son);
  }

  async listeyiYenile(): Promise<void> {
    await this.#dene(async () => {
      this.liste = await ipc.oturumlariListele();
    });
  }

  async oturumSec(oturum: OturumId): Promise<void> {
    if (!this.ekranlar.has(oturum)) {
      const acildi = await this.#dene(() => this.#ac(oturum));
      if (acildi === undefined) return;
    }
    this.secili = oturum;
    this.hata = null;
  }

  /** Klasör seçtirir ve varsayılan modelle yeni oturum başlatır. */
  async yeniOturum(): Promise<void> {
    await this.#dene(async () => {
      const alan = await ipc.calismaAlaniSec();
      if (!alan || !this.uygulama) return;
      const acilis = await ipc.oturumBaslat(alan.kimlik, this.uygulama.varsayilan_model);
      await this.#kur(acilis);
      this.secili = acilis.oturum;
      this.hata = null;
      await this.listeyiYenile();
    });
  }

  async gonder(metin: string): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    this.#uygula(oturum, { tip: 'MesajGonderildi', metin });
    await this.#dene(() => ipc.mesajGonder(oturum, metin));
  }

  async iptal(): Promise<void> {
    const oturum = this.secili;
    if (oturum) await this.#dene(() => ipc.turuIptalEt(oturum));
  }

  async onayla(oturum: OturumId, islem: IslemId, kararlar: OnayGirdisi[]): Promise<void> {
    const gitti = await this.#dene(() => ipc.aracOnayi(oturum, kararlar));
    if (gitti === undefined) return;
    this.#uygula(oturum, { tip: 'OnayYanitlandi', islem });
    this.onay = null;
  }

  async modelSec(model: string): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    const oldu = await this.#dene(() => ipc.modelDegistir(oturum, model));
    if (oldu !== undefined) this.#uygula(oturum, { tip: 'ModelDegisti', model });
  }

  /** Yükseltmede Rust yerel onay sorar; reddedilirse eski kip döner (1c tasarımı §6). */
  async kipSec(kip: Kip): Promise<void> {
    const oturum = this.secili;
    if (!oturum) return;
    const yeni = await this.#dene(() => ipc.izinKipiDegistir(oturum, kip));
    if (yeni) this.#uygula(oturum, { tip: 'KipDegisti', kip: yeni });
  }

  /** Yukarı kaydırınca ya da düğmeyle: bir önceki sayfa (1c tasarımı §9.3). */
  async eskileriYukle(oturum: OturumId): Promise<void> {
    const ekran = this.ekranlar.get(oturum);
    const enEski = ekran?.ogeler[0]?.sira;
    if (!ekran?.daha_eski_var || enEski === undefined || this.#eskiYukleniyor.has(oturum)) return;
    this.#eskiYukleniyor.add(oturum);
    try {
      const sayfa = await this.#dene(() => ipc.gecmisSayfasi(oturum, enEski));
      if (sayfa) this.#uygula(oturum, { tip: 'EskiSayfa', sayfa });
    } finally {
      this.#eskiYukleniyor.delete(oturum);
    }
  }

  async kapat(oturum: OturumId): Promise<void> {
    await this.#dene(() => ipc.oturumuKapat(oturum));
    this.ekranlar.delete(oturum);
    if (this.secili === oturum) this.secili = null;
    if (this.onay?.oturum === oturum) this.onay = null;
    await this.listeyiYenile();
  }

  /** Oturumu kapatıp kayıttan yeniden açar: boşlukta, aktarım durunca, kapanınca. */
  async yenidenAc(oturum: OturumId): Promise<void> {
    if (this.#yenidenAciliyor.has(oturum)) return;
    this.#yenidenAciliyor.add(oturum);
    try {
      await ipc.oturumuKapat(oturum).catch(() => undefined);
      this.ekranlar.delete(oturum);
      await this.#dene(() => this.#ac(oturum));
    } finally {
      this.#yenidenAciliyor.delete(oturum);
    }
  }

  onayiAc(oturum: OturumId, islem: IslemId): void {
    this.onay = { oturum, islem };
  }

  onayiKapat(): void {
    this.onay = null;
  }

  baglantiAc = (adres: string): void => {
    void this.#dene(() => ipc.baglantiAc(adres));
  };

  anahtariDegistir(): void {
    void this.#dene(() => ipc.anahtarPenceresiniAc());
  }

  sartPenceresiniGoster(): void {
    void this.#dene(() => ipc.sartPenceresiniAc());
  }

  async #ac(oturum: OturumId): Promise<void> {
    await this.#kur(await ipc.oturumAc(oturum));
  }

  /** Ekranı kurar; yazılabilir oturumun olaylarını dinlemeye başlar. */
  async #kur(acilis: OturumAcilisi): Promise<void> {
    const oturum = acilis.oturum;
    this.ekranlar.set(oturum, acilistanEkran(acilis));
    if (acilis.salt_okunur) return;
    const isleyici = kanalIsleyicisi(
      () => this.ekranlar.get(oturum) ?? acilistanEkran(acilis),
      (yeni) => this.#yaz(oturum, yeni),
      (no) => {
        ipc.paketiIsledim(oturum, no).catch(() => undefined);
      },
    );
    await ipc.oturumuDinle(oturum, isleyici);
  }

  #uygula(oturum: OturumId, girdi: Girdi): void {
    const ekran = this.ekranlar.get(oturum);
    if (ekran) this.#yaz(oturum, uygula(ekran, girdi));
  }

  #yaz(oturum: OturumId, yeni: OturumEkrani): void {
    const onceki = this.ekranlar.get(oturum);
    if (!onceki) return;
    this.ekranlar.set(oturum, yeni);
    this.#duyur(onceki, yeni);
    if (yeni.bosluk && !onceki.bosluk) {
      // Paket numarasında boşluk: oturum kayıttan yeniden açılır (1c tasarımı §4.4).
      queueMicrotask(() => void this.yenidenAc(oturum));
    }
    const turBitti = yeni.ogeler
      .slice(onceki.ogeler.length)
      .some((o) => o.tip === 'TurSonu' || o.tip === 'AktarimDurdu');
    if (turBitti) void this.listeyiYenile();
  }

  /** Onay istekleri ve tur sonları duyurulur; akan token'lar duyurulmaz. */
  #duyur(onceki: OturumEkrani, yeni: OturumEkrani): void {
    for (const o of yeni.ogeler.slice(onceki.ogeler.length)) {
      if (o.tip === 'OnayIstegi') this.duyuru = m('duyuru.onay', { sayi: o.ogeler.length });
      else if (o.tip === 'SartIstegi') this.duyuru = m('duyuru.sart');
      else if (o.tip === 'TurSonu') this.duyuru = m('duyuru.tur_sonu');
    }
  }

  /** Komutu çalıştırır; hata olursa konuşmanın üstünde gösterir ve `undefined` döner. */
  async #dene<T>(is: () => Promise<T>): Promise<T | undefined> {
    try {
      const sonuc = await is();
      return sonuc === undefined ? (null as T) : sonuc;
    } catch (e) {
      this.hata = komutHatasi(e).mesaj;
      return undefined;
    }
  }
}
```

- [ ] **Step 5: Denetimleri koştur**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: 0 hata, 0 uyarı; 12 dosya, 135 test geçti; `Paket denetimi temiz.`

- [ ] **Step 6: Commit**

```bash
git add ui/src/dil/tr.ts ui/src/ana ui/src/sahte
git commit -m "ui: ana pencerenin durumu, biçim yardımcıları ve sahte arka uç"
git push
```

---

### Task 2: Ana pencerenin ekranları

Menü, oturum listesi, konuşma (başlık, uyarı şeritleri, geçmiş ve canlı öğeler, eski sayfa, Composer), onay kartı, onay sayfası ve durum çubuğu. Ayrıca 1c-3a'daki `Input` ve `Textarea`'da, çerçevesi zaten odak gösteren alanın içinde ikinci bir odak halkası çizilmesi düzeltilir. Tasarım sistemindeki `.kalfa-root :focus-visible` kuralı bileşenin `outline: none`'undan daha özgül olduğu için iç alanda da halka çıkıyordu.

**Files:**
- Modify: `ui/src/ana/Ana.svelte` (yer tutucunun yerine), `ui/src/bilesenler/Input.svelte`, `ui/src/bilesenler/Textarea.svelte`
- Create: `ui/src/ana/{Menu,OturumListesi,Konusma,KonusmaOgesi,OnayKarti,OnaySayfasi,DurumCubugu}.svelte`
- Test: `ui/src/ana/ekranlar.test.ts`

**Interfaces:**
- Consumes: Görev 1'in `AnaDurum`, `bicim.ts`, `onay.ts`, `sahte/arka.ts`; 1c-3a'nın bileşenleri (`NavRail`, `NavItem`, `SessionRow`, `Input`, `IconButton`, `Button`, `IconTile`, `Icon`, `Composer`, `Message`, `ThoughtPanel`, `ToolCallCard`, `CodeBlock`, `Card`, `Badge`, `Checkbox`, `DiffView`, `Textarea`, `StatusBar`, `StatusItem`), `diffCozumle`, `diffSayilari`, `Markdown`.
- Produces: `Ana.svelte` (prop: `durum?: AnaDurum`; verilmezse kendisi kurar ve açılışta `baslat()` çağırır).

**Notlar:**
- **Konuşma kaydırması:**
  - Akış içeriği `ResizeObserver` ile izlenir. Kullanıcı en alttaysa yeni öğe, akan metin ya da ekrana giren öğenin gerçek boyu onu alttan koparmaz.
  - Olay tabanlı kaydırma yetmez: Markdown bir sonraki karede çizilir, `content-visibility: auto` öğesi ekrana girince gerçek boyunu alır.
  - jsdom'da `ResizeObserver` yoktur; kod bunu denetler.
- **Eski sayfalar:** yukarı kaydırınca ya da "Daha eski mesajları göster" düğmesiyle yüklenir; okunan yer kaymaz (§9.3).
- **Hata kartı:** kartta "Anahtarı değiştir" yalnızca anahtar hatasında görünür (§10). Hata metni `hata.anahtar_gecersiz` ile karşılaştırılır; `kalfa-evren`'in metni ile katalogdaki metin aynıdır.

- [ ] **Step 1: Başarısız testi yaz**

`ui/src/ana/ekranlar.test.ts`:

```ts
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { onayOgesi, SahteArka } from '../sahte/arka';
import Ana from './Ana.svelte';
import { AnaDurum } from './anaDurum.svelte';

afterEach(() => clearMocks());

const DIFF = ['@@ -1,2 +1,2 @@', ' fn main() {', '-    eski();', '+    yeni();'].join('\n');

async function ciz(ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.oturumEkle(
    {
      oturum: 'o1',
      gecmis: {
        ogeler: [{ tip: 'KullaniciMesaji', sira: 1, tur: 't1', metin: 'Dosyaları incele' }],
        daha_eski_var: false,
      },
    },
    { baslik: 'Dosyaları incele', durum: { tip: 'Tamamlandi' } },
  );
  arka.uygulama.son_oturum = 'o1';
  ayar(arka);
  const durum = new AnaDurum();
  render(Ana, { props: { durum } });
  await screen.findByRole('heading', { name: 'Dosyaları incele' });
  return { arka, durum };
}

function onayIstegi(arka: SahteArka) {
  arka.olaylar('o1', [
    { tip: 'TurBasladi' },
    {
      tip: 'OnayGerekiyor',
      islem: 'i1',
      ogeler: [
        onayOgesi({
          cagri: 'c1',
          aciklama: 'main.rs dosyasını düzenler',
          hedefler: ['C:\\projeler\\kalfa\\src\\main.rs'],
          diff: DIFF,
          uyarilar: ['Dosya çalışma alanının kökünde'],
          muhur: 'm1',
        }),
        onayOgesi({ cagri: 'c2', hedefler: ['C:\\projeler\\kalfa\\src\\lib.rs'], muhur: 'm2' }),
      ],
    },
  ]);
}

describe('ana pencere (1c tasarımı §8.1)', () => {
  it('menü, liste, konuşma ve durum çubuğu', async () => {
    await ciz();
    expect(screen.getByRole('navigation', { name: 'Menü' })).toBeTruthy();
    const liste = screen.getByRole('complementary', { name: 'Oturumlar' });
    expect(within(liste).getByText('Bugün')).toBeTruthy();
    expect(within(liste).getByRole('button', { current: true }).textContent).toContain(
      'Dosyaları incele',
    );
    expect(screen.getAllByText('Dosyaları incele').length).toBeGreaterThan(1);
    expect(screen.getByText('Hazır')).toBeTruthy();
    expect(screen.getByText('Sıkı', { selector: '.durum-ogesi' })).toBeTruthy();
  });

  it('oturum yokken karşılama ekranı yeni oturum başlatır', async () => {
    const arka = new SahteArka().kur();
    render(Ana, { props: { durum: new AnaDurum() } });
    const konusma = await screen.findByRole('region', { name: 'Konuşma' });
    const dugme = within(konusma).getByRole('button', { name: 'Yeni oturum' });
    await fireEvent.click(dugme);
    await waitFor(() => expect(arka.komutlar('oturum_baslat')).toHaveLength(1));
  });

  it('bağlantı yok ve anahtar bellekte uyarıları', async () => {
    await ciz((a) => {
      a.uygulama.baglanti = 'Yok';
      a.uygulama.anahtar_deposu = 'Bellekte';
    });
    expect(screen.getByText(/EVREN bağlantısı yok/)).toBeTruthy();
    expect(screen.getByText('Bağlantı yok')).toBeTruthy();
    expect(screen.getByText('Anahtar bellekte')).toBeTruthy();
  });

  it('salt okunur oturumda mesaj alanı kapalı ve nedeni yazılı (§10)', async () => {
    await ciz((a) =>
      a.oturumEkle(
        { oturum: 'o1', salt_okunur: { tip: 'CalismaAlaniYok', yol: 'C:\\silindi' } },
        { baslik: 'Dosyaları incele' },
      ),
    );
    const neden = 'Çalışma alanı bulunamadı: C:\\silindi';
    expect(screen.getAllByText(neden).length).toBeGreaterThan(0);
    expect(screen.getByRole<HTMLTextAreaElement>('textbox', { name: neden }).disabled).toBe(true);
  });

  it('canlı yanıt, araç kartı ve tur sonu çizilir', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      { tip: 'TurBasladi' },
      { tip: 'TokenParcasi', metin: 'Okuyorum.' },
      { tip: 'AracCagrisiBasladi', cagri: 'c9', arac: 'dosya_oku', aciklama: 'src/main.rs' },
      { tip: 'AracSonucu', cagri: 'c9', arac: 'dosya_oku', durum: 'Basarili', icerik: 'fn main() {}' },
      { tip: 'Tamamlandi', neden: { tip: 'Normal' } },
    ]);
    expect(await screen.findByText('Okuyorum.')).toBeTruthy();
    expect(screen.getByText('Dosya okundu')).toBeTruthy();
    expect(screen.getByText('src/main.rs')).toBeTruthy();
    expect(screen.getAllByText('Tamamlandı').length).toBeGreaterThan(0);
  });

  it('oturum ortasında anahtar geçersizleşirse hata kartında "Anahtarı değiştir" (§10)', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [
      {
        tip: 'Hata',
        mesaj:
          'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
      },
    ]);
    const kart = (await screen.findByText('Tur hatayla bitti')).closest('section');
    await fireEvent.click(within(kart as HTMLElement).getByRole('button', { name: /Anahtarı/ }));
    expect(arka.komutlar('anahtar_penceresini_ac')).toHaveLength(1);
  });

  it('aktarım durunca kart oturumu yeniden açtırır', async () => {
    const { arka } = await ciz();
    arka.olaylar('o1', [{ tip: 'AktarimDurdu', neden: 'TuketiciYetismedi' }]);
    await fireEvent.click(await screen.findByRole('button', { name: 'Yeniden aç' }));
    await waitFor(() => expect(arka.komutlar('oturum_ac')).toHaveLength(2));
  });
});

describe('onay isteği ve onay sayfası (1c tasarımı §8.2, §9.5)', () => {
  it('kart "İncele" ile sayfayı açar; sayfada diff, ne, nerede ve uyarılar', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    expect(await screen.findByText('Onay isteği: 2 değişiklik')).toBeTruthy();
    expect(screen.getByText('Onay bekliyor', { selector: '.durum-ogesi' })).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'İncele' }));
    const sayfa = screen.getByRole('dialog', { name: 'Kalfa 2 değişiklik yapmak istiyor.' });
    expect(within(sayfa).getByText('yeni();')).toBeTruthy();
    expect(within(sayfa).getByText('main.rs dosyasını düzenler')).toBeTruthy();
    expect(within(sayfa).getByText('Dosya çalışma alanının kökünde')).toBeTruthy();
    expect(within(sayfa).queryByText(/geri alınabilir/)).toBeNull();
    expect(document.activeElement).toBe(sayfa);
  });

  it('seçilmeyen değişiklik gerekçeyle reddedilir; kart yanıtlandı olur', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const sayfa = screen.getByRole('dialog');
    await fireEvent.click(within(sayfa).getByRole('checkbox', { name: /lib\.rs/ }));
    await fireEvent.input(within(sayfa).getByRole('textbox'), {
      target: { value: 'lib.rs kalsın' },
    });
    await fireEvent.click(within(sayfa).getByRole('button', { name: 'Seçilenleri onayla' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(arka.komutlar('arac_onayi')).toEqual([
      {
        oturum: 'o1',
        kararlar: [
          { cagri: 'c1', muhur: 'm1', karar: { tip: 'Onayla' } },
          { cagri: 'c2', muhur: 'm2', karar: { tip: 'Reddet', gerekce: 'lib.rs kalsın' } },
        ],
      },
    ]);
    expect(screen.getByText('Yanıtlandı')).toBeTruthy();
  });

  it('Escape karar vermeden kapatır; istek kartta bekler', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(arka.komutlar('arac_onayi')).toEqual([]);
    expect(screen.getByRole('button', { name: 'İncele' })).toBeTruthy();
  });

  it('odak sayfanın içinde döner', async () => {
    const { arka } = await ciz();
    onayIstegi(arka);
    await fireEvent.click(await screen.findByRole('button', { name: 'İncele' }));
    const reddet = screen.getByRole('button', { name: 'Reddet' });
    reddet.focus();
    await fireEvent.keyDown(window, { key: 'Tab' });
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Kapat' }));
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/ana/ekranlar.test.ts`
Expected: FAIL. Yer tutucu `Ana.svelte` `durum` almıyor ve hiçbir şey çizmiyor; ilk `findByRole('heading', …)` zaman aşımına uğrar.

- [ ] **Step 3: Bileşenleri yaz**

`ui/src/ana/Ana.svelte` (dosyanın tamamı):

```svelte
<!--
  Ana pencere (1c tasarımı §8.1): menü, oturum listesi, konuşma ve en altta
  durum çubuğu. Onay sayfası konuşmanın üstünde açılır.
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { AnaDurum } from './anaDurum.svelte';
  import DurumCubugu from './DurumCubugu.svelte';
  import Konusma from './Konusma.svelte';
  import Menu from './Menu.svelte';
  import OnaySayfasi from './OnaySayfasi.svelte';
  import OturumListesi from './OturumListesi.svelte';

  let { durum = new AnaDurum() }: { durum?: AnaDurum } = $props();

  onMount(() => {
    void durum.baslat();
  });

  const onayIstegi = $derived.by(() => {
    const acik = durum.onay;
    if (!acik) return null;
    const oge = durum.ekranlar
      .get(acik.oturum)
      ?.ogeler.find((o) => o.tip === 'OnayIstegi' && o.islem === acik.islem);
    return oge?.tip === 'OnayIstegi' && oge.durum === 'Bekliyor' ? oge : null;
  });
</script>

<div class="ana">
  <div class="govde">
    <Menu {durum} />
    <OturumListesi {durum} />
    <Konusma {durum} />
  </div>
  <DurumCubugu {durum} />
  {#if durum.onay && onayIstegi}
    {@const { oturum, islem } = durum.onay}
    <OnaySayfasi
      ogeler={onayIstegi.ogeler}
      onkarar={(kararlar) => durum.onayla(oturum, islem, kararlar)}
      onkapat={() => durum.onayiKapat()}
    />
  {/if}
  <div class="gizli-erisilebilir" aria-live="polite">{durum.duyuru}</div>
</div>

<style>
  .ana {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg-app);
  }
  .govde {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
```

`ui/src/ana/Menu.svelte`:

```svelte
<!-- Menü (1c tasarımı §8.1): 1c'de yalnızca Oturumlar ve "Anahtarı değiştir". -->
<script lang="ts">
  import NavItem from '../bilesenler/NavItem.svelte';
  import NavRail from '../bilesenler/NavRail.svelte';
  import { m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';

  let { durum }: { durum: AnaDurum } = $props();
</script>

<NavRail label={m('menu.ad')} class="menu">
  {#snippet header()}
    <div class="marka">{m('uygulama.adi')}</div>
    <div class="slogan">{m('uygulama.slogan')}</div>
  {/snippet}
  <NavItem icon="message-square" active>{m('menu.oturumlar')}</NavItem>
  <NavItem icon="key-round" onclick={() => durum.anahtariDegistir()}>
    {m('menu.anahtar')}
  </NavItem>
  {#snippet footer()}
    <div class="cizgi"></div>
    <div class="alt-not">{m('uygulama.alt_not')}</div>
  {/snippet}
</NavRail>

<style>
  .marka {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
  .slogan {
    margin-top: 4px;
    font: var(--type-tagline);
    color: var(--text-muted);
  }
  .cizgi {
    width: 34px;
    height: 2px;
    margin-bottom: 10px;
    background: var(--copper-400);
  }
  .alt-not {
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
```

`ui/src/ana/OturumListesi.svelte`:

```svelte
<!--
  Oturum listesi (1c tasarımı §5.4, §8.1): kayıtlardan okunan oturumlar, gün
  gruplarıyla. Açık oturumun anlık hâli (çalışıyor, onay bekliyor, şart
  bekliyor) kayıttaki son durumdan önce gelir.
-->
<script lang="ts">
  import IconButton from '../bilesenler/IconButton.svelte';
  import Input from '../bilesenler/Input.svelte';
  import SessionRow from '../bilesenler/SessionRow.svelte';
  import { m } from '../dil/tr';
  import type { OturumSatiri } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { GUN_GRUBU_ADI, gruplara, satirBasligi, satirDurumu, zamanEtiketi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  let arama = $state('');

  /** Listeye henüz düşmemiş yeni oturumlar en üstte görünür. */
  const satirlar = $derived.by((): OturumSatiri[] => {
    const listede = new Set(durum.liste.map((s) => s.oturum));
    const yeniler: OturumSatiri[] = [];
    for (const [oturum, ekran] of durum.ekranlar) {
      if (listede.has(oturum)) continue;
      yeniler.push({
        oturum,
        dosya: oturum,
        baslik: null,
        calisma_alani: ekran.calisma_alani,
        son_degisiklik_ms: Date.now(),
        durum: { tip: 'Bos' },
      });
    }
    return [...yeniler, ...durum.liste];
  });

  const gruplar = $derived.by(() => {
    const aranan = arama.trim().toLocaleLowerCase('tr-TR');
    const suzulmus = aranan
      ? satirlar.filter((s) =>
          [satirBasligi(s), s.calisma_alani ?? ''].some((t) =>
            t.toLocaleLowerCase('tr-TR').includes(aranan),
          ),
        )
      : satirlar;
    return gruplara(suzulmus, (s) => s.son_degisiklik_ms, Date.now());
  });
</script>

<aside class="liste" aria-label={m('liste.baslik')}>
  <div class="ust">
    <h2>{m('liste.baslik')}</h2>
    <IconButton icon="plus" label={m('liste.yeni')} onclick={() => durum.yeniOturum()} />
  </div>
  <div class="ara">
    <Input
      type="search"
      iconLeft="search"
      placeholder={m('liste.ara')}
      aria-label={m('liste.ara')}
      bind:value={arama}
    />
  </div>
  <div class="satirlar">
    {#each gruplar as g (g.grup)}
      <div class="grup">{m(GUN_GRUBU_ADI[g.grup])}</div>
      {#each g.ogeler as s (s.dosya)}
        {@const anlik = s.oturum ? (durum.ekranlar.get(s.oturum)?.anlik ?? null) : null}
        {@const d = satirDurumu(s.durum, anlik)}
        <SessionRow
          title={satirBasligi(s)}
          preview={s.durum.tip === 'Okunamadi' ? s.durum.neden : (s.calisma_alani ?? undefined)}
          time={zamanEtiketi(s.son_degisiklik_ms, Date.now())}
          status={d.status}
          statusLabel={d.etiket}
          active={s.oturum !== null && s.oturum === durum.secili}
          disabled={s.oturum === null}
          onclick={() => s.oturum && durum.oturumSec(s.oturum)}
        />
      {/each}
    {:else}
      <p class="bos">{arama.trim() ? m('liste.sonuc_yok') : m('liste.bos')}</p>
    {/each}
  </div>
</aside>

<style>
  .liste {
    width: var(--list-width);
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border-right: var(--border-width) solid var(--border-subtle);
  }
  .ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 18px 10px;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 19px / 1 var(--font-sans);
    color: var(--text-title);
  }
  .ara {
    padding: 0 16px 12px;
  }
  .satirlar {
    flex: 1;
    overflow-y: auto;
    padding: 0 10px 12px;
  }
  .grup {
    padding: 10px 8px 6px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .bos {
    margin: 16px 8px;
    font: var(--type-small);
    color: var(--text-muted);
  }
</style>
```

`ui/src/ana/Konusma.svelte`:

```svelte
<!--
  Konuşma sütunu (1c tasarımı §8.1, §9.3): başlık, uyarı şeritleri, geçmiş ve
  canlı olaylar, Composer. Yukarı kaydırınca eski turlar sayfa sayfa eklenir;
  kullanıcı en alttayken yeni içerik görünür kalır.
-->
<script lang="ts">
  import { tick } from 'svelte';
  import Button from '../bilesenler/Button.svelte';
  import Composer from '../bilesenler/Composer.svelte';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import { KIP_ADI, MODEL_PROFILI_ADI, m } from '../dil/tr';
  import { KIPLER, type Kip } from '../ipc/tipler';
  import type { AnaDurum } from './anaDurum.svelte';
  import { satirBasligi } from './bicim';
  import KonusmaOgesi from './KonusmaOgesi.svelte';

  let { durum }: { durum: AnaDurum } = $props();

  const ekran = $derived(durum.aktif);
  const baslik = $derived.by(() => {
    const satir = durum.liste.find((s) => s.oturum === durum.secili);
    return satir ? satirBasligi(satir) : m('oturum.yeni');
  });
  const modeller = $derived(
    (durum.uygulama?.modeller ?? []).map((s) => ({
      value: s.kimlik,
      label: m(MODEL_PROFILI_ADI[s.profil]),
      hint: s.kimlik,
    })),
  );
  const kipler = KIPLER.map((k) => ({ value: k, label: m(KIP_ADI[k]) }));

  const kapaliNedeni = $derived.by(() => {
    if (!ekran) return null;
    const s = ekran.salt_okunur;
    if (s?.tip === 'CalismaAlaniYok') return m('hata.calisma_alani_yok', { yol: s.yol });
    if (s?.tip === 'CalismaAlaniKayitsiz') return m('hata.calisma_alani_kayitsiz');
    if (ekran.kapandi) return m('konusma.kapandi');
    return null;
  });

  let metin = $state('');
  let akis = $state<HTMLDivElement>();
  let icerik = $state<HTMLDivElement>();
  let altta = true;

  // Oturum değişince en alttan başlanır.
  $effect(() => {
    void durum.secili;
    altta = true;
  });

  // İçerik büyüdüğünde (yeni öğe, akan metin, ekrana giren öğenin gerçek
  // boyu) kullanıcı en alttaysa en altta kalınır.
  $effect(() => {
    void durum.secili;
    if (!akis) return;
    akis.scrollTop = akis.scrollHeight;
    if (!icerik || typeof ResizeObserver === 'undefined') return;
    const kutu = akis;
    const gozcu = new ResizeObserver(() => {
      if (altta) kutu.scrollTop = kutu.scrollHeight;
    });
    gozcu.observe(icerik);
    return () => gozcu.disconnect();
  });

  function kaydirildi() {
    if (!akis) return;
    altta = akis.scrollHeight - akis.scrollTop - akis.clientHeight < 48;
    if (akis.scrollTop < 40) void eskileriYukle();
  }

  /** Eski sayfa eklenince okunan yer kaymaz. */
  async function eskileriYukle() {
    if (!ekran?.daha_eski_var || !akis) return;
    const once = akis.scrollHeight;
    await durum.eskileriYukle(ekran.oturum);
    await tick();
    if (akis) akis.scrollTop += akis.scrollHeight - once;
  }
</script>

<section class="konusma" aria-label={m('konusma.ad')}>
  {#if durum.uygulama?.baglanti === 'Yok'}
    <div class="serit uyari" role="status">
      <Icon name="wifi-off" size={15} />
      <span>{m('hata.baglanti_yok')}</span>
    </div>
  {/if}
  {#if durum.hata}
    <div class="serit tehlike" role="alert">
      <Icon name="triangle-alert" size={15} />
      <span>{durum.hata}</span>
      <IconButton icon="x" size="sm" label={m('genel.kapat')} onclick={() => (durum.hata = null)} />
    </div>
  {/if}

  {#if ekran}
    <header class="ust">
      <div class="baslik-blogu">
        <h2>{baslik}</h2>
        {#if ekran.calisma_alani}
          <span class="yol"><Icon name="folder" size={14} />{ekran.calisma_alani}</span>
        {/if}
      </div>
      <div class="eylemler">
        {#if ekran.kapandi}
          <Button size="sm" iconLeft="rotate-ccw" onclick={() => durum.yenidenAc(ekran.oturum)}>
            {m('konusma.yeniden_ac')}
          </Button>
        {/if}
        <IconButton icon="x" label={m('konusma.kapat')} onclick={() => durum.kapat(ekran.oturum)} />
      </div>
    </header>

    {#if ekran.salt_okunur}
      <div class="serit bilgi" role="status">
        <Icon name="lock" size={15} />
        <span>{kapaliNedeni}</span>
      </div>
    {/if}

    <div class="akis" bind:this={akis} onscroll={kaydirildi}>
      <div class="icerik" bind:this={icerik}>
        {#if ekran.daha_eski_var}
          <div class="eski">
            <Button variant="ghost" size="sm" iconLeft="chevron-down" onclick={eskileriYukle}>
              {m('konusma.eskiler')}
            </Button>
          </div>
        {/if}
        {#each ekran.ogeler as oge (oge.anahtar)}
          <div class="oge">
            <KonusmaOgesi {oge} {durum} oturum={ekran.oturum} />
          </div>
        {:else}
          <p class="bos">{m('konusma.bos')}</p>
        {/each}
      </div>
    </div>

    <div class="alt">
      <Composer
        bind:value={metin}
        models={modeller}
        model={ekran.ozet.model ?? durum.uygulama?.varsayilan_model ?? ''}
        modes={kipler}
        mode={ekran.ozet.kip ?? 'Siki'}
        running={ekran.anlik !== 'Bos'}
        disabled={kapaliNedeni !== null}
        placeholder={kapaliNedeni ?? undefined}
        onsend={(t) => durum.gonder(t)}
        oncancel={() => durum.iptal()}
        onmodelchange={(v) => durum.modelSec(v)}
        onmodechange={(v) => durum.kipSec(v as Kip)}
      />
    </div>
  {:else}
    <div class="karsilama">
      <IconTile icon="message-square" tone="accent" size={40} />
      <h2>{m('konusma.karsilama.baslik')}</h2>
      <p>{m('konusma.karsilama.metin')}</p>
      <Button variant="primary" iconLeft="folder-plus" onclick={() => durum.yeniOturum()}>
        {m('liste.yeni')}
      </Button>
    </div>
  {/if}
</section>

<style>
  .konusma {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-app);
  }
  .serit {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    font: var(--type-small);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .serit span {
    flex: 1;
  }
  .uyari {
    color: var(--status-warn);
    background: var(--amber-tint);
  }
  .tehlike {
    color: var(--status-danger);
    background: var(--red-tint);
  }
  .bilgi {
    color: var(--text-secondary);
    background: var(--surface-sunken);
  }
  .ust {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 14px 22px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .baslik-blogu {
    min-width: 0;
    flex: 1;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 20px / 1.2 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .yol {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 4px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .eylemler {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .akis {
    flex: 1;
    overflow-y: auto;
  }
  .icerik {
    min-height: 100%;
    box-sizing: border-box;
    padding: 20px 22px;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  /* Ekran dışındaki öğeler çizilmez (1c tasarımı §9.3). */
  .oge {
    content-visibility: auto;
    contain-intrinsic-size: auto 96px;
  }
  .eski {
    display: flex;
    justify-content: center;
  }
  .bos {
    margin: auto;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .alt {
    padding: 0 22px 18px;
  }
  .karsilama {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 32px;
    text-align: center;
  }
  .karsilama h2 {
    white-space: normal;
  }
  .karsilama p {
    max-width: 52ch;
    margin: 0 0 8px;
    font: var(--type-body);
    color: var(--text-secondary);
  }
</style>
```

`ui/src/ana/KonusmaOgesi.svelte`:

```svelte
<!-- Konuşmadaki tek öğe (1c tasarımı §5.5, §8.1, §8.2, §10). -->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import CodeBlock from '../bilesenler/CodeBlock.svelte';
  import type { IkonAdi } from '../bilesenler/ikonlar';
  import Message from '../bilesenler/Message.svelte';
  import ThoughtPanel from '../bilesenler/ThoughtPanel.svelte';
  import ToolCallCard from '../bilesenler/ToolCallCard.svelte';
  import { AKTARIM_DURMA_METNI, m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import type { BitisNedeni, OturumId } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';
  import type { AnaDurum } from './anaDurum.svelte';
  import { aracBasligi, aracHedefi, aracKartDurumu, ciktiOnizlemesi } from './bicim';
  import OnayKarti from './OnayKarti.svelte';

  let { oge, durum, oturum }: { oge: EkranOgesi; durum: AnaDurum; oturum: OturumId } = $props();

  const ARAC_IKONU: Record<string, IkonAdi> = {
    dosya_oku: 'file-text',
    dosya_listele: 'folder-open',
    icerik_ara: 'file-search',
    dosya_yaz: 'file-pen',
    dosya_duzenle: 'file-pen',
  };

  function turSonu(neden: BitisNedeni): string {
    if (neden.tip === 'Sinir') return m('tur.bitti.Sinir', { aciklama: neden.aciklama });
    return m(`tur.bitti.${neden.tip}`);
  }
</script>

{#if oge.tip === 'KullaniciMesaji'}
  <Message author="user">
    <p class="duz">{oge.metin}</p>
  </Message>
{:else if oge.tip === 'Yanit'}
  <Message streaming={oge.akista}>
    {#if oge.metin}
      <Markdown metin={oge.metin} akista={oge.akista} baglantiAc={durum.baglantiAc} />
    {/if}
    {#if oge.dusunce}
      <div class="dusunce">
        <ThoughtPanel>
          <Markdown metin={oge.dusunce} akista={oge.akista} baglantiAc={durum.baglantiAc} />
        </ThoughtPanel>
      </div>
    {/if}
  </Message>
{:else if oge.tip === 'AracCagrisi'}
  {@const d = aracKartDurumu(oge.durum)}
  <ToolCallCard
    tool={aracBasligi(oge)}
    target={aracHedefi(oge)}
    status={d.status}
    statusLabel={d.etiket}
    icon={ARAC_IKONU[oge.arac] ?? 'box'}
  >
    {#if oge.sonuc}
      {@const o = ciktiOnizlemesi(oge.sonuc)}
      <CodeBlock code={o.kod} truncated={o.kirpildi} maxHeight="320px" />
    {/if}
  </ToolCallCard>
{:else if oge.tip === 'OnayIstegi'}
  <OnayKarti {oge} onincele={() => durum.onayiAc(oturum, oge.islem)} />
{:else if oge.tip === 'SartIstegi'}
  <Card tone="accent" icon="shield" title={m('sart.kart.baslik', { surum: oge.surum })}>
    {#snippet actions()}
      {#if oge.durum === 'Bekliyor'}
        <Button size="sm" onclick={() => durum.sartPenceresiniGoster()}>
          {m('sart.kart.ac')}
        </Button>
      {/if}
    {/snippet}
    <p class="aciklama">
      {oge.durum === 'Bekliyor' ? m('sart.kart.bekliyor') : m('sart.kart.cozuldu')}
    </p>
  </Card>
{:else if oge.tip === 'Hata'}
  <Card tone="danger" icon="triangle-alert" title={m('hata.kart.baslik')}>
    {#snippet actions()}
      {#if oge.mesaj === m('hata.anahtar_gecersiz')}
        <Button size="sm" iconLeft="key-round" onclick={() => durum.anahtariDegistir()}>
          {m('menu.anahtar')}
        </Button>
      {/if}
    {/snippet}
    <p class="aciklama">{oge.mesaj}</p>
  </Card>
{:else if oge.tip === 'TurSonu'}
  <div class={['tur-sonu', `n-${oge.neden.tip}`]}>
    <span>{turSonu(oge.neden)}</span>
  </div>
{:else if oge.tip === 'AktarimDurdu'}
  <Card tone="danger" icon="triangle-alert" title={m('aktarim.baslik')}>
    {#snippet actions()}
      <Button size="sm" iconLeft="rotate-ccw" onclick={() => durum.yenidenAc(oturum)}>
        {m('konusma.yeniden_ac')}
      </Button>
    {/snippet}
    <p class="aciklama">{m(AKTARIM_DURMA_METNI[oge.neden])}</p>
  </Card>
{/if}

<style>
  .duz {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .dusunce {
    margin-top: 10px;
  }
  .aciklama {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .tur-sonu {
    display: flex;
    align-items: center;
    gap: 12px;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .tur-sonu::before,
  .tur-sonu::after {
    content: '';
    flex: 1;
    height: var(--border-width);
    background: var(--border-subtle);
  }
  .n-Hata {
    color: var(--status-danger);
  }
</style>
```

`ui/src/ana/OnayKarti.svelte`:

```svelte
<!--
  Konuşmadaki onay isteği (1c tasarımı §8.2): küçük bir kart. "İncele" onay
  sayfasını açar; sayfa karar verilmeden kapatılırsa istek burada bekler.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import Card from '../bilesenler/Card.svelte';
  import { m } from '../dil/tr';
  import type { EkranOgesi } from '../durum/oturum';
  import { kisaAd } from './onay';

  let {
    oge,
    onincele,
  }: { oge: Extract<EkranOgesi, { tip: 'OnayIstegi' }>; onincele: () => void } = $props();

  const uyariSayisi = $derived(oge.ogeler.reduce((t, o) => t + o.uyarilar.length, 0));
</script>

<Card
  tone={oge.durum === 'Bekliyor' ? 'accent' : 'default'}
  icon="shield"
  title={m('onay.kart.baslik', { sayi: oge.ogeler.length })}
>
  {#snippet actions()}
    {#if oge.durum === 'Bekliyor'}
      <Button variant="primary" size="sm" onclick={onincele}>{m('onay.kart.incele')}</Button>
    {:else if oge.durum === 'Yanitlandi'}
      <Badge tone="ok">{m('onay.kart.yanitlandi')}</Badge>
    {:else}
      <Badge>{m('onay.kart.gecersiz')}</Badge>
    {/if}
  {/snippet}
  <ul class="hedefler">
    {#each oge.ogeler as o (o.cagri)}
      <li>
        <span class="ad">{kisaAd(o)}</span>
        <span class="aciklama">{o.aciklama}</span>
      </li>
    {/each}
  </ul>
  {#if uyariSayisi > 0}
    <Badge tone="warn" icon="triangle-alert">{m('onay.kart.uyari', { sayi: uyariSayisi })}</Badge>
  {/if}
</Card>

<style>
  .hedefler {
    margin: 0 0 8px;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    gap: 10px;
    font: var(--type-small);
    min-width: 0;
  }
  .ad {
    font-family: var(--font-mono);
    color: var(--text-body);
    flex: none;
  }
  .aciklama {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

`ui/src/ana/OnaySayfasi.svelte`:

```svelte
<!--
  Onay sayfası (1c tasarımı §8.2, §9.5; tasarım sistemindeki ApprovalSheet).
  Değişiklik listesi, seçilenin diff'i, "ne" ve "nerede", uyarılar, ret
  gerekçesi ve üç karar. Seçilmeyen çağrılar gerekçeyle reddedilir. Odak
  sayfanın içinde kalır; Escape karar vermeden kapatır. "Bu değişiklikler geri
  alınabilir" cümlesi geri alma gelene kadar yazılmaz.
-->
<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import Button from '../bilesenler/Button.svelte';
  import Checkbox from '../bilesenler/Checkbox.svelte';
  import DiffView from '../bilesenler/DiffView.svelte';
  import { diffCozumle, diffSayilari } from '../bilesenler/diff';
  import Icon from '../bilesenler/Icon.svelte';
  import IconButton from '../bilesenler/IconButton.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import Textarea from '../bilesenler/Textarea.svelte';
  import { m } from '../dil/tr';
  import type { CagriId, OnayGirdisi, OnayOgesi } from '../ipc/tipler';
  import { kararlariUret, kisaAd, type OnaySecimi } from './onay';

  let {
    ogeler,
    onkarar,
    onkapat,
  }: {
    ogeler: OnayOgesi[];
    onkarar: (kararlar: OnayGirdisi[]) => Promise<void>;
    onkapat: () => void;
  } = $props();

  const kimlik = $props.id();
  const secilenler = new SvelteSet<CagriId>(untrack(() => ogeler.map((o) => o.cagri)));
  let aktif = $state(0);
  let gerekce = $state('');
  let gonderiliyor = $state(false);
  let pencere = $state<HTMLDivElement>();

  const aktifOge = $derived(ogeler[aktif] ?? ogeler[0]);
  const parcalar = $derived(aktifOge?.diff ? diffCozumle(aktifOge.diff) : []);
  const tumu = $derived(secilenler.size === ogeler.length);

  onMount(() => {
    const onceki = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    pencere?.focus();
    return () => onceki?.focus();
  });

  function tus(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onkapat();
      return;
    }
    if (e.key !== 'Tab' || !pencere) return;
    const odaklanabilir = [
      ...pencere.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex="0"]',
      ),
    ];
    const ilk = odaklanabilir[0];
    const son = odaklanabilir.at(-1);
    if (!ilk || !son) return;
    const odak = document.activeElement;
    if (e.shiftKey && (odak === ilk || odak === pencere || !pencere.contains(odak))) {
      e.preventDefault();
      son.focus();
    } else if (!e.shiftKey && (odak === son || !pencere.contains(odak))) {
      e.preventDefault();
      ilk.focus();
    }
  }

  function tumunuSec(sec: boolean) {
    secilenler.clear();
    if (sec) for (const o of ogeler) secilenler.add(o.cagri);
  }

  async function karar(secim: OnaySecimi) {
    gonderiliyor = true;
    try {
      await onkarar(kararlariUret(ogeler, secim, secilenler, gerekce));
    } finally {
      gonderiliyor = false;
    }
  }
</script>

<svelte:window onkeydown={tus} />

<div class="perde">
  <div
    class="pencere"
    bind:this={pencere}
    role="dialog"
    aria-modal="true"
    aria-labelledby="{kimlik}-baslik"
    aria-describedby="{kimlik}-alt"
    tabindex="-1"
  >
    <header class="ust">
      <IconTile icon="shield" tone="accent" size={40} />
      <div class="baslik-blogu">
        <h2 id="{kimlik}-baslik">{m('onay.sayfa.baslik', { sayi: ogeler.length })}</h2>
        <p id="{kimlik}-alt">{m('onay.sayfa.alt')}</p>
      </div>
      <IconButton icon="x" label={m('genel.kapat')} onclick={onkapat} />
    </header>

    <div class="izgara">
      <div class="dosyalar">
        <div class="dosyalar-ust">
          <Checkbox
            checked={tumu}
            indeterminate={!tumu && secilenler.size > 0}
            label={m('onay.sayfa.tumunu_sec')}
            onchange={tumunuSec}
          />
          <span class="sayi">
            {m('onay.sayfa.secili', { secili: secilenler.size, toplam: ogeler.length })}
          </span>
        </div>
        {#each ogeler as o, i (o.cagri)}
          {@const s = o.diff ? diffSayilari(diffCozumle(o.diff)) : null}
          <div class={['dosya', { etkin: i === aktif }]}>
            <Checkbox
              checked={secilenler.has(o.cagri)}
              label={kisaAd(o)}
              hint={s ? `+${s.eklenen} −${s.silinen}` : undefined}
              onchange={(v) => (v ? secilenler.add(o.cagri) : secilenler.delete(o.cagri))}
            />
            <IconButton
              icon="chevron-right"
              size="sm"
              active={i === aktif}
              label={m('onay.sayfa.goster', { ad: kisaAd(o) })}
              onclick={() => (aktif = i)}
            />
          </div>
        {/each}
      </div>

      <div class="onizleme">
        <div class="onizleme-ust">
          <Icon name="file-text" size={16} />
          <div class="yol-blogu">
            <div class="ad">{kisaAd(aktifOge)}</div>
            {#if aktifOge.hedefler[0]}<div class="yol">{aktifOge.hedefler[0]}</div>{/if}
          </div>
        </div>
        <div class="diff">
          {#each parcalar as p, i (i)}
            <DiffView hunkHeader={p.baslik} lines={p.satirlar} />
          {:else}
            <p class="bos">{m('onay.sayfa.diff_yok')}</p>
          {/each}
        </div>
      </div>
    </div>

    <dl class="bilgiler">
      <div class="bilgi">
        <dt><Icon name="info" size={16} />{m('onay.sayfa.ne')}</dt>
        <dd>{aktifOge.aciklama}</dd>
      </div>
      <div class="bilgi">
        <dt><Icon name="folder" size={16} />{m('onay.sayfa.nerede')}</dt>
        <dd class="mono">
          {#each aktifOge.hedefler as h (h)}<span>{h}</span>{/each}
        </dd>
      </div>
      {#if aktifOge.uyarilar.length > 0}
        <div class="bilgi uyari">
          <dt><Icon name="triangle-alert" size={16} />{m('onay.sayfa.uyarilar')}</dt>
          <dd>
            <ul>
              {#each aktifOge.uyarilar as u, i (i)}<li>{u}</li>{/each}
            </ul>
          </dd>
        </div>
      {/if}
    </dl>

    <div class="alt">
      <div class="gerekce">
        <span id="{kimlik}-gerekce">{m('onay.sayfa.gerekce')}</span>
        <Textarea
          rows={2}
          bind:value={gerekce}
          placeholder={m('onay.sayfa.gerekce_yer')}
          aria-labelledby="{kimlik}-gerekce"
        />
      </div>
      <div class="dugmeler">
        <Button
          variant="approve"
          size="lg"
          iconLeft="check"
          disabled={gonderiliyor}
          onclick={() => karar('tumu')}
        >
          {m('onay.sayfa.tumunu_onayla')}
        </Button>
        <Button
          variant="caution"
          size="lg"
          iconLeft="check"
          disabled={gonderiliyor || secilenler.size === 0}
          onclick={() => karar('secilenler')}
        >
          {m('onay.sayfa.secilenleri_onayla')}
        </Button>
        <Button
          variant="danger"
          size="lg"
          iconLeft="x"
          disabled={gonderiliyor}
          onclick={() => karar('hicbiri')}
        >
          {m('onay.sayfa.reddet')}
        </Button>
      </div>
    </div>
  </div>
</div>

<style>
  .perde {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 28px;
    background: var(--scrim);
    backdrop-filter: var(--blur-scrim);
  }
  .pencere {
    width: min(1140px, 100%);
    max-height: 100%;
    overflow-y: auto;
    padding: 26px 28px;
    background: var(--bg-panel);
    border: var(--border-width) solid var(--border-default);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-overlay);
  }
  .pencere:focus {
    outline: none;
  }
  .ust {
    display: flex;
    align-items: flex-start;
    gap: 18px;
    margin-bottom: 22px;
  }
  .baslik-blogu {
    flex: 1;
  }
  h2 {
    margin: 0;
    font: var(--fw-semibold) 23px / 1.3 var(--font-sans);
    letter-spacing: var(--tracking-tight);
    color: var(--text-title);
  }
  .baslik-blogu p {
    margin: 6px 0 0;
    font: var(--type-subtitle);
    color: var(--text-muted);
  }
  .izgara {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 18px;
    margin-bottom: 20px;
  }
  .dosyalar,
  .onizleme {
    background: var(--surface-sunken);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-lg);
  }
  .dosyalar {
    padding: 10px;
  }
  .dosyalar-ust {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px 12px;
    margin-bottom: 8px;
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .sayi {
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .dosya {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 6px 6px 10px;
    border-radius: var(--radius-md);
    transition: var(--transition-control);
  }
  .dosya :global(.onay-kutusu) {
    flex: 1;
  }
  .etkin {
    background: var(--surface-selected);
    box-shadow: var(--shadow-accent-edge);
  }
  .onizleme {
    overflow: hidden;
    min-width: 0;
  }
  .onizleme-ust {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    color: var(--text-muted);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .yol-blogu {
    min-width: 0;
  }
  .ad {
    font: var(--fw-medium) var(--text-small) / 1.3 var(--font-mono);
    color: var(--text-body);
  }
  .yol {
    font: var(--type-meta);
    color: var(--text-muted);
    overflow-wrap: anywhere;
  }
  .diff {
    max-height: 360px;
    overflow: auto;
  }
  .bos {
    margin: 16px;
    font: var(--type-small);
    color: var(--text-muted);
  }
  .bilgiler {
    display: flex;
    flex-direction: column;
    gap: 14px;
    margin: 0 0 20px;
    padding: 20px 0;
    border-top: var(--border-width) solid var(--border-subtle);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .bilgi {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 12px;
    align-items: start;
  }
  dt {
    display: flex;
    align-items: center;
    gap: 10px;
    font: var(--fw-medium) var(--text-small) / 1.5 var(--font-sans);
    color: var(--text-body);
  }
  dd {
    margin: 0;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .mono {
    display: flex;
    flex-direction: column;
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .uyari dt,
  .uyari dd {
    color: var(--status-warn);
  }
  ul {
    margin: 0;
    padding-left: 18px;
  }
  .alt {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 22px;
    align-items: end;
  }
  .gerekce span {
    display: block;
    margin-bottom: 8px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .dugmeler {
    display: flex;
    gap: 12px;
  }
</style>
```

`ui/src/ana/DurumCubugu.svelte`:

```svelte
<!--
  Durum çubuğu (1c tasarımı §8.3): oturumun hâli, bağlantı ve kimlik deposu
  uyarıları; son gözlenen bakiye ve zamanı, token kullanımı (düşünce dökümüyle),
  yönlendirilen model, izin kipi. Bağlam doluluğu gösterilmez.
-->
<script lang="ts">
  import StatusBar from '../bilesenler/StatusBar.svelte';
  import StatusItem from '../bilesenler/StatusItem.svelte';
  import { KIP_ADI, m } from '../dil/tr';
  import type { AnaDurum } from './anaDurum.svelte';
  import { kredi, saat, sayi } from './bicim';

  let { durum }: { durum: AnaDurum } = $props();

  const ekran = $derived(durum.aktif);
  const ozet = $derived(ekran?.ozet ?? null);
</script>

<StatusBar label={m('durum.ad')}>
  {#snippet left()}
    {#if ekran?.anlik === 'Calisiyor'}
      <StatusItem icon="loader-circle" tone="warn">{m('durum.calisiyor')}</StatusItem>
    {:else if ekran?.anlik === 'OnayBekliyor'}
      <StatusItem icon="shield" tone="warn">{m('oturum.durum.onay')}</StatusItem>
    {:else if ekran?.anlik === 'SartBekliyor'}
      <StatusItem icon="shield" tone="warn">{m('oturum.durum.sart')}</StatusItem>
    {:else}
      <StatusItem icon="circle-check" tone="ok">{m('durum.hazir')}</StatusItem>
    {/if}
    {#if durum.uygulama?.baglanti === 'Yok'}
      <StatusItem icon="wifi-off" tone="danger">{m('durum.baglanti_yok')}</StatusItem>
    {/if}
    {#if durum.uygulama?.anahtar_deposu === 'Bellekte'}
      <StatusItem icon="key-round" tone="warn" title={m('hata.anahtar_bellekte')}>
        {m('durum.anahtar_bellekte')}
      </StatusItem>
    {/if}
  {/snippet}
  {#snippet right()}
    {#if ozet?.son_kredi?.kalan}
      <StatusItem icon="coins" title={m('durum.bakiye_ipucu')}>
        {m('durum.bakiye', {
          kalan: kredi(ozet.son_kredi.kalan),
          saat: saat(ozet.son_kredi.zaman_ms),
        })}
      </StatusItem>
    {/if}
    {#if ozet}
      <StatusItem
        title={m('durum.token_ipucu', {
          istem: sayi(ozet.istem),
          tamamlama: sayi(ozet.tamamlama),
          dusunce: sayi(ozet.dusunce),
        })}
      >
        {m('durum.token', { sayi: sayi(ozet.istem + ozet.tamamlama) })}
      </StatusItem>
      {#if ozet.yonlendirilen_model}
        <StatusItem icon="cpu" title={m('durum.model_ipucu')}>{ozet.yonlendirilen_model}</StatusItem>
      {/if}
      {#if ozet.kip}
        <StatusItem icon="shield" tone="accent">{m(KIP_ADI[ozet.kip])}</StatusItem>
      {/if}
    {/if}
  {/snippet}
</StatusBar>
```

- [ ] **Step 4: İç odak halkasını kaldır**

`ui/src/bilesenler/Textarea.svelte`'nin `<style>` bloğunda `textarea { … }` kuralının hemen altına ekle:

```css
  /* Odak çerçevede gösterilir; içteki alan ikinci bir halka çizmez. */
  textarea:focus-visible {
    outline: none;
  }
```

`ui/src/bilesenler/Input.svelte`'nin `<style>` bloğunda `input { … }` kuralının hemen altına (`input.mono`'dan önce) ekle:

```css
  /* Odak çerçevede gösterilir; içteki alan ikinci bir halka çizmez. */
  input:focus-visible {
    outline: none;
  }
```

- [ ] **Step 5: Denetimleri koştur**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: 0 hata, 0 uyarı; 13 dosya, 153 test geçti; `Paket denetimi temiz.`

- [ ] **Step 6: Commit**

```bash
git add ui/src/ana ui/src/bilesenler/Input.svelte ui/src/bilesenler/Textarea.svelte
git commit -m "ui: ana pencerenin ekranları; onay sayfası ve durum çubuğu"
git push
```

---

### Task 3: Şart penceresi

Anahtar formu (§7.2), EVREN'in şart metni (§7.3, §7.4) ve kilitli görünüm. Pencere açık temayla açılır (`sartlar.html`'deki `kalfa-light`). Tasarım sistemindeki `OnboardingScreen` yalnızca görsel referanstır: Kalfa'nın kendi maddelerini ve onay kutusunu göstermez; metin EVREN'den geldiği gibidir.

**Files:**
- Modify: `ui/src/sartlar/Sartlar.svelte` (yer tutucunun yerine)
- Create: `ui/src/sartlar/sartDurum.svelte.ts`, `ui/src/sartlar/{AnahtarFormu,SartMetniEkrani,KilitliEkran}.svelte`
- Test: `ui/src/sartlar/sartlar.test.ts`

**Interfaces:**
- Consumes: `ipc/sartlar.ts` (yalnızca bu), `ipc/hata.ts`, `ipc/tipler.ts`; Görev 1'in `sahte/arka.ts` (`SahteArka`, `GECERSIZ_ANAHTAR`); 1c-3a'nın `Badge`, `Button`, `Input`, `IconTile`, `Markdown`.
- Produces:
  - **`SartDurum`:** alanlar `durum`, `mesgul`, `hata`, `not`, `depo`; yöntemler `yukle()`, `anahtarKaydet(anahtar)`, `kabul(surum)`, `reddet()`, `metniGoster()`, `baglantiAc(adres)`.
  - **`Sartlar.svelte`:** prop `durum?: SartDurum`.

**Davranış notları (testler sınar):**
- **Anahtar formu:** geçersiz anahtar saklanmaz; neden söylenir, alan boşalır. Geçerli anahtardan sonra şart metni gelir, kabul kendiliğinden gitmez. Anahtar bellekte tutulacaksa bu söylenir.
- **Şart metni:** başlıkta sürüm ve esaslılık yazar; görsel yüklenmez, onay kutusu yoktur.
- **Kabul:** "Kabul ediyorum" kabulü bir kez, gösterilen sürümle gönderir. Sürüm okurken değiştiyse yeni metin "Şart metni bu sırada güncellendi." notuyla gösterilir.
- **Ret:** "Kabul etmiyorum" kilitli görünüme geçer; metin yeniden gösterilebilir. Retle yarışan kabul tıklaması kilitli görünümde kalır.

- [ ] **Step 1: Başarısız testi yaz**

`ui/src/sartlar/sartlar.test.ts`:

```ts
import { clearMocks } from '@tauri-apps/api/mocks';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { SartPenceresiDurumu } from '../ipc/tipler';
import { GECERSIZ_ANAHTAR, SahteArka } from '../sahte/arka';
import Sartlar from './Sartlar.svelte';

afterEach(() => clearMocks());

async function ciz(sart: SartPenceresiDurumu, ayar: (a: SahteArka) => void = () => {}) {
  const arka = new SahteArka().kur();
  arka.sart = sart;
  ayar(arka);
  render(Sartlar);
  await waitFor(() => expect(arka.komutlar('sart_penceresi_durumu')).toHaveLength(1));
  return arka;
}

async function anahtarGir(anahtar: string) {
  const alan = await screen.findByLabelText<HTMLInputElement>('Anahtar');
  await fireEvent.input(alan, { target: { value: anahtar } });
  await fireEvent.click(screen.getByRole('button', { name: 'Kaydet' }));
  return alan;
}

describe('anahtar formu (1c tasarımı §7.2)', () => {
  it('geçersiz anahtar saklanmaz; neden söylenir, alan boşalır', async () => {
    const arka = await ciz({ tip: 'AnahtarGerekli', neden: 'Yok' });
    const alan = await anahtarGir(GECERSIZ_ANAHTAR);
    expect(await screen.findByRole('alert')).toHaveProperty(
      'textContent',
      'Anahtar geçersiz ya da eksik. EVREN portalından anahtarınızı kontrol edip yeniden girin.',
    );
    expect(alan.value).toBe('');
    expect(arka.komutlar('anahtari_kaydet')).toEqual([{ anahtar: GECERSIZ_ANAHTAR }]);
  });

  it('kayıtlı anahtar geçersizleştiyse bunu söyler', async () => {
    await ciz({ tip: 'AnahtarGerekli', neden: 'KayitliGecersiz' });
    expect(await screen.findByText(/Kayıtlı anahtar artık geçerli değil/)).toBeTruthy();
  });

  it('geçerli anahtardan sonra şart metni gelir; kabul kendiliğinden gitmez', async () => {
    const arka = await ciz({ tip: 'AnahtarGerekli', neden: 'Yok' }, (a) => {
      a.uygulama.anahtar_deposu = 'Bellekte';
    });
    await anahtarGir('  evren_llm_dogru  ');
    expect(await screen.findByRole('heading', { name: 'EVREN kullanım şartları' })).toBeTruthy();
    expect(arka.komutlar('anahtari_kaydet')).toEqual([{ anahtar: 'evren_llm_dogru' }]);
    expect(screen.getByText(/bellekte tutulacak/)).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });
});

describe('şart metni (1c tasarımı §7.3, §7.4)', () => {
  const METIN = { surum: 3, esasli: true, metin: '## Madde 1\n\n![rozet](https://img.shields.io/x)' };

  it('sürüm ve esaslılık yazılır; görsel yüklenmez; onay kutusu yok', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN });
    expect(await screen.findByText('Sürüm 3')).toBeTruthy();
    expect(screen.getByText('Esaslı değişiklik')).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Madde 1' })).toBeTruthy();
    expect(document.querySelector('img')).toBeNull();
    expect(screen.queryByRole('checkbox')).toBeNull();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });

  it('"Kabul ediyorum" kabulü bir kez, gösterilen sürümle gönderir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => (a.sartMetni = METIN));
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByText('Tamamlandı. Kalfa açılıyor.')).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([{ surum: 3 }]);
  });

  it('sürüm okurken değiştiyse yeni metin notla gösterilir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => {
      a.sartMetni = { surum: 4, esasli: false, metin: 'Yeni metin' };
    });
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByText('Şart metni bu sırada güncellendi.')).toBeTruthy();
    expect(screen.getByText('Sürüm 4')).toBeTruthy();
    expect(screen.getByText('Küçük değişiklik')).toBeTruthy();
    expect(arka.sart.tip).toBe('SartGerekli');
  });

  it('"Kabul etmiyorum" kilitli görünüme geçer; metin yeniden gösterilebilir', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN }, (a) => (a.sartMetni = METIN));
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul etmiyorum' }));
    expect(await screen.findByRole('heading', { name: 'Şartlar kabul edilmedi' })).toBeTruthy();
    expect(screen.getByText(/Anahtarınız saklı kalır/)).toBeTruthy();
    await fireEvent.click(screen.getByRole('button', { name: 'Metni yeniden göster' }));
    expect(await screen.findByText('Sürüm 3')).toBeTruthy();
    expect(arka.komutlar('sartlari_kabul_et')).toEqual([]);
  });

  it('ret ile yarışan kabul tıklaması kilitli görünümde kalır', async () => {
    const arka = await ciz({ tip: 'SartGerekli', metin: METIN });
    arka.sart = { tip: 'Kilitli' };
    await fireEvent.click(await screen.findByRole('button', { name: 'Kabul ediyorum' }));
    expect(await screen.findByRole('heading', { name: 'Şartlar kabul edilmedi' })).toBeTruthy();
  });
});
```

- [ ] **Step 2: Testin başarısız olduğunu gör**

Run: `cd ui && npx vitest run src/sartlar`
Expected: FAIL. Yer tutucu ekran komut çağırmıyor; `sart_penceresi_durumu` beklenirken zaman aşımı olur.

- [ ] **Step 3: Durum sınıfını ve bileşenleri yaz**

`ui/src/sartlar/sartDurum.svelte.ts`:

```ts
// Şart penceresinin durumu ve komutları (1c tasarımı §7). Kabul isteği
// yalnızca `kabul` yönteminden, yani "Kabul ediyorum" düğmesinden gider; bu
// sınıf başka hiçbir yerde `sartlari_kabul_et` çağırmaz (üst belge §11.8).
// Yalnızca şart penceresinin komutlarını içe aktarır.
import { m } from '../dil/tr';
import { komutHatasi } from '../ipc/hata';
import * as ipc from '../ipc/sartlar';
import type { DepoTuru, SartPenceresiDurumu } from '../ipc/tipler';

export class SartDurum {
  durum = $state<SartPenceresiDurumu | null>(null);
  /** Komut sürüyor: düğmeler kapalı. */
  mesgul = $state(false);
  hata = $state<string | null>(null);
  /** "Şart metni bu sırada güncellendi" notu. */
  not = $state<string | null>(null);
  /** Anahtarın kaydedildiği yer; bellekteyse kullanıcıya söylenir. */
  depo = $state<DepoTuru | null>(null);

  async yukle(): Promise<void> {
    await this.#calistir(async () => {
      this.durum = await ipc.sartPenceresiDurumu();
    });
  }

  /** Anahtar Rust'a bir kez gider; bu sınıf onu saklamaz. */
  async anahtarKaydet(anahtar: string): Promise<void> {
    await this.#calistir(async () => {
      const sonuc = await ipc.anahtariKaydet(anahtar);
      switch (sonuc.tip) {
        case 'Gecersiz':
          this.hata = m('hata.anahtar_gecersiz');
          return;
        case 'Ulasilamadi':
          this.hata = m('hata.evren_ulasilamiyor');
          return;
        case 'Kaydedildi':
          this.depo = sonuc.depo;
          this.durum = sonuc.sonraki;
      }
    });
  }

  async kabul(surum: number): Promise<void> {
    await this.#calistir(async () => {
      const yanit = await ipc.sartlariKabulEt(surum);
      switch (yanit.tip) {
        case 'Kabul':
          this.durum = { tip: 'Tamam' };
          return;
        case 'YeniMetin':
          this.durum = { tip: 'SartGerekli', metin: yanit.metin };
          this.not = m('hata.sart_guncellendi');
          return;
        case 'Kilitli':
          this.durum = { tip: 'Kilitli' };
      }
    });
  }

  async reddet(): Promise<void> {
    await this.#calistir(async () => {
      await ipc.sartlariReddet();
      this.durum = await ipc.sartPenceresiDurumu();
    });
  }

  async metniGoster(): Promise<void> {
    await this.#calistir(async () => {
      this.durum = await ipc.sartMetniniGoster();
    });
  }

  baglantiAc = (adres: string): void => {
    ipc.baglantiAc(adres).catch((e: unknown) => {
      this.hata = komutHatasi(e).mesaj;
    });
  };

  async #calistir(is: () => Promise<void>): Promise<void> {
    this.mesgul = true;
    this.hata = null;
    this.not = null;
    try {
      await is();
    } catch (e) {
      this.hata = komutHatasi(e).mesaj;
      // Durum Rust'ta değişmiş olabilir (ör. kabul bu arada geçersizleşti).
      this.durum = await ipc.sartPenceresiDurumu().catch(() => this.durum);
    } finally {
      this.mesgul = false;
    }
  }
}
```

`ui/src/sartlar/Sartlar.svelte` (dosyanın tamamı):

```svelte
<!--
  Şart penceresi (1c tasarımı §7): anahtar formu, EVREN'in şart metni ya da
  kilitli görünüm. Açık temayla açılır (tasarım sistemindeki OnboardingScreen
  görsel referanstır; içeriği EVREN'in metnidir).
-->
<script lang="ts">
  import { onMount } from 'svelte';
  import { m } from '../dil/tr';
  import AnahtarFormu from './AnahtarFormu.svelte';
  import KilitliEkran from './KilitliEkran.svelte';
  import SartMetniEkrani from './SartMetniEkrani.svelte';
  import { SartDurum } from './sartDurum.svelte';

  let { durum = new SartDurum() }: { durum?: SartDurum } = $props();

  onMount(() => {
    void durum.yukle();
  });
</script>

<main class="sartlar">
  <div class="kolon">
    <header class="marka">
      <div class="ad">{m('uygulama.adi')}</div>
      <div class="slogan">{m('uygulama.slogan')}</div>
    </header>

    <div class="kart">
      {#if durum.durum?.tip === 'AnahtarGerekli'}
        <AnahtarFormu {durum} neden={durum.durum.neden} />
      {:else if durum.durum?.tip === 'SartGerekli'}
        <SartMetniEkrani {durum} metin={durum.durum.metin} />
      {:else if durum.durum?.tip === 'Kilitli'}
        <KilitliEkran {durum} />
      {:else if durum.durum?.tip === 'Tamam'}
        <p class="tamam" role="status">{m('sartlar.tamam')}</p>
      {:else if durum.hata}
        <p class="hata" role="alert">{durum.hata}</p>
      {/if}
    </div>

    <p class="alt-not">{m('sartlar.alt_not')}</p>
  </div>
</main>

<style>
  .sartlar {
    height: 100%;
    overflow-y: auto;
    display: flex;
    justify-content: center;
    padding: 44px 28px;
    box-sizing: border-box;
    background: var(--bg-app);
    color: var(--text-body);
  }
  .kolon {
    width: min(760px, 100%);
  }
  .marka {
    text-align: center;
    margin-bottom: 30px;
  }
  .ad {
    font: var(--type-wordmark);
    letter-spacing: var(--tracking-wordmark);
    color: var(--text-title);
  }
  .slogan {
    margin-top: 6px;
    font: var(--type-tagline);
    color: var(--text-muted);
  }
  .kart {
    padding: 32px 36px;
    background: var(--surface-card);
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-2xl);
    box-shadow: var(--shadow-paper);
  }
  .tamam {
    margin: 0;
    font: var(--type-body);
    color: var(--text-secondary);
  }
  .hata {
    margin: 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .alt-not {
    margin-top: 22px;
    text-align: center;
    font: var(--type-meta);
    color: var(--text-muted);
  }
</style>
```

`ui/src/sartlar/AnahtarFormu.svelte`:

```svelte
<!--
  Anahtar formu (1c tasarımı §7.2). Anahtar gönderilince alan boşaltılır;
  anahtar arayüzde tutulmaz ve hiçbir komut onu geri döndürmez.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import Input from '../bilesenler/Input.svelte';
  import { m, type MetinAnahtari } from '../dil/tr';
  import type { AnahtarNedeni } from '../ipc/tipler';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum, neden }: { durum: SartDurum; neden: AnahtarNedeni } = $props();

  const kimlik = $props.id();
  let anahtar = $state('');

  const ACIKLAMA: Record<AnahtarNedeni, MetinAnahtari> = {
    Yok: 'anahtar.aciklama.Yok',
    KayitliGecersiz: 'anahtar.aciklama.KayitliGecersiz',
    Degistir: 'anahtar.aciklama.Degistir',
  };

  async function gonder(e: SubmitEvent) {
    e.preventDefault();
    const girilen = anahtar.trim();
    if (!girilen || durum.mesgul) return;
    anahtar = '';
    await durum.anahtarKaydet(girilen);
  }
</script>

<form onsubmit={gonder} novalidate>
  <h1>{m('anahtar.baslik')}</h1>
  <p class="aciklama">{m(ACIKLAMA[neden])}</p>

  <label for="{kimlik}-anahtar">{m('anahtar.etiket')}</label>
  <Input
    id="{kimlik}-anahtar"
    type="password"
    mono
    size="lg"
    iconLeft="key-round"
    placeholder="evren_llm_…"
    autocomplete="off"
    spellcheck={false}
    invalid={durum.hata !== null}
    aria-describedby="{kimlik}-not"
    bind:value={anahtar}
  />
  {#if durum.hata}
    <p class="hata" role="alert">{durum.hata}</p>
  {/if}
  <p class="not" id="{kimlik}-not">{m('anahtar.not')}</p>

  <div class="dugmeler">
    <Button
      type="submit"
      variant="primary"
      size="lg"
      loading={durum.mesgul}
      disabled={!anahtar.trim() || durum.mesgul}
    >
      {m('anahtar.kaydet')}
    </Button>
  </div>
</form>

<style>
  h1 {
    margin: 0 0 8px;
    font: var(--type-display);
    color: var(--text-title);
  }
  .aciklama {
    margin: 0 0 24px;
    font: var(--type-subtitle);
    color: var(--text-secondary);
  }
  label {
    display: block;
    margin-bottom: 8px;
    font: var(--type-small);
    color: var(--text-secondary);
  }
  .hata {
    margin: 10px 0 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .not {
    margin: 10px 0 0;
    font: var(--type-meta);
    color: var(--text-muted);
  }
  .dugmeler {
    display: flex;
    justify-content: flex-end;
    margin-top: 24px;
  }
</style>
```

`ui/src/sartlar/SartMetniEkrani.svelte`:

```svelte
<!--
  EVREN'in şart metni (1c tasarımı §7.4). Başlıkta sürüm ve değişikliğin esaslı
  olup olmadığı yazar. Metin özetlenmeden, güvenli Markdown ile gösterilir;
  görseller yüklenmez. Onay kutusu yoktur: yalnızca iki düğme.
-->
<script lang="ts">
  import Badge from '../bilesenler/Badge.svelte';
  import Button from '../bilesenler/Button.svelte';
  import { m } from '../dil/tr';
  import type { SartMetni } from '../ipc/tipler';
  import Markdown from '../markdown/Markdown.svelte';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum, metin }: { durum: SartDurum; metin: SartMetni } = $props();

  const kimlik = $props.id();
</script>

<header class="ust">
  <h1 id="{kimlik}-baslik">{m('sart.baslik')}</h1>
  <div class="rozetler">
    <Badge tone="accent">{m('sart.surum', { surum: metin.surum })}</Badge>
    {#if metin.esasli}
      <Badge tone="warn">{m('sart.esasli')}</Badge>
    {:else}
      <Badge>{m('sart.esasli_degil')}</Badge>
    {/if}
  </div>
</header>
<p class="aciklama">{m('sart.aciklama')}</p>

{#if durum.not}
  <p class="not" role="status">{durum.not}</p>
{/if}
{#if durum.depo === 'Bellekte'}
  <p class="uyari" role="status">{m('hata.anahtar_bellekte')}</p>
{/if}

<!-- Kaydırılabilir metin klavyeyle de kaydırılabilsin diye odaklanabilir. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="metin" tabindex="0" role="region" aria-labelledby="{kimlik}-baslik">
  <Markdown metin={metin.metin} belge baglantiAc={durum.baglantiAc} />
</div>

{#if durum.hata}
  <p class="hata" role="alert">{durum.hata}</p>
{/if}

<div class="dugmeler">
  <Button size="lg" disabled={durum.mesgul} onclick={() => durum.reddet()}>
    {m('sart.reddet')}
  </Button>
  <Button
    size="lg"
    variant="primary"
    loading={durum.mesgul}
    disabled={durum.mesgul}
    onclick={() => durum.kabul(metin.surum)}
  >
    {m('sart.kabul')}
  </Button>
</div>

<style>
  .ust {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 6px;
  }
  h1 {
    margin: 0;
    font: var(--type-display);
    color: var(--text-title);
  }
  .rozetler {
    display: flex;
    gap: 8px;
  }
  .aciklama {
    margin: 0 0 20px;
    font: var(--type-subtitle);
    color: var(--text-secondary);
  }
  .not,
  .uyari {
    margin: 0 0 16px;
    padding: 10px 14px;
    border-radius: var(--radius-md);
    font: var(--type-small);
  }
  .not {
    color: var(--status-info);
    background: var(--blue-tint);
  }
  .uyari {
    color: var(--status-warn);
    background: var(--amber-tint);
  }
  .metin {
    max-height: 52vh;
    overflow-y: auto;
    padding: 20px 4px 20px 0;
    border-top: var(--border-width) solid var(--border-subtle);
    border-bottom: var(--border-width) solid var(--border-subtle);
  }
  .metin:focus-visible {
    outline: none;
    box-shadow: var(--shadow-focus);
  }
  .hata {
    margin: 16px 0 0;
    font: var(--type-small);
    color: var(--status-danger);
  }
  .dugmeler {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 22px;
  }
</style>
```

`ui/src/sartlar/KilitliEkran.svelte`:

```svelte
<!--
  Kilitli görünüm (1c tasarımı §7.4): şartlar reddedildi. Anahtar saklı kalır;
  metin yeniden gösterilebilir.
-->
<script lang="ts">
  import Button from '../bilesenler/Button.svelte';
  import IconTile from '../bilesenler/IconTile.svelte';
  import { m } from '../dil/tr';
  import type { SartDurum } from './sartDurum.svelte';

  let { durum }: { durum: SartDurum } = $props();
</script>

<div class="kilitli">
  <IconTile icon="lock" tone="warn" size={40} />
  <h1>{m('kilitli.baslik')}</h1>
  <p>{m('kilitli.metin')}</p>
  {#if durum.hata}
    <p class="hata" role="alert">{durum.hata}</p>
  {/if}
  <Button variant="primary" size="lg" loading={durum.mesgul} onclick={() => durum.metniGoster()}>
    {m('kilitli.goster')}
  </Button>
</div>

<style>
  .kilitli {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font: var(--type-display);
    color: var(--text-title);
  }
  p {
    margin: 0;
    max-width: 62ch;
    font: var(--type-body);
    color: var(--text-secondary);
  }
  .hata {
    font: var(--type-small);
    color: var(--status-danger);
  }
</style>
```

- [ ] **Step 4: Denetimleri koştur**

Run: `cd ui && npm run check && npm test && npm run build`
Expected: 0 hata, 0 uyarı; 14 dosya, 165 test geçti; `Paket denetimi temiz.` Paket denetimi şart penceresinin paketinde ana pencere komutu olmadığını da doğrular.

- [ ] **Step 5: Commit**

```bash
git add ui/src/sartlar
git commit -m "ui: şart penceresi; anahtar formu, şart metni ve kilitli görünüm"
git push
```

---

### Task 4: Ekran önizlemesi, son doğrulama ve durum dosyası

Geliştirme sunucusunda iki pencerenin ekranlarını sahte arka uçla açan önizleme sayfası. Ana pencerede örnek oturumlar ve senaryolu bir tur vardır: mesaj gönderilince düşünce ve yanıt akar, onay isteği gelir, karar verilince tur biter. Şart penceresi `#sartlar`, `#sartlar-anahtar`, `#sartlar-kilitli` ile açılır. Derlemeye girmez.

**Files:**
- Create: `ui/onizleme.html`, `ui/src/onizleme/giris.ts`, `ui/src/onizleme/senaryo.ts`
- Modify: `ui/vite.config.ts` (yalnızca açıklama), `docs/GELISTIRME_DURUMU.md`

- [ ] **Step 1: Önizleme dosyalarını yaz**

`ui/onizleme.html`:

```html
<!doctype html>
<html lang="tr">
  <head>
    <meta charset="utf-8" />
    <title>Kalfa — ekran önizlemesi</title>
  </head>
  <body class="kalfa-root">
    <div id="kok"></div>
    <script type="module" src="/src/onizleme/giris.ts"></script>
  </body>
</html>
```

`ui/src/onizleme/giris.ts`:

```ts
// Ekran önizlemesi: yalnızca geliştirme sunucusunda (`npm run dev`,
// http://localhost:5173/onizleme.html). Ekranlar sahte arka uçla çalışır;
// derlemeye girmez. `#sartlar`, `#sartlar-anahtar`, `#sartlar-kilitli` şart
// penceresini açar.
import { mount } from 'svelte';
import '../stiller';
import Ana from '../ana/Ana.svelte';
import Sartlar from '../sartlar/Sartlar.svelte';
import { anaSenaryosu, sartSenaryosu } from './senaryo';

const kok = document.getElementById('kok');
if (!kok) throw new Error('#kok bulunamadı');

const pencere = location.hash.slice(1);
if (pencere.startsWith('sartlar')) {
  document.body.classList.add('kalfa-light');
  sartSenaryosu(pencere).kur();
  mount(Sartlar, { target: kok });
} else {
  anaSenaryosu().kur();
  mount(Ana, { target: kok });
}
```

`ui/src/onizleme/senaryo.ts` (metinler örnektir, EVREN'den gelmez):

````ts
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
````

`ui/vite.config.ts`'nin başındaki açıklamanın iki satırını şununla değiştir:

```ts
// İki pencere iki ayrı giriş noktasıdır (1c tasarımı §3.1). `vitrin.html` ve
// `onizleme.html` yalnızca geliştirme sunucusunda açılır; derlemeye girmez.
```

- [ ] **Step 2: Bütün denetimleri koştur**

Run:

```bash
cd ui
npm run check
npm test
npm run build
grep -l "mockIPC\|SahteArka" dist/assets/*.js
```

Expected:
- `npm run check`: 0 hata, 0 uyarı.
- `npm test`: 14 dosya, 167 test.
- `npm run build`: `Paket denetimi temiz.`
- Son `grep` hiçbir dosya göstermez: sahte arka uç pakete girmez.

- [ ] **Step 3: Önizlemeyi aç (isteğe bağlı, ekranı olan ortamda)**

Run: `cd ui && npm run dev`, sonra tarayıcıda `http://localhost:5173/onizleme.html`.
Expected:
- Ana pencerede dört oturum ve bir okunamayan kayıt görünür.
- Mesaj gönderilince düşünce ve yanıt akar; "Onay isteği: 2 değişiklik" kartı gelir.
- "İncele" onay sayfasını açar. Karar verilince tur "Tamamlandı" ile biter; durum çubuğundaki bakiye ve token sayısı güncellenir.
- `#sartlar`, `#sartlar-anahtar`, `#sartlar-kilitli` şart penceresini açık temayla açar (adresi değiştirdikten sonra sayfayı yenile).

- [ ] **Step 4: Durum dosyasına 1c-3b bölümünü ekle**

`docs/GELISTIRME_DURUMU.md`'nin sonuna, 1c-3a bölümünden sonra ekle. Sayıları kendi çıktılarına göre yaz; Linux sütununu inceleyici doldurur.

```markdown
### 1c-3b — ekranlar (tamamlandı)

Plan: [`2026-09-23-kalfa-1c-3b-ekranlar.md`](superpowers/plans/2026-09-23-kalfa-1c-3b-ekranlar.md).
İki pencerenin ekranları yazılıp komutlara bağlandı: ana pencerede menü,
oturum listesi, konuşma, onay kartı ve onay sayfası, durum çubuğu; şart
penceresinde anahtar formu, EVREN şart metni ve kilitli görünüm. Testler ve
geliştirmedeki ekran önizlemesi (`ui/onizleme.html`) sahte bir arka uçla
(`ui/src/sahte/arka.ts`) çalışır; uygulama paketine girmez.

| Kontrol | Windows | Linux |
| --- | --- | --- |
| `npm ci` | … | (inceleme) |
| `npm run check` | … | (inceleme) |
| `npm test` | … dosya, … test geçti | (inceleme) |
| `npm run build` + paket denetimi | … | (inceleme) |
```

- [ ] **Step 5: Commit**

```bash
git add ui/onizleme.html ui/src/onizleme ui/vite.config.ts docs/GELISTIRME_DURUMU.md
git commit -m "ui: ekran önizlemesi; docs: 1c-3b tamamlandı, doğrulama kaydı"
git push
```

---

## İnceleme (Claude)

1. **Plana uygunluk:** kodu plan metniyle karşılaştırır; planda olmayan satırları ayrıca okur.
2. **Windows ve Linux:** `npm ci`, `npm run check`, `npm test`, `npm run build`.
3. **Tasarımla görsel karşılaştırma:** önizlemeyi tarayıcıda açar, senaryolu turu oynatır. Ana pencereyi, onay sayfasını ve şart penceresinin üç hâlini tasarım sisteminin görselleriyle karşılaştırır (`assets/generated/kalfa-ui-*.png`).
4. **Durum dosyası:** Linux sütununu doldurur, PR'a yorum yazar; temizse birleştirir.
