import os

fp = "tests/arch_audit.rs"
if os.path.exists(fp):
    with open(fp, "r", encoding="utf-8") as f:
        c = f.read()
    
    # Replace references to services, repo, domain
    c = c.replace('"src/services"', '"src/modules"')
    c = c.replace('"src/repo"', '"src/modules"')
    c = c.replace('"domain/mod_path.rs"', '"modules/system/domain/mod_path.rs"')
    c = c.replace('repo::mods', 'modules::library::adapters::outbound::sqlite::mods')
    c = c.replace('services::mods', 'modules::library::application::mods')
    c = c.replace('services::workspace', 'modules::workspace::application::workspace')
    c = c.replace('services::explorer', 'modules::workspace::application::explorer')

    with open(fp, "w", encoding="utf-8") as f:
        f.write(c)

print("Fixed arch_audit paths")
