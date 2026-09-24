include!("src/komut_adlari.rs");

fn main() {
    let mut komutlar: Vec<&str> = ANA_PENCERE_KOMUTLARI
        .iter()
        .chain(SART_PENCERESI_KOMUTLARI.iter())
        .copied()
        .collect();
    if std::env::var_os("CARGO_FEATURE_E2E").is_some() {
        komutlar.extend(E2E_KOMUTLARI);
    }
    komutlar.sort_unstable();
    komutlar.dedup();
    // `AppManifest` 'static dilim ister; build betiği kısa ömürlüdür.
    let komutlar: &'static [&'static str] = Box::leak(komutlar.into_boxed_slice());
    println!("cargo:rerun-if-changed=src/komut_adlari.rs");
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(komutlar)),
    )
    .expect("tauri-build başarısız");

    // tauri-winres Common Controls v6 manifest'ini yalnızca ana ikiliye
    // bağlar; test ikilileri onsuz açılamaz (STATUS_ENTRYPOINT_NOT_FOUND).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-test.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
