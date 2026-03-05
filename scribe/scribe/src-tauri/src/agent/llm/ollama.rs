// Ollama Client - Pure Infrastructure
//
// This is a simple HTTP client for Ollama's chat API.
// No business logic here - just network I/O.
// Uses OLLAMA_URL (default http://localhost:11434) and OLLAMA_MODEL for the planner.

use serde_json::json;

fn default_ollama_url() -> String {
    std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Result of checking Ollama availability and configured model.
#[derive(serde::Serialize)]
pub struct OllamaCheck {
    pub ok: bool,
    pub configured_url: String,
    pub configured_model: String,
    pub available_models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Check Ollama: ping base URL and list available models. Use for UI hints when 404.
pub async fn check_ollama() -> OllamaCheck {
    let configured_url = default_ollama_url().trim_end_matches('/').to_string();
    let configured_model =
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2".to_string());
    let tags_url = format!("{}/api/tags", configured_url);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return OllamaCheck {
                ok: false,
                configured_url: configured_url.clone(),
                configured_model: configured_model.clone(),
                available_models: vec![],
                error: Some(format!("Failed to create HTTP client: {}", e)),
            };
        }
    };

    let resp = match client.get(&tags_url).send().await {
        Ok(r) => r,
        Err(e) => {
            return OllamaCheck {
                ok: false,
                configured_url: configured_url.clone(),
                configured_model: configured_model.clone(),
                available_models: vec![],
                error: Some(format!(
                    "Ollama unreachable at {}. Is Ollama running? Error: {}",
                    configured_url, e
                )),
            };
        }
    };

    if !resp.status().is_success() {
        return OllamaCheck {
            ok: false,
            configured_url: configured_url.clone(),
            configured_model: configured_model.clone(),
            available_models: vec![],
            error: Some(format!(
                "Ollama returned {} at {}. Is Ollama running?",
                resp.status(),
                configured_url
            )),
        };
    }

    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(e) => {
            return OllamaCheck {
                ok: false,
                configured_url: configured_url.clone(),
                configured_model: configured_model.clone(),
                available_models: vec![],
                error: Some(format!("Failed to parse Ollama response: {}", e)),
            };
        }
    };

    let models: Vec<String> = json["models"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let model_available = models
        .iter()
        .any(|n| n == &configured_model || n.starts_with(&format!("{}:", configured_model)));

    let error_msg = if model_available {
        None
    } else if models.is_empty() {
        Some(format!(
            "Model '{}' not found. Run: ollama pull {} — and ensure Ollama is running at {}.",
            configured_model, configured_model, configured_url
        ))
    } else {
        Some(format!(
            "Model '{}' not found. Run: ollama pull {} — or set OLLAMA_MODEL to one of: {}",
            configured_model,
            configured_model,
            models.join(", ")
        ))
    };

    OllamaCheck {
        ok: model_available,
        configured_url: configured_url.clone(),
        configured_model: configured_model.clone(),
        available_models: models,
        error: error_msg,
    }
}

#[derive(Clone)]
pub struct OllamaClient {
    pub model: String,
    pub endpoint: String,
    pub base_url: String,
}

impl OllamaClient {
    /// Create client with model name. Base URL is read from OLLAMA_URL (default http://localhost:11434).
    pub fn new(model: &str) -> Self {
        let base_url = default_ollama_url().trim_end_matches('/').to_string();
        let endpoint = format!("{}/api/chat", base_url);
        Self {
            model: model.to_string(),
            endpoint,
            base_url,
        }
    }

    /// Send a chat request to Ollama
    /// Returns the raw response content (should be JSON)
    pub async fn chat(
        &self,
        system: &str,
        user: &str,
    ) -> Result<String, String> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "stream": false
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        let resp = client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Ollama request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let hint = if status.as_u16() == 404 {
                format!(
                    " 404 Not Found — model may not be pulled or Ollama may be unreachable. \
                    Run: ollama pull {} — and ensure Ollama is running (e.g. {}). \
                    Set OLLAMA_URL if Ollama is on another host.",
                    self.model,
                    self.base_url
                )
            } else {
                format!(" {} — check OLLAMA_URL ({}) and that Ollama is running.", status, self.base_url)
            };
            return Err(format!("Ollama returned error:{}", hint));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

        json["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Invalid Ollama response: missing content".to_string())
    }
}
