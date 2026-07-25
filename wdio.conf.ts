import * as path from 'path';
import * as fs from 'fs';
import { spawn, spawnSync, type ChildProcess } from 'child_process';
import { fileURLToPath } from 'url';

// @ts-expect-error This package does not have TypeScript definitions
import edgeDriver from 'msedgedriver';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

let tauriDriver: ChildProcess;

function resolveNativeEdgeDriverPath(): string {
  if (fs.existsSync(edgeDriver.path)) {
    return edgeDriver.path;
  }

  const lookupCommand = process.platform === 'win32' ? 'where' : 'which';
  const lookup = spawnSync(lookupCommand, ['msedgedriver'], { encoding: 'utf8' });
  const candidate = lookup.stdout
    .split(/\r?\n/)
    .map((entry) => entry.trim())
    .find((entry) => entry.length > 0);

  if (candidate && fs.existsSync(candidate)) {
    return candidate;
  }

  throw new Error(
    `msedgedriver binary not found. Expected ${edgeDriver.path}; install or approve the msedgedriver build before running WDIO E2E.`,
  );
}

export const config = {
  hostname: '127.0.0.1',
  port: 4444,
  specs: ['./test/specs/**/*.e2e.ts'],
  maxInstances: 1,
  capabilities: [
    {
      browserName: 'webview2',
      'tauri:options': {
        application: path.resolve(__dirname, 'src-tauri/target/debug/emmm.exe'),
      },
    },
  ],
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },
  // ensure the rust project is built since we expect this binary to exist for the webdriver sessions
  // we use debug build because devtools (required for webdriver) is enabled by default in debug
  onPrepare: () => {
    const binPath = path.resolve(__dirname, 'src-tauri/target/debug/emmm.exe');
    if (fs.existsSync(binPath)) {
      console.log('Binary already exists, skipping build...');
      return;
    }
    // build the app using tauri-cli to ensure frontend is bundled and production protocol is used
    // --debug flag ensures devtools are still enabled for webdriver
    spawnSync('pnpm', ['tauri', 'build', '--debug'], { stdio: 'inherit', shell: true });
  },
  // ensure we are running `tauri-driver` before the session starts so that wdio can connect to it
  beforeSession: async () => {
    // Purge any ghost processes before starting
    try {
      spawnSync('taskkill', ['/F', '/IM', 'emmm.exe', '/T'], { shell: true });
      spawnSync('taskkill', ['/F', '/IM', 'msedgedriver.exe', '/T'], { shell: true });
      spawnSync('taskkill', ['/F', '/IM', 'tauri-driver.exe', '/T'], { shell: true });
    } catch {
      // ignore
    }

    tauriDriver = spawn('tauri-driver', ['--native-driver', resolveNativeEdgeDriverPath()], {
      stdio: [null, process.stdout, process.stderr],
      shell: true,
    });

    // give tauri-driver some time to start up and listen on port 4444
    await new Promise((resolve) => setTimeout(resolve, 20000));
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
