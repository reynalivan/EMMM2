# Database schema policy

The migration set describes the canonical fresh-install SQLite schema. It does
not upgrade databases created by pre-canonical corridor/PIN collection builds.
During development, delete/reset an older local database before starting this
version; startup intentionally fails instead of applying compatibility fallbacks.

Collection draft visibility is owned by
`collection_runtime_state.draft_collection_id`. A draft remains a regular
`collections` row for snapshot/recovery integrity, but list queries exclude it
through that pointer rather than a second persisted flag.
