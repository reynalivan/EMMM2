import os

def rep(fp, old, new):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace(old, new)
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/system/application/app/bootstrap.rs", "services::workspace_read_model", "crate::modules::workspace::application::workspace_read_model")
rep("src/lib.rs", "services::scanner", "crate::modules::workspace::application::scanner")

print("Fixed specific workspace imports")
