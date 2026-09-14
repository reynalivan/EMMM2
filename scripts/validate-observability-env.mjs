const requiredWhenEnabled = [
  'EMMM_GRAFANA_OTLP_METRICS_ENDPOINT',
  'EMMM_GRAFANA_OTLP_AUTHORIZATION',
  'VITE_GRAFANA_FARO_URL',
];
const observabilityEnvironmentNames = new Set([
  ...requiredWhenEnabled,
  'VITE_GRAFANA_FARO_API_KEY',
  'VITE_GRAFANA_FARO_TRACING_ENABLED',
  'VITE_APP_VERSION',
]);

function loadProjectEnvironment() {
  const originalEnvironment = new Map(Object.entries(process.env));
  try {
    process.loadEnvFile('.env');
  } catch (error) {
    if (typeof error === 'object' && error !== null && 'code' in error && error.code === 'ENOENT') {
      return;
    }
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

function isHttpsUrl(value) {
  try {
    return new URL(value).protocol === 'https:';
  } catch {
    return false;
  }
}

function validateUrl(name, expectedPath) {
  const value = process.env[name] ?? '';
  if (!isHttpsUrl(value)) {
    throw new Error(`${name} must be an HTTPS URL.`);
  }

  if (expectedPath && !new URL(value).pathname.endsWith(expectedPath)) {
    throw new Error(`${name} must end with ${expectedPath}.`);
  }
}

function main() {
  loadProjectEnvironment();
  if (process.env.EMMM_REQUIRE_GRAFANA_OBSERVABILITY !== 'true') {
    console.log('Grafana observability is optional; configuration validation skipped.');
    return;
  }

  const missing = requiredWhenEnabled.filter((name) => !(process.env[name] ?? '').trim());
  if (missing.length > 0) {
    throw new Error(`Missing required Grafana release configuration: ${missing.join(', ')}.`);
  }

  validateUrl('EMMM_GRAFANA_OTLP_METRICS_ENDPOINT', '/v1/metrics');
  validateUrl('VITE_GRAFANA_FARO_URL');
  console.log('Grafana observability release configuration is valid.');
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
