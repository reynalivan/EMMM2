import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { after, test } from 'node:test';
import { pathToFileURL } from 'node:url';

const require = createRequire(import.meta.url);
const loader = require.resolve('tsx', { paths: [require.resolve('@wdio/cli')] });
const temporary = mkdtempSync(path.join(tmpdir(), 'emmm-startup-policy-tests-'));
const profile = path.join(temporary, 'emmm-startup-policy-' + 'a'.repeat(32));
mkdirSync(profile);
after(() => {
  rmdirSync(profile);
  rmdirSync(temporary);
});

function loadConfig(overrides, code) {
  return spawnSync(
    process.execPath,
    ['--import', pathToFileURL(loader).href, '--input-type=module', '-e', code],
    {
      env: {
        ...process.env,
        GITHUB_ACTIONS: 'true',
        RUNNER_ENVIRONMENT: 'github-hosted',
        RUNNER_TEMP: temporary,
        EMMM_E2E_POLICY_PROFILE: profile,
        ...overrides,
      },
      encoding: 'utf8',
      timeout: 10000,
      windowsHide: true,
    },
  );
}

for (const [name, overrides] of [
  ['local execution', { GITHUB_ACTIONS: 'false' }],
  ['self-hosted execution', { RUNNER_ENVIRONMENT: 'self-hosted' }],
  ['foreign profile parent', { RUNNER_TEMP: path.dirname(temporary) }],
  ['unowned profile name', { EMMM_E2E_POLICY_PROFILE: temporary }],
]) {
  test(`config rejects ${name} before any harness hooks run`, () => {
    const result = loadConfig(
      overrides,
      "await import('./wdio.startup-policy-experiment.conf.ts');",
    );
    assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0);
    assert.match(
      result.stderr,
      /Startup policy experiment requires its fresh GitHub-hosted profile/,
    );
  });
}

test('experiment config preserves fresh native hooks and restricts the run to startup', () => {
  const result = loadConfig(
    {},
    `
    import assert from 'node:assert/strict';
    const { config } = await import('./wdio.startup-policy-experiment.conf.ts');
    assert.deepEqual(config.specs, ['./tests/e2e/specs/app-bootstrap.e2e.ts']);
    assert.equal(config.capabilities.length, 1);
    const options = config.capabilities[0]['tauri:options'];
    assert.equal(options.webviewOptions.userDataFolder, process.env.EMMM_E2E_POLICY_PROFILE);
    assert.ok(options.application.endsWith('emmm-e2e.exe'));
    assert.equal(typeof config.onPrepare, 'function');
    assert.equal(typeof config.beforeSession, 'function');
    assert.equal(typeof config.afterSession, 'function');
  `,
  );
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
});

test(
  'policy script refuses local execution before touching registry',
  { skip: process.platform !== 'win32' },
  () => {
    const result = spawnSync(
      'pwsh',
      ['-NoProfile', '-File', 'scripts/native-startup-policy-experiment.ps1'],
      {
        env: { ...process.env, GITHUB_ACTIONS: 'false', RUNNER_ENVIRONMENT: 'self-hosted' },
        encoding: 'utf8',
        timeout: 10000,
        windowsHide: true,
      },
    );
    assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0);
    assert.match(
      result.stderr,
      /This experiment may run only on an ephemeral GitHub-hosted Windows runner/,
    );
  },
);
