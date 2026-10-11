import {
  initializeFaro,
  TransportItemType,
  type BeforeSendHook,
  type EventEvent,
} from '@grafana/faro-web-sdk';

const APP_NAME = 'emmm-desktop';
const FARO_URL = import.meta.env.VITE_GRAFANA_FARO_URL?.trim();
const FARO_API_KEY = import.meta.env.VITE_GRAFANA_FARO_API_KEY?.trim();
const APP_VERSION = import.meta.env.VITE_APP_VERSION?.trim() || 'unknown';
const FARO_SESSION_SAMPLING_RATE = 0.1;
const SESSION_SAMPLED = Math.random() < FARO_SESSION_SAMPLING_RATE;
const DIAGNOSTIC_EVENT = 'emmm_frontend_error';
const APP_METADATA = {
  name: APP_NAME,
  version: APP_VERSION,
  environment: import.meta.env.DEV ? 'development' : 'production',
};
const DIAGNOSTIC_OPERATIONS = new Set([
  'app_bootstrap',
  'react_render',
  'unhandled_rejection',
  'window_error',
]);
const DIAGNOSTIC_ERROR_CODES = new Set([
  'type_error',
  'range_error',
  'syntax_error',
  'error',
  'ipc_error',
  'unknown_error',
  'none',
  'validation',
  'io',
  'database',
  'network',
  'conflict',
  'timeout',
  'cancelled',
  'panic',
  'invariant',
  'permission',
  'not_found',
  'unsupported',
  'external',
  'unknown',
]);

function isDiagnosticAttributes(
  attributes: Record<string, string> | undefined,
): attributes is Record<string, string> {
  return Boolean(
    attributes &&
    DIAGNOSTIC_OPERATIONS.has(attributes.operation) &&
    DIAGNOSTIC_ERROR_CODES.has(attributes.error_code) &&
    (/^[a-f0-9]{8}$/.test(attributes.fingerprint) ||
      (attributes.operation === 'app_bootstrap' &&
        attributes.fingerprint === 'native-abnormal-exit')),
  );
}

const privacyFilter: BeforeSendHook = (item) => {
  if (item.type !== TransportItemType.EVENT) return null;

  const event = item.payload as EventEvent;
  const attributes = event.attributes;
  if (event.name !== DIAGNOSTIC_EVENT || !isDiagnosticAttributes(attributes)) return null;

  return {
    type: TransportItemType.EVENT,
    payload: {
      name: DIAGNOSTIC_EVENT,
      timestamp: event.timestamp,
      attributes: {
        operation: attributes.operation,
        error_code: attributes.error_code,
        fingerprint: attributes.fingerprint,
      },
    },
    meta: { app: APP_METADATA },
  };
};

export type FrontendTelemetryOperation =
  'app_bootstrap' | 'react_render' | 'unhandled_rejection' | 'window_error';

export interface FrontendDiagnostic {
  operation: FrontendTelemetryOperation;
  errorCode: string;
  fingerprint: string;
}

type FaroClient = ReturnType<typeof initializeFaro>;
type DiagnosticListener = (diagnostic: FrontendDiagnostic) => void;

let faro: FaroClient | null = null;
let telemetryEnabled = false;
const diagnosticListeners = new Set<DiagnosticListener>();

function normalizeErrorCode(error: unknown): string {
  if (error instanceof TypeError) return 'type_error';
  if (error instanceof RangeError) return 'range_error';
  if (error instanceof SyntaxError) return 'syntax_error';
  if (error instanceof Error) return 'error';
  if (typeof error === 'string') return 'ipc_error';
  return 'unknown_error';
}

function stackSignature(error: unknown, componentStack?: string): string {
  const stack = error instanceof Error ? error.stack : undefined;
  const source = componentStack ?? stack ?? '';
  return source
    .replace(/[A-Za-z]:\\[^\n)]+/g, '<path>')
    .replace(/\/[^\n)]+/g, '<path>')
    .replace(/:\d+:\d+/g, ':<line>')
    .slice(0, 1024);
}

function stableHash(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, '0');
}

function ensureFaro(): FaroClient | null {
  if (faro || !FARO_URL) return faro;

  faro = initializeFaro({
    url: FARO_URL,
    apiKey: FARO_API_KEY || undefined,
    app: APP_METADATA,
    paused: true,
    batching: { enabled: false },
    sessionTracking: { enabled: false, persistent: false },
    metas: [],
    beforeSend: privacyFilter,
    instrumentations: [],
  });
  faro.pause();
  return faro;
}

function emitToFaro(diagnostic: FrontendDiagnostic): boolean {
  const attributes = {
    operation: diagnostic.operation,
    error_code: diagnostic.errorCode,
    fingerprint: diagnostic.fingerprint,
  };
  if (!isDiagnosticAttributes(attributes)) return false;
  const client = ensureFaro();
  if (!client) return false;

  client.api.pushEvent(DIAGNOSTIC_EVENT, attributes);
  return true;
}

export function setFrontendTelemetryEnabled(enabled: boolean): void {
  telemetryEnabled = enabled;
  if (!enabled) {
    faro?.pause();
    return;
  }
  const client = ensureFaro();
  if (!client) return;
  client.unpause();
}

export function subscribeToFrontendDiagnostics(listener: DiagnosticListener): () => void {
  diagnosticListeners.add(listener);
  return () => diagnosticListeners.delete(listener);
}

export function reportFrontendError(
  error: unknown,
  operation: FrontendTelemetryOperation,
  options: { componentStack?: string; promptUser?: boolean } = {},
): FrontendDiagnostic {
  const errorCode = normalizeErrorCode(error);
  const fingerprint = stableHash(
    `${APP_VERSION}:${operation}:${errorCode}:${stackSignature(error, options.componentStack)}`,
  );
  const diagnostic = { operation, errorCode, fingerprint };

  if (telemetryEnabled && SESSION_SAMPLED) emitToFaro(diagnostic);
  if (options.promptUser) diagnosticListeners.forEach((listener) => listener(diagnostic));
  return diagnostic;
}

export function sendVoluntaryFrontendDiagnostic(diagnostic: FrontendDiagnostic): boolean {
  const wasEnabled = telemetryEnabled;
  const client = ensureFaro();
  if (!client) return false;
  client.unpause();
  const sent = emitToFaro(diagnostic);
  if (!wasEnabled) client.pause();
  return sent;
}
