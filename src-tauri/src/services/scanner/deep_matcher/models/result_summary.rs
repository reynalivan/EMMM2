use crate::services::scanner::deep_matcher::{
    Candidate, Confidence, MatchStatus, Reason, StagedMatchResult,
};

impl StagedMatchResult {
    /// Human-friendly summary for the frontend. Hides technical details.
    pub fn summary(&self) -> String {
        match self.status {
            MatchStatus::AutoMatched => {
                let candidate = self.best.as_ref().or_else(|| self.candidates_topk.first());
                let Some(candidate) = candidate else {
                    return "Strong match found".to_string();
                };

                let reason_msg = candidate
                    .reasons
                    .iter()
                    .min_by_key(|reason| summary_reason_priority(reason))
                    .map(format_reason_message)
                    .unwrap_or_else(|| "Strong match found".to_string());

                reason_msg
            }
            MatchStatus::NeedsReview => {
                let first = self.best.as_ref().or_else(|| self.candidates_topk.first());
                let second = self.candidates_topk.get(1);
                match (first, second) {
                    (Some(_), Some(_)) => "Multiple possible matches found".to_string(),
                    (Some(first_candidate), None) => {
                        format!("Possible match: {}", first_candidate.name)
                    }
                    (None, _) => "No strong matches found".to_string(),
                }
            }
            MatchStatus::NoMatch => "No reliable match found".to_string(),
        }
    }

    /// Compute a 0–100 confidence score percentage based on the candidate score
    /// and the assigned confidence tier.
    pub fn confidence_score(&self) -> u8 {
        let candidate = self.best.as_ref().or_else(|| self.candidates_topk.first());
        match self.status {
            MatchStatus::NoMatch => 0,
            MatchStatus::NeedsReview => {
                let Some(c) = candidate else { return 10 };
                score_to_percentage(c)
            }
            MatchStatus::AutoMatched => {
                let Some(c) = candidate else { return 80 };
                score_to_percentage(c)
            }
        }
    }
}

/// Map a candidate's raw score + confidence tier to a 0–100 percentage.
/// Uses confidence tier as a floor/ceiling guide, then interpolates
/// based on the raw score within that tier's expected range.
pub(crate) fn score_to_percentage(candidate: &Candidate) -> u8 {
    // Confidence tier boundaries (min%, max%)
    let (floor, ceiling) = match candidate.confidence {
        Confidence::Excellent => (90, 100),
        Confidence::High => (75, 95),
        Confidence::Medium => (45, 74),
        Confidence::Low => (15, 44),
        Confidence::None => (0, 14),
    };

    // Raw score normalization: 0..35 -> 0.0..1.0
    let raw_ratio = (candidate.score / 35.0).clamp(0.0, 1.0);
    let pct = floor as f32 + raw_ratio * (ceiling - floor) as f32;
    (pct as u8).clamp(floor, ceiling)
}

fn summary_reason_priority(reason: &Reason) -> usize {
    match reason {
        Reason::HashOverlap { .. } => 0,
        Reason::AliasStrict { .. } => 1,
        Reason::SubstringName { .. } => 2,
        Reason::DeepNameToken { .. } => 3,
        Reason::IniSectionToken { .. } => 4,
        Reason::IniContentToken { .. } => 5,
        Reason::TokenOverlap { .. } => 6,
        Reason::DirectNameSupport { .. } => 7,
        Reason::FolderNameRescue { .. } => 8,
        Reason::NegativeEvidence { .. } => 9,
        Reason::AiRerank { .. } => 10,
    }
}

/// Human-friendly reason messages — no technical jargon.
fn format_reason_message(reason: &Reason) -> String {
    match reason {
        Reason::HashOverlap { overlap, .. } => {
            if *overlap >= 3 {
                "Verified by multiple file signatures".to_string()
            } else {
                "Verified by file signature".to_string()
            }
        }
        Reason::AliasStrict { .. } => "Exact name match".to_string(),
        Reason::SubstringName { .. } => "Name pattern detected".to_string(),
        Reason::DirectNameSupport { .. } => "Name match detected".to_string(),
        Reason::TokenOverlap { .. } => "Strong similarity detected".to_string(),
        Reason::DeepNameToken { .. } => "Matched from folder contents".to_string(),
        Reason::IniSectionToken { .. } => "Matched from config data".to_string(),
        Reason::IniContentToken { .. } => "Matched from config data".to_string(),
        Reason::FolderNameRescue { .. } => "Matched from folder name".to_string(),
        Reason::AiRerank { .. } => "AI-assisted match".to_string(),
        Reason::NegativeEvidence { .. } => "Resolved with additional analysis".to_string(),
    }
}
