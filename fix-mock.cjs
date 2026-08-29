const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace(
    "openInExplorer: vi.fn(),",
    "openInExplorer: vi.fn(),\n    getSettings: vi.fn(),\n    saveSettings: vi.fn(),"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
