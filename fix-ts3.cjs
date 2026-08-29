const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace(
    "app_language: 'en',\n      app_theme: 'dark',",
    "language: 'en',\n      theme: 'dark',"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
