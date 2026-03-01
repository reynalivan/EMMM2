const fs = require('fs');

const filesToPatch = [
  'src/hooks/useDedup.test.ts',
  'src/hooks/useFileDrop.test.tsx',
  'src/hooks/useFolderNavigation.test.ts',
  'src/hooks/useFolders.test.tsx',
  'src/hooks/useFolders.test.ts', // also check .ts
  'src/hooks/useObjects.test.tsx',
  'src/hooks/useThumbnail.test.tsx',
  'src/stores/useToastStore.test.ts',
  'src/services/dedupService.test.ts',
];

filesToPatch.forEach((f) => {
  if (!fs.existsSync(f)) return;
  const before = fs.readFileSync(f, 'utf8');
  let after = before;

  // 3 levels up -> 2 levels up
  after = after.replace(/from '\.\.\/\.\.\/\.\.\/([^']+)'/g, "from '../../$1'");

  // 2 levels up -> 1 level up
  after = after.replace(/from '\.\.\/\.\.\/([^']+)'/g, "from '../$1'");

  // 1 level up -> current folder. BUT only if it is pointing to a peer file
  // Like '../useDedup' -> './useDedup'
  // But NOT '../stores/useToastStore' -> './stores/useToastStore' because stores isn't here.
  // The rule is: if there are no slashes after the initial '../', it's a peer.
  after = after.replace(/from '\.\.\/([^/']+)'/g, "from './$1'");

  if (before !== after) {
    fs.writeFileSync(f, after, 'utf8');
    console.log('Fixed', f);
  }
});

const fgTest = 'src/features/foldergrid/hooks/useFolderGrid.test.ts';
if (fs.existsSync(fgTest)) {
  let c = fs.readFileSync(fgTest, 'utf8');
  c = c.replace(/from '\.\.\/\.\.\/\.\.\/test-utils'/g, "from '../../../test-utils'");
  c = c.replace(/from '\.\.\/\.\.\/stores\/useAppStore'/g, "from '../../stores/useAppStore'");
  c = c.replace(/from '\.\.\/useFolderGrid'/g, "from './useFolderGrid'");
  fs.writeFileSync(fgTest, c, 'utf8');
  console.log('Fixed foldergrid test');
}
