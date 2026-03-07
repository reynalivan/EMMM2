# Epic 19: Image Gallery

## 1. Executive Summary

- **Problem Statement**: Visual mods are inherently graphical — users need to verify what a mod looks like (textures, lighting, outfit) without equipping it in-game, but mod folders often contain multiple screenshots and reference images with no in-app viewer.
- **Proposed Solution**: A `GallerySection` inside the Preview Panel that auto-discovers image files (PNG, JPG, JPEG, WebP) in the mod root and `images/` subfolder, displays them in a responsive thumbnail grid, serves them via Tauri `asset://` protocol, and allows setting any image as the `preview.png` main thumbnail.
- **Success Criteria**:
  - `list_mod_preview_images` returns results in ≤ 100ms for a folder with ≤ 50 images.
  - Gallery thumbnail grid renders first image within ≤ 200ms of section mount.
  - Lightbox opens on click within ≤ 100ms.
  - "Set as Thumbnail" copies to `preview.png` in ≤ 300ms and the `FolderCard` thumbnail in the grid refreshes within ≤ 200ms of cache invalidation.
  - No memory leak from large images (> 5MB) — images load via `<img src="asset://...">` with lazy loading; they are not base64-encoded into memory.

---

## 2. User Experience & Functionality

### User Stories

#### US-19.1: Auto-Detect Images

As a user, I want the Preview Panel to automatically find all images in a mod folder, so that I can visually verify the mod without manually navigating to the folder.

| ID        | Type        | Criteria                                                                                                                                                                                                      |
| --------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-19.1.1 | ✅ Positive | Given a selected mod folder, when the Gallery section mounts, then all `.png`, `.jpg`, `.jpeg`, and `.webp` files inside the mod root and subfolders (max depth 3) are displayed as thumbnails within ≤ 200ms |
| AC-19.1.2 | ✅ Positive | Given a mod folder with no valid images, then the Gallery shows an "Add Images" prompt — no error, no empty white space                                                                                       |
| AC-19.1.3 | ❌ Negative | Given a mod folder with ≥ 50 images, then images are rendered with lazy loading — only visible thumbnails are decoded; images below the fold are not eagerly loaded                                           |
| AC-19.1.4 | ⚠️ Edge     | Given a mod folder containing an image > 10MB, then that image is still loaded as a thumbnail (browser performs decoding); no OOM error is thrown in the web process                                          |
| AC-19.1.5 | ✅ Positive | Given the gallery is focused, pressing `Ctrl+V` pastes an image from the OS clipboard, saves it as `preview_custom.png`, sets it as the thumbnail, and refreshes the gallery in ≤ 500ms                       |

---

#### US-19.2: Thumbnail Management

As a user, I want to set any gallery image as the mod's main thumbnail, so that it shows up in my Folder Grid card without manual file renaming.

| ID        | Type        | Criteria                                                                                                                                                                                  |
| --------- | ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-19.2.1 | ✅ Positive | Given an image in the gallery, when I hover and click "Set as Thumbnail", then the file is copied to `preview.png` in the mod root in ≤ 300ms                                             |
| AC-19.2.2 | ✅ Positive | Given a successful "Set as Thumbnail", then `queryClient.invalidateQueries(['folders', gameId])` is called — the `FolderCard` in the grid shows the new thumbnail in ≤ 200ms              |
| AC-19.2.3 | ❌ Negative | Given "Set as Thumbnail" is clicked on the file that is already `preview.png`, then the operation is no-op — no file copy, no error                                                       |
| AC-19.2.4 | ⚠️ Edge     | Given a `preview.png` already exists and is the primary thumbnail, when replaced, then the old file is overwritten (not left alongside) — no duplicate `preview.jpg` or `preview_old.png` |

---

#### US-19.3: Image Paging & Lightbox

As a user, I want to click a thumbnail to see a full-size version with prev/next navigation, so that I can inspect fine texture or lighting details.

| ID        | Type        | Criteria                                                                                                                                             |
| --------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| AC-19.3.1 | ✅ Positive | Given a gallery with images, then thumbnails are displayed in a responsive grid layout (auto-fill columns, min 100px each)                           |
| AC-19.3.2 | ✅ Positive | Given I click a thumbnail, then a modal lightbox opens within ≤ 100ms — shows the full-resolution image with "Previous" and "Next" navigation arrows |
| AC-19.3.3 | ✅ Positive | Given the lightbox is open, when I press Escape or click outside the image, then the lightbox closes in ≤ 100ms                                      |
| AC-19.3.4 | ⚠️ Edge     | Given the mod has only 1 image, then the lightbox's "Previous" and "Next" controls are hidden — no wrapping or disabled state confusion              |

---

### Non-Goals

- No in-app image editing (cropping, resizing).
- No external image upload from disk dialog — only images already inside the mod folder are shown. Clipboard-paste thumbnail is handled by Epic 15 (Context Menu).
- No animated GIF support in the lightbox — only static images.
- Gallery does not scan recursively beyond depth 3 — deeper nested images are ignored to protect performance.

---

## 3. Technical Specifications

### Architecture Overview

```
GallerySection.tsx
  └── useModImages(folderPath) → invoke('list_mod_preview_images', { folderPath }) → PathBuf[]
      └── map paths → convertFileSrc(path) → asset:// URLs
          ├── ThumbGrid (CSS grid, auto-fill, minmax(100px, 1fr))
          │   └── <img src={assetUrl} loading="lazy" onClick → openLightbox(index) />
          └── Lightbox modal (react-photo-album or custom)
              ├── full-size <img src={assetUrl} />
              └── Prev/Next → setCurrentIndex

"Set as Thumbnail" btn (per thumbnail hover):
  → invoke('set_mod_thumbnail', { folderPath, imagePath })
  → reads file bytes → writes to {folderPath}/preview.png
  → onSuccess: invalidateQueries(['folders', gameId]) + invalidateQueries(['modImages', folderPath])

Backend:
  list_mod_preview_images(folder_path) → Vec<PathBuf>
    └── scan root + subfolders (max_depth=3) for {.png,.jpg,.jpeg,.webp}

  set_mod_thumbnail(folder_path, image_path) → ()
    └── validate both paths within mods_path
        → fs::copy(image_path, folder_path/preview.png)  [overwrites]
```

### Integration Points

| Component        | Detail                                                                                    |
| ---------------- | ----------------------------------------------------------------------------------------- |
| File Discovery   | `invoke('list_mod_preview_images', { folderPath })` → `preview_cmds.rs`                   |
| Asset Protocol   | Tauri `asset:` protocol — configured in `tauri.conf.json` `allowlist.protocol.asset`      |
| Image URL        | `convertFileSrc(absolutePath)` from `@tauri-apps/api/tauri`                               |
| Set Thumbnail    | `invoke('set_mod_thumbnail', { folderPath, imagePath })`                                  |
| Cache Invalidate | `queryClient.invalidateQueries(['folders', gameId])` on successful thumbnail set          |
| Lazy Loading     | `<img loading="lazy">` — browser native, no additional virtualizer needed for ≤ 50 images |

### Security & Privacy

- **All `imagePath` values are validated** backend-side via `canonicalize()` + `starts_with(mods_path)` — no arbitrary read or write outside the mod folder.
- **Tauri `asset:` protocol scope** is restricted to `mods_path` in `tauri.conf.json` `fs.scope` — images outside that scope cannot be served as assets.
- **Safe Mode**: If `safe_mode = true` and the selected mod has `is_safe = false`, the Gallery section is hidden entirely — no image paths are requested from the backend.

---

## 4. Dependencies

- **Blocked by**: Epic 16 (Preview Panel — mounting context and `folderPath` prop).
- **Blocks**: Nothing — leaf component of the Preview Panel sub-tree.
