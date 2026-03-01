use crate::services::explorer::classifier::{classify_folder, NodeType};
use tempfile::TempDir;

#[test]
fn empty_dir_is_container() {
    let tmp = TempDir::new().unwrap();
    let (node_type, _) = classify_folder(tmp.path());
    assert_eq!(node_type, NodeType::ContainerFolder);
}
