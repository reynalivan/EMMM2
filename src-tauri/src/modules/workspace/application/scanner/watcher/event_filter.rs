use std::path::Path;

const RELEVANT_EXTENSIONS: &[&str] = &["ini", "json", "png", "jpg", "jpeg", "webp"];

fn is_relevant_path(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("info.json"))
    {
        return true;
    }

    match path.extension().and_then(|e| e.to_str()) {
        None => true,
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            RELEVANT_EXTENSIONS.contains(&lower.as_str())
        }
    }
}

pub(crate) fn should_keep_event_path(path: &Path, watcher_path: &Path) -> bool {
    if !should_keep_structural_event_path(path, watcher_path) {
        return false;
    }

    let relative = path
        .strip_prefix(watcher_path)
        .expect("structural containment was checked above");
    if relative.components().nth(2).is_none() {
        return true;
    }

    if path.extension().is_none() {
        return true;
    }

    if is_relevant_path(path) {
        return true;
    }

    // Existing deep dotted directories are distinguishable from asset files.
    // Removed/rename paths use `should_keep_structural_event_path` instead,
    // because stat is no longer reliable after the filesystem event.
    path.is_dir()
}

pub(crate) fn should_keep_structural_event_path(path: &Path, watcher_path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(watcher_path) else {
        return false;
    };

    if relative
        .components()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
    {
        return false;
    }

    true
}
