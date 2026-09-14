# INI Encoding Support: GBK & UTF-16LE

## Context
When users edit game locations (which triggers classification and INI parsing), they were encountering an error: `Unsupported INI encoding... use UTF-8 or Shift-JIS`. This blocked them from using mods that had INI files saved in GBK (very common for Chinese users) or UTF-16LE.

## Changes
- Updated `IniEncoding` enum to include `Gbk` and `Utf16Le` variants.
- Modified `decode_ini_source` in `document/encoding.rs` to attempt decoding with `encoding_rs::GBK` and `encoding_rs::UTF_16LE` (including BOM detection for UTF-16).
- Modified `encode_ini_text` to correctly encode back to `GBK` and `UTF-16LE` when saving the INI files.
- Updated the error message in `classifier.rs` to reflect the newly supported encodings.

## Impacted Files
- `src-tauri/src/modules/library/application/ini/encoding.rs` (modified)
- `src-tauri/src/modules/workspace/domain/classifier.rs` (modified)

## Goal
The system now gracefully handles and preserves GBK and UTF-16LE INI files during classification, editing, and saving operations, preventing encoding panics for a wider variety of mod sources.

## Impact
- Better compatibility with Chinese mods (GBK) and Windows-native saved files (UTF-16LE).
- No breaking changes.
- Performance impact is negligible (a few extra decoding attempts on parse failure).
