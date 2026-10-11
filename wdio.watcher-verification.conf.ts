import os from 'node:os';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
import { createNativeE2EConfig } from './wdio.conf.js';

process.env.EMMM_WATCHER_VERIFICATION_NONCE ??= randomUUID();
const fixtureRoot = path.join(
  os.tmpdir(),
  `EMMM_WatcherVerification_${process.env.EMMM_WATCHER_VERIFICATION_NONCE}`,
);
process.env.EMMM_WATCHER_VERIFICATION_ROOT = fixtureRoot;

const nativeConfig = createNativeE2EConfig('fixture', true);

export const config = {
  ...nativeConfig,
  specs: ['./tests/e2e/specs/watcher-suppression-native.e2e.ts'],
  mochaOpts: { ...nativeConfig.mochaOpts, timeout: 120_000 },
};
