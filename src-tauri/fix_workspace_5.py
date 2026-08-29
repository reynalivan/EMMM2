import os

def rep(fp):
    if not os.path.exists(fp): return
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    c = c.replace("repo::task", "crate::modules::workspace::adapters::outbound::sqlite::task")
    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

rep("src/modules/system/application/app/bootstrap.rs")
print("Fixed repo::task")
