-- Discover state belongs to the selected game. Existing global state is
-- preserved under the game that was active at the time of migration.

ALTER TABLE browser_session_tabs RENAME TO browser_session_tabs_legacy;

CREATE TABLE browser_session_tabs (
    game_id TEXT NOT NULL,
    position INTEGER NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    active INTEGER NOT NULL DEFAULT 0 CHECK(active IN (0, 1)),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (game_id, position)
) STRICT;

INSERT INTO browser_session_tabs (game_id, position, url, title, active, updated_at)
SELECT
    COALESCE((SELECT value FROM app_settings WHERE key = 'active_game_id'), '__legacy__'),
    position,
    url,
    title,
    active,
    updated_at
FROM browser_session_tabs_legacy;

DROP TABLE browser_session_tabs_legacy;

CREATE TABLE browser_game_settings (
    game_id TEXT PRIMARY KEY NOT NULL,
    homepage_url TEXT NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

INSERT INTO browser_game_settings (game_id, homepage_url, updated_at)
SELECT
    active_game.value,
    COALESCE(homepage.value, 'https://www.google.com'),
    unixepoch()
FROM app_settings AS active_game
LEFT JOIN browser_settings AS homepage ON homepage.key = 'homepage_url'
WHERE active_game.key = 'active_game_id';

ALTER TABLE browser_downloads ADD COLUMN game_id TEXT;

UPDATE browser_downloads
SET game_id = COALESCE(
    (SELECT value FROM app_settings WHERE key = 'active_game_id'),
    '__legacy__'
);

CREATE INDEX idx_browser_downloads_game_started_at
    ON browser_downloads (game_id, started_at DESC, queue_order DESC);
