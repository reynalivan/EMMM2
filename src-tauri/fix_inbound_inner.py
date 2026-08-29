import os

def rep(fp):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    
    c = c.replace("&op_guard,", "op_guard.op_guard(),")
    c = c.replace("&operation_guard,", "operation_guard.op_guard(),")
    c = c.replace("operation_lock: operation_lock.inner(),", "operation_lock: operation_lock.inner().inner_lock(),")
    c = c.replace("operation_lock: &operation_lock,", "operation_lock: operation_lock.inner_lock(),")
    
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/storage_optimizer/adapters/inbound/tauri.rs")
rep("src/modules/workspace/adapters/inbound/disk_reconcile_cmds.rs")
rep("src/modules/workspace/adapters/inbound/tauri.rs")
rep("src/modules/workspace/adapters/inbound/workspace_cmds.rs")

print("Fixed op_guard and inner_lock passing")
