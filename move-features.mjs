import fs from 'fs';
import path from 'path';

const srcDir = path.resolve('src');

const moves = {
  // file-watcher
  'features/file-watcher/hooks': 'features/file-watcher/hooks/useFileWatcher',
  'features/file-watcher/pathUtils': 'features/file-watcher/utils/pathUtils',
  'features/file-watcher/reconcileProgress': 'features/file-watcher/utils/reconcileProgress',
  'features/file-watcher/reconcileRefresh': 'features/file-watcher/utils/reconcileRefresh',
  'features/file-watcher/reconcileSelection': 'features/file-watcher/utils/reconcileSelection',
  'features/file-watcher/reconcileToast': 'features/file-watcher/utils/reconcileToast',
  'features/file-watcher/watcherError': 'features/file-watcher/utils/watcherError',
  'features/file-watcher/watcherLifecycle': 'features/file-watcher/utils/watcherLifecycle',
  
  // preview
  'features/preview/keybindingValidator': 'features/preview/utils/keybindingValidator',
  'features/preview/previewPanelUtils': 'features/preview/utils/previewPanelUtils',
  
  // scanner
  'features/scanner/DedupFeature': 'features/scanner/components/DedupFeature',
  'features/scanner/dedupProgress': 'features/scanner/utils/dedupProgress',
  
  // match-wizard
  'features/match-wizard/ImportBatchWizardItemRow': 'features/match-wizard/components/ImportBatchWizardItemRow',
  'features/match-wizard/ObjectClassificationWizardHost': 'features/match-wizard/components/ObjectClassificationWizardHost',
  'features/match-wizard/importBatchDecision': 'features/match-wizard/utils/importBatchDecision',
  
  // onboarding
  'features/onboarding/AutoDetectResult': 'features/onboarding/components/AutoDetectResult',
  'features/onboarding/ManualSetupForm': 'features/onboarding/components/ManualSetupForm',
  'features/onboarding/indexingProgress': 'features/onboarding/utils/indexingProgress',
  'features/onboarding/useOnboardingDiskProgress': 'features/onboarding/hooks/useOnboardingDiskProgress',
  'features/onboarding/welcome': 'features/onboarding/components/welcome',
  
  // settings
  'features/settings/tabs/hotkeyConflicts': 'features/settings/utils/hotkeyConflicts',
  'features/settings/theme/useCustomThemes': 'features/settings/hooks/useCustomThemes',
  'features/settings/theme/useThemeRuntime': 'features/settings/hooks/useThemeRuntime',
  'features/settings/tabs': 'features/settings/components/tabs',
  'features/settings/theme': 'features/settings/components/theme'
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
    // If it's a directory
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
  } else {
    console.log(`WARNING: Could not find ${oldPath}`);
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
  
  if (changed) {
    fs.writeFileSync(file, content, 'utf8');
    console.log(`Updated imports in ${path.relative(srcDir, file)}`);
  }
});
