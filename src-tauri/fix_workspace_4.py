import os

def rep(fp):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace("services::disk_reconcile", "crate::modules::workspace::application::disk_reconcile")
    c = c.replace("services::scanner", "crate::modules::workspace::application::scanner")
    c = c.replace("services::explorer", "crate::modules::workspace::application::explorer")
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/system/application/app/bootstrap.rs")
rep("src/lib.rs")

print("Fixed specific workspace imports 4")
