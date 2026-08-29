
const fs = require('fs');
const path = require('path');

const srcDir = path.join(process.cwd(), 'src');

const rewriteRules = [
    { regex: /@\/(?:types|entities\/common)\/collection/g, target: '@/entities/collection/model/collection' },
    { regex: /@\/(?:types|entities\/common)\/dashboard/g, target: '@/pages/dashboard/model/dashboard' },
    { regex: /@\/(?:types|entities\/common)\/game/g, target: '@/entities/game/model/game' },
    { regex: /@\/(?:types|entities\/common)\/mod/g, target: '@/entities/mod/model/mod' },
    { regex: /@\/(?:types|entities\/common)\/object/g, target: '@/entities/game-object/model/object' },
    { regex: /@\/(?:types|entities\/common)\/scanner/g, target: '@/entities/workspace/model/scanner' },
    { regex: /@\/(?:types|entities\/common)\/settings/g, target: '@/pages/settings/model/settings' },
    { regex: /@\/(?:types|entities\/common)\/task/g, target: '@/entities/task/model/task' },
    { regex: /@\/(?:types|entities\/common)\/workspace/g, target: '@/entities/workspace/model/workspace' },
];

function walk(dir, callback) {
    fs.readdirSync(dir).forEach(f => {
        let dirPath = path.join(dir, f);
        let isDirectory = fs.statSync(dirPath).isDirectory();
        isDirectory ? walk(dirPath, callback) : callback(dirPath);
    });
}

let changedFiles = 0;
walk(srcDir, (filePath) => {
    if (!filePath.endsWith('.ts') && !filePath.endsWith('.tsx')) return;
    
    let original = fs.readFileSync(filePath, 'utf8');
    let content = original;
    
    content = content.replace(/from\s+['\x22](?:\.\.\/)+types\/(.*?)['\x22]/g, 'from \x22@/\x22');
    content = content.replace(/from\s+['\x22](?:\.\.\/)+entities\/common\/(.*?)['\x22]/g, 'from \x22@/\x22');
    
    rewriteRules.forEach(rule => {
        content = content.replace(rule.regex, rule.target);
    });
    
    const catchAllMap = {
        '@/collection': '@/entities/collection/model/collection',
        '@/dashboard': '@/pages/dashboard/model/dashboard',
        '@/game': '@/entities/game/model/game',
        '@/mod': '@/entities/mod/model/mod',
        '@/object': '@/entities/game-object/model/object',
        '@/scanner': '@/entities/workspace/model/scanner',
        '@/settings': '@/pages/settings/model/settings',
        '@/task': '@/entities/task/model/task',
        '@/workspace': '@/entities/workspace/model/workspace'
    };
    
    for (const [key, val] of Object.entries(catchAllMap)) {
        content = content.replace(new RegExp('from [\\'\\x22]' + key + '[\\'\\x22]', 'g'), 'from \x22' + val + '\x22');
    }

    if (original !== content) {
        fs.writeFileSync(filePath, content, 'utf8');
        changedFiles++;
    }
});
console.log('Fixed imports in ' + changedFiles + ' files.');

