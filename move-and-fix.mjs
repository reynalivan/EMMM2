import fs from 'fs';
import path from 'path';

const srcDir = path.resolve('src');

const moves = {
  'lib/bindings': 'core/tauri/bindings',
  'lib/bindings.gen': 'core/tauri/bindings.gen',
  'lib/logger': 'core/lib/logger',
  'lib/appError': 'core/lib/appError',
  'lib/queryClient': 'core/lib/queryClient',
  'lib/i18n': 'core/lib/i18n',
  'lib/disabledPrefix': 'core/lib/disabledPrefix',
  'lib/dropClassification': 'core/lib/dropClassification',
  'lib/committedMutationWarning': 'core/lib/committedMutationWarning',
  'lib/runtimeEffects': 'core/lib/runtimeEffects',
  'lib/runtimeLabels': 'core/lib/runtimeLabels',
  'lib/themeOptions': 'core/lib/themeOptions',
  'lib/utils': 'core/lib/utils',
  'lib/pathKey': 'core/lib/pathKey',
  'lib/services/objectService': 'core/services/objectService',
  
  'components/ui': 'shared/components/ui',
  'components/layout': 'shared/components/layout',
  'utils': 'shared/utils',
  
  'hooks/useResponsive': 'shared/hooks/useResponsive',
  'hooks/useDragAutoScroll': 'shared/hooks/useDragAutoScroll',
  'hooks/useResolvedTheme': 'shared/hooks/useResolvedTheme',
  'hooks/useFileDrop': 'shared/hooks/useFileDrop',
  'hooks/useDialogSync': 'shared/hooks/useDialogSync',
  'hooks/usePrefersReducedMotion': 'shared/hooks/usePrefersReducedMotion',
  'hooks/useRangeSelection': 'shared/hooks/useRangeSelection',
  'hooks/bulkToastMessages': 'shared/hooks/bulkToastMessages',
  'hooks/fileInUseRetry': 'shared/hooks/fileInUseRetry',
  
  'components/modals/MoveToObjectDialog': 'features/object-list/modals/MoveToObjectDialog',
  'components/modals/ActiveModContextDialog': 'features/mod-runtime/modals/ActiveModContextDialog',
  'components/modals/BulkTagModal': 'features/mod-runtime/modals/BulkTagModal',
  'components/dialogs/FileInUseDialog': 'features/file-watcher/dialogs/FileInUseDialog',
  
  'hooks/useActiveGame': 'features/dashboard/hooks/useActiveGame',
  'hooks/useThumbnail': 'features/dashboard/hooks/useThumbnail',
  'hooks/useBulkModMutations': 'features/mod-runtime/hooks/useBulkModMutations',
  'hooks/useFolderCoreMutations': 'features/folder-grid/hooks/useFolderCoreMutations',
  'hooks/useFolderMutations': 'features/folder-grid/hooks/useFolderMutations',
  'hooks/useModContextMenuItems': 'features/folder-grid/hooks/useModContextMenuItems',
  'hooks/folderMutationPayloads': 'features/folder-grid/hooks/folderMutationPayloads',
  'hooks/folderCache': 'features/folder-grid/hooks/folderCache',
  'hooks/useObjectMutations': 'features/object-list/hooks/useObjectMutations',
  'hooks/useObjectQueries': 'features/object-list/hooks/useObjectQueries',
  'hooks/objectQueryCache': 'features/object-list/hooks/objectQueryCache',
  'hooks/collectionReferenceImpact': 'features/collections/hooks/collectionReferenceImpact',
  'hooks/useSettings': 'features/settings/hooks/useSettings',
  'hooks/settingsQuery': 'features/settings/hooks/settingsQuery',
  
  'testing': 'tests/testing',
  'setupTests': 'tests/setupTests'
};

const exactFileMoves = {
  'App.tsx': 'app/App.tsx',
  'main.tsx': 'app/main.tsx',
  'vite-env.d.ts': 'app/vite-env.d.ts',
  'App.test.tsx': 'app/App.test.tsx',
  'App.css': 'app/App.css'
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
    ensureDir(newAbs);
    fs.renameSync(oldAbs, newAbs);
    console.log(`Moved dir ${oldPath} to ${newPath}`);
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
  } else {
    console.log(`WARNING: Could not find ${oldPath}`);
  }
}

for (let [oldPath, newPath] of Object.entries(exactFileMoves)) {
  const oldAbs = path.resolve(srcDir, oldPath);
  const newAbs = path.resolve(srcDir, newPath);
  if (fs.existsSync(oldAbs)) {
    ensureDir(newAbs);
    fs.renameSync(oldAbs, newAbs);
    console.log(`Moved ${oldPath} to ${newPath}`);
  }
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
      if (relToSrcNow === nPath + '.ts' || relToSrcNow === nPath + '.tsx' || relToSrcNow === nPath + '.test.ts' || relToSrcNow === nPath + '.test.tsx' || relToSrcNow.startsWith(nPath + '/')) {
        oldRelToSrc = relToSrcNow.replace(nPath, oPath);
        break;
      }
    }
    for (const [oPath, nPath] of Object.entries(exactFileMoves)) {
      if (relToSrcNow === nPath) {
        oldRelToSrc = oPath;
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
    for (const [oPath, nPath] of Object.entries(exactFileMoves)) {
      if (oldImportRelToSrc + '.tsx' === oPath || oldImportRelToSrc + '.ts' === oPath) {
        newImportRelToSrc = nPath.replace(/\.tsx?$/, '');
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
  
  if (content.includes("import './App.css'") || content.includes("import '../App.css'")) {
     const newC = content.replace(/import\s+['"]\.\.?\/App\.css['"]/g, "import './App.css'");
     if (newC !== content) {
         content = newC;
         changed = true;
     }
  }
  
  if (changed) {
    fs.writeFileSync(file, content, 'utf8');
    console.log(`Updated imports in ${path.relative(srcDir, file)}`);
  }
});

// Cleanup empty directories
console.log('Cleaning up empty directories...');
const dirsToClean = ['components', 'hooks', 'lib', 'utils', 'testing'].map(d => path.resolve(srcDir, d));
for (const d of dirsToClean) {
  if (fs.existsSync(d)) {
    fs.rmSync(d, { recursive: true, force: true });
  }
}
