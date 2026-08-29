
const fs = require('fs');
const path = require('path');

const srcDir = path.join(process.cwd(), 'src');

const map = {
    'collection': '@/entities/collection/model/collection',
    'dashboard': '@/pages/dashboard/model/dashboard',
    'game': '@/entities/game/model/game',
    'mod': '@/entities/mod/model/mod',
    'object': '@/entities/game-object/model/object',
    'scanner': '@/entities/workspace/model/scanner',
    'settings': '@/pages/settings/model/settings',
    'task': '@/entities/task/model/task',
    'workspace': '@/entities/workspace/model/workspace'
};

function walk(dir) {
    let files = [];
    fs.readdirSync(dir).forEach(f => {
        let dirPath = path.join(dir, f);
        if (fs.statSync(dirPath).isDirectory()) {
            files = files.concat(walk(dirPath));
        } else {
            files.push(dirPath);
        }
    });
    return files;
}

let count = 0;
const files = walk(srcDir);
files.forEach(f => {
    if (!f.endsWith('.ts') && !f.endsWith('.tsx')) return;
    let content = fs.readFileSync(f, 'utf8');
    let original = content;

    for (const [key, target] of Object.entries(map)) {
        const regex1 = new RegExp('from [\\'\\x22](?:@\/|\.\.\/|\.\.\/\.\.\/|\.\.\/\.\.\/\.\.\/|\.\.\/\.\.\/\.\.\/\.\.\/)+(?:types|entities\/common)\/' + key + '[\'\\x22]', 'g');
        content = content.replace(regex1, 'from \'' + target + '\'');
    }

    if (content !== original) {
        fs.writeFileSync(f, content, 'utf8');
        count++;
    }
});
console.log('Fixed files: ' + count);

