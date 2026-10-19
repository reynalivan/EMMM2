import * as path from 'path';
import * as fs from 'fs';
import { spawn, spawnSync, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { createHash } from 'node:crypto';
import { download as downloadEdgeDriver } from 'edgedriver';
import {
  abortNativeRun,
  assertDriverPortsAvailable,
  assertCurrentBuild,
  copyE2EBinarySnapshot,
  stopOwnedDriver,
  waitForOwnedDriver,
} from './tests/e2e/support/driverLifecycle.js';
import { cleanupScheduledMockGames } from './tests/e2e/support/fixtures.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let tauriDriver: ChildProcess | undefined;
const DRIVER_PORT = 4444;
const NATIVE_DRIVER_PORT = 4445;

function cleanupOwnedDriver(): boolean {
  const owned = tauriDriver;
  tauriDriver = undefined;
  if (!owned?.pid || owned.exitCode !== null || owned.signalCode !== null) return false;
  stopOwnedDriver(owned);
  return true;
}

/**
 * The suite runs a private copy under its own name. `tauri dev` rebuilds
 * `emmm.exe` in dev mode — which loads from the dev server instead of embedding
 * the frontend — so a dev build started mid-run used to replace the binary the
 * suite was launching, and every spec after that died on "asset not found:
 * index.html".
 */
export function createNativeE2EConfig(
  profile: 'fixture' | 'readonly-audit' = 'fixture',
  watcherVerification = false,
  buildMode: 'debug' | 'release' = 'debug',
) {
  if (watcherVerification && profile === 'readonly-audit') {
    throw new Error('Watcher verification requires the fixture profile');
  }
  const audit = profile === 'readonly-audit';
  const builtBinary = path.resolve(__dirname, 'src-tauri', 'target', buildMode, 'emmm.exe');
  const E2E_IDENTIFIER = audit
    ? 'com.reynalivan.emmm.e2e.readonly-audit'
    : 'com.reynalivan.emmm.e2e';
  const e2eConfig = audit
    ? 'src-tauri/tauri.readonly-audit.conf.json'
    : 'src-tauri/tauri.e2e.conf.json';
  const E2E_BINARY = path.resolve(
    __dirname,
    'src-tauri',
    'target',
    buildMode,
    audit ? 'emmm-readonly-audit.exe' : 'emmm-e2e.exe',
  );
  return {
    hostname: '127.0.0.1',
    port: DRIVER_PORT,
    logLevel: 'warn',
    outputDir: path.resolve(__dirname, '.audit-output/native-e2e'),
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
    // Tauri's existing devtools Cargo feature also enables release WebView2 testing.
    onPrepare: async () => {
      try {
        delete process.env.EMMM_E2E_BUILD_SHA256;
        process.env.EMMM_E2E_EXPECTED_IDENTIFIER = E2E_IDENTIFIER;
        process.env.EMMM_E2E_BUILD_MODE = buildMode;
        // A whitespace override survives Windows env handling and trims to no collector.
        process.env.VITE_GRAFANA_FARO_URL = ' ';
        await assertDriverPortsAvailable([DRIVER_PORT, NATIVE_DRIVER_PORT]);
        const overrides: unknown = JSON.parse(
          fs.readFileSync(path.resolve(__dirname, e2eConfig), 'utf8'),
        );
        if (
          typeof overrides !== 'object' ||
          overrides === null ||
          !('identifier' in overrides) ||
          overrides.identifier !== E2E_IDENTIFIER
        ) {
          throw new Error('Refusing E2E build without its isolated app identifier');
        }
        const buildArgs = [
          path.resolve(__dirname, 'scripts', 'tauri.mjs'),
          'build',
          ...(buildMode === 'debug' ? ['--debug'] : []),
          '--no-bundle',
          '--config',
          e2eConfig,
        ];
        if (watcherVerification) buildArgs.push('--features', 'watcher-verification');
        if (buildMode === 'release' && audit) buildArgs.push('--features', 'readonly-audit');
        const built = spawnSync(process.execPath, buildArgs, {
          cwd: __dirname,
          // WDIO sets NODE_ENV=test; release must embed React's production runtime.
          env: buildMode === 'release' ? { ...process.env, NODE_ENV: 'production' } : process.env,
          stdio: 'inherit',
          windowsHide: true,
        });
        if (built.status !== 0) {
          throw new Error(`tauri build failed with exit code ${built.status}`);
        }
        if (!fs.existsSync(builtBinary)) {
          throw new Error('tauri build reported success but produced no emmm.exe');
        }
        // Snapshot it under the suite's own name so nothing can swap it mid-run.
        await copyE2EBinarySnapshot(builtBinary, E2E_BINARY, fs.copyFileSync);
        process.env.EMMM_E2E_BUILD_SHA256 = createHash('sha256')
          .update(fs.readFileSync(E2E_BINARY))
          .digest('hex');
      } catch (error) {
        abortNativeRun(error, cleanupOwnedDriver);
      }
    },
    // ensure we are running `tauri-driver` before the session starts so that wdio can connect to it
    beforeSession: async () => {
      try {
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
        await waitForOwnedDriver(tauriDriver, `http://127.0.0.1:${DRIVER_PORT}/status`);
      } catch (error) {
        abortNativeRun(error, cleanupOwnedDriver);
      }
    },
    // Each spec file gets a clean slate: games, objects, collections and trash
    // otherwise accumulate across the run and leak between specs. Safe to wipe
    // because the identifier override above gives this build its own app_data.
    before: async () => {
      try {
        const { browser } = await import('@wdio/globals');
        await browser.url('http://tauri.localhost/');
        const result = (await browser.executeAsync(
          (expectedIdentifier: string, done: (r: unknown) => void) => {
            const core = (
              window as unknown as {
                __TAURI__: {
                  core: { invoke: (cmd: string) => Promise<unknown> };
                };
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
      } catch (error) {
        abortNativeRun(error, cleanupOwnedDriver);
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
}

export const config = createNativeE2EConfig();

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
