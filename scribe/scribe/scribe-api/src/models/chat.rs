use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub user_message: String,
    pub system_prompt: Option<String>,
    pub image_base64: Option<serde_json::Value>, // Can be String or array
    pub history: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub success: bool,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub provider: String,
    pub name: String,
    pub id: String,
    pub model: String,
    pub description: String,
    pub modality: String,
    #[serde(rename = "isAvailable")]
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptResponse {
    pub prompt_name: String,
    pub system_prompt: String,
}
