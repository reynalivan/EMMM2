import * as path from 'path';
import * as fs from 'fs';
import { spawn, spawnSync, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';
import { download as downloadEdgeDriver } from 'edgedriver';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let tauriDriver: ChildProcess;

/** Where `tauri build` drops the binary — shared with `tauri dev` and `cargo build`. */
const BUILT_BINARY = path.resolve(__dirname, 'src-tauri/target/debug/emmm.exe');

/**
 * The suite runs a private copy under its own name. `tauri dev` rebuilds
 * `emmm.exe` in dev mode — which loads from the dev server instead of embedding
 * the frontend — so a dev build started mid-run used to replace the binary the
 * suite was launching, and every spec after that died on "asset not found:
 * index.html". Copying also keeps `taskkill` scoped to this name, so purging
 * ghost processes no longer kills the app the developer is running.
 */
const E2E_BINARY = path.resolve(__dirname, 'src-tauri/target/debug/emmm-e2e.exe');

export const config = {
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./test/specs/**/*.e2e.ts'],
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
  onPrepare: () => {
    const built = spawnSync(
      'pnpm',
      ['tauri', 'build', '--debug', '--no-bundle', '--config', 'src-tauri/tauri.e2e.conf.json'],
      { stdio: 'inherit', shell: true },
    );
    if (built.status !== 0) {
      throw new Error(`tauri build failed with exit code ${built.status}`);
    }
    if (!fs.existsSync(BUILT_BINARY)) {
      throw new Error('tauri build reported success but produced no emmm.exe');
    }
    // Snapshot it under the suite's own name so nothing can swap it mid-run.
    fs.copyFileSync(BUILT_BINARY, E2E_BINARY);
  },
  // ensure we are running `tauri-driver` before the session starts so that wdio can connect to it
  beforeSession: async () => {
    // Purge ghost processes from an earlier run. Scoped to `emmm-e2e.exe` so a
    // developer's own running app is left alone.
    try {
      spawnSync('taskkill', ['/F', '/IM', 'emmm-e2e.exe', '/T'], { shell: true });
      spawnSync('taskkill', ['/F', '/IM', 'msedgedriver.exe', '/T'], { shell: true });
      spawnSync('taskkill', ['/F', '/IM', 'tauri-driver.exe', '/T'], { shell: true });
    } catch {
      // ignore
    }

    const edgeDriverPath = await downloadEdgeDriver();

    tauriDriver = spawn('tauri-driver', ['--native-driver', edgeDriverPath], {
      stdio: [null, process.stdout, process.stderr],
      shell: true,
    });

    // give tauri-driver some time to start up and listen on port 4444
    await new Promise((resolve) => setTimeout(resolve, 20000));
  },
  // Each spec file gets a clean slate: games, objects, collections and trash
  // otherwise accumulate across the run and leak between specs. Safe to wipe
  // because the identifier override above gives this build its own app_data.
  before: async () => {
    const { browser } = await import('@wdio/globals');
    await browser.url('http://tauri.localhost/');
    const result = (await browser.executeAsync((done: (r: unknown) => void) => {
      const core = (
        window as unknown as {
          __TAURI__: { core: { invoke: (cmd: string) => Promise<unknown> } };
        }
      ).__TAURI__.core;
      core.invoke('reset_database').then(
        () => done({ ok: true }),
        (error: unknown) => done({ ok: false, error: String(error) }),
      );
    })) as { ok: boolean; error?: string };

    if (!result.ok) {
      throw new Error(`reset_database failed before spec: ${result.error}`);
    }
  },
  // clean up the `tauri-driver` process we spawned at the start of the session
  afterSession: () => {
    tauriDriver?.kill();
  },
  baseUrl: 'http://tauri.localhost',
};

function onShutdown(fn: () => void) {
  const cleanup = () => {
    try {
      fn();
    } finally {
      process.exit();
    }
  };

  process.on('exit', cleanup);
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
  process.on('SIGHUP', cleanup);
  process.on('SIGBREAK', cleanup);
}

onShutdown(() => {
  tauriDriver?.kill();
});
