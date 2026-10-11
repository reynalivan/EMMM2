import assert from 'node:assert/strict';
import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';

let sessionRequests = 0;
const server = http.createServer((request, response) => {
  if (request.method === 'POST' && request.url === '/session') sessionRequests += 1;
  response.writeHead(500, { 'Content-Type': 'application/json' });
  response.end(
    JSON.stringify({
      value: {
        error: 'session not created',
        message: 'Owned interlock test server rejects all sessions',
        stacktrace: '',
      },
    }),
  );
});
await new Promise((resolve, reject) => {
  server.once('error', reject);
  server.listen(4444, '127.0.0.1', resolve);
});

let output = '';
let timedOut = false;
const child = spawn(
  process.execPath,
  [
    path.resolve('node_modules/@wdio/cli/bin/wdio.js'),
    'run',
    'wdio.conf.ts',
    '--spec',
    'tests/e2e/specs/startup-reopen.e2e.ts',
  ],
  {
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  },
);
const timeout = setTimeout(() => {
  timedOut = true;
  if (child.pid && child.exitCode === null) {
    const stopped = spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], {
      encoding: 'utf8',
      windowsHide: true,
    });
    output += `\nOwned test timeout cleanup: ${stopped.status}\n`;
  }
}, 45_000);
child.stdout.on('data', (chunk) => {
  output += chunk.toString();
});
child.stderr.on('data', (chunk) => {
  output += chunk.toString();
});
let exitCode;
try {
  exitCode = await new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('close', resolve);
  });
} finally {
  clearTimeout(timeout);
  server.closeAllConnections();
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
}
const directory = path.resolve('.audit-output/gimi-final-stabilization-20261010');
fs.mkdirSync(directory, { recursive: true });
fs.writeFileSync(path.join(directory, `native-interlock-${process.pid}.log`), output);
console.info(JSON.stringify({ exitCode, timedOut, sessionRequests }));
assert.equal(timedOut, false, 'Native prepare failure must terminate promptly');
assert.notEqual(exitCode, 0, 'Occupied driver port must reject this run');
assert.match(output, /E2E driver port 4444 is unavailable/);
assert.equal(
  sessionRequests,
  0,
  'Rejected preparation must never POST a session to another driver',
);
