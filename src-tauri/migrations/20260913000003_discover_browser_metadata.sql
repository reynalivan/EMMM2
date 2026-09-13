CREATE TABLE IF NOT EXISTS browser_bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    url TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL DEFAULT '',
    favicon TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS browser_history (
    url TEXT PRIMARY KEY NOT NULL,
    hostname TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    favicon TEXT,
    visit_count INTEGER NOT NULL DEFAULT 1,
    last_visited_at INTEGER NOT NULL
) STRICT;

CREATE INDEX IF NOT EXISTS idx_browser_history_last_visited
    ON browser_history(last_visited_at DESC);

CREATE TABLE IF NOT EXISTS browser_session_tabs (
    position INTEGER PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    active INTEGER NOT NULL DEFAULT 0 CHECK(active IN (0, 1)),
    updated_at INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS browser_permissions (
    origin TEXT NOT NULL,
    permission_kind TEXT NOT NULL,
    decision TEXT NOT NULL CHECK(decision IN ('allow', 'deny')),
    expires_at INTEGER,
    PRIMARY KEY (origin, permission_kind)
) STRICT;
