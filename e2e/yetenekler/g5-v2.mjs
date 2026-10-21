// G5 v2: gerçek libtest kanıtı; sıfır/atlanan test, timeout ve eksik önkoşul geçmez.
import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, renameSync } from 'node:fs';
import { dirname, resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const kok = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const sha = veri => createHash('sha256').update(veri).digest('hex');
const git = args => execFileSync('git', args, { cwd: kok, encoding: 'utf8', windowsHide: true }).trim();
const bas = git(['rev-parse', 'HEAD']);
function m01KaynakOzeti(ref = bas) {
  if (!/^[0-9a-f]{40}$/.test(ref)) throw new Error('M01 kaynak başı tam commit SHA olmalı.');
  // Kurucu/NSIS/araç zinciri girdileri dahil bütün kaynak kapanımı.
  // Salt belge değişikliği geçmiş makbuzu bozmaz; eksik/yeni yollar hash'i değiştirir.
  const kayitlar = git(['ls-tree','-r','-z',ref]).split('\0').filter(Boolean).filter(k => {
    const yol = k.slice(k.indexOf('\t') + 1);
    return !yol.startsWith('docs/') && !/^README(?:\.[^/]+)?$/i.test(yol);
  });
  return sha(kayitlar.sort().join('\0'));
}
const args = process.argv.slice(2);
const secenek = ad => { const i = args.indexOf(ad); return i < 0 ? null : args[i + 1]; };
if (args.includes('--m01-kaynak-ozeti')) {
  process.stdout.write(JSON.stringify({ bas, sha256: m01KaynakOzeti(secenek('--m01-kaynak-ozeti') || bas) }) + '\n');
  process.exit(0);
}
const cikti = secenek('--cikti');
if (!cikti) throw new Error('--cikti ile makbuz dizini gereklidir.');
const dizin = resolve(cikti);
mkdirSync(dizin, { recursive: true });
const manifestYolu = join(kok, 'e2e/yetenekler/g5-v2.json');
const hamManifest = readFileSync(manifestYolu);
const manifest = JSON.parse(hamManifest);
const beklenen = [...'KSMW'].flatMap(a => Array.from({ length: 6 }, (_, n) => `${a}${String(n + 1).padStart(2, '0')}`));
if (manifest.sema !== 'kalfa.yetenek.g5.v2' || manifest.tekrar !== 2 || manifest.asgari_gecen !== 20 ||
    JSON.stringify(manifest.gorevler.map(g => g.kimlik)) !== JSON.stringify(beklenen)) throw new Error('24 görevli sabit G5 v2 manifesti bekleniyordu.');
const zorunluKimlikler = new Set(['M01','M04','M06','S04','S06','W03','W04','W05']);
for (const g of manifest.gorevler) {
  if (g.zorunlu_guvenlik !== zorunluKimlikler.has(g.kimlik) ||
      g.gecis_turu !== (g.kimlik === 'S06' ? 'guvenli_seri_desteklenmiyor' : 'gercek_kabul')) throw new Error(`${g.kimlik} zorunlu güvenlik/özellik sınıfı değiştirilemez.`);
  const dosya = readFileSync(join(kok, `crates/kalfa-tools/tests/yetenek_senaryolari/tanimlar/${g.kimlik}.json`), 'utf8').replaceAll('\r\n', '\n');
  if (sha(dosya) !== g.g0_sha256) throw new Error(`${g.kimlik} dondurulmuş G0 SHA uyuşmuyor.`);
  if (!g.testler.length || g.testler.some(t => !/^[a-z_][a-z_0-9:]*$/.test(t.ad))) throw new Error(`${g.kimlik} test eşlemesi geçersiz.`);
}
const fark = git(['status', '--porcelain', '--', 'Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', '.cargo', '.github', 'crates', 'ui/src', 'e2e/yetenekler']);
if (fark) throw new Error('Kaynak commit edilmeden kabul makbuzu üretilmez:\n' + fark);

async function komut(calisan, argumanlar, gunluk) {
  return new Promise(coz => {
    const p = spawn(calisan, argumanlar, { cwd: kok, shell: false, windowsHide: true, detached: process.platform !== 'win32',
      env: { ...process.env, CARGO_TERM_COLOR: 'never' }, stdio: ['ignore','pipe','pipe'] });
    let metin = '', zamanAsimi = false, tamam = false;
    const bitir = kod => {
      if (tamam) return; tamam = true; clearTimeout(saat);
      writeFileSync(gunluk, metin);
      coz({ kod, zaman_asimi: zamanAsimi, gunluk, sha256: sha(metin), metin });
    };
    const durdur = () => {
      if (zamanAsimi) return;
      zamanAsimi = true;
      if (process.platform === 'win32' && p.pid) {
        try { execFileSync('taskkill.exe', ['/PID', String(p.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' }); } catch {}
      } else if (p.pid) { try { process.kill(-p.pid, 'SIGKILL'); } catch {} }
    };
    const saat = setTimeout(durdur, 300_000);
    const ekle = b => { metin += b.toString(); if (Buffer.byteLength(metin) > 8 * 1024 * 1024) durdur(); };
    p.stdout.setEncoding('utf8'); p.stderr.setEncoding('utf8');
    p.stdout.on('data', ekle); p.stderr.on('data', ekle);
    p.once('error', e => { metin += `\nÇalıştırılamadı: ${e.message}\n`; bitir(null); });
    p.once('close', bitir);
  });
}
function tekTestKaniti(k) {
  return k.kod === 0 && !k.zaman_asimi &&
    [...k.metin.matchAll(/^test result: ok\. 1 passed; 0 failed; 0 ignored;/gm)].length === 1;
}
function ciM01Oku() {
  const id = secenek('--m01-ci-run');
  if (!id) return null;
  if (!/^[1-9][0-9]*$/.test(id)) throw new Error('CI koşu kimliği pozitif sayı olmalı.');
  const gh = a => execFileSync('gh', a, { cwd: kok, encoding: 'utf8', windowsHide: true, timeout: 60000, maxBuffer: 16 * 1024 * 1024 });
  const run = JSON.parse(gh(['run','view',id,'--repo','heimdilon/kalfa','--json','databaseId,headSha,url,conclusion,jobs,event,workflowName']));
  const is = run.jobs?.find(j => j.name === 'Kur ve kaldır (Windows)');
  if (run.databaseId !== Number(id) || run.workflowName !== 'Sürüm paketleri' || run.event !== 'workflow_dispatch' ||
      run.conclusion !== 'success' || is?.conclusion !== 'success' ||
      !/^https:\/\/github\.com\/heimdilon\/kalfa\/actions\/runs\/\d+$/.test(run.url) ||
      m01KaynakOzeti(run.headSha) !== m01KaynakOzeti()) throw new Error('GitHub aynı kaynakta başarılı geçici kurucu/M01 koşusunu doğrulamadı.');
  // Başarı alanı veya günlük yerel dosyadan alınmaz: job kimliği aynı run'ın
  // doğrulanmış API yanıtından, ham log da aynı repo/run/job'dan okunur.
  const log = gh(['run','view',id,'--repo','heimdilon/kalfa','--job',String(is.databaseId),'--log']);
  const makbuzlar = [...log.matchAll(/G5_GOREV:M01:([0-9a-f]{64})/g)];
  const sayimlar = [...log.matchAll(/test result: ok\. 1 passed; 0 failed; 0 ignored;/g)];
  if (makbuzlar.length !== 2 || sayimlar.length !== 2 || makbuzlar[0][1] !== makbuzlar[1][1]) throw new Error('M01 CI her tekrar için tam1 test ve aynı çalışma izi makbuzu üretmeli.');
  writeFileSync(join(dizin,'M01-CI-API.json'),JSON.stringify(run,null,2));
  writeFileSync(join(dizin,'M01-CI-job.log'),log);
  return { run: run.url, job: is.databaseId, ci_basi: run.headSha, kaynak_sha256: m01KaynakOzeti(), gunluk_sha256: sha(log), makbuzlar: makbuzlar.map(m => m[1]) };
}
let m01CI = null, m01CINedeni = null;
if (process.platform === 'win32') {
  try { m01CI = ciM01Oku(); } catch (hata) { m01CINedeni = hata.message; }
}
const rapor = { sema: 'kalfa.yetenek.g5.v2.sonuc', tarih: new Date().toISOString(), bas, platform: process.platform,
  manifest_sha256: sha(hamManifest), kosucu_sha256: sha(readFileSync(fileURLToPath(import.meta.url))),
  canli_model_basarisi: false, girdi_jetonu: null, cikti_jetonu: null, gorevler: [], m01_ci: m01CI };
function kaydet() {
  const tam = rapor.gorevler.length === 24;
  const herTekrarGecti = g => g.tekrarlar.length === 2 && g.tekrarlar.every(t => t.kanit_gecerli);
  rapor.gecen = rapor.gorevler.filter(g => g.durum === 'gecti').length;
  rapor.zorunlu_guvenlik_gecti = tam && rapor.gorevler.filter(g => g.zorunlu_guvenlik).every(herTekrarGecti);
  rapor.kabul_gecti = tam && rapor.gecen >= 20 && rapor.zorunlu_guvenlik_gecti;
  const yol = join(dizin, 'sonuc.json'); writeFileSync(yol + '.tmp', JSON.stringify(rapor, null, 2) + '\n'); renameSync(yol + '.tmp', yol);
}
// Yardımcı derleme yürütme değildir; hiçbir kurulum/indirme yapılmaz.
const hazirlik = await komut('cargo', ['build','-p','kalfa-mcp','--bin','kalfa-mcp-test-sunucusu','-p','kalfa-sandbox','--bin','kalfa-kopru','--locked','--offline'], join(dizin,'yardimci-derleme.log'));
if (hazirlik.kod !== 0 || hazirlik.zaman_asimi) throw new Error('Gerçek yardımcılar derlenemedi; hiçbir görev Geçti sayılmadı.');
for (const g of manifest.gorevler) {
  const sonuc = { kimlik: g.kimlik, zorunlu_guvenlik: g.zorunlu_guvenlik, g0_sha256: g.g0_sha256, tekrarlar: [], durum: 'calistirilmadi' };
  if (g.kimlik === 'M01' && process.platform === 'win32') {
    if (m01CI) {
      sonuc.tekrarlar = m01CI.makbuzlar.map((sha256, i) => ({ tekrar: i + 1, kanit_gecerli: true, dis_kanit: m01CI.run, sha256 }));
      sonuc.durum = 'gecti';
    } else sonuc.neden = m01CINedeni || 'Windows M01 yalnız geçici kurucu CI makbuzuyla doğrulanır; yerel ACL testi çalıştırılmadı.';
  } else {
    for (let tekrar = 1; tekrar <= 2; tekrar++) {
      const kanitlar = [];
      for (let n = 0; n < g.testler.length; n++) {
        const t = g.testler[n];
        const a = ['test','-p',t.paket, ...(t.hedef ? ['--test',t.hedef] : ['--lib']), '--locked','--offline', ...(t.ozellik ? ['--features',t.ozellik] : []), '--',t.ad,'--exact','--nocapture','--test-threads=1'];
        const k = await komut('cargo',a,join(dizin,`${g.kimlik}-${tekrar}-${n + 1}.log`));
        const gecerli = tekTestKaniti(k) && (g.gecis_turu !== 'guvenli_seri_desteklenmiyor' || k.metin.includes('G5_DURUM:desteklenmiyor:'));
        const { metin, ...makbuz } = k;
        kanitlar.push({ ...makbuz, test: t.ad, kanit_gecerli: gecerli });
      }
      sonuc.tekrarlar.push({ tekrar, kanit_gecerli: kanitlar.every(k => k.kanit_gecerli), kanitlar });
    }
    sonuc.durum = sonuc.tekrarlar.every(t => t.kanit_gecerli) ? (g.gecis_turu === 'guvenli_seri_desteklenmiyor' ? 'desteklenmiyor' : 'gecti') : 'kaldi';
  }
  rapor.gorevler.push(sonuc); kaydet();
  process.stdout.write(`${g.kimlik}: ${sonuc.durum}\n`);
}
kaydet();
process.stdout.write(`G5 v2: ${rapor.gecen}/24; zorunlu güvenlik: ${rapor.zorunlu_guvenlik_gecti}; kabul: ${rapor.kabul_gecti}\n`);
process.exitCode = rapor.kabul_gecti ? 0 : 1;
