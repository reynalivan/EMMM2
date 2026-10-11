use super::hash_similarity;
use std::collections::BTreeMap;

#[test]
fn shared_hashes_with_different_asset_paths_are_not_exact() {
    let left = BTreeMap::from([
        ("mod.ini".to_string(), "same-ini".to_string()),
        ("body.dds".to_string(), "same-asset".to_string()),
    ]);
    let right = BTreeMap::from([
        ("mod.ini".to_string(), "same-ini".to_string()),
        ("face.dds".to_string(), "same-asset".to_string()),
    ]);

    assert_eq!(hash_similarity(&left, &right), (1.0, false));
    assert_eq!(hash_similarity(&right, &left), (1.0, false));
}

#[test]
fn exact_hash_identity_requires_equal_nonempty_maps() {
    let hashes = BTreeMap::from([("mod.ini".to_string(), "same-ini".to_string())]);

    assert_eq!(hash_similarity(&hashes, &hashes), (1.0, true));
    assert_eq!(
        hash_similarity(&BTreeMap::new(), &BTreeMap::new()),
        (0.0, false)
    );
}
