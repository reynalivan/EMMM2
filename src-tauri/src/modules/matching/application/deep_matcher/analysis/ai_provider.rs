use crate::modules::matching::application::deep_matcher::analysis::ai_rerank::AiRerankProvider;
use crate::modules::matching::application::deep_matcher::analysis::content::FolderSignals;
use crate::modules::matching::application::deep_matcher::state::master_db::MasterDb;
use crate::platform::security::credential_store::CredentialStore;
use crate::shared::errors::AppError;
use crate::shared::errors::ScannerError;
use reqwest::blocking::Client;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

pub struct HttpAiRerankProvider {
    client: Client,
    api_key: SecretString,
    base_url: String,
}

impl HttpAiRerankProvider {
    pub fn from_credential_store(
        credentials: &CredentialStore,
        base_url: Option<String>,
    ) -> Result<Option<Self>, AppError> {
        Ok(credentials.get_ai_api_key()?.map(|api_key| Self {
            client: Client::new(),
            api_key,
            base_url: base_url
                .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string()),
        }))
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
        request: &crate::modules::matching::application::deep_matcher::analysis::ai_rerank::AiRerankRequest,
        signals: &FolderSignals,
        db: &MasterDb,
    ) -> Result<std::collections::HashMap<usize, f32>, ScannerError> {
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
                "- ID: {}, Name: {}, aliases: {:?}\n",
                id_str, candidate.name, candidate.aliases
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

        let bearer = SecretString::from(format!("Bearer {}", self.api_key.expose_secret()));
        let mut authorization = HeaderValue::from_bytes(bearer.expose_secret().as_bytes())
            .map_err(|_| {
                ScannerError::Validation("AI API key is not a valid HTTP header".to_string())
            })?;
        authorization.set_sensitive(true);

        let res = self
            .client
            .post(&self.base_url)
            .header(AUTHORIZATION, authorization)
            .header(CONTENT_TYPE, "application/json")
            .json(&payload)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().unwrap_or_default();
            return Err(ScannerError::Validation(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let chat_res: ChatResponse = res.json()?;

        let content_str = chat_res
            .choices
            .first()
            .ok_or_else(|| ScannerError::Validation("No choices in OpenAI response".to_string()))?
            .message
            .content
            .as_str();

        let string_scores: std::collections::HashMap<String, f32> =
            serde_json::from_str(content_str)?;

        let mut result = std::collections::HashMap::new();
        for (string_id, score) in string_scores {
            if let Some(&entry_id) = id_to_entry_id.get(&string_id) {
                result.insert(entry_id, score.clamp(0.0, 1.0));
            }
        }

        Ok(result)
    }
}
