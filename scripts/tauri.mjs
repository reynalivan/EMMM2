import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { constants as fsConstants } from 'node:fs';
import {
  access,
  mkdir,
  open,
  readFile,
  rename,
  rm,
  stat,
  unlink,
  writeFile,
} from 'node:fs/promises';
import { homedir, tmpdir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installDevCatalogPack } from './install-dev-catalog-pack.mjs';

const projectRoot = fileURLToPath(new URL('..', import.meta.url));
const tauriRoot = join(projectRoot, 'src-tauri');
const manifestPath = join(tauriRoot, 'vcpkg.json');
const localVcpkgRoot = join(tauriRoot, '.vcpkg');
const triplet = 'x64-windows-static-md';
const installedTripletRoot = join(localVcpkgRoot, 'installed', triplet);
const stampPath = join(localVcpkgRoot, 'emmm-native-dependencies.json');
const cargoCacheStampPath = join(localVcpkgRoot, 'emmm-cargo-native-cache.json');
const localSigningKeyPath = join(homedir(), '.tauri', 'emmm.key');
const lockId = createHash('sha1').update(projectRoot).digest('hex').slice(0, 12);
const lockPath = join(tmpdir(), `emmm-vcpkg-${lockId}.lock`);
const lockWaitMs = 250;
const lockTimeoutMs = 30 * 60 * 1000;
const requiredArtifacts = [
  join('include', 'archive.h'),
  join('lib', 'archive.lib'),
  join('lib', 'zstd.lib'),
  join('lib', 'lzma.lib'),
  join('lib', 'lz4.lib'),
  join('lib', 'libssl.lib'),
  join('lib', 'libcrypto.lib'),
  join('lib', 'bz2.lib'),
  join('lib', 'zs.lib'),
];
const observabilityEnvironmentNames = new Set([
  'EMMM_GRAFANA_OTLP_METRICS_ENDPOINT',
  'EMMM_GRAFANA_OTLP_AUTHORIZATION',
  'VITE_GRAFANA_FARO_URL',
  'VITE_GRAFANA_FARO_API_KEY',
  'VITE_GRAFANA_FARO_TRACING_ENABLED',
  'VITE_APP_VERSION',
]);

function loadProjectEnvironment() {
  const originalEnvironment = new Map(Object.entries(process.env));
  try {
    process.loadEnvFile(join(projectRoot, '.env'));
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) return;
    throw error;
  }

  for (const [name, value] of originalEnvironment) {
    if (!observabilityEnvironmentNames.has(name)) process.env[name] = value;
  }
  for (const name of Object.keys(process.env)) {
    if (!observabilityEnvironmentNames.has(name) && !originalEnvironment.has(name)) {
      delete process.env[name];
    }
  }
}

async function loadLocalSigningKey() {
  if (process.env.TAURI_SIGNING_PRIVATE_KEY?.trim()) return;

  try {
    const signingKey = await readFile(localSigningKeyPath, 'utf8');
    if (signingKey.trim().length === 0) return;

    process.env.TAURI_SIGNING_PRIVATE_KEY = signingKey;
    console.log(`Using local Tauri updater signing key from ${localSigningKeyPath}`);
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) return;
    throw new Error(`Unable to read local Tauri updater signing key: ${localSigningKeyPath}`);
  }
}

function hasErrorCode(error, code) {
  return typeof error === 'object' && error !== null && 'code' in error && error.code === code;
}

function isProcessAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return hasErrorCode(error, 'EPERM');
  }
}

async function pathExists(path) {
  try {
    await access(path, fsConstants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function isNonEmptyFile(path) {
  try {
    return (await stat(path)).size > 0;
  } catch {
    return false;
  }
}

async function readLockOwnerPid() {
  try {
    const value = await readFile(lockPath, 'utf8');
    const pid = Number.parseInt(value, 10);
    return Number.isInteger(pid) && pid > 0 ? pid : null;
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) return null;
    throw error;
  }
}

async function acquirePreparationLock() {
  const deadline = Date.now() + lockTimeoutMs;

  while (Date.now() < deadline) {
    try {
      const handle = await open(lockPath, 'wx');
      try {
        await handle.writeFile(`${process.pid}\n`);
        return handle;
      } catch (error) {
        await handle.close();
        await unlink(lockPath).catch(() => undefined);
        throw error;
      }
    } catch (error) {
      if (!hasErrorCode(error, 'EEXIST')) throw error;

      const ownerPid = await readLockOwnerPid();
      if (ownerPid === null || !isProcessAlive(ownerPid)) {
        await unlink(lockPath).catch((unlinkError) => {
          if (!hasErrorCode(unlinkError, 'ENOENT')) throw unlinkError;
        });
        continue;
      }

      await new Promise((resolveWait) => setTimeout(resolveWait, lockWaitMs));
    }
  }

  throw new Error('Timed out waiting for another EMMM native dependency setup to finish.');
}

async function releasePreparationLock(handle) {
  await handle.close();
  await unlink(lockPath).catch((error) => {
    if (!hasErrorCode(error, 'ENOENT')) throw error;
  });
}

async function dependencyFingerprint() {
  const manifest = await readFile(manifestPath);
  return createHash('sha256')
    .update(manifest)
    .update('\0')
    .update(triplet)
    .update('\0')
    .update(requiredArtifacts.join('\0'))
    .digest('hex');
}

async function missingArtifacts() {
  const checks = await Promise.all(
    requiredArtifacts.map(async (relativePath) => ({
      relativePath,
      exists: await isNonEmptyFile(join(installedTripletRoot, relativePath)),
    })),
  );
  return checks.filter(({ exists }) => !exists).map(({ relativePath }) => relativePath);
}

async function dependenciesAreReady(fingerprint) {
  try {
    const stamp = JSON.parse(await readFile(stampPath, 'utf8'));
    return (
      stamp.fingerprint === fingerprint &&
      stamp.triplet === triplet &&
      (await missingArtifacts()).length === 0
    );
  } catch {
    return false;
  }
}

async function cargoNativeCacheIsReady(fingerprint) {
  try {
    const stamp = JSON.parse(await readFile(cargoCacheStampPath, 'utf8'));
    return (
      stamp.fingerprint === fingerprint &&
      stamp.vcpkgRoot === resolve(localVcpkgRoot) &&
      stamp.triplet === triplet
    );
  } catch {
    return false;
  }
}

function candidateToolRoots() {
  const executableName = process.platform === 'win32' ? 'vcpkg.exe' : 'vcpkg';
  const pathCandidates = (process.env.PATH ?? '')
    .split(delimiter)
    .filter(Boolean)
    .map((directory) => resolve(directory));
  const configuredCandidates = [
    process.env.EMMM_VCPKG_TOOL_ROOT,
    process.env.VCPKG_INSTALLATION_ROOT,
    process.env.VCPKG_ROOT,
    process.env.LOCALAPPDATA ? join(process.env.LOCALAPPDATA, 'Codex', 'vcpkg') : undefined,
    process.env.USERPROFILE ? join(process.env.USERPROFILE, 'vcpkg') : undefined,
    'C:\\vcpkg',
  ];

  return [...new Set([...configuredCandidates, ...pathCandidates].filter(Boolean))].map((root) => ({
    root: resolve(root),
    executable: join(resolve(root), executableName),
  }));
}

async function findVcpkgTool() {
  for (const candidate of candidateToolRoots()) {
    if (
      candidate.root !== resolve(localVcpkgRoot) &&
      (await pathExists(join(candidate.root, '.vcpkg-root'))) &&
      (await pathExists(candidate.executable))
    ) {
      return candidate;
    }
  }

  throw new Error(
    'A vcpkg tool checkout is required to build EMMM on Windows. Set EMMM_VCPKG_TOOL_ROOT to a vcpkg directory containing vcpkg.exe.',
  );
}

function run(command, args, options = {}) {
  return new Promise((resolveRun, rejectRun) => {
    const child = spawn(command, args, {
      cwd: projectRoot,
      stdio: 'inherit',
      windowsHide: true,
      ...options,
    });
    child.once('error', rejectRun);
    child.once('exit', (code, signal) => {
      if (code === 0) {
        resolveRun();
        return;
      }

      rejectRun(
        new Error(
          signal
            ? `${command} stopped because of signal ${signal}.`
            : `${command} exited with code ${code ?? 'unknown'}.`,
        ),
      );
    });
  });
}

async function writePreparationStamp(fingerprint) {
  const temporaryStamp = `${stampPath}.${process.pid}.tmp`;
  await writeFile(temporaryStamp, `${JSON.stringify({ fingerprint, triplet }, null, 2)}\n`, 'utf8');
  await rm(stampPath, { force: true });
  await rename(temporaryStamp, stampPath);
}

async function refreshCargoNativeCache(fingerprint) {
  console.log('Invalidating stale compress-tools linker metadata.');
  await run(
    'cargo',
    ['clean', '-p', 'compress-tools', '--manifest-path', join(tauriRoot, 'Cargo.toml')],
    {
      env: {
        ...process.env,
        VCPKG_ROOT: localVcpkgRoot,
        VCPKGRS_TRIPLET: triplet,
      },
    },
  );

  const temporaryStamp = `${cargoCacheStampPath}.${process.pid}.tmp`;
  await writeFile(
    temporaryStamp,
    `${JSON.stringify({ fingerprint, triplet, vcpkgRoot: resolve(localVcpkgRoot) }, null, 2)}\n`,
    'utf8',
  );
  await rm(cargoCacheStampPath, { force: true });
  await rename(temporaryStamp, cargoCacheStampPath);
}

async function prepareWindowsDependencies() {
  if (process.platform !== 'win32') return;

  await mkdir(localVcpkgRoot, { recursive: true });
  const fingerprint = await dependencyFingerprint();
  if ((await dependenciesAreReady(fingerprint)) && (await cargoNativeCacheIsReady(fingerprint))) {
    return;
  }

  const lockHandle = await acquirePreparationLock();
  try {
    if (!(await dependenciesAreReady(fingerprint))) {
      const tool = await findVcpkgTool();
      await writeFile(join(localVcpkgRoot, '.vcpkg-root'), '', 'utf8');
      console.log(`Preparing pinned EMMM native dependencies in ${localVcpkgRoot}`);
      await run(
        tool.executable,
        [
          'install',
          `--triplet=${triplet}`,
          `--x-manifest-root=${tauriRoot}`,
          `--x-install-root=${join(localVcpkgRoot, 'installed')}`,
          `--x-buildtrees-root=${join(localVcpkgRoot, 'buildtrees')}`,
          `--x-packages-root=${join(localVcpkgRoot, 'packages')}`,
          `--downloads-root=${join(localVcpkgRoot, 'downloads')}`,
          '--clean-after-build',
        ],
        {
          env: {
            ...process.env,
            VCPKG_ROOT: tool.root,
          },
        },
      );

      const missing = await missingArtifacts();
      if (missing.length > 0) {
        throw new Error(`vcpkg completed without required static libraries: ${missing.join(', ')}`);
      }

      await writePreparationStamp(fingerprint);
    }

    if (!(await cargoNativeCacheIsReady(fingerprint))) {
      await refreshCargoNativeCache(fingerprint);
    }
  } finally {
    await releasePreparationLock(lockHandle);
  }
}

async function runTauri(args) {
  const tauriCli = join(projectRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
  if (!(await pathExists(tauriCli))) {
    throw new Error('Tauri CLI is not installed. Run pnpm install first.');
  }

  const tauriArgs =
    args[0] === 'dev' ? [...args, '--config', 'src-tauri/tauri.dev.conf.json'] : args;

  await run(process.execPath, [tauriCli, ...tauriArgs], {
    env: {
      ...process.env,
      VCPKG_ROOT: localVcpkgRoot,
      VCPKGRS_TRIPLET: triplet,
    },
  });
}

async function main() {
  const args = process.argv.slice(2);
  loadProjectEnvironment();
  if (args[0] === 'build' || args[0] === 'bundle') await loadLocalSigningKey();
  const prepareOnly = args.length === 1 && args[0] === '--prepare-only';
  await prepareWindowsDependencies();
  if (args[0] === 'dev') await installDevCatalogPack();
  if (!prepareOnly) await runTauri(args);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
