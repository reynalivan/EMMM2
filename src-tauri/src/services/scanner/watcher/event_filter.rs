use std::path::Path;
use std::time::Duration;

pub const RENAME_PAIR_TIMEOUT: Duration = Duration::from_millis(100);

const RELEVANT_EXTENSIONS: &[&str] = &["ini", "json", "png", "jpg", "jpeg", "webp"];

pub(crate) fn is_relevant_path(path: &Path) -> bool {
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
    let Ok(relative) = path.strip_prefix(watcher_path) else {
        return false;
    };

    let components = relative.components().collect::<Vec<_>>();
    if components
        .iter()
        .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
    {
        return false;
    }

    if components.len() <= 2 {
        return true;
    }

    if path.extension().is_none() {
        return true;
    }

    is_relevant_path(path)
}
