//! The candidate prefilter enumerates exactly what the exhaustive loop filtered.
//!
//! The windowed form is only worth having if it is indistinguishable from the
//! N(N-1)/2 scan it replaced, so it is checked against that scan rather than
//! against hand-written expectations — a hand-written list would encode
//! whatever the new code happens to do.

use super::phase1_candidate_filtering;
use crate::services::scanner::core::walker::ModCandidate;
use crate::services::scanner::dedup::signals::{weights as w, FileEntry, ModSnapshot};
use crate::services::scanner::dedup::size_ratio;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

/// The prefilter as it was written before windowing: every combination, tested.
fn exhaustive_pairs(snapshots: &[ModSnapshot]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..snapshots.len() {
        for right in (left + 1)..snapshots.len() {
            let first = &snapshots[left];
            let second = &snapshots[right];
            if first.files.is_empty() || second.files.is_empty() {
                continue;
            }
            if first.files.len().abs_diff(second.files.len()) > w::CANDIDATE_FILE_COUNT_WINDOW {
                continue;
            }
            if size_ratio(first.total_size_bytes, second.total_size_bytes)
                < w::CANDIDATE_MIN_SIZE_RATIO
            {
                continue;
            }
            pairs.push((left, right));
        }
    }
    pairs
}

/// Only the three fields the prefilter reads carry information here; the rest
/// is the minimum a `ModSnapshot` needs to exist.
fn snapshot(index: usize, file_count: usize, total_size_bytes: u64) -> ModSnapshot {
    let files: Vec<FileEntry> = (0..file_count)
        .map(|n| FileEntry {
            rel_path: format!("file{n}.dds"),
            abs_path: PathBuf::from(format!("mod{index}/file{n}.dds")),
            size_bytes: 1,
            extension: "dds".to_string(),
        })
        .collect();
    let file_set = files.iter().map(|file| file.rel_path.clone()).collect();

    ModSnapshot {
        candidate: ModCandidate {
            path: PathBuf::from(format!("mod{index}")),
            raw_name: format!("mod{index}"),
            display_name: format!("mod{index}"),
            is_disabled: false,
        },
        files,
        file_set,
        total_size_bytes,
        ini_headers: BTreeSet::new(),
        keybindings: BTreeSet::new(),
        target_hashes: BTreeSet::new(),
        extensions: HashMap::new(),
        normalized_name: format!("mod{index}"),
        version_stripped_name: format!("mod{index}"),
    }
}

/// A seeded LCG rather than `rand`, so a failing round is reproducible from the
/// seed printed in the assertion alone.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % bound
    }
}

#[test]
fn windowing_agrees_with_the_exhaustive_loop() {
    let mut rng = Lcg(0x5EED);

    for round in 0..300_u64 {
        let count = 2 + rng.below(40) as usize;
        // A tight file-count spread is the case that matters: it keeps the
        // window populated, so an off-by-one in the window arithmetic shows up
        // as a missing or extra pair instead of an empty result on both sides.
        let snapshots: Vec<ModSnapshot> = (0..count)
            .map(|index| snapshot(index, rng.below(12) as usize, 1 + rng.below(2_000)))
            .collect();

        assert_eq!(
            phase1_candidate_filtering(&snapshots),
            exhaustive_pairs(&snapshots),
            "round {round}: windowed and exhaustive prefilters disagreed"
        );
    }
}

#[test]
fn the_file_count_window_is_inclusive_at_its_bound() {
    let window = w::CANDIDATE_FILE_COUNT_WINDOW;
    let within = [snapshot(0, 10, 1_000), snapshot(1, 10 + window, 1_000)];
    let beyond = [snapshot(0, 10, 1_000), snapshot(1, 10 + window + 1, 1_000)];

    assert_eq!(phase1_candidate_filtering(&within), vec![(0, 1)]);
    assert!(phase1_candidate_filtering(&beyond).is_empty());
}

#[test]
fn a_size_mismatch_inside_the_window_is_still_rejected() {
    // Same file count, so only the size ratio can reject the pair.
    let pair = [snapshot(0, 10, 1_000), snapshot(1, 10, 699)];
    assert!(phase1_candidate_filtering(&pair).is_empty());

    let borderline = [snapshot(0, 10, 1_000), snapshot(1, 10, 700)];
    assert_eq!(phase1_candidate_filtering(&borderline), vec![(0, 1)]);
}

#[test]
fn an_empty_folder_pairs_with_nothing() {
    // Identical in every respect the prefilter scores, and still excluded:
    // two empty folders are not evidence of duplication.
    let pair = [snapshot(0, 0, 0), snapshot(1, 0, 0)];
    assert!(phase1_candidate_filtering(&pair).is_empty());

    let mixed = [snapshot(0, 0, 0), snapshot(1, 3, 100)];
    assert!(phase1_candidate_filtering(&mixed).is_empty());
}

#[test]
fn pairs_are_emitted_low_index_first_and_in_order() {
    // Sorting by file count reorders the walk, so the emitted pairs have to be
    // put back: `build_groups` breaks score ties by position.
    let snapshots = [
        snapshot(0, 12, 1_000),
        snapshot(1, 4, 1_000),
        snapshot(2, 8, 1_000),
        snapshot(3, 10, 1_000),
        snapshot(4, 6, 1_000),
    ];

    let pairs = phase1_candidate_filtering(&snapshots);

    assert!(pairs.iter().all(|(left, right)| left < right));
    let mut sorted = pairs.clone();
    sorted.sort_unstable();
    assert_eq!(pairs, sorted);
    assert_eq!(pairs, exhaustive_pairs(&snapshots));
}
