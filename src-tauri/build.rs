fn main() {
    // `sqlx::migrate!` embeds the migration list at compile time, but Cargo
    // otherwise has no file dependency on the directory. Keep test and app
    // binaries in sync when a migration is added or changed.
    println!("cargo:rerun-if-changed=migrations");

    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=common-controls-v6.rc");
        println!("cargo:rerun-if-changed=common-controls-v6.manifest");
        embed_resource::compile_for_everything("common-controls-v6.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to embed Common Controls v6 manifest");

        let windows_attributes = tauri_build::WindowsAttributes::new_without_app_manifest();
        let attributes = tauri_build::Attributes::new().windows_attributes(windows_attributes);
        tauri_build::try_build(attributes).expect("failed to run Tauri build script");
    }

    #[cfg(not(windows))]
    tauri_build::build()
}
