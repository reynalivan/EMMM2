# TC-00 — Production Smoke Checklist (Data-Safety Paths)

> Run against a **sandbox mod folder** (a small copy of the original library) using a release build (`pnpm tauri build`), BEFORE dogfooding against the original library. Every item must pass. Feature-specific steps are in the corresponding tc-NN.

| #   | Scenario                                    | Verification                                                                            | Ref          | Status |
| --- | ------------------------------------------- | --------------------------------------------------------------------------------------- | ------------ | ------ |
| 1   | Toggle mod on/off                           | `DISABLED ` prefix is correct on disk, no files are missing, and the UI is in sync after reconciliation | tc-20        | ☐      |
| 2   | Delete mod → restore                        | Folder moves to `./app_data/trash/`, restore returns it intact, and no hard delete occurs | tc-22        | ☐      |
| 3   | Import .zip/.7z (including password-protected archives) | Extraction is correct; a collision dialog appears when the name conflicts       | tc-23, tc-37 | ☐      |
| 4   | Apply collection with some mods missing     | Transactional; partial failures are shown to the user; no half-applied state            | tc-31        | ☐      |
| 5   | Rename/toggle while another process locks a file | Retry dialog appears, the operation can be retried, and no folder is corrupted       | tc-13, tc-21 | ☐      |
| 6   | Force-kill app during bulk toggle           | After restart + reconciliation, disk state is consistent (the filesystem is the source of truth) | tc-14        | ☐      |

| 7 | Force enable + check "don't remind me again" | The duplicate warning no longer appears for the same mod combination; the entry appears in IgnoreManagementModal and can be revoked | tc-29 | ☐ |
