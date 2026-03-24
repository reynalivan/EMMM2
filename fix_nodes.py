import os
import re

files_to_fix = [
    r"src-tauri/src/services/scanner/sync/preview.rs",
    r"src-tauri/src/services/scanner/master_db.rs",
    r"src-tauri/src/services/scanner/deep_matcher/tests/required_tests.rs",
    r"src-tauri/src/services/scanner/deep_matcher/tests/pipeline/quick_pipeline_tests.rs",
    r"src-tauri/src/services/scanner/deep_matcher/tests/pipeline/name_rescue_tests.rs",
    r"src-tauri/src/services/scanner/deep_matcher/tests/pipeline/full_pipeline_tests.rs",
    r"src-tauri/src/services/scanner/deep_matcher/tests/alias_resolution_tests.rs",
    r"src-tauri/src/services/scanner/deep_matcher/golden_corpus.rs",
    r"src-tauri/src/services/scanner/core/organizer.rs",
    r"src-tauri/src/services/scanner/dedup/scanner.rs",
    r"src-tauri/src/services/browser/import_service.rs",
    r"src-tauri/src/commands/scanner/archive_cmds.rs"
]

node_type_str = "\n            node_type: crate::services::explorer::classifier::NodeType::FlatModRoot,"
node_type_str2 = "\n        node_type: crate::services::explorer::classifier::NodeType::FlatModRoot,"

for file_path in files_to_fix:
    full_path = os.path.join(r"e:\Dev\EMMMNEW", file_path)
    if not os.path.exists(full_path):
        continue
    
    with open(full_path, "r", encoding="utf-8") as f:
        content = f.read()

    # The regex targets `is_disabled: <bool_expr>,` (or without comma) inside ModCandidate initialization
    # and adds node_type right after it.
    new_content = re.sub(
        r"(is_disabled:\s*[^,]+),?",
        r"\1," + node_type_str,
        content
    )
    
    if new_content != content:
        with open(full_path, "w", encoding="utf-8") as f:
            f.write(new_content)
        print(f"Fixed {file_path}")
