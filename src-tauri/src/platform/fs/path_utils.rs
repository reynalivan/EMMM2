use std::path::{Component, Path};

/// Validates that the `target_path` strictly resolves _inside_ the `base_path`.
/// Rejects path traversal attempts using `..` or absolute paths aiming outside the allowed directory.
pub fn is_path_safe(base_path: &Path, target_path: &Path) -> bool {
    // If the target is absolute, it MUST start with the base_path
    if target_path.is_absolute() {
        return target_path.starts_with(base_path);
    }

    // Process relative components
    let mut depth = 0;
    for component in target_path.components() {
        match component {
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    // Attempt to escape the bounds of the base_path
                    return false;
                }
            }
            Component::Normal(_) => {
                depth += 1;
            }
            Component::CurDir => {
                // `.` does nothing to depth
            }
            // Roots or prefixes in a relative path evaluated against a base string
            // shouldn't happen, but if they do, it's unsafe.
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }

    true
}
