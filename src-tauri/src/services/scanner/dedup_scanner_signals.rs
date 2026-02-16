use crate::services::scanner::walker::ModCandidate;
use crate::types::dup_scan::DupScanSignal;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const KEY_EXTS: &[&str] = &["ini", "dds", "buf", "ib", "vb"];
const TEXTURE_EXTS: &[&str] = &["dds"];
const PARTIAL_HASH_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct ModSnapshot {
    pub candidate: ModCandidate,
    pub files: Vec<FileEntry>,
    pub total_size_bytes: u64,
    pub ini_headers: BTreeSet<String>,
    pub keybindings: BTreeSet<String>,
    pub extensions: HashMap<String, u64>,
}

#[derive(Debug, Clone)]
pub(crate) struct FileEntry {
    pub rel_path: String,
    pub abs_path: PathBuf,
    pub size_bytes: u64,
    pub extension: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HashProfile {
    pub key_file_hashes: BTreeMap<String, String>,
    pub texture_samples: BTreeMap<String, String>,
}

pub(crate) fn collect_snapshot(candidate: &ModCandidate) -> Result<ModSnapshot, String> {
    let mut files = Vec::new();
    let mut total_size = 0_u64;
    let mut ini_headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();
    let mut extensions: HashMap<String, u64> = HashMap::new();

    for entry in WalkDir::new(&candidate.path)
        .follow_links(false)
        .into_iter()
        .filter_map(|item| item.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let rel = path
            .strip_prefix(&candidate.path)
            .map(|value| value.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|value| value.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        total_size = total_size.saturating_add(size);
        *extensions.entry(extension.clone()).or_insert(0) += 1;
        if extension == "ini" {
            let (headers, bindings) = read_ini_signals(&path);
            ini_headers.extend(headers);
            keybindings.extend(bindings);
        }

        files.push(FileEntry {
            rel_path: rel,
            abs_path: path,
            size_bytes: size,
            extension,
        });
    }

    Ok(ModSnapshot {
        candidate: candidate.clone(),
        files,
        total_size_bytes: total_size,
        ini_headers,
        keybindings,
        extensions,
    })
}

pub(crate) fn hash_snapshot(snapshot: &ModSnapshot) -> HashProfile {
    let mut profile = HashProfile::default();
    for file in &snapshot.files {
        if !KEY_EXTS.contains(&file.extension.as_str()) {
            continue;
        }
        let hash = if TEXTURE_EXTS.contains(&file.extension.as_str())
            && file.size_bytes > PARTIAL_HASH_THRESHOLD_BYTES
        {
            partial_blake3_hash(&file.abs_path)
        } else {
            full_blake3_hash(&file.abs_path)
        };
        if let Ok(value) = hash {
            profile
                .key_file_hashes
                .insert(file.rel_path.clone(), value.clone());
            if TEXTURE_EXTS.contains(&file.extension.as_str()) {
                profile.texture_samples.insert(file.rel_path.clone(), value);
            }
        }
    }
    profile
}

pub(crate) fn aggregate_signals(
    left: &ModSnapshot,
    right: &ModSnapshot,
    left_hash: &HashProfile,
    right_hash: &HashProfile,
) -> (u8, Vec<DupScanSignal>, String) {
    let (name_score, structure_score) = phase2_name_and_structure(left, right);
    let structural_name = ((name_score + structure_score) / 2.0).clamp(0.0, 1.0);

    let (hash_score, exact_hash_match) =
        hash_similarity(&left_hash.key_file_hashes, &right_hash.key_file_hashes);
    let header_score = set_overlap_score(&left.ini_headers, &right.ini_headers);
    let file_identity = ((hash_score * 0.8) + (header_score * 0.2)).clamp(0.0, 1.0);

    let extension_score = extension_distribution_score(&left.extensions, &right.extensions);
    let texture_score = texture_similarity(&left_hash.texture_samples, &right_hash.texture_samples);
    let physical = ((extension_score * 0.5) + (texture_score * 0.5)).clamp(0.0, 1.0);

    let supporting = set_overlap_score(&left.keybindings, &right.keybindings);
    if exact_hash_match {
        let signals = vec![DupScanSignal {
            key: "content_hash".to_string(),
            detail: "All key-file BLAKE3 hashes match exactly".to_string(),
            score: 100,
        }];
        return (100, signals, "Exact hash match".to_string());
    }

    let weighted =
        (structural_name * 40.0) + (file_identity * 30.0) + (physical * 20.0) + (supporting * 10.0);
    let score = weighted.round().clamp(0.0, 99.0) as u8;
    let signals = vec![
        build_signal(
            "name_structure",
            "Front-name and tree similarity",
            structural_name,
        ),
        build_signal(
            "file_identity",
            "BLAKE3 key-file and INI header signal",
            file_identity,
        ),
        build_signal(
            "physical",
            "Extension distribution and texture sample signal",
            physical,
        ),
        build_signal("supporting", "Keybinding/supporting signal", supporting),
    ];

    let reason = if score >= 80 {
        "High name + structure similarity".to_string()
    } else {
        "Low confidence - manual review required".to_string()
    };
    (score, signals, reason)
}

fn phase2_name_and_structure(left: &ModSnapshot, right: &ModSnapshot) -> (f64, f64) {
    let first = normalize_name(&left.candidate.display_name);
    let second = normalize_name(&right.candidate.display_name);
    let front_score = front_name_similarity(&first, &second);
    let levenshtein = strsim::normalized_levenshtein(&first, &second);
    let name_score = (front_score * 0.6) + (levenshtein * 0.4);

    let left_files: BTreeSet<&str> = left
        .files
        .iter()
        .map(|file| file.rel_path.as_str())
        .collect();
    let right_files: BTreeSet<&str> = right
        .files
        .iter()
        .map(|file| file.rel_path.as_str())
        .collect();
    let overlap = left_files.intersection(&right_files).count() as f64;
    let max_len = left_files.len().max(right_files.len()) as f64;
    let structure_score = if max_len == 0.0 {
        0.0
    } else {
        overlap / max_len
    };

    (name_score, structure_score)
}

fn full_blake3_hash(path: &Path) -> Result<String, String> {
    let file =
        File::open(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Failed to hash {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_string())
}

fn partial_blake3_hash(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|error| format!("Failed to stat {}: {error}", path.display()))?
        .len();
    let mut hasher = blake3::Hasher::new();
    let mut head = [0_u8; 1024];
    let head_len = file
        .read(&mut head)
        .map_err(|error| format!("Failed to sample {}: {error}", path.display()))?;
    hasher.update(&head[..head_len]);

    if size > 1024 {
        file.seek(SeekFrom::End(-1024))
            .map_err(|error| format!("Failed to sample tail {}: {error}", path.display()))?;
        let mut tail = [0_u8; 1024];
        let tail_len = file
            .read(&mut tail)
            .map_err(|error| format!("Failed to sample tail {}: {error}", path.display()))?;
        hasher.update(&tail[..tail_len]);
    }

    Ok(hasher.finalize().to_string())
}

fn read_ini_signals(path: &Path) -> (BTreeSet<String>, BTreeSet<String>) {
    let file = match File::open(path) {
        Ok(value) => value,
        Err(_) => return (BTreeSet::new(), BTreeSet::new()),
    };
    let mut headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();

    for line in BufReader::new(file).lines().map_while(Result::ok).take(40) {
        let trimmed = line.trim().to_ascii_lowercase();
        if trimmed.starts_with(';') || trimmed.starts_with('[') {
            headers.insert(trimmed.clone());
        }
        if trimmed.contains("$swapvar") || trimmed.starts_with("key") {
            keybindings.insert(trimmed);
        }
    }

    (headers, keybindings)
}

fn normalize_name(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn front_name_similarity(left: &str, right: &str) -> f64 {
    let base = left.len().min(right.len());
    if base == 0 {
        return 0.0;
    }
    let front_len = ((base as f64) * 0.6).round().max(1.0) as usize;
    strsim::normalized_levenshtein(
        &left[..front_len.min(left.len())],
        &right[..front_len.min(right.len())],
    )
}

fn hash_similarity(
    left: &BTreeMap<String, String>,
    right: &BTreeMap<String, String>,
) -> (f64, bool) {
    if left.is_empty() || right.is_empty() {
        return (0.0, false);
    }
    let shared: Vec<_> = left
        .keys()
        .filter(|path| right.contains_key(*path))
        .collect();
    if shared.is_empty() {
        return (0.0, false);
    }
    let same = shared
        .iter()
        .filter(|path| left.get(**path) == right.get(**path))
        .count();
    let score = same as f64 / shared.len() as f64;
    (score, score == 1.0 && left.len() == right.len())
}

fn texture_similarity(left: &BTreeMap<String, String>, right: &BTreeMap<String, String>) -> f64 {
    let (score, _) = hash_similarity(left, right);
    score
}

fn set_overlap_score(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let intersection = left.intersection(right).count() as f64;
    let max_len = left.len().max(right.len()) as f64;
    intersection / max_len
}

fn extension_distribution_score(left: &HashMap<String, u64>, right: &HashMap<String, u64>) -> f64 {
    let keys: HashSet<_> = left.keys().chain(right.keys()).collect();
    if keys.is_empty() {
        return 0.0;
    }
    let total: f64 = keys
        .iter()
        .map(|key| {
            let l = *left.get(*key).unwrap_or(&0) as f64;
            let r = *right.get(*key).unwrap_or(&0) as f64;
            if l.max(r) == 0.0 {
                0.0
            } else {
                l.min(r) / l.max(r)
            }
        })
        .sum();
    total / keys.len() as f64
}

fn build_signal(key: &str, detail: &str, score: f64) -> DupScanSignal {
    DupScanSignal {
        key: key.to_string(),
        detail: detail.to_string(),
        score: (score * 100.0).round().clamp(0.0, 100.0) as u8,
    }
}
