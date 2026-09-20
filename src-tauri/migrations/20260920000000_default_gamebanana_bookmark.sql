INSERT OR IGNORE INTO browser_bookmarks (
    id,
    url,
    title,
    favicon,
    created_at,
    updated_at
)
VALUES (
    'default-gamebanana-bookmark',
    'https://gamebanana.com/',
    'GameBanana',
    NULL,
    unixepoch(),
    unixepoch()
);
