use super::{build_groups, ScoredPair};
use crate::modules::workspace::application::scanner::core::walker::ModCandidate;
use crate::modules::workspace::application::scanner::dedup::snapshot::{FileEntry, ModSnapshot};
use crate::types::dup_scan::DupScanSignal;
use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

fn snapshot(name: &str) -> ModSnapshot {
    let path = PathBuf::from(name);
    let file = FileEntry {
        rel_path: "body.dds".to_string(),
        abs_path: path.join("body.dds"),
        size_bytes: 10,
        extension: "dds".to_string(),
    };

    ModSnapshot {
        candidate: ModCandidate {
            path,
            raw_name: name.to_string(),
            display_name: name.to_string(),
            is_disabled: false,
        },
        files: vec![file],
        total_size_bytes: 10,
        ini_headers: BTreeSet::new(),
        keybindings: BTreeSet::new(),
        target_hashes: BTreeSet::new(),
        extensions: HashMap::from([("dds".to_string(), 1)]),
        file_set: BTreeSet::from(["body.dds".to_string()]),
        normalized_name: name.to_ascii_lowercase(),
        version_stripped_name: name.to_ascii_lowercase(),
    }
}

fn inexact_pair(left: usize, right: usize) -> ScoredPair {
    (
        left,
        right,
        85,
        vec![DupScanSignal {
            key: "similarity".to_string(),
            detail: "similar but not identical".to_string(),
            score: 85,
        }],
        "High name + structure similarity".to_string(),
    )
}

#[test]
fn inexact_similarity_edges_do_not_form_transitive_groups() {
    let snapshots = [snapshot("A"), snapshot("B"), snapshot("C")];
    let pairs = [inexact_pair(0, 1), inexact_pair(1, 2)];

    let groups = build_groups(&snapshots, &pairs, &HashMap::new());

    assert_eq!(groups.len(), 2);
    assert!(groups.iter().all(|group| group.members.len() == 2));
    assert!(groups.iter().all(|group| {
        let paths: Vec<_> = group
            .members
            .iter()
            .map(|member| member.folder_path.as_str())
            .collect();
        paths == ["A", "B"] || paths == ["B", "C"]
    }));
}
