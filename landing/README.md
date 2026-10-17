# EMMM landing page

Static Astro site for the EMMM project. It is independent of the Tauri frontend.

```powershell
cd landing
npm ci
npm run dev
npm run build
```

The site uses the onyx colors and restrained glass treatment from the desktop app. `quick-liquid` enhances the navigation and main hero control on capable desktop browsers; CSS provides the fallback. Reduced motion and small screens use the fallback without loading the liquid engine.

The Astro `site` and `base` settings target `https://reynalivan.github.io/EMMM2/`. The Pages workflow builds this directory when landing files are pushed to `main`.

## Application screenshots

The images in `public/screenshots/` are optimized WebP captures of the native Tauri app with a fictional sample library. Raw captures are kept out of the published site.

| Screenshot | What it shows |
| --- | --- |
| `aster-costume-grid.webp` | One character in distinct outfits, a full Object List, and selected preview; used in hero and CTA |
| `workspace-library.webp` | Dense Object List, costume mod grid, and selected preview |
| `dashboard-play.webp` | Dashboard quick actions, fixture statistics, and charts |
| `home-key-mapping.webp`, `home-key-mapping-focus.webp` | Searchable active mod keybinds on Home and a readable crop for the keybind section |
| `settings-hotkeys.webp`, `hotkey-settings-focus.webp` | Collection shortcuts, optional game notices, and KeyViewer controls in Settings |
| `settings-catalog.webp`, `settings-catalog-focus.webp` | Catalog Assets settings and the ZIP import controls |
| `metadata-filters-focus.webp` | Search, status, category, and metadata controls in the Object sidebar |
| `mod-inbox.webp` | New mods waiting in the Inbox |
| `smart-classification.webp` | High-confidence suggestions alongside matches needing review |
| `nested-subfolders.webp` | Variants inside an existing nested folder |
| `collections-safe-mode.webp` | Saved Collections and their safety labels |
| `collection-switch-preview.webp` | Compact before/after review before applying a Collection |
| `paste-thumbnail.webp` | Paste image from clipboard in preview image actions |
| `switch-controls.webp` | Enabled and disabled switches on costume cards |
| `switch-bulk-actions.webp` | Multi-selection and bulk action toolbar |
| `switch-toggle-detail.webp`, `switch-bulk-detail.webp` | Tight crops of switches and the bulk toolbar for the feature card |
| `pin-mod-menu.webp` | Pin Selected in the mod context menu |
| `search-filters.webp` | Object search and filters |
| `favorites-filters.webp` | Favorite mod and pinned Object List entry |
| `mod-health-details.webp` | Detected hotkey in Mod Health details |
| `storage-optimizer.webp` | Duplicate scan results and resolution actions |
| `setup-games.webp` | Game path and executable settings |
| `setup-xxmi-welcome.webp`, `setup-xxmi-actions.webp` | Native onboarding entry with XXMI Auto-Detect and manual setup |
| `quick-play-focus.webp`, `game-settings-focus.webp` | Tight crops of Quick Play and the Game Settings dialog |

The sample characters and mod names are fictional. The captures show the actual application UI; no screenshot elements are composited into the images. The Game Settings capture displays sample paths so a local Windows account name is not published.
