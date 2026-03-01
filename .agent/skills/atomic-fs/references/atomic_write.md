# Atomic Write Strategy

## The Problem

Writing directly to a file (`File::create`) truncates it immediately. If the program crashes during write (or power loss), the file is corrupt/empty.

## The Solution: Write-and-Swap

1.  **Create Temp:** `filename.ext.tmp`.
2.  **Write:** Stream data to `.tmp`.
3.  **Flush:** Force sync disk buffer.
4.  **Rename:** `fs::rename("filename.ext.tmp", "filename.ext")`.

## Implementation (Rust)

```rust
use std::io::Write;

pub fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let tmp_path = path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp_path)?;
    file.write_all(content)?;
    file.sync_data()?; // CRITICAL: Ensure data hits disk

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
```
