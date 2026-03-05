use crate::config::Config;
use futures::stream::BoxStream;
use futures::StreamExt;

#[derive(Clone)]
pub struct OpenRouterService {
    pub config: Config,
}

impl OpenRouterService {
    pub fn new() -> Self {
        Self {
            config: Config::from_env().expect("Failed to load config from environment (DATABASE_URL, API_ACCESS_KEY, OPENROUTER_API_KEY, OPENAI_API_KEY required)"),
        }
    }

    pub async fn chat(
        &self,
        user_message: &str,
        system_prompt: Option<&str>,
        images: Option<&serde_json::Value>,
        history: Option<&str>,
        model_id: Option<&str>,
    ) -> Result<BoxStream<'static, Result<String, reqwest::Error>>, reqwest::Error> {
        let client = reqwest::Client::new();
        
        // Select model: require client-provided model_id; if not provided and images present,
        // the client should have chosen a vision-capable model. We do not hardcode defaults.
        let model = model_id.unwrap_or("");

        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": prompt
            }));
        }

        // Add history if provided
        if let Some(hist) = history {
            if let Ok(hist_array) = serde_json::from_str::<Vec<serde_json::Value>>(hist) {
                messages.extend(hist_array);
            }
        }

        // Build user message content
        let mut user_content = Vec::new();
        user_content.push(serde_json::json!({"type": "text", "text": user_message}));
        
        if let Some(imgs) = images {
            // Handle image data
            if let Some(img_array) = imgs.as_array() {
                for img in img_array {
                    user_content.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": {"url": format!("data:image/png;base64,{}", img.as_str().unwrap_or(""))}
                    }));
                }
            } else if let Some(img_str) = imgs.as_str() {
                user_content.push(serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": format!("data:image/png;base64,{}", img_str)}
                }));
            }
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": user_content
        }));

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "reasoning": {
                "exclude": true
            }
        });

        let url = format!("{}/chat/completions", self.config.openrouter_base_url);
        let api_key_len = self.config.openrouter_api_key.len();
        eprintln!("[OpenRouter] 🌐 Request: url={}, model={}, api_key_len={}", url, model, api_key_len);
        tracing::info!(
            "🌐 OpenRouter request: url={}, model={}, api_key_len={}",
            url,
            model,
            api_key_len
        );
        if api_key_len == 0 {
            eprintln!("[OpenRouter] ❌ OPENROUTER_API_KEY is empty! Set it in scribe-api/.env");
            tracing::error!("❌ OPENROUTER_API_KEY is empty! Set it in scribe-api/.env");
        }
        tracing::info!(
            "   payload bytes={}",
            serde_json::to_string(&body).unwrap_or_default().len()
        );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.openrouter_api_key))
            .header("HTTP-Referer", "https://exora.solutions")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("❌ OpenRouter HTTP request failed: {}", e);
                e
            })?;

        let status = response.status();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        eprintln!("[OpenRouter] 📥 Response: status={}, content-type={}", status, content_type);
        tracing::info!(
            "🌐 OpenRouter response: status={}, content-type={}",
            status,
            content_type
        );

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            eprintln!("[OpenRouter] ❌ API error ({}): {}", status, error_text);
            tracing::error!("❌ OpenRouter API error (status {}): {}", status, error_text);
            return Ok(futures::stream::once(async move { Ok(error_text) }).boxed());
        }

        // Return streaming response with per-chunk logging for debugging empty responses
        let chunk_log_interval = 10;
        let chunk_counter = std::sync::atomic::AtomicU32::new(0);
        Ok(response.bytes_stream()
            .map(move |chunk| {
                chunk.map(|bytes| {
                    let idx = chunk_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    let s = String::from_utf8_lossy(&bytes).to_string();
                    if idx <= chunk_log_interval || idx % 50 == 0 {
                        let preview = s.chars().take(150).collect::<String>();
                        tracing::info!("📦 OpenRouter chunk #{}: {} bytes, preview: {:?}", idx, s.len(), preview);
                    }
                    s
                })
            })
            .boxed())
    }
}
