# Configured startup responsiveness

User authorized implementation with `perbaiki` after the startup diagnosis.

## Evidence

The recorded configured restart scanned 30,290 directories in 9,936 ms:
census 1,649 ms, classification 5,624 ms, database projection 2,652 ms.
Settings hydration awaited that recovery. Frontend boot repeated on navigation
and overlapping store initialization could submit duplicate activation requests.
Further inspection confirmed the Unicode backfill marker targeted `app_meta`,
which does not exist in the migrated schema. Ignored read/write errors made every
restart rewrite all index keys. The verified improvement is skipping those
backfill writes; no classification, identity-staging or size-cache improvement
is claimed without a fresh configured-restart trace.

## Change

1. Store the Unicode version in existing `app_settings`, atomically with the
   backfill. Serialize the marker check and writes using `BEGIN IMMEDIATE`.
2. Run router boot once and share overlapping store initialization.
3. Preserve recovery/readiness gates and verify repeat startup performs zero
   backfill writes, including failure rollback.
4. Measure missing-marker versus durable-marker startup on isolated fixtures.

Full disk observation remains required after a process restart because changes
while the app was closed are not covered by a watcher. Persistent classifier or
census caches are deferred: they need a separate invalidation design and storage
measurements. A nonblocking settings/dashboard experiment was discarded after
review found it superseded pending startup recovery and could retain stale cold
queries. This change eliminates repeated migration work without changing disk
authority or UI readiness.

## Validation

Use regression tests for StrictMode/navigation, overlapping initialization,
backfill idempotence and failure rollback. Run relevant frontend/native tests,
types, lint, builds, formatting and a focused correctness review. Record actual
measurements separately from the historic startup timing.
