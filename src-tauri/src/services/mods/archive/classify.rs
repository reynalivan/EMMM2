use std::fs;
use std::path::{Path, PathBuf};

/// Extensions considered 3DMigoto mod assets.
const MOD_EXTENSIONS: &[&str] = &["ini", "dds", "ib", "vb", "buf", "hlsl"];

/// Extensions considered loose/non-mod files (readme, previews, etc).
const LOOSE_EXTENSIONS: &[&str] = &[
    "txt", "md", "png", "jpg", "jpeg", "gif", "webp", "bmp", "url", "html", "pdf",
];

/// Sections that make a .ini file a valid 3DMigoto mod ini.
const VALID_INI_SECTIONS: &[&str] = &["[TextureOverride", "[ShaderOverride", "[Resource"];

/// Recursively find the shallowest folders containing a valid 3DMigoto .ini.
///
/// Once a valid .ini is found at a level, we stop recursing deeper into that branch.
/// That folder is the mod root — its subfolders are internal assets or variants.
///
/// Returns the list of mod root paths found.
pub fn find_mod_roots(folder: &Path, max_depth: usize) -> Vec<PathBuf> {
    if max_depth == 0 {
        return Vec::new();
    }

    // If this folder itself contains a valid mod ini at root level, it's a mod root.
    if has_valid_mod_ini(folder) {
        return vec![folder.to_path_buf()];
    }

    // Otherwise, recurse into subfolders
    let mut results = Vec::new();
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Skip hidden/system folders
            if name.starts_with('.') {
                continue;
            }
            results.extend(find_mod_roots(&path, max_depth - 1));
        }
    }

    results
}

/// Check if a folder's root (non-recursive) contains at least one valid 3DMigoto .ini.
///
/// A valid .ini must contain at least one `[TextureOverride*]`, `[ShaderOverride*]`,
/// or `[Resource*]` section header (AC-11.3.6).
pub fn has_valid_mod_ini(folder: &Path) -> bool {
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

        use std::io::{BufRead, BufReader};
        // Read the file line-by-line and check for valid section headers
        if let Ok(file) = fs::File::open(&path) {
            let reader = BufReader::new(file);
            for line_res in reader.lines() {
                if let Ok(line) = line_res {
                    let trimmed = line.trim();
                    for section in VALID_INI_SECTIONS {
                        if trimmed.starts_with(section) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
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
        // Don't recurse into mod roots — those are the actual mods
        if mod_roots.iter().any(|mr| *mr == path) {
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
            // Safety valve — shouldn't happen in practice
            return parent_dir.join(format!("{} ({})", name, uuid::Uuid::new_v4()));
        }
    }
}

/// Check if a file has a mod-relevant extension.
pub fn is_mod_file(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    MOD_EXTENSIONS.contains(&ext.as_str())
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
        // Loose files at root
        create_file(tmp.path(), "README.txt", "readme");

        let roots = find_mod_roots(tmp.path(), 5);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], deep);
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
        // Variant subfolders — should NOT be returned as separate roots
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
