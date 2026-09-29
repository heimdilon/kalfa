# Git korumalarının çözümü: ölçüm ve karar (2026-09-28)

## Sorun

`Siniflandirici::sinifla`, yazılan her hedef için git korumalarını yeniden
çözüyordu (`git_koruma::korumali_yollar`). Bu çözüm, `hooks_path_degeri` ile
`git -C <kök> config --get core.hooksPath` sürecini başlatır. Oturumun araç
aşamasında bu iki yerde oluyordu:

- **Planlama:** `Oturum::karar`, her yazma hedefi için bir kez çalışıyordu.
- **Yürütme:** `tek_calistir`, kilit altında aynı kararı yeniden
  hesaplıyordu.

İki çağrı da async bağlamda, engelleyerek yapılıyordu. Yürütmedeki çağrılar
`FuturesUnordered` içinde tek görevde koştuğu için paralel araçlar da sırayla
bekliyordu.

## Ölçüm

`crates/kalfa-tools/tests/olcum_planlama.rs` (`#[ignore]`) ölçüm düzeneğidir.
Düzenek geçici bir git deposu açar ve Serbest kipte gerçek dosya araçları
kullanır. Model tek bir yanıtta 20 `dosya_yaz` çağrısı yapar. İki süre
ölçülür:

- **Planlama:** mesajdan ilk `AracCagrisiBasladi` olayına kadar geçen süre.
- **Yürütme:** oradan `Tamamlandi` olayına kadar geçen süre.

Ölçüm Windows 11'de, debug derlemesiyle ve git 2.55.0.windows.5 ile yapıldı.
Eski kod (`kalfa-asama-1`, a7aed36) ayrı bir worktree'de ve ayrı hedef
dizininde derlendi. Eski ve yeni kod 5'er kez, sırayla koşturuldu.

| | `git config` / çağrı | Planlama | Yürütme | Toplam |
| --- | --- | --- | --- | --- |
| Eski | 26–44 ms | 511–616 ms | 541–680 ms | 1,09–1,30 s |
| Yeni | 25–39 ms | 31–40 ms | 145–287 ms | 0,18–0,33 s |

WSL Ubuntu-24.04'te (yeni kod, 3 koşu) `git config` çağrı başına ~1,2 ms
sürüyor. Orada planlama 5 ms, toplam 13–14 ms tuttu; sorun Windows'ta süreç
başlatmanın pahalı olmasından geliyor.

Windows'ta `git config` çağrı başına 25–100 ms sürdü; 250 ms'ye
ulaşmadı. Süre soğuk önbellekte, virüs taramasında ya da ağır global
yapılandırmada uzar. Eski kodda planlama ve yürütme ayrı ayrı yaklaşık
`20 × çağrı süresi` tutuyordu. Çağrı 250 ms sürseydi her biri yaklaşık 5 s
olurdu.

## Değişiklik

- **Planlama:** Korumalar model yanıtı başına bir kez, ilk yazan hazırlıkta
  çözülür (`Siniflandirici::toplu_engellemeden`). Yalnızca okuyan yanıt
  git'i hiç çağırmaz; bu davranış eskisiyle aynıdır.
- **Yürütme:** Her çağrı korumaları yine kendisi çözer: kilit altında ve taze
  hazırlıktan sonra (spec §7.3). Çözüm engelleyen iş parçacığında koştuğu
  için aynı adımdaki çağrılar birbirini beklemez. Yürütmedeki kalan süre, 20
  paralel `git config` sürecinden gelir.
- **3a ile ortak kod:** `Siniflandirici::toplu` / `TopluSiniflandirici`,
  3a'daki (geri alma planı) biçimiyle aynen eklendi.

Planın kararı onay kartını belirler; son sözü yürütme söyler. Onay
beklenirken `core.hooksPath` hedefe çevrilirse yürütme yeni korumayı görür.
Bu durumda kararlar tutmaz ve çağrı `PolitikaDegisti` ile çalıştırılmaz. Bunu
`onay_beklerken_kanca_dizini_olan_hedef_eski_onayla_yazilmaz` testi
doğrular.

## Neden `.git/config` doğrudan okunmuyor

`core.hooksPath` depo yapılandırmasının dışından da gelebilir:

- sistem yapılandırması (git'in `etc/gitconfig`'i, Windows'ta ayrıca
  `C:\ProgramData\Git\config`),
- global yapılandırma (`~/.gitconfig`, `$XDG_CONFIG_HOME/git/config`),
- `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM` ve `GIT_CONFIG_NOSYSTEM`,
- `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n`,
- `include.path` ve `includeIf` (`gitdir:`, `onbranch:`,
  `hasconfig:remote.*.url:`),
- `extensions.worktreeConfig` açıksa `config.worktree`.

Bunları kendi ayrıştırıcımızla eksik okumak, korumayı olduğundan dar
çözmek demektir; yani güvenlik sınırı açık kalır (fail-open). Bu yüzden
değer git'e sorulmaya devam eder.

`alt_modul_mu` ise `config`'i doğrudan okur. Orada eksik okuma yalnızca
doğrulamayı başarısız kılar ve korumayı daraltmaz.

Yapılandırma dosyalarının değişiklik zamanına bağlı bir önbellek de
düşünüldü. Ortam değişkenlerini ve koşullu içermeleri güvenle kapsayamadığı
için yapılmadı.
