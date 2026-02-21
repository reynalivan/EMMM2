use std::collections::{HashMap, HashSet};

use super::scoring::cap_evidence;
use super::types::{sort_candidates_deterministic, Candidate, Confidence, Evidence, ScoreState};
use super::MasterDb;

pub(super) fn collect_candidates(
    db: &MasterDb,
    states: &HashMap<usize, ScoreState>,
) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = states
        .iter()
        .map(|(entry_id, state)| Candidate {
            entry_id: *entry_id,
            name: db.entries[*entry_id].name.clone(),
            object_type: db.entries[*entry_id].object_type.clone(),
            score: state.score,
            confidence: Confidence::Low,
            reasons: state.reasons.clone(),
        })
        .collect();
    sort_candidates_deterministic(&mut candidates);
    candidates
}

pub(super) fn build_evidence(
    db: &MasterDb,
    signals: &super::content::FolderSignals,
    best: &Candidate,
) -> Evidence {
    let entry_tokens = entry_tokens(db, best.entry_id);

    let mut matched_hashes: Vec<String> = signals
        .ini_hashes
        .iter()
        .filter(|hash| {
            db.indexes
                .hash_index
                .get(*hash)
                .is_some_and(|posting| posting.contains(&best.entry_id))
        })
        .cloned()
        .collect();
    matched_hashes.sort();
    matched_hashes.dedup();

    let mut matched_tokens: Vec<String> = signals
        .folder_tokens
        .iter()
        .chain(signals.deep_name_tokens.iter())
        .chain(signals.ini_content_tokens.iter())
        .filter(|token| entry_tokens.contains(*token))
        .cloned()
        .collect();
    matched_tokens.sort();
    matched_tokens.dedup();

    let mut matched_sections: Vec<String> = signals
        .ini_section_tokens
        .iter()
        .filter(|token| entry_tokens.contains(*token))
        .cloned()
        .collect();
    matched_sections.sort();
    matched_sections.dedup();

    let mut evidence = Evidence {
        matched_hashes,
        matched_tokens,
        matched_sections,
        scanned_ini_files: signals.scanned_ini_files,
        scanned_name_items: signals.scanned_name_items,
    };
    cap_evidence(&mut evidence);
    evidence
}

pub(super) fn empty_evidence(signals: &super::content::FolderSignals) -> Evidence {
    Evidence {
        matched_hashes: Vec::new(),
        matched_tokens: Vec::new(),
        matched_sections: Vec::new(),
        scanned_ini_files: signals.scanned_ini_files,
        scanned_name_items: signals.scanned_name_items,
    }
}

fn entry_tokens<'a>(db: &'a MasterDb, entry_id: usize) -> &'a HashSet<String> {
    &db.keywords[entry_id].1
}
