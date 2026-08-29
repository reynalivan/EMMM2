const fs = require('fs');

const logPath = 'docs/history/202608295078-src-architecture-migration.md';
let content = fs.readFileSync(logPath, 'utf8');

if (!content.includes('Moved remaining root files in `src/features/workspace-runtime/`')) {
  // Replace the previous Goal with an updated one, and add the final fix to the Changes list
  content = content.replace(
    '- Updated test setup to resolve SVG mock issues with `motion/react`.',
    '- Updated test setup to resolve SVG mock issues with `motion/react`.\n- Moved remaining root files in `src/features/workspace-runtime/` into `hooks/` and `utils/` subfolders.'
  );

  content = content.replace(
    '- All internal feature tests (Fixed relative import paths via regex)',
    '- All internal feature tests (Fixed relative import paths via regex)\n- `src/features/workspace-runtime/` (Fixed final loose files root)'
  );

  fs.writeFileSync(logPath, content);
}
console.log('Post-log updated');
