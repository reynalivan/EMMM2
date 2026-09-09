import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { open, readFile, unlink } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = fileURLToPath(new URL('..', import.meta.url));
const devUrl = 'http://localhost:1420/';
const emmmTitle = '<title>EMMM</title>';
const emmmEntryPoint = '/src/app/entrypoint/main.tsx';
const probeTimeoutMs = 750;
const serverStartupTimeoutMs = 10000;
const serverStartupPollMs = 100;
const workspaceId = createHash('sha1').update(projectRoot).digest('hex').slice(0, 12);
const lockPath = join(tmpdir(), `emmm-tauri-dev-${workspaceId}.lock`);

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

async function probeDevServer() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), probeTimeoutMs);

  try {
    const response = await fetch(devUrl, { signal: controller.signal });
    const body = await response.text();

    return response.ok && body.includes(emmmTitle) && body.includes(emmmEntryPoint)
      ? 'emmm'
      : 'occupied';
  } catch {
    return 'free';
  } finally {
    clearTimeout(timeout);
  }
}

async function waitForDevServer() {
  const deadline = Date.now() + serverStartupTimeoutMs;

  while (Date.now() < deadline) {
    const serverState = await probeDevServer();
    if (serverState !== 'free') return serverState;
    await new Promise((resolve) => setTimeout(resolve, serverStartupPollMs));
  }

  return probeDevServer();
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

async function acquireStartupLock() {
  while (true) {
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
      if (ownerPid !== null && isProcessAlive(ownerPid)) return null;

      await unlink(lockPath).catch((unlinkError) => {
        if (!hasErrorCode(unlinkError, 'ENOENT')) throw unlinkError;
      });
    }
  }
}

async function releaseStartupLock(handle) {
  await handle.close();
  await unlink(lockPath).catch((error) => {
    if (!hasErrorCode(error, 'ENOENT')) throw error;
  });
}

function terminateChild(child) {
  if (!child.pid || child.exitCode !== null) return Promise.resolve();

  if (process.platform === 'win32') {
    return new Promise((resolve) => {
      const killer = spawn('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
        stdio: 'ignore',
        windowsHide: true,
      });
      killer.once('close', resolve);
      killer.once('error', resolve);
    });
  }

  return new Promise((resolve) => {
    const forceKillTimer = setTimeout(() => {
      if (child.exitCode === null) child.kill('SIGKILL');
      resolve();
    }, 2000);

    child.once('exit', () => {
      clearTimeout(forceKillTimer);
      resolve();
    });
    child.kill('SIGTERM');
  });
}

async function startDevServer() {
  const serverState = await probeDevServer();

  if (serverState === 'emmm') {
    console.log(`Reusing the existing EMMM Vite server at ${devUrl}`);
    return;
  }

  if (serverState === 'occupied') {
    throw new Error(
      `Port 1420 is already serving another application. Stop that process before starting EMMM.`,
    );
  }

  const lockHandle = await acquireStartupLock();
  if (lockHandle === null) {
    const waitingState = await waitForDevServer();

    if (waitingState === 'emmm') {
      console.log(`Reusing the existing EMMM Vite server at ${devUrl}`);
      return;
    }

    if (waitingState === 'occupied') {
      throw new Error(
        `Port 1420 is already serving another application. Stop that process before starting EMMM.`,
      );
    }

    throw new Error(
      `Another EMMM Vite server is starting but did not become ready within ${serverStartupTimeoutMs / 1000} seconds.`,
    );
  }

  const lockedServerState = await probeDevServer();
  if (lockedServerState !== 'free') {
    await releaseStartupLock(lockHandle);

    if (lockedServerState === 'emmm') {
      console.log(`Reusing the existing EMMM Vite server at ${devUrl}`);
      return;
    }

    throw new Error(
      `Port 1420 is already serving another application. Stop that process before starting EMMM.`,
    );
  }

  const command = process.platform === 'win32' ? (process.env.ComSpec ?? 'cmd.exe') : 'pnpm';
  const commandArgs =
    process.platform === 'win32' ? ['/d', '/s', '/c', 'pnpm exec vite'] : ['exec', 'vite'];
  let child;
  try {
    child = spawn(command, commandArgs, {
      cwd: projectRoot,
      env: process.env,
      stdio: 'inherit',
      windowsHide: true,
    });
  } catch (error) {
    await releaseStartupLock(lockHandle);
    throw error;
  }
  let shuttingDown = false;

  const shutdown = async (exitCode) => {
    if (shuttingDown) return;
    shuttingDown = true;
    await terminateChild(child);
    await releaseStartupLock(lockHandle);
    process.exit(exitCode);
  };

  process.once('SIGINT', () => void shutdown(130));
  process.once('SIGTERM', () => void shutdown(143));

  child.once('error', (error) => {
    console.error(`Unable to start Vite: ${error.message}`);
    void shutdown(1);
  });
  child.once('exit', (code, signal) => {
    if (shuttingDown) return;
    shuttingDown = true;
    if (signal) {
      console.error(`Vite stopped because of signal ${signal}.`);
    }
    void releaseStartupLock(lockHandle).then(
      () => process.exit(code ?? 1),
      (error) => {
        console.error(`Unable to release the Vite startup lock: ${error.message}`);
        process.exit(1);
      },
    );
  });
}

startDevServer().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
