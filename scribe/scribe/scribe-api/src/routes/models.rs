use axum::{extract::State, response::Json};

use crate::models::{ModelsResponse, SystemPromptResponse, Model};
use crate::services::AppState;

pub async fn list_models(State(state): State<AppState>) -> Result<Json<ModelsResponse>, String> {
    tracing::info!("🔥 FETCH MODELS REQUEST RECEIVED - Fetching from OpenRouter");
    
    // Fetch models from OpenRouter
    let client = reqwest::Client::new();
    let url = format!("{}/models", state.openrouter_service.config.openrouter_base_url);
    
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", state.openrouter_service.config.openrouter_api_key))
        .header("HTTP-Referer", "https://exora.solutions")
        .header("X-Title", "Ghost AI")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models from OpenRouter: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("OpenRouter API error ({}): {}", status, error_text));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenRouter response: {}", e))?;

    // Extract models from the response
    let openrouter_models = json.get("data")
        .and_then(|d| d.as_array())
        .ok_or("Invalid response format from OpenRouter")?;

    // Transform OpenRouter models to our Model format
    let models: Vec<Model> = openrouter_models
        .iter()
        .filter_map(|m| {
            let id = m.get("id")?.as_str()?.to_string();
            
            // Extract provider from ID (e.g., "anthropic/claude-3.5-sonnet" -> "anthropic")
            let provider = id.split('/').next()?.to_string();
            
            let name = m.get("name")?.as_str()?.to_string();
            let context_length = m.get("context_length").and_then(|c| c.as_i64())?;
            let pricing = m.get("pricing").and_then(|p| p.as_object())?;
            
            // Determine modality based on model capabilities
            let modality = if m.get("architecture")
                .and_then(|a| a.as_object())
                .and_then(|a| a.get("modality"))
                .and_then(|m| m.as_str()) == Some("vision")
                || id.contains("vision") || name.contains("vision")
            {
                "text,vision".to_string()
            } else {
                "text".to_string()
            };

            // Calculate description from available fields
            let description = format!(
                "Context: {} tokens | Provider: {} | Pricing: Request ${}/1M, Completion ${}/1M",
                context_length.to_string(),
                provider,
                pricing.get("prompt").and_then(|p| p.as_str()).unwrap_or("N/A"),
                pricing.get("completion").and_then(|c| c.as_str()).unwrap_or("N/A"),
            );

            // Extract short model name (e.g., "gpt-4o-mini" from "openai/gpt-4o-mini")
            let model_short_name = id.split('/').nth(1).unwrap_or(&id).to_string();
            
            Some(Model {
                provider,
                name: name.clone(),
                id: id.clone(),
                model: model_short_name,
                description,
                modality,
                is_available: m.get("permission")
                    .and_then(|p| p.as_object())
                    .map(|_| true)
                    .unwrap_or(true),
            })
        })
        .collect();

    tracing::info!("✅ Successfully fetched {} models from OpenRouter", models.len());

    Ok(Json(ModelsResponse { models }))
}

pub async fn generate_prompt() -> Json<SystemPromptResponse> {
    Json(SystemPromptResponse {
        prompt_name: "Default Prompt".to_string(),
        system_prompt: "You are a helpful AI assistant. Be concise, accurate, and friendly in your responses.".to_string(),
    })
}
