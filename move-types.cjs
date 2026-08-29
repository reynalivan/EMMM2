const fs = require('fs');
const path = require('path');

const srcDir = path.join(process.cwd(), 'src');
const mappings = {
    'collection.ts': 'entities/collection/model/collection.ts',
    'dashboard.ts': 'pages/dashboard/model/dashboard.ts',
    'game.ts': 'entities/game/model/game.ts',
    'mod.ts': 'entities/mod/model/mod.ts',
    'object.ts': 'entities/game-object/model/object.ts',
    'scanner.ts': 'entities/workspace/model/scanner.ts',
    'settings.ts': 'pages/settings/model/settings.ts',
    'task.ts': 'entities/task/model/task.ts',
    'workspace.ts': 'entities/workspace/model/workspace.ts'
};

const commonDir = path.join(srcDir, 'entities', 'common');
if (fs.existsSync(commonDir)) {
    for (const [file, target] of Object.entries(mappings)) {
        const sourcePath = path.join(commonDir, file);
        if (fs.existsSync(sourcePath)) {
            const targetPath = path.join(srcDir, target);
            fs.mkdirSync(path.dirname(targetPath), { recursive: true });
            fs.renameSync(sourcePath, targetPath);
            console.log('Moved ' + file + ' to ' + target);
        }
    }
    try {
        fs.rmdirSync(commonDir);
        console.log('Deleted src/entities/common');
    } catch (e) {
        console.log('Could not delete src/entities/common, might not be empty');
    }
}
