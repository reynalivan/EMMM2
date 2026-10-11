import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest';
import type { BeforeSendHook, initializeFaro } from '@grafana/faro-web-sdk';
import { TransportItemType } from '@grafana/faro-web-sdk';

type FaroConfiguration = Parameters<typeof initializeFaro>[0];
type TelemetryPacket = Parameters<BeforeSendHook>[0];

const sdk = vi.hoisted(() => ({
  initialize: vi.fn<(configuration: FaroConfiguration) => unknown>(),
  pushError: vi.fn<(error: Error) => void>(),
  pushEvent: vi.fn<(name: string, attributes: Record<string, string>) => void>(),
  pause: vi.fn(),
  unpause: vi.fn(),
}));

vi.mock('@grafana/faro-web-sdk', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@grafana/faro-web-sdk')>()),
  initializeFaro: sdk.initialize,
}));

const TIMESTAMP = '2026-10-11T00:00:00.000Z';
const PRIVATE_VALUE = 'E:/private-fixture/Secret Mod';

function configuration(): FaroConfiguration {
  const config = sdk.initialize.mock.calls[0]?.[0];
  if (!config) throw new Error('SDK was not initialized');
  return config;
}

function diagnosticPacket(name = 'emmm_frontend_error'): TelemetryPacket {
  return {
    type: TransportItemType.EVENT,
    payload: {
      name,
      timestamp: TIMESTAMP,
      attributes: {
        operation: 'window_error',
        error_code: 'type_error',
        fingerprint: 'abcdef12',
        path: PRIVATE_VALUE,
      },
      trace: { trace_id: 'private-trace', span_id: 'private-span' },
    },
    meta: {
      user: { id: 'private-user', email: 'private@example.invalid' },
      page: { url: 'https://private.example.invalid/page' },
      session: { id: 'private-session' },
    },
  };
}

async function enabledTelemetry() {
  const telemetry = await import('./telemetry');
  telemetry.setFrontendTelemetryEnabled(true);
  return telemetry;
}

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  vi.stubEnv('VITE_GRAFANA_FARO_URL', 'https://diagnostics.example.invalid/collect');
  vi.stubEnv('VITE_APP_VERSION', '0.1.0');
  vi.spyOn(Math, 'random').mockReturnValue(0.05);
  sdk.initialize.mockReturnValue({
    api: { pushError: sdk.pushError, pushEvent: sdk.pushEvent },
    pause: sdk.pause,
    unpause: sdk.unpause,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllEnvs();
});

describe('anonymous frontend diagnostics boundary', () => {
  it('rejects raw SDK exceptions that contain a private error message', async () => {
    await enabledTelemetry();
    const packet: TelemetryPacket = {
      type: TransportItemType.EXCEPTION,
      payload: { timestamp: TIMESTAMP, type: 'Error', value: PRIVATE_VALUE },
      meta: { user: { email: 'private@example.invalid' } },
    };
    expect(configuration().beforeSend?.(packet)).toBeNull();
  });

  it('sends only bounded diagnostic attributes and application metadata', async () => {
    await enabledTelemetry();
    const filtered = configuration().beforeSend?.(diagnosticPacket());
    expect(filtered).toEqual({
      type: TransportItemType.EVENT,
      payload: {
        name: 'emmm_frontend_error',
        timestamp: TIMESTAMP,
        attributes: {
          operation: 'window_error',
          error_code: 'type_error',
          fingerprint: 'abcdef12',
        },
      },
      meta: { app: { name: 'emmm-desktop', version: '0.1.0', environment: 'development' } },
    });
    expect(JSON.stringify(filtered)).not.toContain('private');
  });

  it('drops automatic SDK events outside the app diagnostic contract', async () => {
    await enabledTelemetry();
    expect(configuration().beforeSend?.(diagnosticPacket('faro.session.start'))).toBeNull();
  });

  it('does not install automatic error, page, action or tracing producers', async () => {
    await enabledTelemetry();
    const names = configuration().instrumentations?.map((instrumentation) => instrumentation.name);
    expect(
      names?.some((name) => /errors|console|user-actions|view|navigation|tracing/.test(name)),
    ).toBe(false);
  });

  it('retains local prompt delivery while diagnostics are disabled', async () => {
    const telemetry = await import('./telemetry');
    const receive = vi.fn();
    const unsubscribe = telemetry.subscribeToFrontendDiagnostics(receive);
    const result = telemetry.reportFrontendError(new Error(PRIVATE_VALUE), 'window_error', {
      promptUser: true,
    });
    expect(receive).toHaveBeenCalledWith(result);
    expect(result.errorCode).toBe('error');
    expect(result.fingerprint).toMatch(/^[a-f0-9]{8}$/);
    expect(sdk.initialize).not.toHaveBeenCalled();
    unsubscribe();
  });

  it('samples automatic reports locally without attaching a session identifier', async () => {
    vi.mocked(Math.random).mockReturnValue(0.95);
    const telemetry = await enabledTelemetry();
    telemetry.reportFrontendError(new Error(PRIVATE_VALUE), 'window_error');
    expect(sdk.pushEvent).not.toHaveBeenCalled();
    expect(sdk.pushError).not.toHaveBeenCalled();
  });

  it('allows an explicit default-off native crash report without enabling diagnostics', async () => {
    vi.mocked(Math.random).mockReturnValue(0.95);
    const telemetry = await import('./telemetry');
    expect(
      telemetry.sendVoluntaryFrontendDiagnostic({
        operation: 'app_bootstrap',
        errorCode: 'panic',
        fingerprint: 'native-abnormal-exit',
      }),
    ).toBe(true);
    expect(sdk.pushEvent).toHaveBeenCalledWith('emmm_frontend_error', {
      operation: 'app_bootstrap',
      error_code: 'panic',
      fingerprint: 'native-abnormal-exit',
    });
    telemetry.reportFrontendError(new Error(PRIVATE_VALUE), 'window_error');
    expect(sdk.pushEvent).toHaveBeenCalledOnce();
  });

  it('rejects invalid voluntary diagnostics instead of reporting them accepted', async () => {
    const telemetry = await import('./telemetry');
    expect(
      telemetry.sendVoluntaryFrontendDiagnostic({
        operation: 'app_bootstrap',
        errorCode: PRIVATE_VALUE,
        fingerprint: 'native-abnormal-exit',
      }),
    ).toBe(false);
    expect(sdk.pushEvent).not.toHaveBeenCalled();
  });
});
