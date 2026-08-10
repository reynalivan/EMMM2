//! Evidence-backed 3DMigoto resource and shader conflict detection.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKind {
    ResourceHash,
    ShaderHash,
    ShaderReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCertainty {
    Definite,
    Potential,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ConflictEvidence {
    pub mod_path: String,
    pub source_path: String,
    pub section_name: String,
    pub namespace: Option<String>,
    pub condition: Option<String>,
    pub priority: Option<i64>,
    pub match_first_index: Option<u32>,
    pub shader_stage: Option<String>,
}

/// A potential collision plus the source facts needed to judge it.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct ConflictInfo {
    pub hash: String,
    pub section_name: String,
    pub mod_paths: Vec<String>,
    pub is_active: bool,
    pub kind: ConflictKind,
    pub certainty: ConflictCertainty,
    pub has_conditional_evidence: bool,
    pub evidence: Vec<ConflictEvidence>,
}

#[derive(Debug, Clone)]
struct HashEntry {
    kind: ConflictKind,
    hash: String,
    evidence: ConflictEvidence,
}

/// Detect collisions across INI overrides and legacy ShaderFixes replacements.
pub fn detect_conflicts(ini_files: &[(PathBuf, PathBuf)]) -> Vec<ConflictInfo> {
    let mod_roots: Vec<_> = ini_files
        .iter()
        .map(|(mod_root, _)| mod_root.clone())
        .collect();
    detect_conflicts_with_roots(ini_files, &mod_roots)
}

/// Variant used when a mod contains only a legacy ShaderFixes replacement.
pub fn detect_conflicts_with_roots(
    ini_files: &[(PathBuf, PathBuf)],
    mod_roots: &[PathBuf],
) -> Vec<ConflictInfo> {
    let mut groups: HashMap<(ConflictKind, String, Option<String>), Vec<HashEntry>> =
        HashMap::new();

    for (mod_root, ini_path) in ini_files {
        add_entries(&mut groups, parse_ini_hashes(ini_path, mod_root));
    }
    let mut scanned_roots = HashSet::new();
    for mod_root in mod_roots {
        if scanned_roots.insert(mod_root) {
            add_entries(&mut groups, scan_shader_replacements(mod_root));
        }
    }

    let mut conflicts: Vec<_> = groups
        .into_iter()
        .filter_map(|((_kind, _hash, _stage), entries)| build_conflict(entries))
        .collect();
    conflicts.sort_by(|left, right| {
        left.hash
            .cmp(&right.hash)
            .then_with(|| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
    });
    conflicts
}

fn add_entries(
    groups: &mut HashMap<(ConflictKind, String, Option<String>), Vec<HashEntry>>,
    entries: Vec<HashEntry>,
) {
    for entry in entries {
        let key = (
            entry.kind,
            entry.hash.clone(),
            entry.evidence.shader_stage.clone(),
        );
        groups.entry(key).or_default().push(entry);
    }
}

fn build_conflict(entries: Vec<HashEntry>) -> Option<ConflictInfo> {
    let involved: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entries.iter().any(|other| {
                entry.evidence.mod_path != other.evidence.mod_path
                    && indices_can_overlap(
                        entry.evidence.match_first_index,
                        other.evidence.match_first_index,
                    )
            })
        })
        .collect();

    let mut mod_paths: Vec<_> = involved
        .iter()
        .map(|entry| entry.evidence.mod_path.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    if mod_paths.len() < 2 {
        return None;
    }
    mod_paths.sort();

    let first = involved.first()?;
    let mut evidence: Vec<_> = involved
        .iter()
        .map(|entry| entry.evidence.clone())
        .collect();
    evidence.sort_by(|left, right| {
        left.mod_path
            .cmp(&right.mod_path)
            .then_with(|| left.source_path.cmp(&right.source_path))
            .then_with(|| left.section_name.cmp(&right.section_name))
    });

    Some(ConflictInfo {
        hash: first.hash.clone(),
        section_name: first.evidence.section_name.clone(),
        is_active: mod_paths.iter().filter(|path| path_is_active(path)).count() >= 2,
        kind: first.kind,
        certainty: if evidence.iter().any(|item| item.condition.is_some()) {
            ConflictCertainty::Potential
        } else {
            ConflictCertainty::Definite
        },
        has_conditional_evidence: evidence.iter().any(|item| item.condition.is_some()),
        mod_paths,
        evidence,
    })
}

fn indices_can_overlap(left: Option<u32>, right: Option<u32>) -> bool {
    left.is_none() || right.is_none() || left == right
}

fn path_is_active(path: &str) -> bool {
    Path::new(path).components().all(|component| {
        !crate::common::normalizer::is_disabled_folder(&component.as_os_str().to_string_lossy())
    })
}

fn parse_ini_hashes(ini_path: &Path, mod_root: &Path) -> Vec<HashEntry> {
    let bytes = match fs::read(ini_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            log::warn!("Failed to read INI {}: {error}", ini_path.display());
            return Vec::new();
        }
    };
    let (content, _, _) = crate::services::ini::document::decode_ini_bytes(&bytes);
    let namespace = content
        .lines()
        .find_map(|line| property(line, "namespace").map(str::to_string));

    let mut entries = Vec::new();
    let mut section: Option<SectionEvidence> = None;
    for line in content.lines() {
        if let Some(name) = section_name(line) {
            if let Some(previous) = section.take() {
                entries.extend(previous.finish(ini_path, mod_root, namespace.as_deref()));
            }
            section = section_kind(&name).map(|kind| SectionEvidence::new(kind, name));
            continue;
        }
        if let Some(current) = section.as_mut() {
            current.observe(line);
        }
    }
    if let Some(last) = section {
        entries.extend(last.finish(ini_path, mod_root, namespace.as_deref()));
    }
    entries
}

struct SectionEvidence {
    kind: ConflictKind,
    name: String,
    hashes: Vec<String>,
    condition: Option<String>,
    priority: Option<i64>,
    match_first_index: Option<u32>,
}

impl SectionEvidence {
    fn new(kind: ConflictKind, name: String) -> Self {
        Self {
            kind,
            name,
            hashes: Vec::new(),
            condition: None,
            priority: None,
            match_first_index: None,
        }
    }

    fn observe(&mut self, line: &str) {
        if let Some(value) = property(line, "hash").and_then(normalize_hash) {
            self.hashes.push(value);
        }
        if let Some(value) = property(line, "condition") {
            self.condition = Some(value.to_string());
        }
        self.priority = property(line, "priority")
            .and_then(|value| value.parse().ok())
            .or(self.priority);
        self.match_first_index = property(line, "match_first_index")
            .and_then(parse_u32)
            .or(self.match_first_index);
    }

    fn finish(self, ini_path: &Path, mod_root: &Path, namespace: Option<&str>) -> Vec<HashEntry> {
        self.hashes
            .into_iter()
            .map(|hash| HashEntry {
                kind: self.kind,
                hash,
                evidence: ConflictEvidence {
                    mod_path: mod_root.to_string_lossy().to_string(),
                    source_path: ini_path.to_string_lossy().to_string(),
                    section_name: self.name.clone(),
                    namespace: namespace.map(str::to_string),
                    condition: self.condition.clone(),
                    priority: self.priority,
                    match_first_index: self.match_first_index,
                    shader_stage: None,
                },
            })
            .collect()
    }
}

fn section_name(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let name = trimmed.strip_prefix('[')?.split_once(']')?.0.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn section_kind(name: &str) -> Option<ConflictKind> {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("textureoverride") {
        return Some(ConflictKind::ResourceHash);
    }
    if lower.starts_with("shaderoverride") {
        return Some(ConflictKind::ShaderHash);
    }
    None
}

fn property<'a>(line: &'a str, expected: &str) -> Option<&'a str> {
    let code = line.split(';').next()?.trim();
    let (key, value) = code.split_once('=')?;
    key.trim()
        .eq_ignore_ascii_case(expected)
        .then(|| value.trim())
        .filter(|value| !value.is_empty())
}

fn normalize_hash(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty() && value.chars().all(|character| character.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn parse_u32(value: &str) -> Option<u32> {
    value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(
            || value.parse().ok(),
            |hex| u32::from_str_radix(hex, 16).ok(),
        )
}

fn scan_shader_replacements(mod_root: &Path) -> Vec<HashEntry> {
    WalkDir::new(mod_root)
        .max_depth(8)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| replacement_entry(entry.path(), mod_root))
        .collect()
}

fn replacement_entry(path: &Path, mod_root: &Path) -> Option<HashEntry> {
    let extension = path.extension()?.to_str()?;
    if !matches!(extension.to_ascii_lowercase().as_str(), "txt" | "bin") {
        return None;
    }
    let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
    let (hash, suffix) = stem.split_once('-')?;
    let stage = suffix.strip_suffix("_replace")?;
    if hash.len() != 16
        || !hash.chars().all(|character| character.is_ascii_hexdigit())
        || !matches!(stage, "ps" | "vs" | "cs" | "gs" | "hs" | "ds")
    {
        return None;
    }

    Some(HashEntry {
        kind: ConflictKind::ShaderReplacement,
        hash: hash.to_string(),
        evidence: ConflictEvidence {
            mod_path: mod_root.to_string_lossy().to_string(),
            source_path: path.to_string_lossy().to_string(),
            section_name: path.file_name()?.to_string_lossy().to_string(),
            namespace: None,
            condition: None,
            priority: None,
            match_first_index: None,
            shader_stage: Some(stage.to_string()),
        },
    })
}

#[cfg(test)]
#[path = "tests/conflict_tests.rs"]
mod tests;
