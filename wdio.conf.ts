import * as path from 'path';
import * as fs from 'fs';
import { spawn, spawnSync, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { createHash } from 'node:crypto';
import { download as downloadEdgeDriver } from 'edgedriver';
import {
  assertDriverPortsAvailable,
  assertCurrentBuild,
  stopOwnedDriver,
  waitForOwnedDriver,
} from './tests/e2e/support/driverLifecycle.js';
import { cleanupScheduledMockGames } from './tests/e2e/support/fixtures.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let tauriDriver: ChildProcess | undefined;
const E2E_IDENTIFIER = 'com.reynalivan.emmm.e2e';
const DRIVER_PORT = 4444;
const NATIVE_DRIVER_PORT = 4445;

function cleanupOwnedDriver(): boolean {
  const owned = tauriDriver;
  tauriDriver = undefined;
  if (!owned?.pid || owned.exitCode !== null || owned.signalCode !== null) return false;
  stopOwnedDriver(owned);
  return true;
}

/** Where `tauri build` drops the binary — shared with `tauri dev` and `cargo build`. */
const BUILT_BINARY = path.resolve(__dirname, 'src-tauri/target/debug/emmm.exe');

/**
 * The suite runs a private copy under its own name. `tauri dev` rebuilds
 * `emmm.exe` in dev mode — which loads from the dev server instead of embedding
 * the frontend — so a dev build started mid-run used to replace the binary the
 * suite was launching, and every spec after that died on "asset not found:
 * index.html".
 */
const E2E_BINARY = path.resolve(__dirname, 'src-tauri/target/debug/emmm-e2e.exe');

export const config = {
  hostname: '127.0.0.1',
  port: DRIVER_PORT,
  logLevel: 'warn',
  specs: ['./tests/e2e/specs/**/*.e2e.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'webview2',
      'tauri:options': {
        application: E2E_BINARY,
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  // Always rebuild — never reuse whatever binary happens to be lying around.
  // `tauri.e2e.conf.json` overrides the bundle identifier, which is what gives
  // this build its own `app_data_dir`. A binary left over from a plain
  // `pnpm tauri build --debug` carries the PRODUCTION identifier, so reusing it
  // would point the suite (and its `reset_database`) at the real library.
  // Debug build: devtools stay enabled, which tauri-driver requires.
  onPrepare: async () => {
    delete process.env.EMMM_E2E_BUILD_SHA256;
    await assertDriverPortsAvailable([DRIVER_PORT, NATIVE_DRIVER_PORT]);
    const overrides: unknown = JSON.parse(
      fs.readFileSync(path.resolve(__dirname, 'src-tauri/tauri.e2e.conf.json'), 'utf8'),
    );
    if (
      typeof overrides !== 'object' ||
      overrides === null ||
      !('identifier' in overrides) ||
      overrides.identifier !== E2E_IDENTIFIER
    ) {
      throw new Error('Refusing E2E build without its isolated app identifier');
    }
    const built = spawnSync(
      'pnpm',
      ['tauri', 'build', '--debug', '--no-bundle', '--config', 'src-tauri/tauri.e2e.conf.json'],
      { stdio: 'inherit', shell: true, windowsHide: true },
    );
    if (built.status !== 0) {
      throw new Error(`tauri build failed with exit code ${built.status}`);
    }
    if (!fs.existsSync(BUILT_BINARY)) {
      throw new Error('tauri build reported success but produced no emmm.exe');
    }
    // Snapshot it under the suite's own name so nothing can swap it mid-run.
    fs.copyFileSync(BUILT_BINARY, E2E_BINARY);
    process.env.EMMM_E2E_BUILD_SHA256 = createHash('sha256')
      .update(fs.readFileSync(E2E_BINARY))
      .digest('hex');
  },
  // ensure we are running `tauri-driver` before the session starts so that wdio can connect to it
  beforeSession: async () => {
    assertCurrentBuild(
      process.env.EMMM_E2E_BUILD_SHA256,
      createHash('sha256').update(fs.readFileSync(E2E_BINARY)).digest('hex'),
    );
    await assertDriverPortsAvailable([DRIVER_PORT, NATIVE_DRIVER_PORT]);
    console.info('Checking Microsoft Edge driver; missing matching driver may be downloaded.');
    const edgeDriverPath = await downloadEdgeDriver();

    tauriDriver = spawn(
      'tauri-driver',
      [
        '--port',
        String(DRIVER_PORT),
        '--native-port',
        String(NATIVE_DRIVER_PORT),
        '--native-driver',
        edgeDriverPath,
      ],
      {
        stdio: [null, process.stdout, process.stderr],
        windowsHide: true,
      },
    );
    try {
      await waitForOwnedDriver(tauriDriver, `http://127.0.0.1:${DRIVER_PORT}/status`);
    } catch (error) {
      cleanupOwnedDriver();
      throw error;
    }
  },
  // Each spec file gets a clean slate: games, objects, collections and trash
  // otherwise accumulate across the run and leak between specs. Safe to wipe
  // because the identifier override above gives this build its own app_data.
  before: async () => {
    const { browser } = await import('@wdio/globals');
    await browser.url('http://tauri.localhost/');
    const result = (await browser.executeAsync(
      (expectedIdentifier: string, done: (r: unknown) => void) => {
        const core = (
          window as unknown as {
            __TAURI__: { core: { invoke: (cmd: string) => Promise<unknown> } };
          }
        ).__TAURI__.core;
        void core
          .invoke('plugin:app|identifier')
          .then((identifier) => {
            if (identifier !== expectedIdentifier)
              throw new Error('Refusing database reset outside the E2E application');
            return core.invoke('reset_database');
          })
          .then(
            () => done({ ok: true }),
            (error: unknown) =>
              done({
                ok: false,
                error: error instanceof Error ? error.message : JSON.stringify(error),
              }),
          );
      },
      E2E_IDENTIFIER,
    )) as { ok: boolean; error?: string };

    if (!result.ok) {
      throw new Error(`reset_database failed before spec: ${result.error}`);
    }
  },
  // clean up the `tauri-driver` process we spawned at the start of the session
  afterSession: async () => {
    if (cleanupOwnedDriver()) {
      const removed = await cleanupScheduledMockGames();
      console.info(`Owned native process tree stopped; removed ${removed} scheduled fixtures.`);
    } else {
      console.warn('Native shutdown could not be verified; scheduled E2E fixtures retained.');
    }
  },
  baseUrl: 'http://tauri.localhost',
};

process.once('exit', () => {
  try {
    cleanupOwnedDriver();
  } catch (error) {
    console.error('E2E driver cleanup failed:', error);
  }
});
for (const [signal, exitCode] of [
  ['SIGINT', 130],
  ['SIGTERM', 143],
  ['SIGHUP', 129],
  ['SIGBREAK', 149],
] as const) {
  process.once(signal, () => {
    try {
      cleanupOwnedDriver();
    } catch (error) {
      console.error('E2E driver cleanup failed:', error);
    }
    process.exit(exitCode);
  });
}
