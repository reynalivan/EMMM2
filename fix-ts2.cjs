const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace(
    "game_exe: 'game.exe'",
    "game_exe: 'game.exe',\n        loader_exe: null,\n        launch_args: null"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
