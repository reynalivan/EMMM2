import os

fp = "src/lib.rs"
if os.path.exists(fp):
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    
    inject = """
        .manage(crate::app::runtime::operation_journal::OperationJournal::new())
        .manage(crate::app::runtime::mutation_coordinator::MutationCoordinator::new(std::sync::Arc::new(crate::app::runtime::operation_journal::OperationJournal::new())))
"""
    c = c.replace(".manage(crate::platform::fs::operation_lock::OperationLock::new())", ".manage(crate::platform::fs::operation_lock::OperationLock::new())" + inject)
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

print("Wired MutationCoordinator in lib.rs")
