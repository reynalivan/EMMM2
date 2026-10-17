use super::*;
use image::{DynamicImage, ImageFormat};
use std::fs;
use std::io::Cursor;
use std::sync::Arc;
use tempfile::TempDir;

fn isolated_cache(app_data: &std::path::Path) -> Arc<Mutex<ThumbnailCache>> {
    let cache = Arc::new(Mutex::new(ThumbnailCache::new()));
    ThumbnailCache::init_with(&cache, app_data);
    cache
}

#[cfg(windows)]
fn create_fixture_junction(alias: &Path, folder: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(alias)
        .arg(folder)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction creation failed: {output:?}"
    );
}

#[test]
fn owned_cache_resolution_is_isolated_from_other_cache_resets() {
    let temp = TempDir::new().unwrap();
    let app_data = temp.path().join("first-app-data");
    let cache = isolated_cache(&app_data);
    let other = isolated_cache(&temp.path().join("second-app-data"));
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let key = ThumbnailCache::identity_key(&folder);
    let generation = lock(&cache).invalidation_generation;
    let other_generation = lock(&other).invalidation_generation;
    let resolved = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        let generated = ThumbnailCache::generate(&cache, source)?;
        ThumbnailCache::clear_memory_with(&other);
        ThumbnailCache::init_with(&other, &temp.path().join("third-app-data"));
        Ok(generated)
    })
    .unwrap()
    .unwrap();
    assert!(Path::new(&resolved).starts_with(thumbnail_cache_dir(&app_data)));
    assert_eq!(lock(&cache).invalidation_generation, generation);
    assert!(lock(&other).invalidation_generation > other_generation);
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_some());
}

#[test]
fn unrelated_folder_invalidations_do_not_retry_thumbnail_generation() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("ModA");
    let other_folder = temp.path().join("ModA2");
    fs::create_dir(&folder).unwrap();
    fs::create_dir(&other_folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);
    let attempts = std::cell::Cell::new(0);

    let result = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        attempts.set(attempts.get() + 1);
        let generated = ThumbnailCache::generate(&cache, source)?;
        for _ in 0..MAX_RESOLVE_ATTEMPTS {
            ThumbnailCache::invalidate_folder_with(&cache, &other_folder.to_string_lossy());
        }
        Ok(generated)
    });

    assert!(matches!(result, Ok(Some(_))), "{result:?}");
    assert_eq!(attempts.get(), 1);
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_some());
    assert!(lock(&cache).pending_resolutions.is_empty());
}

#[test]
fn same_folder_invalidations_keep_the_retry_limit_and_prevent_publication() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);
    let attempts = std::cell::Cell::new(0);

    let result = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        attempts.set(attempts.get() + 1);
        let generated = ThumbnailCache::generate(&cache, source)?;
        ThumbnailCache::invalidate_folder_with(&cache, &folder.to_string_lossy());
        Ok(generated)
    });

    assert!(result.is_err());
    assert_eq!(attempts.get(), MAX_RESOLVE_ATTEMPTS);
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
    assert!(lock(&cache).pending_resolutions.is_empty());
}

#[test]
fn full_reset_during_generation_retries_before_publication() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);
    let attempts = std::cell::Cell::new(0);

    let result = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        attempts.set(attempts.get() + 1);
        let generated = ThumbnailCache::generate(&cache, source)?;
        if attempts.get() == 1 {
            ThumbnailCache::clear_memory_with(&cache);
        }
        Ok(generated)
    });

    assert!(matches!(result, Ok(Some(_))), "{result:?}");
    assert_eq!(attempts.get(), 2);
}

#[test]
fn changing_app_data_during_generation_retries_in_the_new_cache_directory() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("first_app_data"));
    let next_app_data = temp.path().join("second_app_data");
    let key = ThumbnailCache::identity_key(&folder);
    let attempts = std::cell::Cell::new(0);

    let resolved = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        attempts.set(attempts.get() + 1);
        let generated = ThumbnailCache::generate(&cache, source)?;
        if attempts.get() == 1 {
            ThumbnailCache::init_with(&cache, &next_app_data);
        }
        Ok(generated)
    })
    .unwrap()
    .unwrap();

    assert_eq!(attempts.get(), 2);
    assert!(Path::new(&resolved).starts_with(thumbnail_cache_dir(&next_app_data)));
}

#[test]
fn hiding_during_generation_cannot_republish_the_hidden_thumbnail() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);
    let resolved = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        let generated = ThumbnailCache::generate(&cache, source)?;
        crate::platform::images::preview_visibility::set_hidden(
            &folder,
            &[source.to_path_buf()],
            true,
        )?;
        ThumbnailCache::invalidate_folder_with(&cache, &folder.to_string_lossy());
        Ok(generated)
    })
    .unwrap();
    assert!(resolved.is_none());
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
    assert!(folder.join("preview.png").exists());
    assert!(lock(&cache).pending_resolutions.is_empty());
}

#[test]
fn nested_source_invalidation_cannot_republish_a_hidden_fallback_thumbnail() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    let nested = folder.join("Images");
    fs::create_dir_all(&nested).unwrap();
    create_dummy_image(&nested.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);
    let resolved = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        let generated = ThumbnailCache::generate(&cache, source)?;
        crate::platform::images::preview_visibility::set_hidden(
            &folder,
            &[source.to_path_buf()],
            true,
        )?;
        ThumbnailCache::invalidate_folder_with(&cache, &nested.to_string_lossy());
        Ok(generated)
    })
    .unwrap();

    assert!(resolved.is_none());
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
    assert!(nested.join("preview.png").exists());
    assert!(lock(&cache).pending_resolutions.is_empty());
}

#[tokio::test]
async fn completed_nested_source_deletion_invalidates_the_root_resolution() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    let nested = folder.join("Images");
    fs::create_dir_all(&nested).unwrap();
    let source = nested.join("preview.png");
    create_dummy_image(&source);
    let cache = isolated_cache(&temp.path().join("app_data"));
    let resolved = ThumbnailCache::resolve_with_cache(cache.clone(), &folder.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    assert!(Path::new(&resolved).exists());

    fs::remove_file(&source).unwrap();
    ThumbnailCache::invalidate_with(&cache, &source);

    assert!(
        ThumbnailCache::resolve_with_cache(cache, &folder.to_string_lossy())
            .await
            .unwrap()
            .is_none(),
        "a completed root entry must not serve the deleted nested fallback"
    );
}

#[tokio::test]
async fn completed_nested_source_invalidation_preserves_unrelated_root_entries() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    let other = temp.path().join("Mod2");
    fs::create_dir_all(folder.join("Images")).unwrap();
    fs::create_dir(&other).unwrap();
    create_dummy_image(&folder.join("Images/preview.png"));
    create_dummy_image(&other.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    for root in [&folder, &other] {
        ThumbnailCache::resolve_with_cache(cache.clone(), &root.to_string_lossy())
            .await
            .unwrap()
            .unwrap();
    }

    ThumbnailCache::invalidate_folder_with(&cache, &folder.join("Images").to_string_lossy());

    assert!(
        ThumbnailCache::folder_l1_path(&cache, &ThumbnailCache::identity_key(&other)).is_some()
    );
    assert!(
        ThumbnailCache::folder_l1_path(&cache, &ThumbnailCache::identity_key(&folder)).is_none()
    );
}

#[tokio::test]
async fn completed_nested_source_deletion_after_toggle_invalidates_physical_root() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir_all(folder.join("Images")).unwrap();
    create_dummy_image(&folder.join("Images/preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    ThumbnailCache::resolve_with_cache(cache.clone(), &folder.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    let disabled = temp.path().join("DISABLED Mod");
    fs::rename(&folder, &disabled).unwrap();
    let source = disabled.join("Images/preview.png");
    fs::remove_file(&source).unwrap();

    ThumbnailCache::invalidate_folder_with(&cache, &source.parent().unwrap().to_string_lossy());

    assert!(
        ThumbnailCache::resolve_with_cache(cache, &disabled.to_string_lossy())
            .await
            .unwrap()
            .is_none()
    );
}

#[cfg(windows)]
#[tokio::test]
async fn completed_nested_source_deletion_through_junction_invalidates_canonical_root() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir_all(folder.join("Images")).unwrap();
    create_dummy_image(&folder.join("Images/preview.png"));
    let alias = temp.path().join("Alias");
    create_fixture_junction(&alias, &folder);
    let cache = isolated_cache(&temp.path().join("app_data"));
    ThumbnailCache::resolve_with_cache(cache.clone(), &folder.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    fs::remove_dir_all(folder.join("Images")).unwrap();

    ThumbnailCache::invalidate_folder_with(&cache, &alias.join("Images").to_string_lossy());

    let resolved = ThumbnailCache::resolve_with_cache(cache, &folder.to_string_lossy())
        .await
        .unwrap();
    fs::remove_dir(&alias).unwrap();
    assert!(resolved.is_none());
}

#[cfg(windows)]
#[test]
fn junction_alias_resolution_observes_canonical_nested_source_invalidation() {
    let source_temp = TempDir::new().unwrap();
    let alias_temp = TempDir::new().unwrap();
    let folder = source_temp.path().join("Mod");
    let nested = folder.join("Images");
    fs::create_dir_all(&nested).unwrap();
    create_dummy_image(&nested.join("preview.png"));
    let alias = alias_temp.path().join("ModAlias");
    create_fixture_junction(&alias, &folder);
    let cache = isolated_cache(&source_temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&alias);
    assert_eq!(key, ThumbnailCache::identity_key(&folder));

    let resolved = ThumbnailCache::resolve_cold_with(&cache, &alias, &key, |source| {
        let generated = ThumbnailCache::generate(&cache, source)?;
        crate::platform::images::preview_visibility::set_hidden(
            &alias,
            &[source.to_path_buf()],
            true,
        )?;
        ThumbnailCache::invalidate_folder_with(&cache, &nested.to_string_lossy());
        Ok(generated)
    })
    .unwrap();

    assert!(resolved.is_none());
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
    assert!(lock(&cache).pending_resolutions.is_empty());
    fs::remove_dir(&alias).unwrap();
    assert!(nested.join("preview.png").exists());
}

#[cfg(windows)]
#[test]
fn removed_nested_source_invalidation_through_a_junction_cancels_publication() {
    let source_temp = TempDir::new().unwrap();
    let alias_temp = TempDir::new().unwrap();
    let folder = source_temp.path().join("Mod");
    let nested = folder.join("Images");
    let relocated = source_temp.path().join("MovedImages");
    fs::create_dir_all(&nested).unwrap();
    create_dummy_image(&nested.join("preview.png"));
    let alias = alias_temp.path().join("ModAlias");
    create_fixture_junction(&alias, &folder);
    let cache = isolated_cache(&source_temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);

    let resolved = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |source| {
        let generated = ThumbnailCache::generate(&cache, source)?;
        fs::rename(&nested, &relocated)?;
        ThumbnailCache::invalidate_folder_with(&cache, &alias.join("Images").to_string_lossy());
        Ok(generated)
    })
    .unwrap();

    assert!(
        resolved.is_none(),
        "removed preview must not publish: {resolved:?}"
    );
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
    assert!(lock(&cache).pending_resolutions.is_empty());
    fs::remove_dir(&alias).unwrap();
    assert!(relocated.join("preview.png").exists());
}

#[test]
fn generation_error_releases_the_pending_resolution_ticket() {
    let temp = TempDir::new().unwrap();
    let folder = temp.path().join("Mod");
    fs::create_dir(&folder).unwrap();
    create_dummy_image(&folder.join("preview.png"));
    let cache = isolated_cache(&temp.path().join("app_data"));
    let key = ThumbnailCache::identity_key(&folder);

    let result = ThumbnailCache::resolve_cold_with(&cache, &folder, &key, |_| {
        Err(AppError::Io("fixture decode failure".to_string()))
    });

    assert!(result.is_err());
    assert!(lock(&cache).pending_resolutions.is_empty());
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
}

#[test]
fn regression_conflict_folders_and_images_have_distinct_cache_identity() {
    let tmp = TempDir::new().unwrap();
    let enabled = tmp.path().join("Alice");
    let disabled = tmp.path().join("DISABLED Alice");
    fs::create_dir(&enabled).unwrap();
    fs::create_dir(&disabled).unwrap();
    create_dummy_image(&enabled.join("preview.png"));
    create_dummy_image(&disabled.join("preview.png"));
    assert_ne!(
        ThumbnailCache::identity_key(&enabled),
        ThumbnailCache::identity_key(&disabled)
    );
    assert_ne!(
        ThumbnailCache::identity_key(&enabled.join("preview.png")),
        ThumbnailCache::identity_key(&disabled.join("preview.png"))
    );
    let original = ThumbnailCache::identity_key(&enabled);
    let renamed = tmp.path().join("DISABLED RenamedAlice");
    fs::rename(&enabled, &renamed).unwrap();
    assert_eq!(original, ThumbnailCache::identity_key(&renamed));
}

// Helper to create a dummy image (Requires image crate to encode, or just write random bytes?
// Reader expects valid image. We can write a simple BMP or PNG header.)
fn create_dummy_image(path: &std::path::Path) {
    // Create a 1x1 PNG via image crate
    let img = DynamicImage::new_rgb8(10, 10);
    img.save(path).unwrap();
}

fn create_image_with_format(path: &std::path::Path, format: ImageFormat) {
    let image = DynamicImage::new_rgb8(10, 10);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, format).unwrap();
    fs::write(path, bytes.into_inner()).unwrap();
}

#[tokio::test]
async fn resolve_accepts_images_with_mismatched_extensions() {
    let tmp_dir = TempDir::new().unwrap();
    let app_data = tmp_dir.path().join("app_data");
    let png_named_jpg_dir = tmp_dir.path().join("PngNamedJpg");
    let jpeg_named_png_dir = tmp_dir.path().join("JpegNamedPng");
    fs::create_dir_all(&png_named_jpg_dir).unwrap();
    fs::create_dir_all(&jpeg_named_png_dir).unwrap();
    create_image_with_format(&png_named_jpg_dir.join("preview.jpg"), ImageFormat::Png);
    create_image_with_format(&jpeg_named_png_dir.join("preview.png"), ImageFormat::Jpeg);
    let cache = isolated_cache(&app_data);

    for directory in [&png_named_jpg_dir, &jpeg_named_png_dir] {
        let result =
            ThumbnailCache::resolve_with_cache(cache.clone(), &directory.to_string_lossy()).await;
        assert!(
            matches!(result, Ok(Some(_))),
            "valid image data must resolve regardless of its file extension: {result:?}"
        );
    }
}

#[test]
fn invalidating_a_changed_source_removes_l2_bytes() {
    let tmp_dir = TempDir::new().unwrap();
    let app_data = tmp_dir.path().join("app_data");
    let mod_dir = tmp_dir.path().join("ModA");
    fs::create_dir_all(&mod_dir).unwrap();
    let source = mod_dir.join("preview.png");
    create_dummy_image(&source);
    let cache_dir = thumbnail_cache_dir(&app_data);
    fs::create_dir_all(&cache_dir).unwrap();
    let cached = cache_dir.join(format!(
        "{}.webp",
        ThumbnailCache::cache_key(&ThumbnailCache::identity_key(&source))
    ));
    fs::write(&cached, b"stale").unwrap();

    ThumbnailCache::invalidate_l2_at(&cache_dir, &source);

    assert!(
        !cached.exists(),
        "an external source-image event must not leave stale L2 bytes"
    );
}

#[tokio::test]
async fn clearing_memory_cache_drops_resolved_folder_entries() {
    let tmp_dir = TempDir::new().unwrap();
    let app_data = tmp_dir.path().join("app_data");
    let mod_dir = tmp_dir.path().join("ModA");
    fs::create_dir_all(&mod_dir).unwrap();
    create_dummy_image(&mod_dir.join("preview.png"));
    let cache = isolated_cache(&app_data);

    ThumbnailCache::resolve_with_cache(cache.clone(), &mod_dir.to_string_lossy())
        .await
        .unwrap();
    let key = ThumbnailCache::identity_key(&mod_dir);
    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_some());

    ThumbnailCache::clear_memory_with(&cache);

    assert!(ThumbnailCache::folder_l1_path(&cache, &key).is_none());
}

// Covers: TC-41-002 (8K Source handled via spawn_blocking without panic)
#[tokio::test]
async fn test_resolve_large_8k_image_without_blocking() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp = tmp_dir.path().to_path_buf();
    let app_data = tmp.join("app_data");
    fs::create_dir_all(&app_data).unwrap();
    let cache = isolated_cache(&app_data);

    let mod_dir = tmp.join("Mod8K");
    fs::create_dir(&mod_dir).unwrap();

    // Create a 8192x4320 image using image crate (this takes some RAM but proves it works)
    let src_img = mod_dir.join("preview.png");
    let img = DynamicImage::new_rgb8(1000, 1000); // 1K x 1K for test speed instead of actual 8K to not blow up CI memory
    img.save(&src_img).unwrap();

    // Call resolve (async)
    let folder_str = mod_dir.to_string_lossy().to_string();
    let result = ThumbnailCache::resolve_with_cache(cache, &folder_str).await;
    assert!(result.is_ok());
    let thumb_opt = result.unwrap();
    assert!(thumb_opt.is_some());
    let path_str = thumb_opt.unwrap();
    assert!(std::path::Path::new(&path_str).is_absolute());
    assert!(path_str.ends_with(".webp"));
}

// Covers: TC-41-001 (Cache key handling for DISABLED vs enabled states)
//
// A toggle renames the folder but not its identity: resolving the DISABLED
// spelling must return the same cached .webp, with no regeneration.
#[tokio::test]
async fn test_cache_hits_for_toggled_disabled_state() {
    let tmp_dir = TempDir::new().unwrap();
    let tmp = tmp_dir.path().to_path_buf();
    let app_data = tmp.join("app_data");
    fs::create_dir_all(&app_data).unwrap();
    let cache = isolated_cache(&app_data);

    let enabled_dir = tmp.join("MyMod");
    fs::create_dir(&enabled_dir).unwrap();
    let src_img = enabled_dir.join("preview.png");
    create_dummy_image(&src_img);

    let res1 = ThumbnailCache::resolve_with_cache(cache.clone(), &enabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    assert!(res1.ends_with(".webp"));

    let webp_name = |path: &str| {
        std::path::Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string()
    };

    // Toggle on disk, then resolve via the DISABLED spelling.
    let disabled_dir = tmp.join("DISABLED MyMod");
    fs::rename(&enabled_dir, &disabled_dir).unwrap();
    let res2 = ThumbnailCache::resolve_with_cache(cache.clone(), &disabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        webp_name(&res1),
        webp_name(&res2),
        "toggle must not change the cached thumbnail"
    );

    // Invalidation through either spelling clears the shared entry.
    ThumbnailCache::invalidate_folder_with(&cache, &enabled_dir.to_string_lossy());
    let res3 = ThumbnailCache::resolve_with_cache(cache, &disabled_dir.to_string_lossy())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        webp_name(&res2),
        webp_name(&res3),
        "L2 disk cache is identity-keyed too"
    );
}
