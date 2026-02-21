use super::types::{MatchStatus, Reason, StagedMatchResult};

impl StagedMatchResult {
    pub fn summary(&self) -> String {
        match self.status {
            MatchStatus::AutoMatched => {
                let candidate = self.best.as_ref().or_else(|| self.candidates_topk.first());
                let Some(candidate) = candidate else {
                    return "Matched by ranked signals".to_string();
                };

                let reason_label = candidate
                    .reasons
                    .iter()
                    .min_by_key(|reason| summary_reason_priority(reason))
                    .map(summary_reason_label)
                    .unwrap_or("ranked signals");
                format!("Matched by {reason_label}")
            }
            MatchStatus::NeedsReview => {
                let first = self.best.as_ref().or_else(|| self.candidates_topk.first());
                let second = self.candidates_topk.get(1);
                match (first, second) {
                    (Some(first_candidate), Some(second_candidate)) => {
                        format!(
                            "Ambiguous: {} vs {}",
                            first_candidate.name, second_candidate.name
                        )
                    }
                    (Some(first_candidate), None) => {
                        format!("Needs review: {}", first_candidate.name)
                    }
                    (None, _) => "Needs review: no candidates".to_string(),
                }
            }
            MatchStatus::NoMatch => "No reliable match".to_string(),
        }
    }
}

fn summary_reason_priority(reason: &Reason) -> usize {
    match reason {
        Reason::HashOverlap { .. } => 0,
        Reason::AliasStrict { .. } => 1,
        Reason::DeepNameToken { .. } => 2,
        Reason::IniSectionToken { .. } => 3,
        Reason::IniContentToken { .. } => 4,
        Reason::TokenOverlap { .. } => 5,
        Reason::DirectNameSupport { .. } => 6,
        Reason::NegativeEvidence { .. } => 7,
        Reason::AiRerank { .. } => 8,
    }
}

fn summary_reason_label(reason: &Reason) -> &'static str {
    match reason {
        Reason::HashOverlap { .. } => "HashOverlap",
        Reason::AliasStrict { .. } => "AliasStrict",
        Reason::DirectNameSupport { .. } => "DirectNameSupport",
        Reason::TokenOverlap { .. } => "TokenOverlap",
        Reason::DeepNameToken { .. } => "DeepNameToken",
        Reason::IniSectionToken { .. } => "IniSectionToken",
        Reason::IniContentToken { .. } => "IniContentToken",
        Reason::AiRerank { .. } => "AiRerank",
        Reason::NegativeEvidence { .. } => "NegativeEvidence",
    }
}
