/**
 * Kalfa Yetenek Programı — G5 Bütünleşik Kabul Sözleşmesi (`e2e/yetenekler/`).
 *
 * Not: Mevcut `e2e/calistir.mjs` yapılandırmasına dokunulmaz. Bu modül,
 * Hat 02 (#38), Hat 03 (#39), Hat 04 (#40) ve Hat 05 (#41) kaynakları birlikte doğrulandıktan sonra
 * G5 kabul kapısında koşulacak uçtan uca senaryo bildirimlerini sabitler.
 */

export const G5_SEMA_SURUMU = "kalfa.yetenek.g5.e2e.v1";
export const G5_SABIT_TARIH = "2026-10-20T00:00:00Z";

export const G5_KABUL_SENARYOLARI = Object.freeze([
  {
    kimlik: "G5-E2E-01",
    baslik: "Proje talimatları, plan ve kullanıcı sorusu",
    gorevler: ["K06", "S05"],
    bagimliHat: "Hat 02 (PR #38)",
    durum: "calistirilmadi",
    devamKosulu:
      "Doğrulanmış bütünleşik başta `AGENTS.md` kapsamı, `plan_guncelle` ve `kullaniciya_sor` arayüz olayları hazır olduğunda koşulur.",
  },
  {
    kimlik: "G5-E2E-02",
    baslik: "Arka plan süreçleri, PTY terminal ve çalışma alanı kilidi",
    gorevler: ["S01", "S02", "S03", "S04", "S06"],
    bagimliHat: "Hat 03 (PR #39)",
    durum: "calistirilmadi",
    devamKosulu:
      "Doğrulanmış bütünleşik başta arka plan süreç yönetimi, PTY terminal uçları ve çalışma alanı yazma kilidi hazır olduğunda koşulur.",
  },
  {
    kimlik: "G5-E2E-03",
    baslik: "Uzak HTTP MCP, OAuth PKCE ve dinamik araç keşfi",
    gorevler: ["M01", "M02", "M03", "M04", "M05", "M06"],
    bagimliHat: "Hat 04 (PR #40)",
    durum: "calistirilmadi",
    devamKosulu:
      "Doğrulanmış bütünleşik başta uzak Streamable HTTP MCP istemcisi, OAuth PKCE kasası ve dinamik araç keşfi hazır olduğunda koşulur.",
  },
  {
    kimlik: "G5-E2E-04",
    baslik: "Güvenli web getirme, SSRF engeli ve kaynak atfı",
    gorevler: ["W01", "W02", "W03", "W04", "W05", "W06"],
    bagimliHat: "Hat 05 (PR #41)",
    durum: "calistirilmadi",
    devamKosulu:
      "Doğrulanmış bütünleşik başta `web_getir`, SSRF doğrulaması, akışkan boyut sınırı ve kaynak atıf kartları hazır olduğunda koşulur.",
  },
]);
