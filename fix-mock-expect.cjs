const fs = require('fs');
let code = fs.readFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', 'utf8');

code = code.replace(
    "title: 'actions.choose_location'",
    "title: 'Choose Location'"
);

fs.writeFileSync('src/pages/mod-inbox/ModInboxPage.test.tsx', code);
