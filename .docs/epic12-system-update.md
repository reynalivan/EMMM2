# Epic 12: System Maintenance & Dynamic Updates

**Focus:** Automatically managing game metadata updates (character/weapon databases), visual assets (thumbnails), and application versions using free _hosting_ services (GitHub/Gitee) with `tauri-plugin-updater` v2 integration.

## Dependencies

| Direction    | Epic   | Relationship                                            |
| ------------ | ------ | ------------------------------------------------------- |
| ⬇ Downstream | Epic 3 | Provides updated `schema.json` and character metadata   |
| ⬇ Downstream | Epic 4 | Provides updated thumbnail assets                       |
| Standalone   | Self   | App update is self-contained via `tauri-plugin-updater` |

## Cross-Cutting Requirements

- **HTTP Client:** Use `reqwest` crate (per TRD) for all network requests.
- **App Updater:** Use `tauri-plugin-updater` v2 with signature verification.
- **Caching:** Use `If-Modified-Since` / `ETag` headers to avoid redundant downloads.
- **Background:** All network operations run in `tokio` async tasks. Never block main thread.
- **Startup Impact:** Metadata check must add **< 500ms** to startup time.

---

## 1. User Stories & Acceptance Criteria

### US-12.1: Dynamic Metadata & DB Sync

**As a** user, **I want** the lists of characters and weapons to always be up-to-date (e.g., when a new character is released), **So that** the application recognizes the latest mods without a binary update.

- **Acceptance Criteria:**
  - **Remote Manifest**: The system checks the `manifest.json` file on GitHub (`raw.githubusercontent.com`) at startup.
  - **Diff Check**: Compares the local `db_char.json` version with the _remote_ version.
  - **Silent Update (Background)**: If the _remote_ version is newer, download the JSON -> Parse -> Update the `metadata` table in SQLite -> "New Data Available" notification.

### US-12.2: Zero-Cost App Updater

**As a** user, **I want** to receive notifications when a new application version is available and be able to update it with a single click, **So that** I always have the latest features.

- **Acceptance Criteria:**
  - **Tauri Updater v2**: Uses standard Tauri Updater endpoints (GitHub Releases).
  - **Update Prompt**: "New Version Available (v1.2.0)" dialog with a changelog.
  - **One-Click Install**: Click "Update" -> Download -> Verify Sig -> Install -> Restart.

### US-12.3: Lazy Asset Fetching

**As a** user, **I want** missing thumbnails/icons to be automatically downloaded when needed, **So that** the application installer remains small while the UI stays complete.

- **Acceptance Criteria:**
  - **On-Demand Download**: If loading the `Element_Dendro.png` icon fails (File Not Found), trigger a download from the asset CDN.
  - **Cache Strategy**: Save downloaded assets to `app_data/cache/assets/` for future use.

---

## 2. Technical Specifications (Rust/Tauri Implementation)

### A. Tauri Plugin Updater Integration

Uses the official plugin for security and easy binary updates.

```rust
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/user/repo/releases/latest/download/latest.json"
      ],
      "pubkey": "YOUR_ED25519_PUBLIC_KEY"
    }
  }
}
```

```rust
// Rust Logic
use tauri_plugin_updater::UpdaterExt;

async fn check_update(app: tauri::AppHandle) -> Result<(), tauri::Error> {
    if let Some(update) = app.updater().check().await? {
        // Show update dialog to user...
        // If user accepts:
        update.download_and_install(|downloaded, total| {
             // Update progress bar UI
        }, || {
             // Install complete
        }).await?;

        app.restart();
    }
    Ok(())
}
```

### B. Metadata Sync (Reqwest + SQLx)

Synchronizes the character database without application updates.

```rust
use reqwest::Client;
use serde_json::Value;

async fn sync_metadata(db: &SqlitePool) -> Result<(), AppError> {
    let client = Client::new();
    let remote_manifest: Value = client.get("https://.../manifest.json")
        .send().await?.json().await?;

    let remote_ver = remote_manifest["db_version"].as_u64().unwrap();
    let local_ver = get_local_db_version(db).await?;

    if remote_ver > local_ver {
        // Download Payload
        let chars: Vec<CharacterData> = client.get("https://.../db_char.json")
            .send().await?.json().await?;

        // Bulk Insert/Update to SQLite
        update_character_table(db, chars).await?;
    }
    Ok(())
}
```

---

## 3. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Data Update**: Startup → New Character "Mavuika" in remote DB → App downloads → Appears in Filter List.
- [ ] **App Update**: Click "Check Update" → New version found → Install → App restarts with new version.
- [ ] **Asset Fetch**: Open Menu → "Geo Icon" missing locally → App fetches from GitHub → Icon appears.
- [ ] **Download Progress**: Large data update shows progress bar with "Downloading... X/Y MB".

### 2. Negative Cases (Error Handling)

- [ ] **Offline**: No Internet → Update check fails silently → Log "Network Error" → App continues normally.
- [ ] **Corrupt Download**: Signature verification fails → Update aborted → "Update Failed: Invalid Signature" alert.
- [ ] **Rate Limited**: GitHub API rate limit hit → Retry with exponential backoff → Max 3 retries.

### 3. Edge Cases (Stability)

- [ ] **Mid-Update Crash**: App killed during download → Temp files cleaned up on next run → Update retries safely.
- [ ] **Version Conflict**: Debug Build (v0.0.0) → Ignored by update logic to prevent overwriting dev features.
- [ ] **Bandwidth Limit**: Large update (> 50MB) → User prompted to confirm before download starts.

### 4. Technical Metrics

- [ ] **Startup Check**: Metadata check adds **< 500ms** to startup time.
- [ ] **Efficiency**: Only downloads if `If-Modified-Since` / `ETag` indicates changes.
- [ ] **Accessibility**: Update notifications have ARIA live region for screen readers.
