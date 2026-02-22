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

#[derive(Debug, Clone, Default)]
pub struct IniTokenizationConfig {
    pub stopwords: Vec<String>,
    pub short_token_whitelist: Vec<String>,
    pub ini_key_blacklist: Vec<String>,
    pub ini_key_whitelist: Vec<String>,
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
    config: &IniTokenizationConfig,
) -> IniTokenBuckets {
    let stopwords = merged_stopwords(config);
    let short_whitelist = normalized_set(&config.short_token_whitelist);
    let key_blacklist = merged_key_blacklist(config);
    let key_whitelist = merged_key_whitelist(config);

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
            insert_tokens(
                &mut section_tokens,
                tokenize_structural(&cleaned_section),
                &stopwords,
                &short_whitelist,
            );
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
        if key_normalized.is_empty() || key_blacklist.contains(&key_normalized) {
            continue;
        }

        if !key_whitelist.is_empty() && !key_whitelist.contains(&key_normalized) {
            continue;
        }

        insert_tokens(
            &mut key_tokens,
            tokenize_structural(lhs),
            &stopwords,
            &short_whitelist,
        );

        if looks_like_path(rhs) {
            insert_tokens(
                &mut path_tokens,
                tokenize_path_like_rhs(rhs),
                &stopwords,
                &short_whitelist,
            );
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
    if config.ini_key_whitelist.is_empty() {
        return DEFAULT_INI_KEY_WHITELIST
            .iter()
            .map(|key| (*key).to_string())
            .collect();
    }

    let mut set = BTreeSet::new();
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

fn insert_tokens(
    destination: &mut BTreeSet<String>,
    tokens: Vec<String>,
    stopwords: &BTreeSet<String>,
    short_whitelist: &BTreeSet<String>,
) {
    for token in tokens {
        if should_keep_token(&token, stopwords, short_whitelist) {
            destination.insert(token);
        }
    }
}

fn tokenize_structural(input: &str) -> Vec<String> {
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
        .split_whitespace()
        .map(std::string::ToString::to_string)
        .collect()
}

fn tokenize_path_like_rhs(input: &str) -> Vec<String> {
    let value = input.trim().trim_matches('"').trim_matches('\'');
    let mut tokens = Vec::new();

    for segment in value
        .split(['/', '\\', ',', ';', ' ', '\t'])
        .filter(|part| !part.is_empty())
    {
        let cleaned = segment.trim_matches('"').trim_matches('\'').trim();
        if cleaned.is_empty() {
            continue;
        }

        tokens.extend(tokenize_structural(cleaned));

        if let Some((stem, _ext)) = cleaned.rsplit_once('.') {
            tokens.extend(tokenize_structural(stem));
        }
    }

    tokens
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

fn strip_section_prefixes(section: &str) -> String {
    let mut current = section.trim().to_string();
    let mut changed = true;

    while changed {
        changed = false;
        let current_lower = current.to_ascii_lowercase();
        for prefix in SECTION_PREFIX_BLACKLIST {
            if current_lower.starts_with(prefix) {
                current = current[prefix.len()..].to_string();
                changed = true;
            }
        }
    }

    current
}

fn looks_like_path(rhs: &str) -> bool {
    let value = rhs.trim().to_ascii_lowercase();
    PATH_EXT_HINTS.iter().any(|ext| value.contains(ext))
}

fn should_keep_token(
    token: &str,
    stopwords: &BTreeSet<String>,
    short_whitelist: &BTreeSet<String>,
) -> bool {
    if token.is_empty() || token.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    if stopwords.contains(token) {
        return false;
    }

    if token.len() >= 4 {
        return true;
    }

    short_whitelist.contains(token)
}
