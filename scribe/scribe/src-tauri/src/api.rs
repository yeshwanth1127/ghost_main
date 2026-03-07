use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_machine_uid::MachineUidExt;

const LOCAL_BACKEND: &str = "http://127.0.0.1:8083";

fn get_app_endpoint() -> Result<String, String> {
    // Force local backend when USE_LOCAL_BACKEND=1 or when APP_ENDPOINT points to remote
    if env::var("USE_LOCAL_BACKEND").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false) {
        return Ok(LOCAL_BACKEND.to_string());
    }

    let endpoint = env::var("APP_ENDPOINT")
        .ok()
        .or_else(|| option_env!("APP_ENDPOINT").map(String::from))
        .unwrap_or_else(|| LOCAL_BACKEND.to_string());

    let trimmed = endpoint.trim();
    // Override remote URL with local backend for development
    if trimmed.contains("ghost.exora.solutions") {
        return Ok(LOCAL_BACKEND.to_string());
    }

    Ok(trimmed.to_string())
}

fn get_api_access_key() -> Result<String, String> {
    if let Ok(key) = env::var("API_ACCESS_KEY") {
        return Ok(key);
    }

    match option_env!("API_ACCESS_KEY") {
        Some(key) => Ok(key.to_string()),
        None => Err("API_ACCESS_KEY environment variable not set. Please ensure it's set during the build process.".to_string())
    }
}

// Secure storage functions
fn get_secure_storage_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir.join("secure_storage.json"))
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct SecureStorage {
    license_key: Option<String>,
    instance_id: Option<String>,
    selected_Scribe_model: Option<String>,
}

pub async fn get_stored_credentials(
    app: &AppHandle,
) -> Result<(String, String, Option<Model>), String> {
    let storage_path = get_secure_storage_path(app)?;

    if !storage_path.exists() {
        return Err("No license found. Please activate your license first.".to_string());
    }

    let content = fs::read_to_string(&storage_path)
        .map_err(|e| format!("Failed to read storage file: {}", e))?;

    let storage: SecureStorage = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse storage file: {}", e))?;

    let license_key = storage
        .license_key
        .ok_or("License key not found".to_string())?;
    let instance_id = storage
        .instance_id
        .ok_or("Instance ID not found".to_string())?;

    let selected_model: Option<Model> = storage
        .selected_Scribe_model
        .and_then(|json_str| serde_json::from_str(&json_str).ok());

    Ok((license_key, instance_id, selected_model))
}

// Audio API Structs
#[derive(Debug, Serialize, Deserialize)]
pub struct AudioRequest {
    audio_base64: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioResponse {
    success: bool,
    transcription: Option<String>,
    error: Option<String>,
}

// Chat API Structs
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    user_message: String,
    system_prompt: Option<String>,
    image_base64: Option<serde_json::Value>, // Can be string or array
    history: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    success: bool,
    message: Option<String>,
    error: Option<String>,
}

// Model API Structs
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Model {
    provider: String,
    name: String,
    id: String,
    model: String,
    description: String,
    modality: String,
    #[serde(rename = "isAvailable")]
    is_available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelsResponse {
    models: Vec<Model>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemPromptResponse {
    prompt_name: String,
    system_prompt: String,
}

// Audio API Command
#[tauri::command]
pub async fn transcribe_audio(
    app: AppHandle,
    audio_base64: String,
) -> Result<AudioResponse, String> {
    // Get environment variables
    let app_endpoint = get_app_endpoint()?;
    let api_access_key = get_api_access_key()?;
    let machine_id: String = app.machine_uid().get_machine_uid().unwrap().id.unwrap();
    // Get stored credentials
    let (license_key, instance_id, _) = get_stored_credentials(&app).await?;

    // Prepare audio request
    let audio_request = AudioRequest { audio_base64 };

    // Make HTTP request to audio endpoint
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/audio", app_endpoint);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_access_key))
        .header("license_key", &license_key)
        .header("instance", &instance_id)
        .header("machine_id", &machine_id)
        .json(&audio_request)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("{}", e);
            if error_msg.contains("url (") {
                // Remove the URL part from the error message
                let parts: Vec<&str> = error_msg.split(" for url (").collect();
                if parts.len() > 1 {
                    format!("Failed to make audio request: {}", parts[0])
                } else {
                    format!("Failed to make audio request: {}", error_msg)
                }
            } else {
                format!("Failed to make audio request: {}", error_msg)
            }
        })?;

    // Check if the response is successful
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown server error".to_string());

        // Try to parse error as JSON to get a more specific error message
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            if let Some(error_msg) = error_json.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Server error ({}): {}", status, error_msg));
            } else if let Some(message) = error_json.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Server error ({}): {}", status, message));
            }
        }

        return Err(format!("Server error ({}): {}", status, error_text));
    }

    let audio_response: AudioResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse audio response: {}", e))?;

    Ok(audio_response)
}

// Chat API Command with Streaming
#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    user_message: String,
    system_prompt: Option<String>,
    image_base64: Option<serde_json::Value>,
    history: Option<String>,
) -> Result<String, String> {
    // Get environment variables
    let app_endpoint = get_app_endpoint()?;
    let api_access_key = get_api_access_key()?;
    let machine_id: String = app.machine_uid().get_machine_uid().unwrap().id.unwrap();
    // Get stored credentials
    let (license_key, instance_id, selected_model) = get_stored_credentials(&app).await?;
    let (provider, model) = selected_model.as_ref().map_or((None, None), |m| {
        (Some(m.provider.clone()), Some(m.model.clone()))
    });
    let provider_header = provider.clone().unwrap_or("None".to_string());
    let model_header = model.clone().unwrap_or("None".to_string());
    
    // Validate model is selected
    if provider.as_deref().unwrap_or("").is_empty() || 
       provider.as_deref() == Some("None") ||
       model.as_deref().unwrap_or("").is_empty() || 
       model.as_deref() == Some("None") {
        tracing::error!("❌ No model selected. Provider: {:?}, Model: {:?}", provider, model);
        return Err("No model selected. Please select a provider and model in settings.".to_string());
    }

    // Log request details before moving values (eprintln always visible in terminal)
    let url = format!("{}/api/v1/chat?stream=true", app_endpoint);
    eprintln!("[chat_stream] 📤 Sending to scribe-api: {} (provider={}, model={})", url, provider_header, model_header);
    tracing::info!("📤 Sending chat request to: {}", url);
    tracing::info!("   Provider: {:?}, Model: {:?}", provider, model);
    tracing::info!("   License key: {}...", &license_key[..license_key.len().min(8)]);
    tracing::info!("   User message length: {} chars", user_message.len());

    // Prepare chat request
    let chat_request = ChatRequest {
        user_message,
        system_prompt,
        image_base64,
        history,
    };

    // Make HTTP request to chat endpoint with streaming (timeout prevents indefinite hang)
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .http1_only()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    eprintln!("[chat_stream] 🌐 Sending POST (waiting for response)...");
    tracing::info!("🌐 Making HTTP POST request to: {}", url);
    tracing::info!("   Provider: {:?}, Model: {:?}", provider, model);
    tracing::info!("   Request body size: {} bytes", serde_json::to_string(&chat_request).unwrap_or_default().len());

    let response = client
        .post(&url)
        .timeout(std::time::Duration::from_secs(120))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("Accept-Encoding", "identity")
        .header("Connection", "close")
        .header("Authorization", format!("Bearer {}", api_access_key))
        .header("license_key", &license_key)
        .header("instance", &instance_id)
        .header("provider", &provider_header)
        .header("model", &model_header)
        .header("machine_id", &machine_id)
        .json(&chat_request)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("{}", e);
            eprintln!("[chat_stream] ❌ Request failed: {}", error_msg);
            tracing::error!("❌ Chat request failed: {}", error_msg);
            if error_msg.contains("Failed to connect") || error_msg.contains("Connection refused") {
                format!("Failed to connect to API server at {}. Is the server running? Error: {}", app_endpoint, error_msg)
            } else if error_msg.contains("url (") {
                // Remove the URL part from the error message
                let parts: Vec<&str> = error_msg.split(" for url (").collect();
                if parts.len() > 1 {
                    format!("Failed to make chat request: {}", parts[0])
                } else {
                    format!("Failed to make chat request: {}", error_msg)
                }
            } else {
                format!("Failed to make chat request: {}", error_msg)
            }
        })?;
    
    let status = response.status();
    eprintln!("[chat_stream] 📥 Response: status={}, content-type={:?}", status, response.headers().get("content-type").and_then(|h| h.to_str().ok()));
    tracing::info!("📥 Received response status: {}", status);
    
    // Check content type first - API might return JSON error even with 200 status
    let content_type = response.headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    tracing::info!("📥 Response content-type: {}", content_type);
    
    // Log all response headers for debugging
    tracing::info!("📥 Response headers:");
    for (name, value) in response.headers() {
        let name_str = name.as_str();
        if let Ok(value_str) = value.to_str() {
            if name_str.to_lowercase().contains("content") || name_str.to_lowercase().contains("x-") {
                tracing::info!("   {}: {}", name_str, value_str);
            }
        }
    }

    // Check if the response is successful
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown server error".to_string());

        eprintln!("[chat_stream] ❌ Server error {}: {}", status, error_text);
        tracing::error!("❌ Chat API returned error status {}: {}", status, error_text);

        // Try to parse error as JSON to get a more specific error message
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            if let Some(error_msg) = error_json.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Server error ({}): {}", status, error_msg));
            } else if let Some(message) = error_json.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Server error ({}): {}", status, message));
            }
        }

        return Err(format!("Server error ({}): {}", status, error_text));
    }
    
    // Check if response is JSON (error) instead of SSE stream
    if content_type.contains("application/json") {
        // This is an error response, not SSE - read it as JSON
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("[chat_stream] ❌ API returned JSON (not SSE): {}", error_text);
        tracing::error!("❌ API returned JSON error (not SSE stream): {}", error_text);
        
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            if let Some(error_msg) = error_json.get("error").and_then(|e| e.as_str()) {
                return Err(error_msg.to_string());
            }
        }
        return Err(format!("API error: {}", error_text));
    }
    
    if !content_type.contains("text/event-stream") {
        tracing::warn!("⚠️ Unexpected content-type: {}, expected text/event-stream", content_type);
    }

    // Handle streaming response
    eprintln!("[chat_stream] 📡 Reading SSE stream...");
    tracing::info!("📡 Starting to read SSE stream...");
    let mut stream = response.bytes_stream();
    let mut full_response = String::new();
    let mut buffer = String::new();
    let mut raw_bytes = String::new();
    let mut chunk_count = 0;
    let mut total_bytes = 0;
    let mut raw_lines: Vec<String> = Vec::new();

    let mut iteration_count = 0;
    let mut process_line = |line: &str,
                            chunk_count: &mut usize,
                            full_response: &mut String,
                            raw_lines: &mut Vec<String>| {
        let trimmed_line = line.trim();

        if trimmed_line.starts_with("data:") {
            let json_str = trimmed_line
                .strip_prefix("data:")
                .unwrap_or("")
                .trim();

            // Skip OPENROUTER PROCESSING comments/error wrappers - never display in chat
            if json_str.contains(r#""error":": OPENROUTER PROCESSING""#)
                || json_str.contains(r#""error":" OPENROUTER PROCESSING""#)
                || json_str.trim() == ": OPENROUTER PROCESSING"
            {
                return;
            }

            // Emit raw SSE line for debugging only (not as chat content)
            if !json_str.is_empty() {
                if let Err(e) = app.emit("chat_stream_raw_line", json_str) {
                    tracing::error!("❌ Failed to emit chat_stream_raw_line: {}", e);
                }
                raw_lines.push(json_str.to_string());
            }

            if json_str == "[DONE]" {
                return;
            }

            if !json_str.is_empty() {
                // Try to parse the JSON and extract content
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // Debug: Log the structure of incoming JSON
                    let json_preview = serde_json::to_string(&parsed)
                        .unwrap_or_default()
                        .chars()
                        .take(300)
                        .collect::<String>();
                    tracing::info!("🔍 Parsed SSE JSON structure: {}", json_preview);
                    
                    // Heuristic extractor: try common paths then fall back to a recursive search
                    fn find_text(v: &serde_json::Value) -> Option<String> {
                        // Try known shapes quickly (OpenAI, Nemotron, etc.)
                        if let Some(delta) = v.get("choices")
                            .and_then(|c| c.as_array()).and_then(|a| a.get(0))
                            .and_then(|c0| c0.get("delta"))
                        {
                            // Emit only content, NOT reasoning (reasoning = model's internal thinking, hide from user)
                            if let Some(s) = delta.get("content") {
                                if let Some(text) = s.as_str() {
                                    if !text.is_empty() {
                                        return Some(text.to_string());
                                    }
                                }
                            }
                            if let Some(arr) = delta.get("content").and_then(|c| c.as_array()) {
                                if let Some(text) = arr.get(0)
                                    .and_then(|p| p.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    return Some(text.to_string());
                                }
                            }
                        }
                        if let Some(s) = v.get("delta").and_then(|d| d.get("text")).and_then(|t| t.as_str()) {
                            return Some(s.to_string());
                        }
                        if let Some(s) = v.get("text").and_then(|t| t.as_str()) { return Some(s.to_string()); }
                        if let Some(s) = v.get("choices")
                            .and_then(|c| c.as_array()).and_then(|a| a.get(0))
                            .and_then(|c0| c0.get("message"))
                            .and_then(|m| m.get("content"))
                        {
                            if let Some(text) = s.as_str() {
                                return Some(text.to_string());
                            }
                            if let Some(arr) = s.as_array() {
                                if let Some(text) = arr.get(0)
                                    .and_then(|p| p.get("text"))
                                    .and_then(|t| t.as_str())
                                {
                                    return Some(text.to_string());
                                }
                            }
                        }
                        if let Some(s) = v.get("candidates")
                            .and_then(|c| c.as_array()).and_then(|a| a.get(0))
                            .and_then(|c0| c0.get("content"))
                            .and_then(|cnt| cnt.get("parts"))
                            .and_then(|p| p.as_array()).and_then(|a| a.get(0))
                            .and_then(|p0| p0.get("text"))
                            .and_then(|t| t.as_str())
                        { return Some(s.to_string()); }

                        // Recursive search: prefer keys commonly used for text
                        fn dfs(val: &serde_json::Value) -> Option<String> {
                            match val {
                                serde_json::Value::String(s) => {
                                    if !s.is_empty() { return Some(s.clone()); }
                                    None
                                }
                                serde_json::Value::Object(map) => {
                                    // Prioritize likely keys
                                    // Exclude "reasoning" - it's internal thinking, not user-facing content
                                    let preferred = [
                                        "content", "text", "delta", "output_text", "generated_text", "message"
                                    ];
                                    for k in preferred {
                                        if let Some(v) = map.get(k) {
                                            if let Some(s) = dfs(v) { return Some(s); }
                                        }
                                    }
                                    // Fallback: any key
                                    for (_k, v) in map {
                                        if let Some(s) = dfs(v) { return Some(s); }
                                    }
                                    None
                                }
                                serde_json::Value::Array(arr) => {
                                    for v in arr {
                                        if let Some(s) = dfs(v) { return Some(s); }
                                    }
                                    None
                                }
                                _ => None
                            }
                        }
                        dfs(v)
                    }

                    if let Some(content) = find_text(&parsed) {
                        if !content.is_empty() {
                            *chunk_count += 1;
                            full_response.push_str(&content);
                            tracing::info!("📤 Emitting chunk #{}: {} chars, content: {:?}", 
                                *chunk_count, content.len(), content.chars().take(100).collect::<String>());
                            let _ = app.emit("chat_stream_chunk", &content);
                        } else {
                            tracing::warn!("⚠️ Parsed SSE but content empty. json_len={}", json_str.len());
                        }
                    } else {
                        tracing::warn!("⚠️ Parsed SSE but no content found. json_preview: {}", 
                            serde_json::to_string(&parsed).unwrap_or_default().chars().take(200).collect::<String>());
                    }
                } else {
                    tracing::warn!(
                        "⚠️ Failed to parse SSE JSON. json_len={}",
                        json_str.len()
                    );
                }
            }
        }
    };
    let mut process_buffer = |buffer: &mut String,
                              chunk_count: &mut usize,
                              full_response: &mut String,
                              raw_lines: &mut Vec<String>| {
        loop {
            if let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                let rest = buffer[pos + 1..].to_string();
                *buffer = rest;
                process_line(&line, chunk_count, full_response, raw_lines);
                continue;
            }

            let trimmed = buffer.trim();
            if trimmed.starts_with("data:") {
                let candidate = trimmed.strip_prefix("data:").unwrap_or("").trim();
                if candidate == "[DONE]" || serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                    let line = trimmed.to_string();
                    buffer.clear();
                    process_line(&line, chunk_count, full_response, raw_lines);
                }
            }
            break;
        }
    };

    while let Some(chunk_result) = stream.next().await {
        iteration_count += 1;
        match chunk_result {
            Ok(bytes) => {
                if iteration_count == 1 {
                    eprintln!("[chat_stream] 📦 First chunk received ({} bytes)", bytes.len());
                }
                total_bytes += bytes.len();
                let chunk_str = String::from_utf8_lossy(&bytes);
                tracing::info!("📦 Iteration {}: Received {} bytes of raw data", iteration_count, bytes.len());
                if bytes.len() > 0 {
                    tracing::info!("   First 100 chars: {}", chunk_str.chars().take(100).collect::<String>());
                }
                buffer.push_str(&chunk_str);
                raw_bytes.push_str(&chunk_str);
                process_buffer(&mut buffer, &mut chunk_count, &mut full_response, &mut raw_lines);
            }
            Err(e) => {
                eprintln!("[chat_stream] ❌ Stream error: {}", e);
                tracing::error!("❌ Stream error: {}", e);
                if !full_response.trim().is_empty() {
                    tracing::warn!(
                        "⚠️ Stream decode error after partial content ({} chars); returning partial response",
                        full_response.len()
                    );
                    break;
                }

                tracing::warn!("⚠️ Stream failed before content; attempting non-streaming fallback");
                eprintln!("[chat_stream] ⚠️ Stream failed before content; trying /api/v1/chat fallback...");

                let fallback_client = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(30))
                    .http1_only()
                    .no_gzip()
                    .no_brotli()
                    .no_deflate()
                    .no_zstd()
                    .build()
                    .map_err(|builder_err| format!("Stream error: {}. Fallback client build failed: {}", e, builder_err))?;

                let fallback_url = format!("{}/api/v1/chat", app_endpoint);
                let fallback_response = fallback_client
                    .post(&fallback_url)
                    .timeout(std::time::Duration::from_secs(120))
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json")
                    .header("Accept-Encoding", "identity")
                    .header("Connection", "close")
                    .header("Authorization", format!("Bearer {}", api_access_key))
                    .header("license_key", &license_key)
                    .header("instance", &instance_id)
                    .header("provider", &provider_header)
                    .header("model", &model_header)
                    .header("machine_id", &machine_id)
                    .json(&chat_request)
                    .send()
                    .await
                    .map_err(|fallback_err| format!("Stream error: {}. Fallback failed: {}", e, fallback_err))?;

                let fallback_status = fallback_response.status();
                let fallback_text = match fallback_response.text().await {
                    Ok(text) => text,
                    Err(text_err) => {
                        let msg = format!("Stream decode failed and fallback decode failed: {}", text_err);
                        eprintln!("[chat_stream] ❌ {}", msg);
                        let _ = app.emit("chat_stream_chunk", &msg);
                        let _ = app.emit("chat_stream_complete", &msg);
                        return Err(msg);
                    }
                };

                eprintln!(
                    "[chat_stream] 📥 Fallback response: status={}, bytes={}",
                    fallback_status,
                    fallback_text.len()
                );

                if fallback_status.is_success() {
                    if let Ok(parsed) = serde_json::from_str::<ChatResponse>(&fallback_text) {
                        if let Some(message) = parsed.message {
                            full_response = message;
                            let _ = app.emit("chat_stream_chunk", &full_response);
                            break;
                        }
                    }

                    if !fallback_text.trim().is_empty() {
                        full_response = fallback_text;
                        let _ = app.emit("chat_stream_chunk", &full_response);
                        break;
                    }

                    let msg = "Fallback succeeded but returned empty body".to_string();
                    let _ = app.emit("chat_stream_chunk", &msg);
                    let _ = app.emit("chat_stream_complete", &msg);
                    return Err(msg);
                }

                return Err(format!("Stream error: {}", e));
            }
        }
    }

    // Emit completion event
    eprintln!("[chat_stream] ✅ Stream done: iterations={}, bytes={}, chunks={}", iteration_count, total_bytes, chunk_count);
    tracing::info!("✅ Stream complete. Iterations: {}, Total bytes: {}, Chunks extracted: {}, Full response length: {}", iteration_count, total_bytes, chunk_count, full_response.len());
    
    if iteration_count == 0 {
        eprintln!("[chat_stream] ⚠️ No data received - server may have returned empty stream or connection dropped");
        tracing::warn!("⚠️ Stream completed with 0 iterations - no data was received from the server");
    }
    if total_bytes == 0 {
        tracing::warn!("⚠️ Stream completed with 0 bytes - empty response from server");
    }
    // Process any remaining buffered data (no trailing newline)
    if !buffer.trim().is_empty() {
        process_buffer(&mut buffer, &mut chunk_count, &mut full_response, &mut raw_lines);
    }

    if chunk_count == 0 && total_bytes > 0 {
        tracing::warn!("⚠️ Received {} bytes but extracted 0 chunks - parsing issue?", total_bytes);
        if !raw_lines.is_empty() {
            full_response = raw_lines.join("\n");
        } else if !raw_bytes.trim().is_empty() {
            full_response = raw_bytes;
        } else if !buffer.trim().is_empty() {
            full_response = buffer;
        }
        if !full_response.is_empty() {
            let _ = app.emit("chat_stream_chunk", &full_response);
        }
    }

    // Fallback to non-streaming call if we still have nothing
    if full_response.trim().is_empty() {
        tracing::warn!("⚠️ Empty response after streaming. Falling back to non-streaming request.");
        eprintln!("[chat_stream] ⚠️ Empty stream output; trying /api/v1/chat fallback...");

        let fallback_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .http1_only()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .build()
            .map_err(|e| format!("Fallback client build failed: {}", e))?;

        let fallback_url = format!("{}/api/v1/chat", app_endpoint);
        let fallback_response = fallback_client
            .post(&fallback_url)
            .timeout(std::time::Duration::from_secs(120))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Accept-Encoding", "identity")
            .header("Connection", "close")
            .header("Authorization", format!("Bearer {}", api_access_key))
            .header("license_key", &license_key)
            .header("instance", &instance_id)
            .header("provider", &provider_header)
            .header("model", &model_header)
            .header("machine_id", &machine_id)
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| format!("Fallback request failed: {}", e))?;

        let fallback_status = fallback_response.status();
        let fallback_text = fallback_response
            .text()
            .await
            .map_err(|e| format!("Fallback decode failed: {}", e))?;

        eprintln!(
            "[chat_stream] 📥 Fallback response: status={}, bytes={}",
            fallback_status,
            fallback_text.len()
        );

        if fallback_status.is_success() {
            if let Ok(parsed) = serde_json::from_str::<ChatResponse>(&fallback_text) {
                if let Some(message) = parsed.message {
                    full_response = message;
                    let _ = app.emit("chat_stream_chunk", &full_response);
                }
            } else if !fallback_text.trim().is_empty() {
                full_response = fallback_text;
                let _ = app.emit("chat_stream_chunk", &full_response);
            }
        } else {
            tracing::error!("❌ Fallback request failed: {}", fallback_text);
        }
    }

    if full_response.trim().is_empty() {
        let msg = "No response generated from stream or fallback".to_string();
        let _ = app.emit("chat_stream_chunk", &msg);
        full_response = msg;
    }
    
    let _ = app.emit("chat_stream_complete", &full_response);

    Ok(full_response)
}

// Models API Command
#[tauri::command]
pub async fn fetch_models() -> Result<Vec<Model>, String> {
    // Get environment variables
    let app_endpoint = get_app_endpoint()?;
    let api_access_key = get_api_access_key()?;

    // Make HTTP request to models endpoint
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/models", app_endpoint);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_access_key))
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("{}", e);
            if error_msg.contains("url (") {
                // Remove the URL part from the error message
                let parts: Vec<&str> = error_msg.split(" for url (").collect();
                if parts.len() > 1 {
                    format!("Failed to make models request: {}", parts[0])
                } else {
                    format!("Failed to make models request: {}", error_msg)
                }
            } else {
                format!("Failed to make models request: {}", error_msg)
            }
        })?;

    // Check if the response is successful
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown server error".to_string());

        // Try to parse error as JSON to get a more specific error message
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            if let Some(error_msg) = error_json.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Server error ({}): {}", status, error_msg));
            } else if let Some(message) = error_json.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Server error ({}): {}", status, message));
            }
        }

        return Err(format!("Server error ({}): {}", status, error_text));
    }

    let models_response: ModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse models response: {}", e))?;

    Ok(models_response.models)
}

// Create System Prompt API Command
#[tauri::command]
pub async fn create_system_prompt(
    app: AppHandle,
    user_prompt: String,
) -> Result<SystemPromptResponse, String> {
    // Get environment variables
    let app_endpoint = get_app_endpoint()?;
    let api_access_key = get_api_access_key()?;
    let (license_key, instance_id, _) = get_stored_credentials(&app).await?;
    let machine_id: String = app.machine_uid().get_machine_uid().unwrap().id.unwrap();
    // Make HTTP request to models endpoint
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/prompt", app_endpoint);

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_access_key))
        .header("license_key", &license_key)
        .header("instance", &instance_id)
        .header("machine_id", &machine_id)
        .json(&serde_json::json!({
            "user_prompt": user_prompt
        }))
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("{}", e);
            if error_msg.contains("url (") {
                // Remove the URL part from the error message
                let parts: Vec<&str> = error_msg.split(" for url (").collect();
                if parts.len() > 1 {
                    format!("Failed to make models request: {}", parts[0])
                } else {
                    format!("Failed to make models request: {}", error_msg)
                }
            } else {
                format!("Failed to make models request: {}", error_msg)
            }
        })?;

    // Check if the response is successful
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown server error".to_string());

        // Try to parse error as JSON to get a more specific error message
        if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(&error_text) {
            if let Some(error_msg) = error_json.get("error").and_then(|e| e.as_str()) {
                return Err(format!("Server error ({}): {}", status, error_msg));
            } else if let Some(message) = error_json.get("message").and_then(|m| m.as_str()) {
                return Err(format!("Server error ({}): {}", status, message));
            }
        }

        return Err(format!("Server error ({}): {}", status, error_text));
    }

    let system_prompt_response: SystemPromptResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse system prompt response: {}", e))?;

    Ok(system_prompt_response)
}

// Helper command to check if license is available
#[tauri::command]
pub async fn check_license_status(app: AppHandle) -> Result<bool, String> {
    match get_stored_credentials(&app).await {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}
