use super::*;
use std::{fs, io::Read, path::Path};

fn read(
    context: &mut DiscoveryContext,
    path: &Path,
    source: GameExecutableSource,
) -> Option<Vec<u8>> {
    if !context.within_budget(source) {
        return None;
    }
    let result = (|| {
        let canonical = path.canonicalize()?;
        if validate_local_path(&canonical.to_string_lossy()).is_err() {
            context.issue(source, GameExecutableIssueCode::InvalidLocation);
            return Ok(None);
        }
        let file = fs::File::open(canonical)?;
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("Not a regular file"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    })();
    match result {
        Ok(Some(bytes)) if bytes.len() as u64 <= MAX_CONFIG_BYTES => Some(bytes),
        Ok(Some(_)) => {
            context.issue(source, GameExecutableIssueCode::ResourceLimit);
            None
        }
        Ok(None) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            context.issue(source, io_code(&error));
            None
        }
    }
}

pub(super) fn io_code(error: &std::io::Error) -> GameExecutableIssueCode {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        GameExecutableIssueCode::AccessDenied
    } else {
        GameExecutableIssueCode::ReadFailed
    }
}

fn json(
    context: &mut DiscoveryContext,
    path: &Path,
    source: GameExecutableSource,
) -> Option<serde_json::Value> {
    let bytes = read(context, path, source)?;
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(_) => {
            context.issue(source, GameExecutableIssueCode::MalformedConfig);
            None
        }
    }
}

pub(super) fn xxmi(context: &mut DiscoveryContext, roots: &[PathBuf]) {
    let source = GameExecutableSource::XxmiConfig;
    for input in roots {
        for root in known_xxmi_roots(input) {
            if !context.within_budget(source) {
                return;
            }
            let Some(value) = json(context, &root.join("XXMI Launcher Config.json"), source) else {
                continue;
            };
            let Some(importers) = value
                .get("Importers")
                .and_then(serde_json::Value::as_object)
            else {
                context.issue(source, GameExecutableIssueCode::MalformedConfig);
                continue;
            };
            for game_type in GameType::ALL {
                let Some(importer) = importers.get(&game_type.to_string()) else {
                    continue;
                };
                match importer.pointer("/Importer/game_folder") {
                    Some(serde_json::Value::String(path)) if !path.is_empty() => {
                        context.location(game_type, path, source, None, None)
                    }
                    None | Some(serde_json::Value::Null) => {}
                    Some(serde_json::Value::String(_)) => {}
                    Some(_) => context.issue(source, GameExecutableIssueCode::MalformedConfig),
                }
            }
            break;
        }
    }
}

fn known_xxmi_roots(input: &Path) -> Vec<PathBuf> {
    let is_executable = input
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"));
    let own_root = if is_executable {
        input.parent().unwrap_or(input)
    } else {
        input
    };
    let mut roots = vec![own_root.to_path_buf()];
    let importer = |path: &Path| {
        path.file_name().is_some_and(|name| {
            GameType::ALL
                .iter()
                .any(|game| name.eq_ignore_ascii_case(game.to_string()))
        })
    };
    let named = |path: &Path, name: &str| {
        path.file_name()
            .is_some_and(|component| component.eq_ignore_ascii_case(name))
    };
    let inferred = if is_executable
        && named(input, "XXMI Launcher.exe")
        && named(own_root, "Bin")
        && own_root
            .parent()
            .is_some_and(|parent| named(parent, "Resources"))
    {
        own_root.parent().and_then(Path::parent)
    } else if importer(own_root) {
        own_root.parent()
    } else if named(own_root, "Mods") && own_root.parent().is_some_and(importer) {
        own_root.parent().and_then(Path::parent)
    } else {
        None
    };
    if let Some(root) = inferred {
        if root != own_root {
            roots.push(root.to_path_buf());
        }
    }
    roots
}

fn walk(
    context: &mut DiscoveryContext,
    root: &Path,
    filename: &str,
    source: GameExecutableSource,
    mut visit: impl FnMut(&mut DiscoveryContext, &Path),
) {
    if !context.within_budget(source) {
        return;
    }
    if validate_local_path(&root.to_string_lossy()).is_err() {
        context.issue(source, GameExecutableIssueCode::InvalidLocation);
        return;
    }
    let root = match root.canonicalize() {
        Ok(root) if validate_local_path(&root.to_string_lossy()).is_ok() => root,
        Ok(_) => {
            context.issue(source, GameExecutableIssueCode::InvalidLocation);
            return;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            context.issue(source, io_code(&error));
            return;
        }
    };
    let mut stack = vec![(root, 0)];
    let mut visited = 0;
    while let Some((dir, depth)) = stack.pop() {
        if !context.within_budget(source) {
            break;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => {
                context.issue(source, io_code(&error));
                continue;
            }
        };
        for entry in entries {
            if !context.within_budget(source) {
                return;
            }
            visited += 1;
            if visited > MAX_PROVIDER_ENTRIES {
                context.issue(source, GameExecutableIssueCode::ResourceLimit);
                return;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    context.issue(source, io_code(&error));
                    continue;
                }
            };
            let metadata = match fs::symlink_metadata(entry.path()) {
                Ok(metadata) => metadata,
                Err(error) => {
                    context.issue(source, io_code(&error));
                    continue;
                }
            };
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if metadata.file_attributes()
                    & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
                    != 0
                {
                    continue;
                }
            }
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_file() && entry.file_name().eq_ignore_ascii_case(filename) {
                visit(context, &entry.path());
            } else if metadata.is_dir() {
                if depth < MAX_CONFIG_DEPTH {
                    stack.push((entry.path(), depth + 1));
                } else {
                    context.issue(source, GameExecutableIssueCode::ResourceLimit);
                }
            }
        }
    }
}

pub(super) fn kuro(context: &mut DiscoveryContext, root: &Path) {
    if !context.requested(GameType::Wwmi) {
        return;
    }
    let source = GameExecutableSource::KuroConfig;
    walk(
        context,
        root,
        "kr_starter_game.json",
        source,
        |context, path| {
            if let Some(value) = json(context, path, source) {
                match value.get("path").and_then(serde_json::Value::as_str) {
                    Some(path) if !path.is_empty() => {
                        context.location(GameType::Wwmi, path, source, None, None)
                    }
                    _ => context.issue(source, GameExecutableIssueCode::MalformedConfig),
                }
            }
        },
    );
}

pub(super) fn hoyoplay(context: &mut DiscoveryContext, root: &Path) {
    if ![GameType::Gimi, GameType::Srmi, GameType::Zzmi]
        .iter()
        .any(|game| context.requested(*game))
    {
        return;
    }
    let source = GameExecutableSource::HoyoplayConfig;
    // HYP gamedata.dat is a launcher-owned text container, not a stable JSON
    // schema. Decode only quoted strings; never evaluate its contents.
    walk(context, root, "gamedata.dat", source, |context, path| {
        let Some(bytes) = read(context, path, source) else {
            return;
        };
        let Ok(text) = std::str::from_utf8(&bytes) else {
            context.issue(source, GameExecutableIssueCode::MalformedConfig);
            return;
        };
        static PATH_FIELDS: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let pattern = PATH_FIELDS.get_or_init(|| {
            regex::Regex::new(
                r#""(?:installPath|persistentInstallPath)"\s*:\s*("(?:[^"\\\x00-\x1f]|\\.)*")"#,
            )
            .expect("constant HoYoPlay path-field pattern")
        });
        let mut entries = 0;
        for matched in pattern.captures_iter(text) {
            if !context.within_budget(source) {
                break;
            }
            let Ok(value) = serde_json::from_str::<String>(&matched[1]) else {
                continue;
            };
            if validate_local_path(&value).is_err() {
                continue;
            }
            entries += 1;
            if entries > MAX_PROVIDER_ENTRIES {
                context.issue(source, GameExecutableIssueCode::ResourceLimit);
                break;
            }
            for game in [GameType::Gimi, GameType::Srmi, GameType::Zzmi] {
                context.location(game, &value, source, None, None);
            }
        }
    });
}
