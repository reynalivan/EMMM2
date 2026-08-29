import os, re
src_dir = os.path.join(os.getcwd(), 'src')
mapping = {
    'collection': '@/entities/collection/model/collection',
    'dashboard': '@/pages/dashboard/model/dashboard',
    'game': '@/entities/game/model/game',
    'mod': '@/entities/mod/model/mod',
    'object': '@/entities/game-object/model/object',
    'scanner': '@/entities/workspace/model/scanner',
    'settings': '@/pages/settings/model/settings',
    'task': '@/entities/task/model/task',
    'workspace': '@/entities/workspace/model/workspace'
}

count = 0
for root, dirs, files in os.walk(src_dir):
    for f in files:
        if not (f.endswith('.ts') or f.endswith('.tsx')): continue
        filepath = os.path.join(root, f)
        with open(filepath, 'r', encoding='utf-8') as file:
            content = file.read()
        
        original = content
        for key, target in mapping.items():
            # match `from '...'`, `import('...')` etc. by not requiring 'from '
            pattern = r"['\"](?:@/|(?:\.\./)+)(?:types|entities/common)/" + key + r"['\"]"
            content = re.sub(pattern, f"'{target}'", content)
            
        if content != original:
            with open(filepath, 'w', encoding='utf-8') as file:
                file.write(content)
            count += 1

print(f"Fixed files: {count}")
