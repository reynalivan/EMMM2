use super::types::{
    ModAssetEntry, ModAssetManifest, ModAssetManifestCounts, ModControl, ModControlKind,
    ModFileManifestEntry, ModHealthIssue, ModHealthReport, ModHealthSeverity,
    ModHealthSupportLevel, ModViewerLaunchReceipt,
};
use crate::modules::games::domain::models::GameType;
use crate::modules::library::application::ini::document::{
    parse_ini_document, IniReadMode, MAX_PARSEABLE_INI_BYTES,
};
use crate::modules::workspace::domain::normalizer::is_disabled_folder;
use crate::shared::errors::AppError;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\[([^\]]+)\]\s*$").expect("valid section regex"));
static RESOURCE_REFERENCE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bref\s+(Resource(?:\\)?[A-Za-z0-9_.-]+)")
        .expect("valid resource reference regex")
});

#[derive(Debug)]
struct ScannedFile {
    absolute_path: PathBuf,
    relative_path: String,
    size_bytes: u64,
    blake3: String,
}

#[derive(Debug)]
struct IniFile {
    path: PathBuf,
    relative_path: String,
    inactive: bool,
}

#[derive(Debug)]
struct ResourceFileReference {
    resource_path: String,
    source_file: String,
    section: String,
    line: u64,
    inactive: bool,
}

#[derive(Debug)]
struct ResourceSymbolReference {
    resource_name: String,
    source_file: String,
    section: String,
    line: u64,
}

#[derive(Debug, Default)]
struct ReferencedAsset {
    active_sources: BTreeSet<String>,
    inactive_sources: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct ParsedIni {
    issues: Vec<ModHealthIssue>,
    declared_resources: HashSet<String>,
    resource_files: Vec<ResourceFileReference>,
    resource_symbols: Vec<ResourceSymbolReference>,
    controls: Vec<ModControl>,
}

#[derive(Debug)]
struct IniSection {
    name: String,
    fields: Vec<IniField>,
}

#[derive(Debug)]
struct IniField {
    key: String,
    value: String,
    line: u64,
}

/// Builds a read-only health report for one canonical mod directory.
///
/// The caller owns user-input validation. This use case never follows links,
/// so a file discovered beneath `mod_root` is guaranteed to be a regular file
/// found by the directory walk rather than a target outside the mod.
pub fn analyze_mod_health(
    mod_root: &Path,
    game_type: GameType,
) -> Result<ModHealthReport, AppError> {
    let mod_root = canonical_mod_root(mod_root)?;
    let files = scan_regular_files(&mod_root)?;
    let file_manifest = file_manifest_from_scanned(&files);
    let support_level = support_level(game_type);
    let ini_files = collect_ini_files(&mod_root, &files);
    let mut parsed = ParsedIni::default();

    for ini in ini_files.iter().filter(|ini| !ini.inactive) {
        merge_parsed_ini(
            &mut parsed,
            parse_ini_file(&ini.path, &ini.relative_path, false, game_type)?,
        );
    }
    for ini in ini_files.iter().filter(|ini| ini.inactive) {
        merge_parsed_ini(
            &mut parsed,
            parse_ini_file(&ini.path, &ini.relative_path, true, game_type)?,
        );
    }

    let mut manifest_issues = Vec::new();
    let manifest = build_asset_manifest(&mod_root, &files, &parsed, &mut manifest_issues)?;
    parsed.issues.append(&mut manifest_issues);
    parsed.issues.sort_by(issue_order);
    parsed.controls.sort_by(control_order);

    Ok(ModHealthReport {
        support_level,
        issues: parsed.issues,
        manifest,
        file_manifest,
        controls: parsed.controls,
    })
}

/// Creates the in-memory payload a caller can retain before launching the
/// external viewer. The receipt intentionally has no persistence layer.
pub fn create_mod_viewer_launch_receipt(
    game_id: String,
    mod_folder: String,
    mod_root: &Path,
) -> Result<ModViewerLaunchReceipt, AppError> {
    let canonical_mod_root = canonical_mod_root(mod_root)?;
    Ok(ModViewerLaunchReceipt {
        game_id,
        mod_folder,
        file_manifest: file_manifest_from_scanned(&scan_regular_files(&canonical_mod_root)?),
    })
}

fn canonical_mod_root(mod_root: &Path) -> Result<PathBuf, AppError> {
    if !mod_root.is_dir() {
        return Err(AppError::Validation(format!(
            "Mod Health target is not a directory: {}",
            mod_root.display()
        )));
    }
    mod_root.canonicalize().map_err(AppError::from)
}

fn support_level(game_type: GameType) -> ModHealthSupportLevel {
    match game_type {
        GameType::GIMI | GameType::ZZMI | GameType::WWMI => ModHealthSupportLevel::Supported,
        GameType::SRMI => ModHealthSupportLevel::Experimental,
        GameType::EFMI => ModHealthSupportLevel::Basic,
    }
}

fn supports_semantic_controls(game_type: GameType) -> bool {
    !matches!(game_type, GameType::EFMI)
}

fn scan_regular_files(mod_root: &Path) -> Result<Vec<ScannedFile>, AppError> {
    let mut files = Vec::new();
    scan_directory(mod_root, mod_root, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(files)
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    output: &mut Vec<ScannedFile>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }

        let path = entry.path();
        if file_type.is_dir() {
            scan_directory(root, &path, output)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }

        let metadata = entry.metadata()?;
        output.push(ScannedFile {
            relative_path: relative_path(root, &path)?,
            blake3: hash_file(&path)?,
            size_bytes: metadata.len(),
            absolute_path: path,
        });
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn file_manifest_from_scanned(files: &[ScannedFile]) -> Vec<ModFileManifestEntry> {
    files
        .iter()
        .map(|file| ModFileManifestEntry {
            relative_path: file.relative_path.clone(),
            size_bytes: file.size_bytes,
            blake3: file.blake3.clone(),
        })
        .collect()
}

fn collect_ini_files(mod_root: &Path, files: &[ScannedFile]) -> Vec<IniFile> {
    files
        .iter()
        .filter(|file| is_ini_file(&file.absolute_path))
        .map(|file| IniFile {
            path: file.absolute_path.clone(),
            relative_path: file.relative_path.clone(),
            inactive: is_inactive_ini(mod_root, &file.absolute_path),
        })
        .collect()
}

fn is_ini_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("ini"))
        && !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("desktop.ini"))
}

fn is_inactive_ini(root: &Path, path: &Path) -> bool {
    let root_disabled = root
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_disabled_folder);
    let filename_disabled = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_disabled_folder);
    root_disabled
        || filename_disabled
        || path
            .strip_prefix(root)
            .ok()
            .into_iter()
            .flat_map(Path::components)
            .filter_map(|component| match component {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .any(is_disabled_folder)
}

fn parse_ini_file(
    path: &Path,
    relative_file_path: &str,
    inactive: bool,
    game_type: GameType,
) -> Result<ParsedIni, AppError> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::Io(format!(
            "Could not read INI metadata for {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > MAX_PARSEABLE_INI_BYTES {
        let mut parsed = ParsedIni::default();
        parsed.issues.push(issue(
            ModHealthSeverity::Warning,
            "ini_too_large",
            "INI exceeds the safe analysis size limit and was not parsed",
            relative_file_path,
            None,
            None,
        ));
        return Ok(parsed);
    }
    let bytes = fs::read(path).map_err(|error| {
        AppError::Io(format!(
            "Could not read INI file {}: {error}",
            path.display()
        ))
    })?;
    let document = parse_ini_document(path, &bytes);
    let mut parsed = ParsedIni::default();
    if document.mode == IniReadMode::RawFallback {
        parsed.issues.push(issue(
            ModHealthSeverity::Warning,
            "ini_raw_fallback",
            "INI could not be safely parsed; showing raw-file diagnostics only",
            relative_file_path,
            None,
            None,
        ));
    }

    let sections = collect_sections(&document.raw_lines, relative_file_path, &mut parsed.issues);
    for section in &sections {
        inspect_condition_structure(section, relative_file_path, &mut parsed.issues);
        if section.name.to_ascii_lowercase().starts_with("resource") {
            parsed
                .declared_resources
                .insert(normalize_resource_name(&section.name));
            collect_resource_file_references(section, relative_file_path, inactive, &mut parsed);
        }
        collect_resource_symbol_references(section, relative_file_path, &mut parsed);
        if !inactive && supports_semantic_controls(game_type) {
            collect_controls(section, relative_file_path, &mut parsed.controls);
        }
    }

    Ok(parsed)
}

fn merge_parsed_ini(target: &mut ParsedIni, mut source: ParsedIni) {
    target.issues.append(&mut source.issues);
    target.declared_resources.extend(source.declared_resources);
    target.resource_files.append(&mut source.resource_files);
    target.resource_symbols.append(&mut source.resource_symbols);
    target.controls.append(&mut source.controls);
}

fn collect_sections(
    raw_lines: &[String],
    relative_file_path: &str,
    issues: &mut Vec<ModHealthIssue>,
) -> Vec<IniSection> {
    let mut sections = Vec::new();
    let mut current: Option<IniSection> = None;
    for (index, raw_line) in raw_lines.iter().enumerate() {
        let line_number = (index + 1) as u64;
        let line = without_comment(raw_line);
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(captures) = SECTION_RE.captures(trimmed) {
                if let Some(previous) = current.take() {
                    sections.push(previous);
                }
                current = Some(IniSection {
                    name: captures[1].trim().to_string(),
                    fields: Vec::new(),
                });
                continue;
            }
            issues.push(issue(
                ModHealthSeverity::Error,
                "section_malformed",
                "Section header is malformed",
                relative_file_path,
                None,
                Some(line_number),
            ));
        }
        let Some(section) = current.as_mut() else {
            continue;
        };
        if let Some((key, value)) = condition_directive(trimmed) {
            section.fields.push(IniField {
                key: key.to_string(),
                value: value.to_string(),
                line: line_number,
            });
            continue;
        }
        if let Some((key, value)) = assignment(trimmed) {
            section.fields.push(IniField {
                key: key.to_ascii_lowercase(),
                value: value.to_string(),
                line: line_number,
            });
            continue;
        }
        if let Some(value) = resource_reference_directive(trimmed) {
            section.fields.push(IniField {
                key: "ref".to_string(),
                value: value.to_string(),
                line: line_number,
            });
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}

fn inspect_condition_structure(
    section: &IniSection,
    relative_file_path: &str,
    issues: &mut Vec<ModHealthIssue>,
) {
    let mut condition_stack: Vec<(u64, bool)> = Vec::new();
    for field in &section.fields {
        let directive = field.key.as_str();
        match directive {
            "if" => condition_stack.push((field.line, false)),
            "elif" => {
                if condition_stack.is_empty()
                    || condition_stack
                        .last()
                        .is_some_and(|(_, seen_else)| *seen_else)
                {
                    issues.push(issue(
                        ModHealthSeverity::Error,
                        "condition_order",
                        "elif must follow an open if before else",
                        relative_file_path,
                        Some(&section.name),
                        Some(field.line),
                    ));
                }
            }
            "else" => {
                let Some((_, seen_else)) = condition_stack.last_mut() else {
                    issues.push(issue(
                        ModHealthSeverity::Error,
                        "condition_order",
                        "else must follow an open if",
                        relative_file_path,
                        Some(&section.name),
                        Some(field.line),
                    ));
                    continue;
                };
                if *seen_else {
                    issues.push(issue(
                        ModHealthSeverity::Error,
                        "condition_order",
                        "if blocks may contain only one else",
                        relative_file_path,
                        Some(&section.name),
                        Some(field.line),
                    ));
                }
                *seen_else = true;
            }
            "endif" => {
                if condition_stack.pop().is_none() {
                    issues.push(issue(
                        ModHealthSeverity::Error,
                        "condition_order",
                        "endif has no matching if",
                        relative_file_path,
                        Some(&section.name),
                        Some(field.line),
                    ));
                }
            }
            _ => {}
        }
    }
    for (line, _) in condition_stack {
        issues.push(issue(
            ModHealthSeverity::Error,
            "condition_unbalanced",
            "if has no matching endif",
            relative_file_path,
            Some(&section.name),
            Some(line),
        ));
    }
}

fn collect_resource_file_references(
    section: &IniSection,
    relative_file_path: &str,
    inactive: bool,
    parsed: &mut ParsedIni,
) {
    for field in section
        .fields
        .iter()
        .filter(|field| field.key == "filename")
    {
        let resource_path = strip_quotes(&field.value).to_string();
        if resource_path.is_empty() {
            parsed.issues.push(issue(
                ModHealthSeverity::Warning,
                "resource_filename_empty",
                "Resource filename is empty",
                relative_file_path,
                Some(&section.name),
                Some(field.line),
            ));
            continue;
        }
        parsed.resource_files.push(ResourceFileReference {
            resource_path,
            source_file: relative_file_path.to_string(),
            section: section.name.clone(),
            line: field.line,
            inactive,
        });
    }
}

fn collect_resource_symbol_references(
    section: &IniSection,
    relative_file_path: &str,
    parsed: &mut ParsedIni,
) {
    for field in &section.fields {
        let source = format!("{} {}", field.key, field.value);
        for captures in RESOURCE_REFERENCE_RE.captures_iter(&source) {
            parsed.resource_symbols.push(ResourceSymbolReference {
                resource_name: normalize_resource_name(&captures[1]),
                source_file: relative_file_path.to_string(),
                section: section.name.clone(),
                line: field.line,
            });
        }
    }
}

fn collect_controls(
    section: &IniSection,
    relative_file_path: &str,
    controls: &mut Vec<ModControl>,
) {
    let section_lower = section.name.to_ascii_lowercase();
    if section_lower == "present"
        && field_value(section, "run").is_some_and(|value| !value.trim().is_empty())
    {
        controls.push(ModControl {
            kind: ModControlKind::Present,
            section: section.name.clone(),
            file_path: relative_file_path.to_string(),
            key: None,
            back: None,
            variable: None,
            values: Vec::new(),
            default_value: None,
        });
        return;
    }
    if !section_lower.starts_with("key") || !section_has_cycle_type(section) {
        return;
    }

    let key = field_value(section, "key");
    let back = field_value(section, "back");
    let Some(variable_field) = section
        .fields
        .iter()
        .find(|field| field.key.starts_with('$') && !split_cycle_values(&field.value).is_empty())
    else {
        return;
    };
    let values = split_cycle_values(&variable_field.value);
    if values.len() < 2 {
        return;
    }
    let kind = if section_lower.contains("menu") {
        ModControlKind::MenuToggle
    } else {
        ModControlKind::KeyToggle
    };
    let default_value = field_value(section, "default")
        .filter(|value| values.iter().any(|candidate| candidate == value));
    let control = ModControl {
        kind,
        section: section.name.clone(),
        file_path: relative_file_path.to_string(),
        key,
        back,
        variable: Some(variable_field.key.clone()),
        values: values.clone(),
        default_value: default_value.clone(),
    };
    let shape_variable = control
        .variable
        .as_deref()
        .is_some_and(|variable| variable.to_ascii_lowercase().contains("shape"));
    controls.push(control);
    if shape_variable {
        controls.push(ModControl {
            kind: ModControlKind::ShapeVariable,
            section: section.name.clone(),
            file_path: relative_file_path.to_string(),
            key: None,
            back: None,
            variable: Some(variable_field.key.clone()),
            values,
            default_value,
        });
    }
}

fn section_has_cycle_type(section: &IniSection) -> bool {
    field_value(section, "type").is_some_and(|value| value.eq_ignore_ascii_case("cycle"))
}

fn field_value(section: &IniSection, key: &str) -> Option<String> {
    section
        .fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
}

fn split_cycle_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn build_asset_manifest(
    mod_root: &Path,
    files: &[ScannedFile],
    parsed: &ParsedIni,
    issues: &mut Vec<ModHealthIssue>,
) -> Result<ModAssetManifest, AppError> {
    let mut referenced_assets: BTreeMap<String, ReferencedAsset> = BTreeMap::new();
    let mut external_references: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let file_index = files
        .iter()
        .map(|file| (path_key(&file.relative_path), file))
        .collect::<HashMap<_, _>>();

    for reference in &parsed.resource_files {
        if resource_path_is_unsafe(&reference.resource_path) {
            issues.push(issue(
                ModHealthSeverity::Error,
                "resource_path_unsafe",
                "Resource filename must stay inside the mod folder",
                &reference.source_file,
                Some(&reference.section),
                Some(reference.line),
            ));
            external_references
                .entry(reference.resource_path.clone())
                .or_default()
                .insert(reference.source_file.clone());
            continue;
        }

        let candidate = mod_root.join(resource_path_to_path(&reference.resource_path));
        let Ok(metadata) = fs::symlink_metadata(&candidate) else {
            issues.push(issue(
                ModHealthSeverity::Error,
                "resource_missing",
                "Referenced resource file is missing",
                &reference.source_file,
                Some(&reference.section),
                Some(reference.line),
            ));
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            issues.push(issue(
                ModHealthSeverity::Error,
                "resource_path_unsafe",
                "Referenced resource must be a regular file inside the mod folder",
                &reference.source_file,
                Some(&reference.section),
                Some(reference.line),
            ));
            continue;
        }
        let canonical_candidate = candidate.canonicalize()?;
        if !canonical_candidate.starts_with(mod_root) {
            issues.push(issue(
                ModHealthSeverity::Error,
                "resource_path_unsafe",
                "Referenced resource resolves outside the mod folder",
                &reference.source_file,
                Some(&reference.section),
                Some(reference.line),
            ));
            continue;
        }
        let relative = relative_path(mod_root, &canonical_candidate)?;
        let Some(_) = file_index.get(&path_key(&relative)) else {
            issues.push(issue(
                ModHealthSeverity::Error,
                "resource_missing",
                "Referenced resource file is unavailable for analysis",
                &reference.source_file,
                Some(&reference.section),
                Some(reference.line),
            ));
            continue;
        };
        let asset = referenced_assets.entry(path_key(&relative)).or_default();
        if reference.inactive {
            asset.inactive_sources.insert(reference.source_file.clone());
        } else {
            asset.active_sources.insert(reference.source_file.clone());
        }
    }

    for reference in &parsed.resource_symbols {
        if parsed.declared_resources.contains(&reference.resource_name) {
            continue;
        }
        if reference.resource_name.contains('\\') {
            external_references
                .entry(reference.resource_name.clone())
                .or_default()
                .insert(reference.source_file.clone());
            continue;
        }
        issues.push(issue(
            ModHealthSeverity::Error,
            "resource_undeclared",
            "Referenced resource has no matching [Resource...] declaration",
            &reference.source_file,
            Some(&reference.section),
            Some(reference.line),
        ));
    }

    let mut manifest = ModAssetManifest::default();
    for file in files
        .iter()
        .filter(|file| !excluded_from_assets(&file.relative_path))
    {
        let Some(reference) = referenced_assets.get(&path_key(&file.relative_path)) else {
            manifest.orphan.push(asset_entry(file, BTreeSet::new()));
            continue;
        };
        let mut sources = reference.active_sources.clone();
        sources.extend(reference.inactive_sources.iter().cloned());
        if reference.active_sources.is_empty() {
            manifest.inactive_only.push(asset_entry(file, sources));
        } else {
            manifest.referenced.push(asset_entry(file, sources));
        }
    }
    manifest.external_reference = external_references
        .into_iter()
        .map(|(resource_path, source_files)| ModAssetEntry {
            relative_path: resource_path,
            size_bytes: 0,
            source_files: source_files.into_iter().collect(),
        })
        .collect();
    manifest.counts = ModAssetManifestCounts {
        referenced: manifest.referenced.len() as u64,
        inactive_only: manifest.inactive_only.len() as u64,
        orphan: manifest.orphan.len() as u64,
        external_reference: manifest.external_reference.len() as u64,
    };
    Ok(manifest)
}

fn resource_path_is_unsafe(raw_path: &str) -> bool {
    let path = raw_path.trim();
    path.starts_with("\\\\")
        || path.starts_with('/')
        || path.starts_with('\\')
        || path.get(1..2).is_some_and(|marker| marker == ":")
        || path
            .replace('\\', "/")
            .split('/')
            .any(|component| component == "..")
}

fn resource_path_to_path(raw_path: &str) -> PathBuf {
    raw_path
        .replace('\\', "/")
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .collect()
}

fn excluded_from_assets(relative_path: &str) -> bool {
    let file_name = Path::new(relative_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = Path::new(relative_path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    extension == "ini"
        || extension == "bak"
        || file_name == "info.json"
        || file_name == ".mod_viewer.json"
        || file_name.starts_with("preview.")
}

fn asset_entry(file: &ScannedFile, source_files: BTreeSet<String>) -> ModAssetEntry {
    ModAssetEntry {
        relative_path: file.relative_path.clone(),
        size_bytes: file.size_bytes,
        source_files: source_files.into_iter().collect(),
    }
}

fn normalize_resource_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn without_comment(line: &str) -> &str {
    let semicolon = line.find(';');
    let hash = line.find('#');
    match (semicolon, hash) {
        (Some(left), Some(right)) => &line[..left.min(right)],
        (Some(index), None) | (None, Some(index)) => &line[..index],
        (None, None) => line,
    }
}

fn assignment(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    let key = key.trim();
    let value = value.trim();
    (!key.is_empty()).then_some((key, value))
}

fn condition_directive(line: &str) -> Option<(&str, &str)> {
    let (keyword, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let directive = match keyword.to_ascii_lowercase().as_str() {
        "if" => "if",
        "elif" => "elif",
        "else" => "else",
        "endif" => "endif",
        _ => return None,
    };
    Some((directive, rest.trim()))
}

fn resource_reference_directive(line: &str) -> Option<&str> {
    let (keyword, value) = line.split_once(char::is_whitespace)?;
    keyword.eq_ignore_ascii_case("ref").then_some(value.trim())
}

fn strip_quotes(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|trimmed| trimmed.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|trimmed| trimmed.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim()
}

fn path_key(value: &str) -> String {
    value.replace('\\', "/").to_ascii_lowercase()
}

fn relative_path(root: &Path, path: &Path) -> Result<String, AppError> {
    path.strip_prefix(root)
        .map_err(|_| {
            AppError::Security(format!(
                "Path escaped mod folder during analysis: {}",
                path.display()
            ))
        })
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn issue(
    severity: ModHealthSeverity,
    code: &str,
    message: &str,
    file_path: &str,
    section: Option<&str>,
    line: Option<u64>,
) -> ModHealthIssue {
    ModHealthIssue {
        severity,
        code: code.to_string(),
        message: message.to_string(),
        file_path: Some(file_path.to_string()),
        section: section.map(str::to_string),
        line,
    }
}

fn issue_order(left: &ModHealthIssue, right: &ModHealthIssue) -> std::cmp::Ordering {
    left.file_path
        .cmp(&right.file_path)
        .then_with(|| left.line.cmp(&right.line))
        .then_with(|| left.code.cmp(&right.code))
}

fn control_order(left: &ModControl, right: &ModControl) -> std::cmp::Ordering {
    left.file_path
        .cmp(&right.file_path)
        .then_with(|| left.section.cmp(&right.section))
        .then_with(|| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
}
