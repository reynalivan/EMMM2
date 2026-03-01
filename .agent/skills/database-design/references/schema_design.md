# Schema Design (SQLite)

## 1. Standard Table Template

```sql
CREATE TABLE items (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    is_active INTEGER DEFAULT 1, -- Boolean
    metadata JSON,               -- JSON Object
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
) STRICT;
```

## 2. Constraints

- **Foreign Keys**: Always enforce via `PRAGMA foreign_keys = ON`.
  ```sql
  game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE
  ```
- **Unique**: `UNIQUE(game_id, actual_name)` to prevent duplicates.

## 3. JSON Support

SQLite supports JSON via `json_extract`.

```sql
SELECT json_extract(metadata, '$.author') FROM items;
```

Store complex, non-searchable data as JSON blobs to avoid over-normalization.
