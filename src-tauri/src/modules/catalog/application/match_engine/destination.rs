use super::inspection::normalized_match_name;
use super::types::CanonicalIdentity;
use crate::modules::ingestion::application::import_batch::types::{
    ConfidenceTier, DestinationKind, DestinationMatchMethod, DestinationSuggestion, StableCategory,
};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistingDestination {
    pub object_id: String,
    pub name: String,
    pub folder_name: String,
    pub canonical_entry_key: Option<String>,
    pub aliases: Vec<String>,
    pub category: StableCategory,
}

impl ExistingDestination {
    pub fn new(
        object_id: impl Into<String>,
        name: impl Into<String>,
        folder_name: impl Into<String>,
        canonical_entry_key: Option<String>,
        aliases: Vec<&str>,
        category: StableCategory,
    ) -> Self {
        Self {
            object_id: object_id.into(),
            name: name.into(),
            folder_name: folder_name.into(),
            canonical_entry_key,
            aliases: aliases.into_iter().map(str::to_string).collect(),
            category,
        }
    }
}

pub struct DestinationContext<'a> {
    pub source_name: &'a str,
    pub category: StableCategory,
    pub specific_target: Option<&'a ExistingDestination>,
    pub existing: &'a [ExistingDestination],
    pub canonical: Option<&'a CanonicalIdentity>,
    pub mods_root: &'a str,
    pub enforce_category: bool,
}

#[derive(Debug, Clone, Copy)]
struct DestinationScore {
    value: u8,
    method: DestinationMatchMethod,
}

pub fn resolve_destination_candidates(
    context: DestinationContext<'_>,
) -> Vec<DestinationSuggestion> {
    resolve_destination_candidates_inner(context, false)
}

pub fn resolve_all_destination_candidates(
    context: DestinationContext<'_>,
) -> Vec<DestinationSuggestion> {
    resolve_destination_candidates_inner(context, true)
}

fn resolve_destination_candidates_inner(
    context: DestinationContext<'_>,
    include_all: bool,
) -> Vec<DestinationSuggestion> {
    let source_key = normalized_match_name(context.source_name);
    let mut suggestions = Vec::new();
    let mut seen_objects = BTreeSet::new();

    if let Some(target) = context.specific_target {
        let warning = (context.enforce_category && target.category != context.category).then(|| {
            format!(
                "Specific target category '{}' differs from confirmed category '{}'",
                target.category.as_str(),
                context.category.as_str()
            )
        });
        seen_objects.insert(target.object_id.clone());
        suggestions.push(existing_suggestion(
            target,
            context.mods_root,
            DestinationKind::SpecificTarget,
            destination_score(
                &source_key,
                target,
                context.enforce_category.then_some(context.category),
            ),
            warning,
        ));
    }

    if let Some(canonical) = context.canonical {
        for target in context.existing.iter().filter(|target| {
            target.canonical_entry_key.as_deref() == Some(canonical.entry_key.as_str())
        }) {
            if seen_objects.insert(target.object_id.clone()) {
                suggestions.push(existing_suggestion(
                    target,
                    context.mods_root,
                    DestinationKind::ExistingObject,
                    DestinationScore {
                        value: 96,
                        method: DestinationMatchMethod::CanonicalIdentity,
                    },
                    category_warning(&context, target),
                ));
            }
        }
    }

    let mut scored_existing = context
        .existing
        .iter()
        .filter(|target| !seen_objects.contains(&target.object_id))
        .map(|target| {
            (
                target,
                destination_score(
                    &source_key,
                    target,
                    context.enforce_category.then_some(context.category),
                ),
            )
        })
        .collect::<Vec<_>>();
    scored_existing.sort_by(|(left, left_score), (right, right_score)| {
        right_score
            .value
            .cmp(&left_score.value)
            .then_with(|| left.name.cmp(&right.name))
    });
    for &(target, score) in &scored_existing {
        if score.value >= 45 && seen_objects.insert(target.object_id.clone())
        {
            suggestions.push(existing_suggestion(
                target,
                context.mods_root,
                DestinationKind::ExistingObject,
                score,
                category_warning(&context, target),
            ));
        }
    }

    if let Some(canonical) = context.canonical.filter(|identity| {
        identity.entry_kind
            == crate::modules::matching::application::deep_matcher::EntryKind::Canonical
    }) {
        if !context.existing.iter().any(|target| {
            target.canonical_entry_key.as_deref() == Some(canonical.entry_key.as_str())
        }) {
            suggestions.push(DestinationSuggestion {
                kind: DestinationKind::CreateCanonical,
                object_id: None,
                canonical_entry_key: Some(canonical.entry_key.clone()),
                folder_name: canonical.name.clone(),
                target_path: Path::new(context.mods_root)
                    .join(&canonical.name)
                    .to_string_lossy()
                    .into_owned(),
                confidence_percentage: 75,
                confidence_tier: ConfidenceTier::High,
                match_method: DestinationMatchMethod::CanonicalIdentity,
                warning: None,
            });
        }
    }

    if include_all {
        for (target, score) in scored_existing {
            if !seen_objects.insert(target.object_id.clone()) {
                continue;
            }
            suggestions.push(existing_suggestion(
                target,
                context.mods_root,
                DestinationKind::ExistingObject,
                score,
                category_warning(&context, target),
            ));
        }
    }

    suggestions
}

fn category_warning(
    context: &DestinationContext<'_>,
    target: &ExistingDestination,
) -> Option<String> {
    (context.enforce_category && target.category != context.category).then(|| {
        format!(
            "Destination category '{}' differs from confirmed category '{}'",
            target.category.as_str(),
            context.category.as_str()
        )
    })
}

fn existing_suggestion(
    target: &ExistingDestination,
    mods_root: &str,
    kind: DestinationKind,
    score: DestinationScore,
    warning: Option<String>,
) -> DestinationSuggestion {
    DestinationSuggestion {
        kind,
        object_id: Some(target.object_id.clone()),
        canonical_entry_key: target.canonical_entry_key.clone(),
        folder_name: target.folder_name.clone(),
        target_path: Path::new(mods_root)
            .join(&target.folder_name)
            .to_string_lossy()
            .into_owned(),
        confidence_percentage: score.value,
        confidence_tier: ConfidenceTier::from_percentage(score.value),
        match_method: score.method,
        warning,
    }
}

fn destination_score(
    source_key: &str,
    target: &ExistingDestination,
    source_category: Option<StableCategory>,
) -> DestinationScore {
    let mut best = score_term(
        source_key,
        &target.name,
        DestinationMatchMethod::ExactName,
        DestinationMatchMethod::NameSubstring,
    );
    best = best.max_by_value(score_term(
        source_key,
        &target.folder_name,
        DestinationMatchMethod::ExactName,
        DestinationMatchMethod::NameSubstring,
    ));
    for alias in &target.aliases {
        best = best.max_by_value(score_term(
            source_key,
            alias,
            DestinationMatchMethod::ExactAlias,
            DestinationMatchMethod::AliasSubstring,
        ));
    }
    if best.value > 0 {
        if let Some(category) = source_category.filter(|category| *category != StableCategory::Other)
        {
            best.value = if target.category == category {
                best.value.saturating_add(5).min(100)
            } else {
                best.value.saturating_sub(15)
            };
        }
    }
    best
}

fn score_term(
    source_key: &str,
    raw_term: &str,
    exact_method: DestinationMatchMethod,
    substring_method: DestinationMatchMethod,
) -> DestinationScore {
    if raw_term.trim().is_empty() {
        return DestinationScore::none();
    }
    let term = normalized_match_name(raw_term);
    if source_key == term {
        return DestinationScore {
            value: 95,
            method: exact_method,
        };
    }
    if source_key.contains(&term) || term.contains(source_key) {
        return DestinationScore {
            value: 82,
            method: substring_method,
        };
    }
    if deunicode::deunicode(raw_term)
        .split(|character: char| !character.is_alphabetic())
        .map(normalized_match_name)
        .any(|token| token.len() >= 3 && source_key.contains(&token))
    {
        return DestinationScore {
            value: 72,
            method: DestinationMatchMethod::TokenSubstring,
        };
    }
    let fuzzy_similarity = strsim::jaro_winkler(source_key, &term);
    if fuzzy_similarity >= 0.55 {
        return DestinationScore {
            value: (fuzzy_similarity * 44.0).round() as u8,
            method: DestinationMatchMethod::FuzzyName,
        };
    }
    DestinationScore::none()
}

impl DestinationScore {
    fn none() -> Self {
        Self {
            value: 0,
            method: DestinationMatchMethod::NoNameMatch,
        }
    }

    fn max_by_value(self, other: Self) -> Self {
        if other.value > self.value {
            other
        } else {
            self
        }
    }
}
