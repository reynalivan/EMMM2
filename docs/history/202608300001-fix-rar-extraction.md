# Fix RAR Extraction Path Not Found Bug

### Context
Users encountered "Could not open Match Wizard: RAR extraction failed: IO error: The system cannot find the path specified. (os error 3)" when dropping ZIP or RAR archives containing subdirectories into the Mod Manager's Auto Organize. The `rar` crate does not automatically create parent directories before extracting files nested in directories, leading to a file creation failure on Windows.

### Changes
- Copied the `rar` crate (`0.4.0`) to a local workspace (`src-tauri/rar-patch`).
- Modified `rar-patch/src/file_writer.rs` to explicitly create parent directories for file paths using `std::fs::create_dir_all`.
- Modified `rar-patch/src/extractor.rs` to gracefully handle directory entries in RAR archives (which end with `/` or `\`) by creating the directory directly and skipping `fs::File::create`.
- Patched the Cargo dependency in `Cargo.toml` to use the local modified `rar-patch`.

### Impacted Files
- `src-tauri/Cargo.toml` (modified)
- `src-tauri/rar-patch/src/file_writer.rs` (added/modified)
- `src-tauri/rar-patch/src/extractor.rs` (added/modified)

### Goal
RAR archives containing subfolders (or ZIP archives containing such RARs) now extract flawlessly without failing in the Match Wizard pipeline.

### Impact
- Eliminates "os error 3" during Auto Organize for RAR archives.
- Modifies build dependency to a locally-patched fork of `rar`.
