# Epic 6: Preview Panel & Detail View (The Command Center)

**Focus:** Providing in-depth information about the selected mod, visual management (images), and integration of a `.ini` configuration editor capable of manipulating _keybindings_ and mod variables directly with _native_ Rust performance.

## Dependencies

| Direction  | Epic   | Relationship                                           |
| ---------- | ------ | ------------------------------------------------------ |
| ⬆ Upstream | Epic 4 | Requires grid selection state for which mod to display |
| ⬆ Upstream | Epic 4 | Reads `info.json` (created by E4)                      |
| References | Epic 5 | Triggers toggle via E5's toggle service                |

## Cross-Cutting Requirements

- **info.json Editing:** This Epic is the **editor** of `info.json`. E4 creates it. E5/E7/E8 read it.
- **INI Parser:** Custom line-based parser (TRD §3.3). Must strip BOM (U+FEFF) from first line. Saves without BOM.
- **Image Slider:** Only current image and ±1 adjacent are loaded. Others use placeholder until scrolled to (lazy loading).
- **Clipboard:** Images > 10MB rejected with toast "Image too large". Auto-resize to max 1920×1080, save as WebP.

---

## 1. User Stories & Acceptance Criteria

### US-6.1: Mod Detail & Info Management

**As a** user, **I want to** view and edit mod metadata (Title, Author, Description) in real-time, **So that** I have clear and organized notes regarding the mod.

- **Acceptance Criteria:**
  - **Metadata View:** Displays `actual_name`, `author`, `version`, and `description` read directly from `info.json`.
  - **Live Edit (Debounced):** Description/title field edits will be automatically saved to disk (`info.json`) after the user stops typing for 500ms.
  - **Status Toggle:** A large "Enable/Disable" switch directly connected to the `ModManager::toggle_state` logic (Epic 5), providing instant visual feedback.

### US-6.2: Multi-Image Thumbnail & Slider

**As a** user, **I want to** view a gallery of mod preview images in a smooth slider format, **So that** I can check mod variants (e.g., different hair colors) before using them.

- **Acceptance Criteria:**
  - **Discovery Logic (Recursive):**
    - Priority 0: `preview_custom.(png|jpg|webp)` file (User generated).
    - Priority 1: `preview*.(jpg|png|webp)` files in the root folder.
    - Priority 2: Mod image files in subfolders (Max depth: 3).
  - **Interactive Slider:**
    - `<` and `>` navigation buttons appear if images > 1.
    - Position indicators (dots) below the slider.
    - Support for _Touch/Swipe_ gestures (if on a touch screen).
  - **Clipboard Paste:** `Ctrl+V` shortcut or "Paste Image" Context Menu will take an image from the clipboard, convert it to `preview_custom.png`, and instantly refresh the slider.

### US-6.3: 3DMigoto Config Editor (.ini)

**As a** user, **I want to** configure _keybindings_ and _mod variables_ (such as toggle dress/hair) through a graphical UI, **So that** I don't need to edit complex and error-prone text files.

- **Acceptance Criteria:**
  - **INI File Detection:** A dropdown list displays all valid `.ini` files in the mod folder (ignoring `desktop.ini`, etc.).
  - **Intelligent Parser (Rust):**
    - **Keybindings:** Recognizes `[Key...]` sections and displays inputs for `key` (trigger key) and `back` (return key).
    - **Variables:** Detects `$variable = value` patterns and displays them as number inputs or _Cycle Buttons_ if a rotation pattern (0, 1, 2) is detected.
  - **Atomic Save & Backup:**
    - Save changes only to the modified lines.
    - Automatically create `config.ini.backup` before overwriting.
    - Basic syntax validation before saving.

---

## 2. Technical Specifications (Rust/Tauri Implementation)

### A. Rust INI Parser & Modifier

Replacing fragile Python regex approaches with secure _line-by-line_ parsing in Rust to preserve comments and the original file structure.

```rust
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
struct IniVariable {
    name: String,   // "$dress_color"
    value: String,  // "0"
    line_idx: usize, // Line number in the original file
}

#[derive(Debug, Clone)]
struct KeyBinding {
    section_name: String, // "KeyChangeDress"
    key: String,          // "v"
    back: Option<String>,
    line_idx: usize,
}

struct IniEditor {
    file_path: PathBuf,
    raw_lines: Vec<String>, // Stores the entire file content as-is (including comments)
}

impl IniEditor {
    // Load file into memory
    fn load(path: &PathBuf) -> Self {
        let content = fs::read_to_string(path).unwrap_or_default();
        IniEditor {
            file_path: path.clone(),
            raw_lines: content.lines().map(|s| s.to_string()).collect(),
        }
    }

    // Update specific variable without touching other lines
    fn update_variable(&mut self, line_idx: usize, new_value: &str) {
        if let Some(line) = self.raw_lines.get_mut(line_idx) {
            // Regex replace value only, preserve whitespace/comments logic here
            // Ex: " $var = 0  ; comment" -> " $var = 1  ; comment"
        }
    }

    // Atomic Save
    fn save(&self) -> Result<(), std::io::Error> {
        // 1. Create Backup
        let backup_path = self.file_path.with_extension("ini.backup");
        fs::write(&backup_path, self.raw_lines.join("\n"))?;

        // 2. Write New Content
        fs::write(&self.file_path, self.raw_lines.join("\n"))?;
        Ok(())
    }
}
```

### B. Image Discovery Strategy (Async Walkdir)

Efficient image search system using `walkdir` with depth limitations for maximum performance.

```rust
use walkdir::WalkDir;
use std::path::Path;

fn scan_preview_images(root: &Path) -> Vec<PathBuf> {
    let mut images = Vec::new();

    // 1. Check Custom & Root High Priority
    // Logic to check preview_custom.png first

    // 2. If Empty, Deep Scan (Max Depth 3)
    if images.is_empty() {
        for entry in WalkDir::new(root).max_depth(3) {
            if let Ok(e) = entry {
                let name = e.file_name().to_string_lossy().to_lowercase();
                if is_image_extension(&name) {
                    images.push(e.path().to_path_buf());
                }
            }
        }
    }

    // 3. Fallback
    if images.is_empty() {
        images.push(PathBuf::from("assets/placeholder.png"));
    }

    images
}

fn is_image_extension(name: &str) -> bool {
    name.ends_with(".png") || name.ends_with(".jpg") || name.ends_with(".webp")
}
```

---

## 3. Frontend Data Structure

### ViewModel (TypeScript Interface)

Data sent from Rust to the Frontend to render the detail panel.

```typescript
interface ModDetail {
  id: string;
  info: {
    title: string;
    author: string;
    description: string;
    version: string;
  };
  images: string[]; // List of local asset URLs
  ini_files: {
    filename: string;
    path: string; // Full path for reference
  }[];
  status: {
    is_enabled: boolean;
    is_safe: boolean;
  };
}
```

---

## 4. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Metadata Display**: Click a mod in the list → The panel correctly displays `actual_name` and `description` from `info.json`.
- [ ] **Slider Navigation**: Mod with 3 preview images → Slider appears → Clicking arrow changes images → Loops from last to first.
- [ ] **INI Edit & Save**: Change `$dress_color` from 0 to 1 → Click Save → `.ini` updated on disk → `.ini.bak` backup created.
- [ ] **Clipboard Paste**: Copy image from browser → Focus panel → Ctrl+V → Image saved as `preview_custom.webp` → Appears in slider.
- [ ] **Unsaved Changes Guard**: Edit info.json → Click another mod → "Discard changes?" confirmation appears.

### 2. Negative Cases (Error Handling)

- [ ] **Missing INI**: Mod without `.ini` file → Configuration tab hidden/disabled → Notification "No Configuration File Found".
- [ ] **Corrupt JSON**: Broken `info.json` → Panel still loads with folder name as fallback → Error logged.
- [ ] **Read-Only File**: Save Read-Only `.ini` → Toast "Permission Denied" → Suggest checking file attributes.
- [ ] **Large Image Paste**: Clipboard image > 10MB → Rejected with toast "Image too large. Max 10MB.".

### 3. Edge Cases (Stability)

- [ ] **Concurrency**: Simultaneously edit INI + Toggle Enable Mod → Operation lock prevents corruption (TRD §3.6).
- [ ] **Large Description**: 500-word `info.json` description → Text area is scrollable, layout does not break.
- [ ] **Encoding Handling**: `.ini` with Shift-JIS encoding → Auto-detect and display correctly.
- [ ] **BOM Handling**: `.ini` with BOM (U+FEFF) → Stripped on read, not added on save.
- [ ] **Lazy Loading**: Mod with 20 images → Only 3 loaded initially (current ± 1) → Others load on scroll.

### 4. Technical Metrics

- [ ] **Preview Load**: Transition between mods **< 100ms**.
- [ ] **Save Speed**: Config file save **< 50ms**.
- [ ] **Accessibility**: All form inputs have labels. ARIA roles on slider controls.
