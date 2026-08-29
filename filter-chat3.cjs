const fs = require('fs');
const lines = fs.readFileSync('.docs/history/continuechat3.md', 'utf8').split('\n');
const recentLines = lines.slice(-400);

let output = '';
for (const line of recentLines) {
    if (line.startsWith('> **Tool') || line.startsWith('> [Tool result') || line.startsWith('*Tokens') || line.startsWith('*** Begin Patch') || line.startsWith('---')) {
        continue;
    }
    if (line.trim().length > 0) {
        output += line + '\n';
    }
}
console.log(output);
