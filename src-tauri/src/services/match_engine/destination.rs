use super::inspection::normalized_match_name;
use super::types::CanonicalIdentity;
use crate::services::import_batch::types::{
    ConfidenceTier, DestinationKind, DestinationSuggestion, StableCategory,
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

pub fn resolve_destination_candidates(
    context: DestinationContext<'_>,
) -> Vec<DestinationSuggestion> {
    let source_key = normalized_match_name(context.source_name);
    let mut suggestions = Vec::new();
    let mut seen_objects = BTreeSet::new();

    if let Some(target) = context.specific_target {
        let warning = (target.category != context.category).then(|| {
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
            name_score(&source_key, target),
            warning,
        ));
    }

    if let Some(canonical) = context.canonical {
        for target in context.existing.iter().filter(|target| {
            (!context.enforce_category || target.category == context.category)
                && target.canonical_entry_key.as_deref() == Some(canonical.entry_key.as_str())
        }) {
            if seen_objects.insert(target.object_id.clone()) {
                suggestions.push(existing_suggestion(
                    target,
                    context.mods_root,
                    DestinationKind::ExistingObject,
                    96,
                    None,
                ));
            }
        }
    }

    let mut folder_matches = context
        .existing
        .iter()
        .filter_map(|target| {
            if context.enforce_category && target.category != context.category {
                return None;
            }
            let score = name_score(&source_key, target);
            (score >= 45 && !seen_objects.contains(&target.object_id)).then_some((target, score))
        })
        .collect::<Vec<_>>();
    folder_matches.sort_by(|(left, left_score), (right, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| left.name.cmp(&right.name))
    });
    for (target, score) in folder_matches {
        seen_objects.insert(target.object_id.clone());
        suggestions.push(existing_suggestion(
            target,
            context.mods_root,
            DestinationKind::ExistingObject,
            score,
            None,
        ));
    }

    if let Some(canonical) = context.canonical.filter(|identity| {
        identity.entry_kind == crate::services::scanner::deep_matcher::EntryKind::Canonical
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
                warning: None,
            });
        }
    }

    suggestions
}

fn existing_suggestion(
    target: &ExistingDestination,
    mods_root: &str,
    kind: DestinationKind,
    score: u8,
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
        confidence_percentage: score,
        confidence_tier: ConfidenceTier::from_percentage(score),
        warning,
    }
}

fn name_score(source_key: &str, target: &ExistingDestination) -> u8 {
    let mut terms = vec![target.name.as_str(), target.folder_name.as_str()];
    terms.extend(target.aliases.iter().map(String::as_str));
    terms
        .into_iter()
        .filter(|term| !term.trim().is_empty())
        .map(|raw_term| {
            let term = normalized_match_name(raw_term);
            if source_key == term {
                95
            } else if source_key.contains(&term) || term.contains(source_key) {
                82
            } else {
                deunicode::deunicode(raw_term)
                    .split(|character: char| !character.is_alphabetic())
                    .map(normalized_match_name)
                    .filter(|token| token.len() >= 3 && source_key.contains(token))
                    .map(|_| 72)
                    .max()
                    .unwrap_or(0)
            }
        })
        .max()
        .unwrap_or(0)
}
