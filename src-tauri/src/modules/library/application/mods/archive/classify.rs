use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MOD_ROOT_MAX_DEPTH: usize = 12;
pub const MOD_ROOT_MAX_ENTRIES: usize = 10_000;

/// Extensions considered loose/non-mod files (readme, previews, etc).
const LOOSE_EXTENSIONS: &[&str] = &[
    "txt", "md", "png", "jpg", "jpeg", "gif", "webp", "bmp", "url", "html", "pdf",
];

/// Section families that give an INI file executable 3DMigoto behavior.
const VALID_INI_SECTION_PREFIXES: &[&str] = &[
    "textureoverride",
    "shaderoverride",
    "resource",
    "key",
    "commandlist",
    "shaderregex",
];

/// Recursively find the shallowest folders containing a valid 3DMigoto .ini.
///
/// Once a valid .ini is found at a level, we stop recursing deeper into that branch.
/// That folder is the mod root — its subfolders are internal assets or variants.
///
/// Returns the list of mod root paths found.
pub fn find_mod_roots(folder: &Path, max_depth: usize) -> Vec<PathBuf> {
    find_mod_roots_with_limits(folder, max_depth, MOD_ROOT_MAX_ENTRIES, None)
        .map(|search| search.roots)
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModRootSearch {
    pub roots: Vec<PathBuf>,
    pub unreadable_ini_files: usize,
    pub depth_limit_reached: bool,
}

/// Finds shallowest runnable mod roots without following links. This is used by
/// import staging, where a traversal limit must fail explicitly instead of
/// silently turning a deeply wrapped mod into an unknown archive.
pub fn find_mod_roots_with_limits(
    folder: &Path,
    max_depth: usize,
    max_entries: usize,
    cancel_token: Option<&AtomicBool>,
) -> Result<ModRootSearch, crate::shared::errors::AppError> {
    let mut search = ModRootSearch {
        roots: Vec::new(),
        unreadable_ini_files: 0,
        depth_limit_reached: false,
    };
    let mut visited_entries = 0_usize;
    visit_mod_roots(
        folder,
        0,
        max_depth,
        max_entries,
        cancel_token,
        &mut visited_entries,
        &mut search,
    )?;
    Ok(search)
}

#[allow(clippy::too_many_arguments)]
fn visit_mod_roots(
    folder: &Path,
    depth: usize,
    max_depth: usize,
    max_entries: usize,
    cancel_token: Option<&AtomicBool>,
    visited_entries: &mut usize,
    search: &mut ModRootSearch,
) -> Result<(), crate::shared::errors::AppError> {
    if cancel_token.is_some_and(|token| token.load(Ordering::SeqCst)) {
        return Err(crate::shared::errors::AppError::Cancelled);
    }
    if depth > max_depth {
        return Ok(());
    }
    if has_valid_mod_ini_with_diagnostics(folder, &mut search.unreadable_ini_files) {
        search.roots.push(folder.to_path_buf());
        return Ok(());
    }
    if depth == max_depth {
        for entry in fs::read_dir(folder)? {
            if cancel_token.is_some_and(|token| token.load(Ordering::SeqCst)) {
                return Err(crate::shared::errors::AppError::Cancelled);
            }
            *visited_entries = visited_entries.checked_add(1).ok_or_else(|| {
                crate::shared::errors::AppError::Validation(
                    "Mod root search entry count overflow".to_string(),
                )
            })?;
            if *visited_entries > max_entries {
                return Err(crate::shared::errors::AppError::Validation(format!(
                    "Mod root search exceeded the {max_entries} entry limit"
                )));
            }
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_dir()
                && !file_type.is_symlink()
                && !entry.file_name().to_string_lossy().starts_with('.')
            {
                search.depth_limit_reached = true;
                break;
            }
        }
        return Ok(());
    }

    for entry in fs::read_dir(folder)? {
        if cancel_token.is_some_and(|token| token.load(Ordering::SeqCst)) {
            return Err(crate::shared::errors::AppError::Cancelled);
        }
        *visited_entries = visited_entries.checked_add(1).ok_or_else(|| {
            crate::shared::errors::AppError::Validation(
                "Mod root search entry count overflow".to_string(),
            )
        })?;
        if *visited_entries > max_entries {
            return Err(crate::shared::errors::AppError::Validation(format!(
                "Mod root search exceeded the {max_entries} entry limit"
            )));
        }
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_dir() || file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        visit_mod_roots(
            &entry.path(),
            depth + 1,
            max_depth,
            max_entries,
            cancel_token,
            visited_entries,
            search,
        )?;
    }
    Ok(())
}

/// Check if a folder's root (non-recursive) contains at least one valid 3DMigoto .ini.
///
/// A valid .ini must contain an override/resource, input, command-list,
/// shader-regex, or include section.
pub fn has_valid_mod_ini(folder: &Path) -> bool {
    has_valid_mod_ini_with_diagnostics(folder, &mut 0)
}

fn has_valid_mod_ini_with_diagnostics(folder: &Path, unreadable_ini_files: &mut usize) -> bool {
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return false,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if ext != "ini" {
            continue;
        }

        let Ok(bytes) = fs::read(&path) else {
            *unreadable_ini_files = unreadable_ini_files.saturating_add(1);
            continue;
        };
        let (content, _, clean) =
            crate::modules::library::application::ini::document::decode_ini_bytes(&bytes);
        if !clean {
            *unreadable_ini_files = unreadable_ini_files.saturating_add(1);
        }
        if content.lines().any(is_runtime_section) {
            return true;
        }
    }

    false
}

fn is_runtime_section(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(section) = trimmed
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']').map(|(name, _)| name.trim()))
    else {
        return false;
    };

    let section = section.to_ascii_lowercase();
    section == "include"
        || VALID_INI_SECTION_PREFIXES
            .iter()
            .any(|prefix| section.starts_with(prefix))
}

/// Collect loose non-mod files from a folder (non-recursive).
///
/// Returns paths to files that are readme, images, etc (not .ini/.dds/.ib/.vb/.buf/.hlsl).
pub fn collect_loose_files(folder: &Path) -> Vec<PathBuf> {
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter(|e| {
            let ext = e
                .path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x.to_lowercase())
                .unwrap_or_default();
            LOOSE_EXTENSIONS.contains(&ext.as_str())
        })
        .map(|e| e.path())
        .collect()
}

/// Collect loose non-mod files recursively from all layers between `root` and the mod roots.
///
/// Walks from `root` downward, collecting loose files at each level, but stops
/// at directories that are in `mod_roots` (doesn't collect from inside mods).
pub fn collect_loose_files_recursive(root: &Path, mod_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut result = collect_loose_files(root);

    let entries = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return result,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if mod_roots.contains(&path) {
            continue;
        }
        result.extend(collect_loose_files_recursive(&path, mod_roots));
    }

    result
}

/// Generate a unique destination path with counter suffix if needed.
///
/// If `mods_dir/name` already exists, tries `name (2)`, `name (3)`, etc.
pub fn resolve_unique_dest(parent_dir: &Path, name: &str) -> PathBuf {
    let dest = parent_dir.join(name);
    if !dest.exists() {
        return dest;
    }

    let mut counter = 2u32;
    loop {
        let new_name = format!("{} ({})", name, counter);
        let check = parent_dir.join(&new_name);
        if !check.exists() {
            return check;
        }
        counter += 1;
        if counter > 999 {
            return parent_dir.join(format!("{} ({})", name, uuid::Uuid::new_v4()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    const VALID_INI: &str = "[TextureOverride_Body]\nhash = abc123\n";
    const INVALID_INI: &str = "[Constants]\nx = 1\n";

    #[test]
    fn find_mod_roots_flat_mod() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "merged.ini", VALID_INI);
        create_file(tmp.path(), "body.dds", "");

        let roots = find_mod_roots(tmp.path(), 5);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], tmp.path());
    }

    #[test]
    fn find_mod_roots_single_wrapper() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("ModName");
        fs::create_dir_all(&mod_dir).unwrap();
        create_file(&mod_dir, "merged.ini", VALID_INI);
        create_file(&mod_dir, "tex.dds", "");

        let roots = find_mod_roots(tmp.path(), 5);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], mod_dir);
    }

    #[test]
    fn find_mod_roots_deeply_nested() {
        let tmp = TempDir::new().unwrap();
        let deep = tmp.path().join("Author").join("Game").join("Character");
        fs::create_dir_all(&deep).unwrap();
        create_file(&deep, "merged.ini", VALID_INI);
        create_file(tmp.path(), "README.txt", "readme");

        let roots = find_mod_roots(tmp.path(), 5);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], deep);
    }

    #[test]
    fn root_search_reaches_the_configured_depth_and_reports_entry_limits() {
        let tmp = TempDir::new().unwrap();
        let deep = (0..12).fold(tmp.path().to_path_buf(), |path, index| {
            path.join(format!("wrapper-{index}"))
        });
        fs::create_dir_all(&deep).unwrap();
        create_file(&deep, "merged.ini", VALID_INI);

        let result =
            find_mod_roots_with_limits(tmp.path(), MOD_ROOT_MAX_DEPTH, MOD_ROOT_MAX_ENTRIES, None)
                .unwrap();
        assert_eq!(result.roots, vec![deep]);
        assert!(!result.depth_limit_reached);

        let error =
            find_mod_roots_with_limits(tmp.path(), MOD_ROOT_MAX_DEPTH, 1, None).unwrap_err();
        assert!(error.to_string().contains("entry limit"));
    }

    #[test]
    fn root_search_reports_when_a_root_may_be_below_the_depth_limit() {
        let tmp = TempDir::new().unwrap();
        let too_deep = (0..13).fold(tmp.path().to_path_buf(), |path, index| {
            let next = path.join(format!("wrapper-{index}"));
            fs::create_dir_all(&next).unwrap();
            next
        });
        create_file(&too_deep, "merged.ini", VALID_INI);

        let result = find_mod_roots_with_limits(tmp.path(), MOD_ROOT_MAX_DEPTH, 100, None).unwrap();
        assert!(result.roots.is_empty());
        assert!(result.depth_limit_reached);
    }

    #[test]
    fn root_search_reads_utf16_ini_sources() {
        let tmp = TempDir::new().unwrap();
        let mut utf16 = vec![0xff, 0xfe];
        utf16.extend(
            "[TextureOverrideBody]\r\nhash = deadbeef\r\n"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>(),
        );
        fs::write(tmp.path().join("merged.ini"), utf16).unwrap();

        let result = find_mod_roots_with_limits(tmp.path(), 1, 10, None).unwrap();
        assert_eq!(result.roots, vec![tmp.path().to_path_buf()]);
        assert_eq!(result.unreadable_ini_files, 0);
    }

    #[test]
    fn find_mod_roots_multi_mod_pack() {
        let tmp = TempDir::new().unwrap();
        let mod_a = tmp.path().join("ModA");
        let mod_b = tmp.path().join("ModB");
        fs::create_dir_all(&mod_a).unwrap();
        fs::create_dir_all(&mod_b).unwrap();
        create_file(&mod_a, "merged.ini", VALID_INI);
        create_file(&mod_b, "merged.ini", VALID_INI);

        let mut roots = find_mod_roots(tmp.path(), 5);
        roots.sort();
        assert_eq!(roots.len(), 2);
    }

    #[test]
    fn find_mod_roots_variant_mod_stops_at_root_ini() {
        let tmp = TempDir::new().unwrap();
        let mod_dir = tmp.path().join("ModName");
        fs::create_dir_all(&mod_dir).unwrap();
        create_file(&mod_dir, "merged.ini", VALID_INI);
        let var_a = mod_dir.join("VariantA");
        let var_b = mod_dir.join("VariantB");
        fs::create_dir_all(&var_a).unwrap();
        fs::create_dir_all(&var_b).unwrap();
        create_file(&var_a, "tex.dds", "");
        create_file(&var_b, "tex.dds", "");

        let roots = find_mod_roots(tmp.path(), 5);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], mod_dir);
    }

    #[test]
    fn find_mod_roots_invalid_archive() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "image.png", "");
        create_file(tmp.path(), "readme.txt", "hello");

        let roots = find_mod_roots(tmp.path(), 5);
        assert!(roots.is_empty());
    }

    #[test]
    fn find_mod_roots_invalid_ini_sections() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "config.ini", INVALID_INI);

        let roots = find_mod_roots(tmp.path(), 5);
        assert!(roots.is_empty());
    }

    #[test]
    fn find_mod_roots_accepts_non_override_runtime_sections() {
        for (index, section) in [
            "[Include]",
            "[KeyToggle]",
            "[CommandListApply]",
            "[ShaderRegexCharacter]",
        ]
        .iter()
        .enumerate()
        {
            let tmp = TempDir::new().unwrap();
            create_file(tmp.path(), &format!("runtime-{index}.ini"), section);

            assert_eq!(
                find_mod_roots(tmp.path(), 1),
                vec![tmp.path().to_path_buf()]
            );
        }
    }

    #[test]
    fn runtime_section_detection_is_case_insensitive_and_requires_a_header() {
        assert!(is_runtime_section("  [cOmMaNdLiStApply] ; comment"));
        assert!(!is_runtime_section("; [TextureOverrideBody]"));
        assert!(!is_runtime_section("[Constants]"));
        assert!(!is_runtime_section("[KeyWithoutClosingBracket"));
    }

    #[test]
    fn collect_loose_files_works() {
        let tmp = TempDir::new().unwrap();
        create_file(tmp.path(), "README.txt", "hello");
        create_file(tmp.path(), "preview.png", "");
        create_file(tmp.path(), "merged.ini", VALID_INI); // Not loose
        create_file(tmp.path(), "texture.dds", ""); // Not loose

        let loose = collect_loose_files(tmp.path());
        assert_eq!(loose.len(), 2);
    }

    #[test]
    fn resolve_unique_dest_no_conflict() {
        let tmp = TempDir::new().unwrap();
        let dest = resolve_unique_dest(tmp.path(), "MyMod");
        assert_eq!(dest, tmp.path().join("MyMod"));
    }

    #[test]
    fn resolve_unique_dest_with_conflict() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("MyMod")).unwrap();
        let dest = resolve_unique_dest(tmp.path(), "MyMod");
        assert_eq!(dest, tmp.path().join("MyMod (2)"));
    }
}
