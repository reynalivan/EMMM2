//! Deep content signal extraction for staged matcher.
//!
//! Provides:
//! - Robust hash extraction from INI text with fallback decoding
//! - INI structural tokenization (stubs for Task 7+)
//! - Deep signal collector with budgets (stubs for Task 8+)

use std::fs;
use std::path::Path;
mod signal_collector;
mod tokenizer;

pub use signal_collector::{
    collect_deep_signals, FolderSignals, FULL_MAX_INI_FILES, FULL_MAX_NAME_ITEMS,
    FULL_MAX_TOTAL_INI_BYTES, QUICK_MAX_INI_BYTES_PER_FILE, QUICK_MAX_INI_FILES,
    QUICK_MAX_NAME_ITEMS,
};
pub use tokenizer::{extract_structural_ini_tokens, IniTokenBuckets, IniTokenizationConfig};

/// Extract hashes from raw INI text.
///
/// Accepts:
/// - `hash = d94c8962` (plain 8-hex)
/// - `hash=0xd94c8962` (0x prefix)
/// - `hash = 00000000d94c8962` (16-hex, takes last 8)
///
/// Returns lowercase 8-hex strings only. Invalid tokens ignored.
///
/// # Covers: Task 6 - Hash Extraction
pub fn extract_hashes_from_ini_text(text: &str) -> Vec<String> {
    let mut hashes = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Match lines with "hash" key (case-insensitive key check)
        if !trimmed.to_lowercase().contains("hash") {
            continue;
        }

        // Split on '=' to separate key and value
        let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
        if parts.len() != 2 {
            continue;
        }

        let key = parts[0].trim().to_lowercase();
        if key != "hash" {
            continue;
        }

        let value = parts[1].trim();
        if value.is_empty() {
            continue;
        }

        // Normalize and validate hash
        if let Some(normalized) = normalize_hash(value) {
            hashes.push(normalized);
        }
    }

    hashes
}

/// Normalize a hash value to lowercase 8-hex.
///
/// Handles:
/// - Strip `0x` prefix
/// - Trim leading zeros (16-hex -> last 8)
/// - Validate hex characters
fn normalize_hash(raw: &str) -> Option<String> {
    let mut value = raw.trim().to_string();

    // Remove `0x` or `0X` prefix
    if value.len() > 2 && value.to_lowercase().starts_with("0x") {
        value = value[2..].to_string();
    }

    // If 16-hex (32 chars), take last 8
    if value.len() == 16 {
        value = value[8..].to_string();
    }

    // Validate: must be exactly 8 hex digits
    if value.len() != 8 {
        return None;
    }

    let lower = value.to_lowercase();

    // Check all chars are valid hex
    if !lower.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some(lower)
}

/// Decode INI file content with UTF-8 first, then UTF-16 LE / lossy fallback.
///
/// Tries:
/// 1. UTF-8 decode
/// 2. UTF-16 LE decode
/// 3. UTF-8 lossy conversion
///
/// Never panics; always returns a usable string.
///
/// # Covers: Task 6 - INI Decode Fallback
pub fn decode_ini_content(path: &Path) -> Result<String, String> {
    decode_ini_content_with_cap(path, None)
}

pub(crate) fn decode_ini_content_with_cap(
    path: &Path,
    max_bytes: Option<usize>,
) -> Result<String, String> {
    let mut bytes =
        fs::read(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    if let Some(cap) = max_bytes {
        bytes.truncate(cap);
    }

    Ok(decode_ini_bytes(&bytes))
}

fn decode_ini_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    // Try UTF-8 first
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }

    // Try UTF-16 LE
    if bytes.len().is_multiple_of(2) {
        let mut utf16_bytes = bytes.to_vec();
        // Check for UTF-16 LE BOM
        if bytes.starts_with(&[0xFF, 0xFE]) {
            utf16_bytes = bytes[2..].to_vec();
        }

        if let Ok(text) = decode_utf16_le(&utf16_bytes) {
            return text;
        }
    }

    // Fall back to lossy UTF-8
    String::from_utf8_lossy(bytes).to_string()
}

/// Decode UTF-16 LE bytes to String.
fn decode_utf16_le(bytes: &[u8]) -> Result<String, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("Odd byte count for UTF-16".to_string());
    }

    let mut result = String::new();
    for chunk in bytes.chunks_exact(2) {
        let word = u16::from_le_bytes([chunk[0], chunk[1]]);
        match char::from_u32(word as u32) {
            Some(c) => result.push(c),
            None => {
                // Replace invalid surrogates with replacement char
                result.push('\u{FFFD}');
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
#[path = "../tests/analysis/content_tests.rs"]
mod tests;
