use super::*;
use crate::domain::collection::MemberKind;

fn make_object(object_id: &str, is_enabled: bool) -> CollectionObject {
    CollectionObject {
        kind: MemberKind::Object,
        collection_id: "col-1".to_string(),
        object_id: object_id.to_string(),
        is_enabled,
        display_name: Some(object_id.to_string()),
        path_key: Some(object_id.to_string()),
    }
}

fn make_member(
    object_id: &str,
    mod_path: &str,
    preview_path: Option<String>,
    node_type: &str,
    is_enabled: bool,
) -> CollectionMod {
    CollectionMod {
        kind: MemberKind::Mod,
        collection_id: "col-1".to_string(),
        mod_id: None,
        mod_path: mod_path.to_string(),
        mod_path_key: None,
        object_id: object_id.to_string(),
        display_name: Some("Blue Dress".to_string()),
        preview_path,
        node_type: Some(node_type.to_string()),
        warnings: Vec::new(),
        is_enabled,
    }
}

fn make_root(object_id: &str, root_key: &str, is_missing: bool) -> ProjectedActiveRoot {
    ProjectedActiveRoot {
        object_id: object_id.to_string(),
        root_key: root_key.to_string(),
        display_name: root_key.to_string(),
        root_type: ROOT_TYPE_FLAT.to_string(),
        source_path: root_key.to_string(),
        thumbnail_hint: None,
        warnings: Vec::new(),
        is_missing,
    }
}

#[test]
fn build_projected_state_with_no_members_is_empty() {
    let state = build_projected_state(&[], &[], None);

    assert!(state.object_states.is_empty());
    assert!(state.active_roots.is_empty());
    assert_eq!(state.summary.object_count, 0);
    assert_eq!(state.summary.enabled_object_count, 0);
    assert_eq!(state.summary.active_root_count, 0);
    assert_eq!(state.summary.missing_root_count, 0);
}

#[test]
fn build_projected_state_counts_visible_roots_and_flags_missing_sources() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mods_root = temp.path().join("Mods");
    let existing_dir = mods_root.join("Alice").join("Blue Dress");
    std::fs::create_dir_all(&existing_dir).expect("mod dir");
    let missing_dir = mods_root.join("Alice").join("Red Hat");
    let mods_root_str = mods_root.to_string_lossy().to_string();

    let objects = vec![make_object("Alice", true), make_object("Bob", false)];
    let mods = vec![
        make_member(
            "Alice",
            "Alice/Blue Dress",
            Some(existing_dir.to_string_lossy().to_string()),
            "FlatModRoot",
            true,
        ),
        make_member(
            "Alice",
            "Alice/Red Hat",
            Some(missing_dir.to_string_lossy().to_string()),
            "FlatModRoot",
            true,
        ),
        // Non-terminal node types never become active roots.
        make_member(
            "Alice",
            "Alice/Nested",
            Some(
                mods_root
                    .join("Alice")
                    .join("Nested")
                    .to_string_lossy()
                    .to_string(),
            ),
            "ContainerFolder",
            true,
        ),
        // Disabled members are ignored entirely.
        make_member(
            "Alice",
            "Alice/Off",
            Some(existing_dir.to_string_lossy().to_string()),
            "FlatModRoot",
            false,
        ),
    ];

    let state = build_projected_state(&mods, &objects, Some(&mods_root_str));

    assert_eq!(state.summary.object_count, 2);
    assert_eq!(state.summary.enabled_object_count, 1);
    assert_eq!(state.summary.active_root_count, 2);
    assert_eq!(state.summary.missing_root_count, 1);

    let alice = state
        .object_states
        .iter()
        .find(|object| object.object_id == "Alice")
        .expect("alice state");
    assert_eq!(alice.active_root_count, 2);
    let bob = state
        .object_states
        .iter()
        .find(|object| object.object_id == "Bob")
        .expect("bob state");
    assert_eq!(bob.active_root_count, 0);

    let missing_root = state
        .active_roots
        .iter()
        .find(|root| root.is_missing)
        .expect("missing root");
    assert_eq!(missing_root.display_name, "Red Hat");
    assert_eq!(missing_root.source_path, "Alice/Red Hat");
}

#[test]
fn signature_ignores_missing_roots_but_tracks_object_toggle() {
    let base = ProjectedCollectionState {
        object_states: vec![ProjectedObjectState {
            object_id: "Alice".to_string(),
            display_name: "Alice".to_string(),
            path_key: "alice".to_string(),
            is_enabled: true,
            active_root_count: 1,
        }],
        active_roots: vec![make_root("Alice", "alice/blue", false)],
        summary: ProjectedStateSummary {
            object_count: 1,
            enabled_object_count: 1,
            active_root_count: 1,
            missing_root_count: 0,
        },
    };

    let mut with_missing_root = base.clone();
    with_missing_root
        .active_roots
        .push(make_root("Alice", "alice/ghost", true));
    assert_eq!(
        signature_for_projected_state(&base),
        signature_for_projected_state(&with_missing_root),
        "missing roots must not affect the signature"
    );

    let mut toggled = base.clone();
    toggled.object_states[0].is_enabled = false;
    assert_ne!(
        signature_for_projected_state(&base),
        signature_for_projected_state(&toggled),
        "object toggle must change the signature"
    );
}

#[test]
fn snapshot_json_round_trips_and_rejects_garbage() {
    let state = ProjectedCollectionState {
        object_states: vec![ProjectedObjectState {
            object_id: "Alice".to_string(),
            display_name: "Alice".to_string(),
            path_key: "alice".to_string(),
            is_enabled: true,
            active_root_count: 1,
        }],
        active_roots: vec![make_root("Alice", "alice/blue", false)],
        summary: ProjectedStateSummary {
            object_count: 1,
            enabled_object_count: 1,
            active_root_count: 1,
            missing_root_count: 0,
        },
    };

    let json = serialize_snapshot_json(&state).expect("state serializes");
    let parsed = parse_snapshot_json(&json).expect("state parses back");
    assert_eq!(
        signature_for_projected_state(&state),
        signature_for_projected_state(&parsed)
    );
    assert_eq!(parsed.summary.active_root_count, 1);

    assert!(parse_snapshot_json("not json").is_none());
}

#[test]
fn preview_tree_marks_missing_roots_and_keeps_object_counts() {
    let state = ProjectedCollectionState {
        object_states: vec![ProjectedObjectState {
            object_id: "Alice".to_string(),
            display_name: "Alice".to_string(),
            path_key: "alice".to_string(),
            is_enabled: true,
            active_root_count: 2,
        }],
        active_roots: vec![
            make_root("Alice", "alice/blue", false),
            make_root("Alice", "alice/ghost", true),
        ],
        summary: ProjectedStateSummary {
            object_count: 1,
            enabled_object_count: 1,
            active_root_count: 2,
            missing_root_count: 1,
        },
    };

    let tree = build_preview_tree_from_projected_state(&state);

    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].mod_count, Some(2));
    assert_eq!(tree[0].children.len(), 2);
    let ghost = tree[0]
        .children
        .iter()
        .find(|child| child.id == "root::alice/ghost")
        .expect("ghost child");
    assert!(!ghost.is_effectively_active);
    assert_eq!(ghost.status_kind.as_deref(), Some("missing"));
}
