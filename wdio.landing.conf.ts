import { createNativeE2EConfig } from './wdio.conf.js';

/** Opt-in native capture run. The regular regression suite does not create marketing assets. */
export const config = {
  ...createNativeE2EConfig(),
  specs: ['./tests/e2e/capture/landing.capture.ts'],
};
