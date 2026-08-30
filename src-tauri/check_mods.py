for root, _, files in os.walk("src/modules"):
    for file in files:
        if file == "mod.rs" and "adapters" in root:
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()
            print(f"--- {filepath} ---")
            print(content)
