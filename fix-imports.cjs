const fs = require('fs');
const path = require('path');

function walk(dir) {
  let results = [];
  const list = fs.readdirSync(dir);
  list.forEach(file => {
    file = path.join(dir, file);
    const stat = fs.statSync(file);
    if (stat && stat.isDirectory()) {
      results = results.concat(walk(file));
    } else if (file.endsWith('.ts') || file.endsWith('.tsx')) {
      results.push(file);
    }
  });
  return results;
}

const allFiles = walk('src');
const fileMap = {};
allFiles.forEach(f => {
    const p = f.split(path.sep).join('/');
    const withoutExt = p.replace(/\.tsx?$/, ''); 
    const parts = withoutExt.split('/');
    let currentKey = '';
    for(let i = parts.length - 1; i >= 0; i--) {
        currentKey = currentKey === '' ? parts[i] : parts[i] + '/' + currentKey;
        if (!fileMap[currentKey]) fileMap[currentKey] = [];
        fileMap[currentKey].push(withoutExt.replace('src/', '@/'));
    }
});

let changedFiles = 0;

allFiles.forEach(f => {
    const dir = path.dirname(f);
    let content = fs.readFileSync(f, 'utf8');
    let changed = false;
    
    const regex = /from\s+['\"](\.[^'\"]+)['\"]/g;
    content = content.replace(regex, (match, importPath) => {
        const absoluteImportPath = path.resolve(dir, importPath);
        const exists = fs.existsSync(absoluteImportPath + '.ts') || 
                       fs.existsSync(absoluteImportPath + '.tsx') || 
                       fs.existsSync(absoluteImportPath + '/index.ts') || 
                       fs.existsSync(absoluteImportPath + '/index.tsx') ||
                       fs.existsSync(absoluteImportPath + '.json') ||
                       fs.existsSync(absoluteImportPath + '.d.ts');
                       
        if (!exists) {
            const parts = importPath.split('/');
            const cleanParts = parts.filter(p => p !== '.' && p !== '..');
            const searchKey = cleanParts.join('/');
            
            if (searchKey && fileMap[searchKey]) {
                changed = true;
                return 'from \'' + fileMap[searchKey][0] + '\'';
            } else {
                const lastPart = parts[parts.length - 1];
                if (fileMap[lastPart] && fileMap[lastPart].length === 1) {
                    changed = true;
                    return 'from \'' + fileMap[lastPart][0] + '\'';
                }
            }
        }
        return match;
    });
    
    // Also dynamic imports
    const dynRegex = /import\(['\"](\.[^'\"]+)['\"]\)/g;
    content = content.replace(dynRegex, (match, importPath) => {
        const absoluteImportPath = path.resolve(dir, importPath);
        const exists = fs.existsSync(absoluteImportPath + '.ts') || 
                       fs.existsSync(absoluteImportPath + '.tsx') || 
                       fs.existsSync(absoluteImportPath + '/index.ts') || 
                       fs.existsSync(absoluteImportPath + '/index.tsx');
                       
        if (!exists) {
            const parts = importPath.split('/');
            let searchKey = parts.filter(p => p !== '.' && p !== '..').join('/');
            
            if (searchKey && fileMap[searchKey] && fileMap[searchKey].length === 1) {
                changed = true;
                return 'import(\'' + fileMap[searchKey][0] + '\')';
            }
        }
        return match;
    });
    
    if (changed) {
        fs.writeFileSync(f, content, 'utf8');
        changedFiles++;
    }
});
console.log('Fixed imports in ' + changedFiles + ' files.');
