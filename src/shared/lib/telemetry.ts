import { initializeFaro } from '@grafana/faro-web-sdk';

const APP_NAME = 'emmm-desktop';
const FARO_URL = import.meta.env.VITE_GRAFANA_FARO_URL?.trim();
const FARO_API_KEY = import.meta.env.VITE_GRAFANA_FARO_API_KEY?.trim();
const APP_VERSION = import.meta.env.VITE_APP_VERSION?.trim() || 'unknown';

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
    app: {
      name: APP_NAME,
      version: APP_VERSION,
      environment: import.meta.env.DEV ? 'development' : 'production',
    },
  });
  faro.pause();
  return faro;
}

function emitToFaro(diagnostic: FrontendDiagnostic): boolean {
  const client = ensureFaro();
  if (!client) return false;

  client.api.pushError(new Error(`EMMM diagnostic: ${diagnostic.errorCode}`));
  client.api.pushEvent('emmm_frontend_error', {
    operation: diagnostic.operation,
    error_code: diagnostic.errorCode,
    fingerprint: diagnostic.fingerprint,
  });
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

  if (telemetryEnabled) emitToFaro(diagnostic);
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
