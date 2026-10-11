import { createNativeE2EConfig } from './wdio.conf.js';
import { abortNativeRun } from './tests/e2e/support/driverLifecycle.js';
import {
  prepareReadonlyAudit,
  restoreReadonlyAudit,
  readonlyAuditPaths,
  READONLY_AUDIT_IDENTIFIER,
  ensureInstalledAppStopped,
  monitorInstalledApp,
} from './tests/e2e/support/readonlyAudit.js';
import fs from 'node:fs';
import path from 'node:path';

export function createReadonlyAuditConfig(buildMode: 'debug' | 'release') {
  const nativeConfig = createNativeE2EConfig('readonly-audit', false, buildMode);
  let stopInterlock: (() => void) | undefined;
  process.once('exit', () => stopInterlock?.());
  return {
    ...nativeConfig,
    specs: ['./tests/e2e/specs/startup-real-library-readonly.e2e.ts'],
    mochaOpts: { ...nativeConfig.mochaOpts, timeout: 300_000 },
    onPrepare: async () => {
      try {
        ensureInstalledAppStopped();
        await nativeConfig.onPrepare();
        await prepareReadonlyAudit();
        const audit = readonlyAuditPaths();
        const localData = process.env.LOCALAPPDATA;
        if (!localData) throw new Error('LOCALAPPDATA is required');
        const logPath = path.join(localData, READONLY_AUDIT_IDENTIFIER, 'logs', 'emmm.log');
        fs.writeFileSync(
          path.join(audit.output, 'native-before-launch.log'),
          fs.existsSync(logPath) ? fs.readFileSync(logPath) : '',
        );
      } catch (error) {
        abortNativeRun(error, restoreReadonlyAudit);
      }
    },
    // Audit seeds only the isolated DB offline; no reset/setting mutation IPC is permitted.
    before: async () => {},
    beforeSession: async () => {
      try {
        ensureInstalledAppStopped();
        await nativeConfig.beforeSession();
        stopInterlock = monitorInstalledApp((message) => abortNativeRun(new Error(message)));
        process.env.EMMM_AUDIT_SESSION_START_UNIX_MS = String(Date.now());
      } catch (error) {
        abortNativeRun(error);
      }
    },
    afterSession: async () => {
      stopInterlock?.();
      stopInterlock = undefined;
      await nativeConfig.afterSession();
    },
    onComplete: async () => {
      try {
        stopInterlock?.();
        restoreReadonlyAudit();
      } catch (error) {
        abortNativeRun(error);
      }
    },
  };
}

export const config = createReadonlyAuditConfig('debug');
