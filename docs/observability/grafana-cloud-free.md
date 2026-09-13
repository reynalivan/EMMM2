# Grafana Cloud Free observability

EMMM diagnostics are opt-in. The application has no collector, proxy, polling loop, installation ID, or automatic GitHub issue integration.

## Release configuration

For local verification, copy `.env.example` to `.env` and fill the values there. The `pnpm tauri` wrapper passes its local `.env` values to the native build; `.env` remains ignored by Git. For releases, create the GitHub Actions Secrets with the exact names below. Grafana is optional: a release without every value still completes without remote telemetry. Once any Grafana value is supplied, the release workflow validates the complete set before packaging so partial configuration does not ship silently.

| Build environment variable | Purpose |
| --- | --- |
| `EMMM_GRAFANA_OTLP_METRICS_ENDPOINT` | HTTPS OTLP metrics endpoint ending in `/v1/metrics` |
| `EMMM_GRAFANA_OTLP_AUTHORIZATION` | Write-only Grafana ingest authorization value |
| `VITE_GRAFANA_FARO_URL` | Grafana Faro collector URL |
| `VITE_GRAFANA_FARO_API_KEY` | Faro write-only ingest key |
| `VITE_APP_VERSION` | Release version attached to frontend diagnostics |

The required GitHub Actions Secrets are `EMMM_GRAFANA_OTLP_METRICS_ENDPOINT`, `EMMM_GRAFANA_OTLP_AUTHORIZATION`, `VITE_GRAFANA_FARO_URL`, and `VITE_GRAFANA_FARO_API_KEY`. `VITE_APP_VERSION` is set automatically from the release tag. For a local readiness check, run `pnpm validate:observability` with `EMMM_REQUIRE_GRAFANA_OBSERVABILITY=true`; it validates only presence and URL format and never prints secret values.

The desktop binary can be inspected, so ingest credentials are treated as public-but-scoped. Create separate write-only tokens for this stack, restrict them to ingestion, and rotate by shipping a new release if abused. Human dashboard access must use separate credentials.

## Data and cadence

Rust queues a seven-day, UTC daily aggregate with only `release`, `operation`, `outcome`, and `error_code` attributes. The OTLP payload deliberately excludes paths, mod/game IDs and names, URLs, timestamps as labels, error text, and diagnostic hashes. New UI errors are sent through Faro only after consent or an explicit one-time send. The app attempts native aggregate export at startup and then once every 24 hours while it stays open; failed delivery waits for the next lifecycle attempt.

Current native emitters cover onboarding, launch, disk reconcile, collection apply, restore, import commit and extraction, watcher lifecycle plus overflow, classification batches and accepted/cancelled review decisions, auto-match outcomes, single-item safety toggles, and bulk toggle item outcomes. A separate `rejected` review outcome is intentionally absent until the product exposes a distinct rejected decision rather than overloading cancellation.

In addition, the shared frontend IPC client records every rejected production command as one `failed` native event. Its operation and error code are reduced to the same bounded vocabulary; raw `AppError` payloads are neither persisted nor exported. Background work that does not return through an IPC command must still emit at its own boundary (the watcher, scheduled exporter, and panic recovery already do).

Each command failure has one owner: successful operation outcomes are recorded at the native operation boundary, while rejected IPC commands record the corresponding failure once through the shared client. Background catalog updates, duplicate scans, hotkeys, and downloads record their own failures because they have no IPC response. Nested `AppError` variants are reduced to their actual category, such as database, I/O, permission, validation, or network, rather than a broad domain default.

The bounded schema reserves the following event-success panels, rather than unique-user conversion:

- Onboarding: `config_saved / started`.
- Auto-match: `matched / attempts`.
- Reconcile and watcher: `success / runs` with failure outcome panels.
- Toggle and bulk actions: success, partial, and failed operation counts.

For mode breakdowns, use the bounded `outcome` label. `auto_match` exposes `matched`, `needs_review`, and `unmatched`; classification exposes `auto_accepted` for canonical catalog choices and `needs_review` for manually supplied classification. This shows where automation is effective without recording object names or selected entries.

Create overview, funnel, reliability, and mutation-safety dashboard sections as their corresponding native operation emitters are added. Alert by email only: a new error code, error rate above 2% over 24 hours with at least 20 operations, and watcher/reconcile failures above 10% with at least 5 runs. Triage manually before creating a GitHub issue: inspect the short diagnostic code, reproduce, fix, then mark the alert resolved.

Import [`grafana/emmm-overview-dashboard.json`](grafana/emmm-overview-dashboard.json) and provision [`grafana/alerts.yaml`](grafana/alerts.yaml) after replacing `__PROMETHEUS_DATASOURCE_UID__` with the Grafana Cloud metrics datasource UID. Route this alert folder to email in Grafana Alerting. Neither artifact contains credentials.
