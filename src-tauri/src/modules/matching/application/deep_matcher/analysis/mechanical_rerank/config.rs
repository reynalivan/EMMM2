//! Tuning knobs for the mechanical reranker.

#[derive(Debug, Clone)]
pub struct MechanicalRerankConfig {
    pub enabled: bool,
    pub gb_file_stems: Vec<String>,
    pub gb_mod_name: Option<String>,
    pub gb_root_category: Option<String>,
    pub gb_description_keywords: Vec<String>,
    pub ai_accept_min: f32, // required min AI score (0-1) to auto-accept
    pub ai_accept_gap: f32, // gap required between #1 and #2 to auto-accept
}

impl Default for MechanicalRerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gb_file_stems: Vec::new(),
            gb_mod_name: None,
            gb_root_category: None,
            gb_description_keywords: Vec::new(),
            ai_accept_min: 0.85,
            ai_accept_gap: 0.15,
        }
    }
}
