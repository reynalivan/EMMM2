const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace("game_type: 'GIMI'", "game_type: 0");
code = code.replace("vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce()", "vi.mocked(modInboxCommands.saveSettings).mockResolvedValueOnce(undefined as any)");

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
