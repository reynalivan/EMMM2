import os

def rep(fp, old, new):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace(old, new)
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

for fp in ["src/modules/system/application/app/bootstrap.rs", "src/lib.rs"]:
    rep(fp, "services::disk_reconcile", "crate::modules::workspace::application::disk_reconcile")
    rep(fp, "services::scanner", "crate::modules::workspace::application::scanner")
    rep(fp, "services::explorer", "crate::modules::workspace::application::explorer")

print("Fixed specific workspace imports 3")
