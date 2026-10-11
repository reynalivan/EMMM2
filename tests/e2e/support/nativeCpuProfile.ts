import { browser } from '@wdio/globals';
import fs from 'node:fs/promises';
import path from 'node:path';

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Profiles the owned native WebView2 session without launching another browser. */
export async function startNativeCpuProfile(output: string, run: number) {
  if (process.env.EMMM_E2E_NATIVE_CPU_PROFILE !== '1') return null;
  const { hostname, port } = browser.options;
  if (hostname !== '127.0.0.1' || port !== 4444) {
    throw new Error('Native CPU profiling requires the owned local Tauri driver');
  }
  const endpoint = `http://${hostname}:${port}/session/${encodeURIComponent(browser.sessionId)}/ms/cdp/execute`;
  const command = async (cmd: string, params: Record<string, unknown> = {}) => {
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cmd, params }),
      signal: AbortSignal.timeout(15_000),
    });
    const result: unknown = await response.json();
    if (!response.ok || !isRecord(result) || !isRecord(result.value) || result.value.error) {
      throw new Error(`Native WebView2 ${cmd} failed: ${JSON.stringify(result)}`);
    }
    return result.value;
  };
  const disable = async (failed: boolean) => {
    const errors: unknown[] = [];
    for (const domain of ['Profiler', 'Performance']) {
      await command(`${domain}.disable`).catch((error: unknown) => {
        errors.push(error);
        console.error(`Native ${domain} cleanup failed`, error);
      });
    }
    if (!failed && errors.length > 0) throw errors[0];
  };

  await command('Profiler.enable');
  let metricsBefore: Record<string, unknown>;
  try {
    await command('Profiler.setSamplingInterval', { interval: 1_000 });
    await command('Profiler.start');
    await command('Performance.enable');
    metricsBefore = await command('Performance.getMetrics');
  } catch (error) {
    await disable(true);
    throw error;
  }
  let stopped = false;
  return async () => {
    if (stopped) return;
    stopped = true;
    let failed = false;
    try {
      const result = await command('Profiler.stop');
      if (!isRecord(result.profile) || !Array.isArray(result.profile.nodes)) {
        throw new Error('Native WebView2 returned no CPU samples');
      }
      await fs.writeFile(
        path.join(output, `native-cpu-run-${run}.cpuprofile`),
        JSON.stringify(result.profile),
      );
      const metricsAfter = await command('Performance.getMetrics');
      await fs.writeFile(
        path.join(output, `native-performance-run-${run}.json`),
        JSON.stringify({ before: metricsBefore, after: metricsAfter }),
      );
    } catch (error) {
      failed = true;
      throw error;
    } finally {
      await disable(failed);
    }
  };
}
