import fs from 'fs';
import path from 'path';

const srcDir = path.resolve('src');

const moves = {
  'features/workspace-runtime/useWorkspaceViewModel': 'features/workspace-runtime/hooks/useWorkspaceViewModel',
  'features/workspace-runtime/pathRewrite': 'features/workspace-runtime/utils/pathRewrite',
  'features/workspace-runtime/selectionReconciliation': 'features/workspace-runtime/utils/selectionReconciliation',
  'features/workspace-runtime/workspaceIntentBus': 'features/workspace-runtime/utils/workspaceIntentBus',
  'features/workspace-runtime/workspaceSemantics': 'features/workspace-runtime/utils/workspaceSemantics'
};

function ensureDir(filePath) {
  const dir = path.dirname(filePath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
}

// 1. Move files
console.log('Moving files...');
for (let [oldPath, newPath] of Object.entries(moves)) {
  const oldAbs = path.resolve(srcDir, oldPath);
  const newAbs = path.resolve(srcDir, newPath);
  
  if (fs.existsSync(oldAbs)) {
    if (fs.statSync(oldAbs).isDirectory()) {
      ensureDir(newAbs);
      fs.renameSync(oldAbs, newAbs);
      console.log(`Moved dir ${oldPath} to ${newPath}`);
    } else {
      ensureDir(newAbs);
      fs.renameSync(oldAbs, newAbs);
      console.log(`Moved file ${oldPath} to ${newPath}`);
    }
  } else if (fs.existsSync(oldAbs + '.ts')) {
    ensureDir(newAbs + '.ts');
    fs.renameSync(oldAbs + '.ts', newAbs + '.ts');
    console.log(`Moved ${oldPath}.ts to ${newPath}.ts`);
    if (fs.existsSync(oldAbs + '.test.ts')) {
       fs.renameSync(oldAbs + '.test.ts', newAbs + '.test.ts');
    }
  } else if (fs.existsSync(oldAbs + '.tsx')) {
    ensureDir(newAbs + '.tsx');
    fs.renameSync(oldAbs + '.tsx', newAbs + '.tsx');
    console.log(`Moved ${oldPath}.tsx to ${newPath}.tsx`);
    if (fs.existsSync(oldAbs + '.test.tsx')) {
       fs.renameSync(oldAbs + '.test.tsx', newAbs + '.test.tsx');
    }
  }
}
// For useWorkspaceViewModel.contract.test.ts specifically
if (fs.existsSync(path.resolve(srcDir, 'features/workspace-runtime/useWorkspaceViewModel.contract.test.ts'))) {
  fs.renameSync(
    path.resolve(srcDir, 'features/workspace-runtime/useWorkspaceViewModel.contract.test.ts'),
    path.resolve(srcDir, 'features/workspace-runtime/hooks/useWorkspaceViewModel.contract.test.ts')
  );
}

// 2. Fix imports
console.log('Fixing imports...');
function getFiles(dir) {
  let results = [];
  const list = fs.readdirSync(dir);
  list.forEach(file => {
    const filePath = path.resolve(dir, file);
    const stat = fs.statSync(filePath);
    if (stat && stat.isDirectory()) {
      results = results.concat(getFiles(filePath));
    } else if (file.endsWith('.ts') || file.endsWith('.tsx')) {
      results.push(filePath);
    }
  });
  return results;
}

const allFiles = getFiles(srcDir);

allFiles.forEach(file => {
  let content = fs.readFileSync(file, 'utf8');
  let changed = false;
  
  const importExportRegex = /((?:import|export)\s+[^'"]*from\s+['"])([^'"]+)(['"])|(?:import\(['"])([^'"]+)(['"]\))/g;
  
  content = content.replace(importExportRegex, (match, p1, p2, p3, p4, p5) => {
    const isDynamic = !!p4;
    const prefix = isDynamic ? 'import("' : p1;
    const suffix = isDynamic ? '")' : p3;
    const importPath = isDynamic ? p4 : p2;
    
    if (!importPath.startsWith('.')) return match;
    
    const relToSrcNow = path.relative(srcDir, file).replace(/\\/g, '/');
    let oldRelToSrc = relToSrcNow;
    
    for (const [oPath, nPath] of Object.entries(moves)) {
      if (relToSrcNow === nPath + '.ts' || relToSrcNow === nPath + '.tsx' || relToSrcNow === nPath + '.test.ts' || relToSrcNow === nPath + '.test.tsx' || relToSrcNow === nPath + '.contract.test.ts' || relToSrcNow.startsWith(nPath + '/')) {
        oldRelToSrc = relToSrcNow.replace(nPath, oPath);
        break;
      }
    }
    
    const oldFileDir = path.dirname(path.resolve(srcDir, oldRelToSrc));
    const oldAbsImportPath = path.resolve(oldFileDir, importPath);
    const oldImportRelToSrc = path.relative(srcDir, oldAbsImportPath).replace(/\\/g, '/');
    
    let newImportRelToSrc = oldImportRelToSrc;
    for (const [oPath, nPath] of Object.entries(moves)) {
      if (oldImportRelToSrc === oPath || oldImportRelToSrc.startsWith(oPath + '/')) {
        newImportRelToSrc = oldImportRelToSrc.replace(oPath, nPath);
        break;
      }
    }
    
    const newAbsImportPath = path.resolve(srcDir, newImportRelToSrc);
    const fileDir = path.dirname(file);
    
    let newImportPath = path.relative(fileDir, newAbsImportPath).replace(/\\/g, '/');
    if (!newImportPath.startsWith('.')) {
      newImportPath = './' + newImportPath;
    }
    
    if (newImportPath !== importPath) {
      changed = true;
      return prefix + newImportPath + suffix;
    }
    
    return match;
  });
  
  // also fix vi.mock
  content = content.replace(/(vi\.mock\(['"])([^'"]+)(['"]\))/g, (match, p1, p2, p3) => {
    const importPath = p2;
    
    if (!importPath.startsWith('.')) return match;
    
    const relToSrcNow = path.relative(srcDir, file).replace(/\\/g, '/');
    let oldRelToSrc = relToSrcNow;
    
    for (const [oPath, nPath] of Object.entries(moves)) {
      if (relToSrcNow === nPath + '.ts' || relToSrcNow === nPath + '.tsx' || relToSrcNow === nPath + '.test.ts' || relToSrcNow === nPath + '.test.tsx' || relToSrcNow === nPath + '.contract.test.ts' || relToSrcNow.startsWith(nPath + '/')) {
        oldRelToSrc = relToSrcNow.replace(nPath, oPath);
        break;
      }
    }
    
    const oldFileDir = path.dirname(path.resolve(srcDir, oldRelToSrc));
    const oldAbsImportPath = path.resolve(oldFileDir, importPath);
    const oldImportRelToSrc = path.relative(srcDir, oldAbsImportPath).replace(/\\/g, '/');
    
    let newImportRelToSrc = oldImportRelToSrc;
    for (const [oPath, nPath] of Object.entries(moves)) {
      if (oldImportRelToSrc === oPath || oldImportRelToSrc.startsWith(oPath + '/')) {
        newImportRelToSrc = oldImportRelToSrc.replace(oPath, nPath);
        break;
      }
    }
    
    const newAbsImportPath = path.resolve(srcDir, newImportRelToSrc);
    const fileDir = path.dirname(file);
    
    let newImportPath = path.relative(fileDir, newAbsImportPath).replace(/\\/g, '/');
    if (!newImportPath.startsWith('.')) {
      newImportPath = './' + newImportPath;
    }
    
    if (newImportPath !== importPath) {
      changed = true;
      return p1 + newImportPath + p3;
    }
    
    return match;
  });

  if (changed) {
    fs.writeFileSync(file, content, 'utf8');
    console.log(`Updated imports in ${path.relative(srcDir, file)}`);
  }
});
