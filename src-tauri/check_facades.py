import os

facades = [
    "src/modules/browser/facade.rs",
    "src/modules/collections/facade.rs",
    "src/modules/duplicates/facade.rs",
    "src/modules/ingestion/facade.rs"
]
for f in facades:
    mod_rs = os.path.join(os.path.dirname(f), "mod.rs")
    with open(mod_rs, "r") as m:
        content = m.read()
    if "facade" in content:
        print(f"Used in {mod_rs}")
    else:
        print(f"NOT used in {mod_rs}")
        os.remove(f)
