# Epic 9: Smart Duplicate Scanner (Storage Optimizer)

**Focus:** Intelligently detecting mod duplicates to save storage space by prioritizing structural similarity and file identity, using a high-speed parallel _hashing_ algorithm (Rust/Rayon).

## Dependencies

| Direction  | Epic   | Relationship                                   |
| ---------- | ------ | ---------------------------------------------- |
| ⬆ Upstream | Epic 2 | Shares file walker and normalization utilities |
| ⬆ Upstream | Epic 4 | Uses custom trash service for soft-delete      |
| References | Epic 5 | Operation lock for bulk resolution             |

## Cross-Cutting Requirements

- **Hashing:** Use `blake3` crate (per TRD). NOT SHA1/SHA256.
- **Parallelism:** Use `rayon` crate for multi-threaded file hashing. CPU usage should scale to 80-90%.
- **Trash:** Deleted duplicates go to `./app_data/trash/` (custom trash), NOT OS Recycle Bin.
- **Operation Lock:** Bulk resolution acquires `OperationLock` (TRD §3.6).
- **TanStack Query:** After resolution, invalidate `['mods', gameId]`.

---

## 1. Multi-Signal Matching Algorithm (Revised Logic)

The system compares two mod folders and calculates a **Confidence Score**. The checking sequence is weight-based for performance efficiency:

### **Signal 1: Structural & Front-Name Similarity (Main Weight: 40%)**

This signal acts as the first filter because most duplicates only differ by prefix or outer folder name.

- **Front-Name Matching**: Compares the first 50-70% of characters in the folder name (ignoring the `DISABLED ` prefix).
- **Tree Architecture**: Compares the subfolder arrangement and internal file hierarchy. If the structure is identical, the probability of it being a duplicate is very high.

### **Signal 2: File Identity & Headers (Weight: 30%)**

- **Content Hashing (Rust)**: Performs a _hash_ (BLAKE3/SHA256) on key files (such as `.ini` and `.dds`).
- **INI Header Match**: Compares the _header_ or initial comments in `.ini` files. Modders often leave version or mod name information there.

### **Signal 3: Physical Fingerprint (Weight: 20%)**

- **Filename Check**: Checks the list of filenames within the folder alphabetically.
- **File Count & Extensions**: Compares the total number of files and their extension distribution (`.dds`, `.buf`, `.ib`).
- **Texture Sampling**: For large files (>5MB), the system performs _partial hashing_ (first 1KB + last 1KB) for a 100x speed increase.

### **Signal 4: Logic Support (Weight: 10%)**

- **Keybinding Match**: Checks for identical `$swapvar` variables or _keybindings_ within the `.ini` file as supporting data if other signals remain inconclusive.

---

## 2. Conflict Resolution Interface (Bulk Report UI)

Instead of one-by-one confirmation, the system presents a **Duplicate Comparison Report** in the form of a table or mass list.

### **Reporting Features:**

- **Side-by-Side Comparison**: Displays Folder A vs Folder B.
- **Detail Stats**: Shows the size of each folder (in MB/GB) and the number of files.
- **Match Assumptions**: Explains why the system considers it a duplicate (e.g., _"Structure 100% Match, Hash 100% Match"_).
- **Visual Preview**: Displays thumbnails from both folders for quick user verification.

### **Bulk Action Radio Buttons:**

Each report row has a _radio button_ option to determine the mod's fate:

| Option            | Action                                                                 |
| :---------------- | :--------------------------------------------------------------------- |
| **Keep Original** | Retains Folder A (typically the older one) and deletes Folder B.       |
| **Replace**       | Deletes Folder A and replaces it with Folder B.                        |
| **Ignore**        | Ignores the scan result and marks it as "Not a Duplicate" (Whitelist). |

_Once everything is set, the user simply presses one **"Apply All Changes"** button._

---

## 3. Technical Specifications (Rust/Tauri Implementation)

### A. Parallel Hashing Strategy (`rayon`)

Uses `rayon` to process thousands of files in parallel, utilizing all CPU cores.

```rust
use rayon::prelude::*;
use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct FileSignature {
    rel_path: String,
    hash: String,
    size: u64,
}

fn calculate_folder_signature(folder_path: &Path) -> Vec<FileSignature> {
    // 1. Collect all files
    let files: Vec<PathBuf> = walkdir::WalkDir::new(folder_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_owned())
        .collect();

    // 2. Parallel Hash Calculation
    files.into_par_iter().map(|path| {
        let mut file = File::open(&path).expect("File open failed");
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192]; // 8KB Buffer

        loop {
            let count = file.read(&mut buffer).unwrap();
            if count == 0 { break; }
            hasher.update(&buffer[..count]);
        }

        FileSignature {
            rel_path: path.strip_prefix(folder_path).unwrap().to_string_lossy().to_string(),
            hash: format!("{:x}", hasher.finalize()),
            size: file.metadata().unwrap().len(),
        }
    }).collect()
}
```

### B. Database Reporting Schema

Stores temporary scan results to be presented in the UI without re-scanning every time the page is opened.

```sql
CREATE TABLE duplicate_reports (
    id INTEGER PRIMARY KEY,
    group_id TEXT NOT NULL, -- UUID for grouping pairs
    mod_a_id TEXT NOT NULL,
    mod_b_id TEXT NOT NULL,
    confidence_score INTEGER,
    match_reason TEXT,
    resolution TEXT DEFAULT 'PENDING' -- KEEP_A, KEEP_B, IGNORE
);
```

### C. Trash Integration

Deletion actions from the Duplicate Scanner will follow the **Soft Delete (Trash)** rules from Epic 4. Deleted mods will be moved to the `./app_data/trash/` folder for safety in case the user makes a wrong choice.

---

## 4. Checklist Success Criteria (Definition of Done)

### 1. Positive Cases (Happy Path)

- [ ] **Scan Detection**: System correctly groups "Albedo" and "DISABLED Albedo" as duplicates if contents match.
- [ ] **Bulk Resolution**: User selects "Keep Original" for 10 items → Click "Apply" → 10 duplicates moved to trash.
- [ ] **Trash Safety**: Deleted duplicates in `./app_data/trash/` with metadata for restore.
- [ ] **Scan Progress**: Progress bar shows "Hashing X/Y folders..." with Cancel button.

### 2. Negative Cases (Error Handling)

- [ ] **False Positive**: Two mods same name but different files → Confidence < 80% → Marked "Low Confidence".
- [ ] **File Locked**: Resolve duplicate open in another app → Skip, log error, process rest.
- [ ] **Scan Cancelled**: Cancel mid-scan → partial results discarded → DB unchanged.

### 3. Edge Cases (Stability)

- [ ] **Multi-Duplicate**: Mod A, B, C identical → Groups all three → User keeps A → B, C to trash.
- [ ] **Zero Results**: Clean library → "No duplicates found" without hang.
- [ ] **Operation Lock**: Bulk resolution blocks concurrent toggle operations (TRD §3.6).

### 4. Technical Metrics

- [ ] **Scan Speed**: `blake3` hash & structure analysis on 1,000 folders in **< 15 seconds** (SSD).
- [ ] **Parallelism**: CPU scales to 80-90% via `rayon` (effective multi-threading).
- [ ] **Accessibility**: All resolution buttons have ARIA labels. Keyboard navigation in results table.
