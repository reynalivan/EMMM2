use std::collections::BTreeSet;

const DEFAULT_STOPWORDS: &[&str] = &[
    "mod",
    "skin",
    "preset",
    "version",
    "ver",
    "v",
    "fix",
    "shader",
    "tex",
    "texture",
    "override",
    "resource",
    "commandlist",
    "key",
    "ini",
    "dds",
];

const DEFAULT_INI_KEY_BLACKLIST: &[&str] = &[
    "run",
    "handling",
    "match_priority",
    "drawindexed",
    "vb",
    "ib",
    "ps",
    "vs",
    "cs",
    "format",
    "stride",
];

const DEFAULT_INI_KEY_WHITELIST: &[&str] = &[
    "texture",
    "resource",
    "filename",
    "path",
    "name",
    "character",
];

const SECTION_PREFIX_BLACKLIST: &[&str] = &[
    "textureoverride",
    "shaderoverride",
    "resource",
    "commandlist",
    "key",
    "present",
    "draw",
];

const PATH_EXT_HINTS: &[&str] = &[".dds", ".png", ".jpg", ".ini", ".buf", ".txt"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IniTokenBuckets {
    pub section_tokens: Vec<String>,
    pub key_tokens: Vec<String>,
    pub path_tokens: Vec<String>,
    /// Continuous stripped section names (e.g. "AyakaBody" from `[TextureOverrideAyakaBody]`)
    /// for substring matching, NOT tokenized.
    pub section_strings: Vec<String>,
    /// Continuous file stems from path-like RHS (e.g. "Raiden_Body" from `filename = Raiden_Body.dds`)
    /// for substring matching, NOT tokenized.
    pub path_strings: Vec<String>,
}

/// The tokenization rules as configured — a spec, not a working set.
///
/// Merging this with the defaults costs roughly forty `String`s and four
/// B-trees, so call [`IniTokenizationConfig::prepare`] once per scan and pass
/// the result down. It used to be merged on entry to
/// [`extract_structural_ini_tokens`], i.e. once per INI file, which is over a
/// million throwaway allocations at the 10k-mod design target.
#[derive(Debug, Clone, Default)]
pub struct IniTokenizationConfig {
    pub stopwords: Vec<String>,
    pub short_token_whitelist: Vec<String>,
    pub ini_key_blacklist: Vec<String>,
    pub ini_key_whitelist: Vec<String>,
}

/// [`IniTokenizationConfig`] merged with the defaults, ready to filter with.
#[derive(Debug, Clone, Default)]
pub struct PreparedTokenFilters {
    stopwords: BTreeSet<String>,
    short_whitelist: BTreeSet<String>,
    key_blacklist: BTreeSet<String>,
    key_whitelist: BTreeSet<String>,
}

impl IniTokenizationConfig {
    /// Merge with the built-in defaults. Do this once per scan.
    pub fn prepare(&self) -> PreparedTokenFilters {
        PreparedTokenFilters {
            stopwords: merged_stopwords(self),
            short_whitelist: normalized_set(&self.short_token_whitelist),
            key_blacklist: merged_key_blacklist(self),
            key_whitelist: merged_key_whitelist(self),
        }
    }
}

/// Extract structural token buckets from INI text.
///
/// Buckets are deterministic (sorted, deduped):
/// - section_tokens: tokens from section headers `[SectionName]`
/// - key_tokens: tokens from key names on `key = value` lines
/// - path_tokens: tokens from RHS values that look like paths or filenames
///
/// Applies default + schema-driven stopwords, key blacklist/whitelist, and
/// short-token whitelist filtering. Numeric-only tokens are always excluded.
pub fn extract_structural_ini_tokens(
    text: &str,
    filters: &PreparedTokenFilters,
) -> IniTokenBuckets {
    let mut section_tokens = BTreeSet::new();
    let mut key_tokens = BTreeSet::new();
    let mut path_tokens = BTreeSet::new();
    let mut section_strings = BTreeSet::new();
    let mut path_strings = BTreeSet::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            let section = &line[1..line.len() - 1];
            let cleaned_section = strip_section_prefixes(section);
            let prepared = prepare_for_tokenizing(&cleaned_section);
            insert_tokens(&mut section_tokens, prepared.split_whitespace(), filters);
            // Collect continuous string for substring matching Pass B
            let trimmed = cleaned_section.trim();
            if !trimmed.is_empty() {
                section_strings.insert(trimmed.to_string());
            }
            continue;
        }

        let Some((lhs, rhs)) = line.split_once('=') else {
            continue;
        };

        let key_normalized = normalize_key(lhs);
        if key_normalized.is_empty() || filters.key_blacklist.contains(&key_normalized) {
            continue;
        }

        if !filters.key_whitelist.is_empty() && !filters.key_whitelist.contains(&key_normalized) {
            continue;
        }

        let prepared_key = prepare_for_tokenizing(lhs);
        insert_tokens(&mut key_tokens, prepared_key.split_whitespace(), filters);

        if looks_like_path(rhs) {
            let prepared_rhs = prepare_path_like_rhs(rhs);
            insert_tokens(&mut path_tokens, prepared_rhs.split_whitespace(), filters);
            // Collect continuous file stems for substring matching Pass B
            let value = rhs.trim().trim_matches('"').trim_matches('\'');
            for segment in value
                .split(['/', '\\', ',', ';', ' ', '\t'])
                .filter(|p| !p.is_empty())
            {
                let cleaned = segment.trim_matches('"').trim_matches('\'').trim();
                if let Some((stem, _ext)) = cleaned.rsplit_once('.') {
                    if !stem.is_empty() {
                        path_strings.insert(stem.to_string());
                    }
                }
            }
        }
    }

    IniTokenBuckets {
        section_tokens: section_tokens.into_iter().collect(),
        key_tokens: key_tokens.into_iter().collect(),
        path_tokens: path_tokens.into_iter().collect(),
        section_strings: section_strings.into_iter().collect(),
        path_strings: path_strings.into_iter().collect(),
    }
}

fn merged_stopwords(config: &IniTokenizationConfig) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for token in DEFAULT_STOPWORDS {
        set.insert((*token).to_string());
    }
    for token in &config.stopwords {
        let normalized = normalize_simple(token);
        if !normalized.is_empty() {
            set.insert(normalized);
        }
    }
    set
}

fn merged_key_blacklist(config: &IniTokenizationConfig) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for key in DEFAULT_INI_KEY_BLACKLIST {
        set.insert((*key).to_string());
    }
    for key in &config.ini_key_blacklist {
        let normalized = normalize_key(key);
        if !normalized.is_empty() {
            set.insert(normalized);
        }
    }
    set
}

fn merged_key_whitelist(config: &IniTokenizationConfig) -> BTreeSet<String> {
    let mut set = DEFAULT_INI_KEY_WHITELIST
        .iter()
        .map(|key| (*key).to_string())
        .collect::<BTreeSet<_>>();
    for key in &config.ini_key_whitelist {
        let normalized = normalize_key(key);
        if !normalized.is_empty() {
            set.insert(normalized);
        }
    }
    set
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for value in values {
        let normalized = normalize_simple(value);
        if !normalized.is_empty() {
            set.insert(normalized);
        }
    }
    set
}

fn insert_tokens<'a>(
    destination: &mut BTreeSet<String>,
    tokens: impl Iterator<Item = &'a str>,
    filters: &PreparedTokenFilters,
) {
    for token in tokens {
        if should_keep_token(token, filters) {
            destination.insert(token.to_string());
        }
    }
}

/// Lowercase `input` and put a space wherever a token boundary is: at every
/// non-alphanumeric run, and at every camelCase hump.
///
/// Returns the prepared string rather than a `Vec<String>` of tokens. Most
/// tokens are dropped by [`should_keep_token`] moments later, so splitting at
/// the call site and allocating only the survivors saves the rest.
fn prepare_for_tokenizing(input: &str) -> String {
    let mut prepared = String::with_capacity(input.len());
    let mut previous_was_lower_or_digit = false;

    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_was_lower_or_digit {
                prepared.push(' ');
            }
            prepared.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            prepared.push(' ');
            previous_was_lower_or_digit = false;
        }
    }

    prepared
}

/// The prepared form of every path segment in `input`.
///
/// The old form also tokenized each segment's file stem separately. That could
/// never add a token: a stem is a byte prefix of its segment, the `.` after it
/// is a separator either way, and the destination is a set — so the stem's
/// tokens were always already there. The stem still matters as a *whole
/// string* for substring matching, which is collected in the caller.
fn prepare_path_like_rhs(input: &str) -> String {
    let value = input.trim().trim_matches('"').trim_matches('\'');
    let mut prepared = String::with_capacity(value.len());

    for segment in value
        .split(['/', '\\', ',', ';', ' ', '\t'])
        .filter(|part| !part.is_empty())
    {
        let cleaned = segment.trim_matches('"').trim_matches('\'').trim();
        if cleaned.is_empty() {
            continue;
        }

        prepared.push(' ');
        prepared.push_str(&prepare_for_tokenizing(cleaned));
    }

    prepared
}

fn normalize_simple(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

fn normalize_key(input: &str) -> String {
    input
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Peel 3DMigoto's stacked section-type prefixes off a section name, so
/// `[TextureOverrideAyakaBody]` contributes "AyakaBody".
///
/// Walks a byte offset into one lowercase copy instead of re-lowercasing and
/// reallocating the remainder after every strip. The prefixes are ASCII and
/// `to_ascii_lowercase` is length-preserving, so the offset is valid in both.
fn strip_section_prefixes(section: &str) -> String {
    let trimmed = section.trim();
    let lowered = trimmed.to_ascii_lowercase();
    let mut offset = 0;

    while let Some(prefix) = SECTION_PREFIX_BLACKLIST
        .iter()
        .find(|prefix| lowered[offset..].starts_with(**prefix))
    {
        offset += prefix.len();
    }

    trimmed[offset..].to_string()
}

fn looks_like_path(rhs: &str) -> bool {
    let value = rhs.trim();
    PATH_EXT_HINTS
        .iter()
        .any(|ext| contains_ignore_ascii_case(value, ext))
}

/// `str::contains`, case-insensitively, without lowercasing the haystack.
///
/// The haystack is the right-hand side of an INI assignment — often a long
/// path — and this runs on every key line of every INI file in the library.
fn contains_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn should_keep_token(token: &str, filters: &PreparedTokenFilters) -> bool {
    if token.is_empty() || token.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    if filters.stopwords.contains(token) {
        return false;
    }

    if token.len() >= 4 {
        return true;
    }

    filters.short_whitelist.contains(token)
}
