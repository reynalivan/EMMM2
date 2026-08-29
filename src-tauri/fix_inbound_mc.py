import os

def rep(fp):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    
    # Imports
    c = c.replace("use crate::platform::fs::operation_lock::OperationLock;", "use crate::app::runtime::mutation_coordinator::MutationCoordinator;")
    c = c.replace("crate::platform::fs::operation_lock::OperationLock", "crate::app::runtime::mutation_coordinator::MutationCoordinator")
    
    # State types
    c = c.replace("State<'_, OperationLock>", "State<'_, MutationCoordinator>")
    
    # op_lock.acquire().await?
    # we need to extract op_guard.op_guard() when passing to functions that expect &OpGuard
    
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/storage_optimizer/adapters/inbound/tauri.rs")
rep("src/modules/workspace/adapters/inbound/disk_reconcile_cmds.rs")
rep("src/modules/workspace/adapters/inbound/tauri.rs")
rep("src/modules/workspace/adapters/inbound/workspace_cmds.rs")

print("Updated inbound adapters to use MutationCoordinator")
