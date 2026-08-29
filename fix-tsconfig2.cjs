const fs = require('fs');
let tsconfig = fs.readFileSync('tsconfig.json', 'utf8');
tsconfig = tsconfig.replace('["src/*"]', '["./src/*"]');
fs.writeFileSync('tsconfig.json', tsconfig, 'utf8');
