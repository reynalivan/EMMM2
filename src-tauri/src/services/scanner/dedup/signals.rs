use crate::domain::errors::ScannerError;
use crate::services::scanner::core::walker::ModCandidate;
use crate::types::dup_scan::DupScanSignal;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const KEY_EXTS: &[&str] = &["ini", "dds", "buf", "ib", "vb"];
/// Textures carry the colour, meshes carry the shape. The two are hashed into
/// separate buckets because a pair that shares its meshes but not its textures
/// is a recolor, and that scores differently from an unrelated pair.
const TEXTURE_EXT: &str = "dds";
const MESH_EXTS: &[&str] = &["ib", "buf"];
const PARTIAL_HASH_THRESHOLD_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct ModSnapshot {
    pub candidate: ModCandidate,
    pub files: Vec<FileEntry>,
    pub total_size_bytes: u64,
    pub ini_headers: BTreeSet<String>,
    pub keybindings: BTreeSet<String>,
    pub target_hashes: BTreeSet<String>,
    pub extensions: HashMap<String, u64>,
    /// The three fields below are per-*snapshot* facts that used to be
    /// recomputed per *pair*. A mod that lands in k candidate pairs rebuilt its
    /// file set and re-ran two name normalizations k times; the folder walk
    /// that produces the snapshot already has everything they need.
    pub file_set: BTreeSet<String>,
    pub normalized_name: String,
    pub version_stripped_name: String,
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
    pub mesh_hashes: BTreeMap<String, String>,
}

pub(crate) fn collect_snapshot(candidate: &ModCandidate) -> Result<ModSnapshot, ScannerError> {
    let mut files = Vec::new();
    let mut total_size = 0_u64;
    let mut ini_headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();
    let mut target_hashes = BTreeSet::new();
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
            let (headers, bindings, hashes) = read_ini_signals(&path);
            ini_headers.extend(headers);
            keybindings.extend(bindings);
            target_hashes.extend(hashes);
        }

        files.push(FileEntry {
            rel_path: rel,
            abs_path: path,
            size_bytes: size,
            extension,
        });
    }

    let file_set: BTreeSet<String> = files.iter().map(|file| file.rel_path.clone()).collect();

    Ok(ModSnapshot {
        normalized_name: normalize_name(&candidate.display_name),
        version_stripped_name: strip_version(&candidate.display_name),
        candidate: candidate.clone(),
        files,
        file_set,
        total_size_bytes: total_size,
        ini_headers,
        keybindings,
        target_hashes,
        extensions,
    })
}

pub(crate) fn hash_snapshot(snapshot: &ModSnapshot) -> HashProfile {
    let mut profile = HashProfile::default();
    for file in &snapshot.files {
        if !KEY_EXTS.contains(&file.extension.as_str()) {
            continue;
        }
        let is_texture = file.extension == TEXTURE_EXT;
        let hash = if is_texture && file.size_bytes > PARTIAL_HASH_THRESHOLD_BYTES {
            partial_blake3_hash(&file.abs_path)
        } else {
            full_blake3_hash(&file.abs_path)
        };
        if let Ok(value) = hash {
            profile
                .key_file_hashes
                .insert(file.rel_path.clone(), value.clone());
            if is_texture {
                profile.texture_samples.insert(file.rel_path.clone(), value);
            } else if MESH_EXTS.contains(&file.extension.as_str()) {
                profile.mesh_hashes.insert(file.rel_path.clone(), value);
            }
        }
    }
    profile
}

static RE_VERSION: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(v|ver|version)\s*\d+(\.\d+)*\b").unwrap()
});

fn strip_version(name: &str) -> String {
    RE_VERSION
        .replace_all(name, "")
        .to_string()
        .replace("  ", " ")
        .trim()
        .to_lowercase()
}

/// Every tunable in the duplicate-similarity model, in one place.
///
/// The four `TIER_*` weights sum to 100; the bonuses can push a pair above that
/// before the clamp, which is why a non-exact match tops out at
/// [`MAX_INEXACT_SCORE`] rather than 100 — only a full hash match scores 100.
pub(super) mod weights {
    /// Candidate prefilter: the two cheap bounds that decide which pairs are
    /// worth hashing at all. Widening either one costs scan time quadratically.
    pub const CANDIDATE_FILE_COUNT_WINDOW: usize = 4;
    pub const CANDIDATE_MIN_SIZE_RATIO: f64 = 0.70;

    /// Tier weights (sum to 100).
    pub const TIER_STRUCTURAL_NAME: f64 = 35.0;
    pub const TIER_FILE_IDENTITY: f64 = 30.0;
    pub const TIER_PHYSICAL: f64 = 20.0;
    pub const TIER_SUPPORTING: f64 = 15.0;

    /// Name mix (sums to 1.0), and how much of the shorter name counts as its
    /// "front" — mod names diverge at the tail (" v2", " (blue)"), so the head
    /// is the more honest comparison.
    pub const NAME_FRONT_MIX: f64 = 0.6;
    pub const NAME_LEVENSHTEIN_MIX: f64 = 0.4;
    pub const FRONT_NAME_FRACTION: f64 = 0.6;

    /// Intra-tier mixes (each group sums to 1.0).
    pub const FILE_IDENTITY_HASH: f64 = 0.8;
    pub const FILE_IDENTITY_HEADERS: f64 = 0.2;
    pub const PHYSICAL_EXTENSIONS: f64 = 0.3;
    pub const PHYSICAL_TEXTURES: f64 = 0.4;
    pub const PHYSICAL_MESHES: f64 = 0.3;
    pub const SUPPORTING_KEYBINDINGS: f64 = 0.5;
    pub const SUPPORTING_LOGICAL: f64 = 0.5;

    /// Bonuses applied before clamping.
    pub const RECOLOR_BONUS: f64 = 25.0;
    pub const LOGICAL_OVERLAP_BONUS: f64 = 15.0;

    /// Thresholds.
    pub const RECOLOR_MIN_SIZE_RATIO: f64 = 0.95;
    pub const LOGICAL_OVERLAP_BONUS_MIN: f64 = 0.8;
    pub const MAX_INEXACT_SCORE: f64 = 99.0;
    /// A same-name/different-version pair is a duplicate regardless of content drift.
    pub const VERSION_UPGRADE_FLOOR: u8 = 85;
    /// Below this a pair is not worth showing the user at all.
    pub const MIN_REPORTED_SCORE: u8 = 45;
    /// At or above this the pair is reported as a name/structure match rather
    /// than as needing manual review.
    pub const HIGH_SIMILARITY_SCORE: u8 = 80;
}

pub(crate) fn aggregate_signals(
    left: &ModSnapshot,
    right: &ModSnapshot,
    left_hash: &HashProfile,
    right_hash: &HashProfile,
) -> (u8, Vec<DupScanSignal>, String) {
    use weights as w;

    let (name_score, structure_score) = phase2_name_and_structure(left, right);
    let structural_name = ((name_score + structure_score) / 2.0).clamp(0.0, 1.0);

    let (hash_score, exact_hash_match) =
        hash_similarity(&left_hash.key_file_hashes, &right_hash.key_file_hashes);
    let header_score = set_overlap_score(&left.ini_headers, &right.ini_headers);
    let file_identity = ((hash_score * w::FILE_IDENTITY_HASH)
        + (header_score * w::FILE_IDENTITY_HEADERS))
        .clamp(0.0, 1.0);

    let extension_score = extension_distribution_score(&left.extensions, &right.extensions);
    let (texture_score, _) =
        hash_similarity(&left_hash.texture_samples, &right_hash.texture_samples);
    let (mesh_score, exact_mesh_match) =
        hash_similarity(&left_hash.mesh_hashes, &right_hash.mesh_hashes);

    let physical = ((extension_score * w::PHYSICAL_EXTENSIONS)
        + (texture_score * w::PHYSICAL_TEXTURES)
        + (mesh_score * w::PHYSICAL_MESHES))
        .clamp(0.0, 1.0);

    let keybinding_score = set_overlap_score(&left.keybindings, &right.keybindings);
    let logical_overlap = set_overlap_score(&left.target_hashes, &right.target_hashes);

    let supporting = ((keybinding_score * w::SUPPORTING_KEYBINDINGS)
        + (logical_overlap * w::SUPPORTING_LOGICAL))
        .clamp(0.0, 1.0);

    let is_version_upgrade = left.version_stripped_name == right.version_stripped_name
        && !left.version_stripped_name.is_empty()
        && left.candidate.display_name != right.candidate.display_name;

    let size_diff = super::size_ratio(left.total_size_bytes, right.total_size_bytes);
    let is_potential_recolor = exact_mesh_match
        && !left_hash.mesh_hashes.is_empty()
        && size_diff > w::RECOLOR_MIN_SIZE_RATIO
        && texture_score < 1.0;

    if exact_hash_match {
        let signals = vec![DupScanSignal {
            key: "content_hash".to_string(),
            detail: "All key-file BLAKE3 hashes match exactly".to_string(),
            score: 100,
        }];
        return (100, signals, "Exact hash match".to_string());
    }

    let mut weighted = (structural_name * w::TIER_STRUCTURAL_NAME)
        + (file_identity * w::TIER_FILE_IDENTITY)
        + (physical * w::TIER_PHYSICAL)
        + (supporting * w::TIER_SUPPORTING);

    if is_potential_recolor {
        weighted += w::RECOLOR_BONUS;
    }

    if logical_overlap > w::LOGICAL_OVERLAP_BONUS_MIN {
        weighted += w::LOGICAL_OVERLAP_BONUS;
    }

    let mut score = weighted.round().clamp(0.0, w::MAX_INEXACT_SCORE) as u8;

    if is_version_upgrade {
        score = score.max(w::VERSION_UPGRADE_FLOOR);
    }

    let mut signals = vec![
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
            "Extension distribution, texture, and mesh signal",
            physical,
        ),
        build_signal(
            "supporting",
            "Keybinding and Logical INI Hash overlap",
            supporting,
        ),
    ];

    if logical_overlap > 0.0 {
        signals.push(build_signal(
            "logical_overlap",
            "INI target hashes overlap (3DMigoto)",
            logical_overlap,
        ));
    }

    let reason = if is_potential_recolor {
        "Potential Recolor / Retexture Variant".to_string()
    } else if is_version_upgrade {
        "Version Upgrade Detected".to_string()
    } else if logical_overlap > w::LOGICAL_OVERLAP_BONUS_MIN {
        "Logical INI Hash Override Conflict".to_string()
    } else if score >= w::HIGH_SIMILARITY_SCORE {
        "High name + structure similarity".to_string()
    } else {
        "Low confidence - manual review required".to_string()
    };

    (score, signals, reason)
}

fn phase2_name_and_structure(left: &ModSnapshot, right: &ModSnapshot) -> (f64, f64) {
    use weights as w;

    let first = &left.normalized_name;
    let second = &right.normalized_name;
    let front_score = front_name_similarity(first, second);
    let levenshtein = strsim::normalized_levenshtein(first, second);
    let name_score = (front_score * w::NAME_FRONT_MIX) + (levenshtein * w::NAME_LEVENSHTEIN_MIX);

    let overlap = left.file_set.intersection(&right.file_set).count() as f64;
    let max_len = left.file_set.len().max(right.file_set.len()) as f64;
    let structure_score = if max_len == 0.0 {
        0.0
    } else {
        overlap / max_len
    };

    (name_score, structure_score)
}

fn full_blake3_hash(path: &Path) -> Result<String, ScannerError> {
    let file = File::open(path)?;
    // blake3's own reader does the buffering; an 8 KiB hand-rolled loop is
    // below the 16 KiB the multi-threaded fast path needs.
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file)?;
    Ok(hasher.finalize().to_string())
}

fn partial_blake3_hash(path: &Path) -> Result<String, ScannerError> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    let mut head = [0_u8; 1024];
    let head_len = file.read(&mut head)?;
    hasher.update(&head[..head_len]);

    if size > 1024 {
        file.seek(SeekFrom::End(-1024))?;
        let mut tail = [0_u8; 1024];
        let tail_len = file.read(&mut tail)?;
        hasher.update(&tail[..tail_len]);
    }

    Ok(hasher.finalize().to_string())
}

fn read_ini_signals(path: &Path) -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    let file = match File::open(path) {
        Ok(value) => value,
        Err(_) => return (BTreeSet::new(), BTreeSet::new(), BTreeSet::new()),
    };
    let mut headers = BTreeSet::new();
    let mut keybindings = BTreeSet::new();
    let mut target_hashes = BTreeSet::new();

    for line in BufReader::new(file).lines().map_while(Result::ok).take(200) {
        let trimmed = line.trim().to_ascii_lowercase();
        if trimmed.starts_with(';') || trimmed.starts_with('[') {
            headers.insert(trimmed.clone());
        } else if trimmed.contains("$swapvar") || trimmed.starts_with("key") {
            keybindings.insert(trimmed.clone());
        } else if trimmed.starts_with("hash") {
            if let Some(hash_val) = trimmed.split('=').nth(1) {
                target_hashes.insert(hash_val.trim().to_string());
            }
        }
    }

    (headers, keybindings, target_hashes)
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
    let front_len = ((base as f64) * weights::FRONT_NAME_FRACTION)
        .round()
        .max(1.0) as usize;
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
