import os

modules_dir = "src/modules"
for mod in os.listdir(modules_dir):
    mod_path = os.path.join(modules_dir, mod)
    if os.path.isdir(mod_path):
        adapters_dir = os.path.join(mod_path, "adapters")
        if os.path.isdir(adapters_dir):
            lines = []
            for item in os.listdir(adapters_dir):
                item_path = os.path.join(adapters_dir, item)
                if item == "mod.rs": continue
                if os.path.isdir(item_path):
                    if os.path.exists(os.path.join(item_path, "mod.rs")):
                        lines.append(f"pub mod {item};\n")
                elif item.endswith(".rs"):
                    mod_name = item[:-3]
                    lines.append(f"pub mod {mod_name};\n")
                    
            mod_file = os.path.join(adapters_dir, "mod.rs")
            with open(mod_file, "w") as f:
                f.writelines(lines)
