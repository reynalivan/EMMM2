use crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankProvider;
use crate::services::scanner::deep_matcher::analysis::content::FolderSignals;
use crate::services::scanner::deep_matcher::state::master_db::MasterDb;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

pub struct HttpAiRerankProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl HttpAiRerankProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: base_url
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string()),
        }
    }
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

impl AiRerankProvider for HttpAiRerankProvider {
    fn rerank(
        &self,
        request: &crate::services::scanner::deep_matcher::analysis::ai_rerank::AiRerankRequest,
        signals: &FolderSignals,
        db: &MasterDb,
    ) -> Result<std::collections::HashMap<usize, f32>, String> {
        let candidate_ids = &request.candidate_entry_ids;
        if candidate_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // Build the prompt
        let mut prompt = String::new();
        prompt.push_str("You are an AI assistant helping a mod manager categorize mod folders. ");
        prompt.push_str("Given a set of signals extracted from a folder, and a list of possible Master Database candidates, ");
        prompt.push_str("your task is to score each candidate based on how likely it is to be a match for the folder. ");
        prompt.push_str("Output ONLY a pure JSON object where the keys are the candidate IDs (`id` field) and the values are the confidence scores (float between 0.0 and 1.0).\n\n");

        prompt.push_str("## Folder Signals\n");
        prompt.push_str(&format!(
            "- Folder Name Tokens: {:?}\n",
            signals.folder_tokens
        ));
        prompt.push_str(&format!(
            "- Deep Extracted Tokens: {:?}\n",
            signals.deep_name_tokens
        ));
        prompt.push_str(&format!(
            "- INI Section Tokens: {:?}\n",
            signals.ini_section_tokens
        ));
        prompt.push_str(&format!(
            "- INI Content Tokens: {:?}\n",
            signals.ini_content_tokens
        ));

        prompt.push_str("\n## Candidates\n");
        let mut id_to_entry_id = std::collections::HashMap::new();
        for &entry_id in candidate_ids {
            let candidate = &db.entries[entry_id];
            let id_str = entry_id.to_string();
            id_to_entry_id.insert(id_str.clone(), entry_id);
            prompt.push_str(&format!(
                "- ID: {}, Name: {}, Tags: {:?}\n",
                id_str, candidate.name, candidate.tags
            ));
        }

        let payload = ChatRequest {
            model: "gpt-3.5-turbo-1106".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            temperature: 0.0,
            response_format: ResponseFormat {
                format_type: "json_object".to_string(),
            },
        };

        let res = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().unwrap_or_default();
            return Err(format!("API error {}: {}", status, text));
        }

        let chat_res: ChatResponse = res
            .json()
            .map_err(|e| format!("Failed to parse JSON response: {}", e))?;

        let content_str = chat_res
            .choices
            .first()
            .ok_or("No choices in OpenAI response")?
            .message
            .content
            .as_str();

        let string_scores: std::collections::HashMap<String, f32> =
            serde_json::from_str(content_str)
                .map_err(|e| format!("Failed to parse score map from LLM: {}", e))?;

        let mut result = std::collections::HashMap::new();
        for (string_id, score) in string_scores {
            if let Some(&entry_id) = id_to_entry_id.get(&string_id) {
                result.insert(entry_id, score.clamp(0.0, 1.0));
            }
        }

        Ok(result)
    }
}
