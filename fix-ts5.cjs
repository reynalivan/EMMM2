const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace(
    "    });\n    vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce(undefined as any);",
    "    } as any);\n    vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce(undefined as any);"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
