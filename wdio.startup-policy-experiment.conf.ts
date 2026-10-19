import * as fs from 'node:fs';
import * as path from 'node:path';
import { createNativeE2EConfig } from './wdio.conf.js';
import { abortNativeRun } from './tests/e2e/support/driverLifecycle.js';

const profile = process.env.EMMM_E2E_POLICY_PROFILE;
const runnerTemp = process.env.RUNNER_TEMP;
if (
  process.env.GITHUB_ACTIONS !== 'true' ||
  process.env.RUNNER_ENVIRONMENT !== 'github-hosted' ||
  !profile ||
  !runnerTemp ||
  path.dirname(profile) !== path.resolve(runnerTemp) ||
  !/^emmm-startup-policy-[a-f0-9]{32}$/.test(path.basename(profile)) ||
  !fs.lstatSync(profile).isDirectory() ||
  fs.lstatSync(profile).isSymbolicLink()
) {
  throw new Error('Startup policy experiment requires its fresh GitHub-hosted profile');
}

const native = createNativeE2EConfig();
export const config = {
  ...native,
  specs: ['./tests/e2e/specs/app-bootstrap.e2e.ts'],
  capabilities: native.capabilities.map((capability) => ({
    ...capability,
    'tauri:options': {
      ...capability['tauri:options'],
      webviewOptions: { userDataFolder: profile },
    },
  })),
  before: async () => {
    try {
      await native.before();
      const portFile = [
        path.join(profile, 'DevToolsActivePort'),
        path.join(profile, 'EBWebView', 'DevToolsActivePort'),
      ].find((candidate) => fs.existsSync(candidate));
      if (!portFile)
        throw new Error('Experiment DevTools port file is absent from its own profile');
      const [portText] = fs.readFileSync(portFile, 'utf8').split(/\r?\n/);
      if (!/^\d+$/.test(portText) || Number(portText) < 1 || Number(portText) > 65535) {
        throw new Error('Experiment did not expose a valid DevTools port in its own profile');
      }
      const response = await fetch(`http://127.0.0.1:${portText}/json/version`, {
        signal: AbortSignal.timeout(5000),
      });
      if (!response.ok) throw new Error('Experiment DevTools endpoint is unavailable');
      const version: unknown = await response.json();
      if (
        typeof version !== 'object' ||
        version === null ||
        !('webSocketDebuggerUrl' in version) ||
        typeof version.webSocketDebuggerUrl !== 'string'
      ) {
        throw new Error('Experiment DevTools endpoint returned an invalid browser identity');
      }
      const endpoint = new URL(version.webSocketDebuggerUrl);
      if (
        endpoint.protocol !== 'ws:' ||
        endpoint.port !== portText ||
        !['127.0.0.1', 'localhost', '[::1]'].includes(endpoint.hostname)
      ) {
        throw new Error('Experiment DevTools endpoint is not loopback-only');
      }
      const evidence = path.resolve('.audit-output/native-startup-experiment');
      fs.mkdirSync(evidence, { recursive: true });
      fs.writeFileSync(
        path.join(evidence, 'session.json'),
        JSON.stringify({
          profile,
          debuggingPort: Number(portText),
          loopbackEndpointVerified: true,
        }),
      );
    } catch (error) {
      try {
        await native.afterSession();
      } finally {
        abortNativeRun(error);
      }
    }
  },
};
