import json
import os

def get_keys(data, prefix=''):
    keys = set()
    if isinstance(data, dict):
        for k, v in data.items():
            new_key = f"{prefix}.{k}" if prefix else k
            keys.add(new_key)
            keys.update(get_keys(v, new_key))
    return keys

def compare_languages(base_dir, base_lang, target_langs):
    base_path = os.path.join(base_dir, base_lang)
    files = [f for f in os.listdir(base_path) if f.endswith('.json')]
    
    report = []
    
    for filename in files:
        with open(os.path.join(base_path, filename), 'r', encoding='utf-8') as f:
            base_data = json.load(f)
        
        base_keys = get_keys(base_data)
        
        for lang in target_langs:
            target_path = os.path.join(base_dir, lang, filename)
            if not os.path.exists(target_path):
                report.append(f"MISSING FILE: {lang}/{filename}")
                continue
            
            with open(target_path, 'r', encoding='utf-8') as f:
                target_data = json.load(f)
            
            target_keys = get_keys(target_data)
            
            missing_in_target = base_keys - target_keys
            extra_in_target = target_keys - base_keys
            
            if missing_in_target:
                report.append(f"\n[{filename}] Missing keys in {lang}:")
                for k in sorted(missing_in_target):
                    report.append(f"  - {k}")
            
            if extra_in_target:
                report.append(f"\n[{filename}] Extra keys in {lang} (not in {base_lang}):")
                for k in sorted(extra_in_target):
                    report.append(f"  - {k}")
                    
    return "\n".join(report)

if __name__ == "__main__":
    locales_dir = r"src\locales"
    result = compare_languages(locales_dir, "en", ["id", "zh"])
    if not result:
        print("No gaps found!")
    else:
        print(result)
