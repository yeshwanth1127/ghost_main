use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{sse::Event, IntoResponse},
};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::models::{ChatRequest, UsageRecord};
use crate::services::AppState;
use crate::services::model_router::ModelRouter;

pub async fn chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatRequest>,
) -> impl IntoResponse {
    eprintln!("[CHAT] 📨 Chat request received");
    tracing::info!("📨 Chat request received");
    tracing::info!("   User message length: {} chars", request.user_message.len());
    tracing::info!("   Has system_prompt: {}", request.system_prompt.is_some());
    tracing::info!("   Has image: {}", request.image_base64.is_some());
    tracing::info!("   Has history: {}", request.history.is_some());
    
    // ============================================
    // 1. EXTRACT USER IDENTITY
    // ============================================
    let license_key = headers
        .get("x-license-key")
        .or_else(|| headers.get("license-key"))
        .or_else(|| headers.get("license_key"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let user_id = if let Some(ref key) = license_key {
        match get_user_id_from_license(&state, key).await {
            Ok(Some(id)) => Some(id),
            Ok(None) => {
                tracing::warn!("⚠️ License key not found or inactive: {}", key);
                None
            }
            Err(e) => {
                tracing::error!("❌ Error looking up license: {}", e);
                None
            }
        }
    } else {
        None
    };

    tracing::info!("   License key: {:?}, User ID: {:?}", license_key, user_id);
    
    // Build model id from headers sent by desktop (provider/model)
    let provider = headers
        .get("provider")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let model_requested = headers
        .get("model")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    
    tracing::info!("   Provider: {:?}, Model: {:?}", provider, model_requested);

    // ============================================
    // 2. USAGE TRACKING & MODEL ROUTING
    // ============================================
    let (model, _routed_by_plan) = if let Some(uid) = user_id {
        // Check token limit first
        match state.usage_service.check_token_limit(uid, 0).await {
            Ok(limit_check) => {
                if !limit_check.allowed {
                    tracing::warn!("❌ Token limit exceeded for user {}", uid);
                    return (
                        StatusCode::PAYMENT_REQUIRED,
                        Json(json!({
                            "error": format!(
                                "Token limit exceeded. You've used {}/{} tokens this month. Please upgrade your plan.",
                                limit_check.tokens_used, limit_check.token_limit
                            )
                        })),
                    )
                        .into_response();
                }
                if let Some(ref warning) = limit_check.warning {
                    tracing::info!("⚠️ Usage warning: {}", warning);
                }
            }
            Err(e) => {
                tracing::error!("❌ Error checking token limit: {}", e);
                // Continue without blocking - graceful degradation
            }
        }

        // Route model based on user plan
        let task_type = ModelRouter::classify_task(&request.user_message);
        let requested_model = if !model_requested.is_empty() && !model_requested.eq_ignore_ascii_case("none") {
            // Extract short model name if it contains provider prefix (e.g., "openai/gpt-4o-mini" -> "gpt-4o-mini")
            let short_model = if model_requested.contains('/') {
                model_requested.split('/').nth(1).unwrap_or(model_requested.as_str()).to_string()
            } else {
                model_requested.to_string()
            };
            Some(short_model)
        } else {
            None
        };

        match state.model_router.route_model(uid, requested_model, task_type).await {
            Ok(routed_model) => {
                let was_routed = Some(routed_model.as_str()) != Some(model_requested.as_str());
                tracing::info!("✅ Model routed: {} (routed_by_plan: {})", routed_model, was_routed);
                (routed_model, was_routed)
            }
            Err(e) => {
                tracing::error!("❌ Error routing model: {}", e);
                // Fall back to requested model
                (model_requested.to_string(), false)
            }
        }
    } else {
        // No user ID - use requested model or default
        (model_requested.to_string(), false)
    };

    // If model already includes provider prefix (contains '/'), use as-is for OpenRouter.
    // Otherwise, use the provider header sent from desktop to build the full model ID.
    // This ensures we support all 346 models, not just the ones with recognizable prefixes.
    let model_id = if model.contains('/') {
        Some(model.clone())
    } else if !model.is_empty() && !provider.is_empty() && !provider.eq_ignore_ascii_case("none") {
        Some(format!("{}/{}", provider, model))
    } else {
        None
    };

    if let Some(ref mid) = model_id {
        tracing::info!("Using model: {}", mid);
    } else {
        tracing::warn!("⚠️ model_id is None - provider={:?} model={:?}", provider, model);
    }

    // Validate model selection - fail fast so the desktop shows a clear error
    if model_id.as_deref().unwrap_or("").is_empty()
        || provider.eq_ignore_ascii_case("none")
        || model.eq_ignore_ascii_case("none")
    {
        tracing::warn!("❌ No model selected. Provider: {:?}, Model: {:?}", provider, model);
        return Json(json!({
            "error": "No model selected. Please pick a provider/model in settings."
        })).into_response();
    }
    
    tracing::info!("✅ Model validated: {}", model_id.as_ref().unwrap());
    
    // ============================================
    // 3. CALL AI PROVIDER
    // ============================================
    tracing::info!("🚀 Calling OpenRouter service...");
    let stream = match state
        .openrouter_service
        .chat(
            &request.user_message,
            request.system_prompt.as_deref(),
            request.image_base64.as_ref(),
            request.history.as_deref(),
            model_id.as_deref(),
        )
        .await
    {
        Ok(s) => {
            tracing::info!("✅ OpenRouter stream created successfully");
            s
        }
        Err(e) => {
            tracing::error!("❌ Failed to start chat: {}", e);
            return Json(json!({"error": format!("Failed to start chat: {}", e)})).into_response();
        }
    };
    
    tracing::info!("📡 Creating SSE event stream...");

    // ============================================
    // 4. TRACK USAGE FROM STREAM
    // ============================================
    // Usage tracking: capture token counts from the final chunk
    let usage_tracker = Arc::new(Mutex::new(UsageTracker {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
        content_chars: 0,
    }));
    let usage_tracker_clone = usage_tracker.clone();

    // Repackage OpenRouter stream into SSE `data: {json}` lines the client expects.
    // Use a line buffer to handle HTTP chunks that split across line boundaries.
    let event_stream = async_stream::stream! {
        futures::pin_mut!(stream);
        let mut chunk_count = 0;
        let mut sse_event_count = 0;
        let mut line_buffer = String::new();
        tracing::info!("📦 Starting to read OpenRouter stream...");
        
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    chunk_count += 1;
                    if chunk_count <= 5 || chunk_count % 20 == 0 {
                        tracing::info!("📦 Chat route received chunk #{}: {} bytes, starts_with: {:?}", chunk_count, chunk.len(), chunk.chars().take(80).collect::<String>());
                    }
                    line_buffer.push_str(&chunk);
                    // Process complete lines only; keep partial line in buffer
                    while let Some(pos) = line_buffer.find('\n') {
                        let line = line_buffer[..pos].trim().to_string();
                        line_buffer = line_buffer[pos + 1..].to_string();
                        if line.is_empty() { continue; }

                        // SSE comments (e.g. ": OPENROUTER PROCESSING") - ignore per spec, prevent timeout
                        if line.starts_with(':') {
                            continue;
                        }

                        // Skip error wrappers that forward the comment (some proxies/clients may emit this)
                        if line.contains(r#""error":": OPENROUTER PROCESSING""#) || line.contains(r#""error":" OPENROUTER PROCESSING""#) {
                            continue;
                        }

                        // Pass through only the JSON part of SSE lines
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str == "[DONE]" { 
                                tracing::info!("✅ Stream complete. Total chunks: {}, SSE events yielded: {}", chunk_count, sse_event_count);
                                continue; 
                            }

                            // Extract usage and content length from each chunk
                            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                                let mut tracker = usage_tracker_clone.lock().await;
                                if let Some(usage) = parsed.get("usage") {
                                    tracker.prompt_tokens = usage.get("prompt_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    tracker.completion_tokens = usage.get("completion_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    tracker.total_tokens = usage.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                                    tracing::info!("📊 Usage captured: prompt={}, completion={}, total={}", 
                                        tracker.prompt_tokens, tracker.completion_tokens, tracker.total_tokens);
                                }
                                // Accumulate content length for fallback when usage is not in stream
                                if let Some(len) = parsed
                                    .get("choices")
                                    .and_then(|c| c.as_array())
                                    .and_then(|a| a.get(0))
                                    .and_then(|c0| c0.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|c| c.as_str())
                                    .map(|s| s.len() as i32)
                                {
                                    tracker.content_chars += len;
                                }
                            }

                            sse_event_count += 1;
                            yield Ok::<Event, std::convert::Infallible>(Event::default().data(json_str.to_string()));
                        } else {
                            // Non-SSE line (e.g. unknown format) - skip, don't forward as error
                            tracing::debug!("⚠️ Skipping non-data line: {:?}", line.chars().take(80).collect::<String>());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Stream error: {}", e);
                    yield Ok::<Event, std::convert::Infallible>(Event::default().data(format!("{{\"error\": \"{}\"}}", e)));
                    break;
                }
            }
        }
        
        if chunk_count == 0 {
            tracing::warn!("⚠️ Stream completed with 0 chunks - OpenRouter may have returned empty or client disconnected early");
        } else if sse_event_count == 0 {
            tracing::warn!("⚠️ Got {} chunks but 0 SSE events yielded - chunks may not contain 'data: ' lines", chunk_count);
        } else {
            tracing::info!("✅ Chat stream finished: {} chunks, {} SSE events yielded", chunk_count, sse_event_count);
        }

        // ============================================
        // 5. RECORD USAGE AFTER STREAM COMPLETES
        // ============================================
        if let Some(uid) = user_id {
            let (prompt_tokens, completion_tokens, total_tokens) = {
                let tracker = usage_tracker.lock().await;
                if tracker.total_tokens > 0 {
                    (tracker.prompt_tokens, tracker.completion_tokens, tracker.total_tokens)
                } else if tracker.content_chars > 0 {
                    // Fallback: estimate completion tokens from content when stream has no usage
                    let estimated = (tracker.content_chars / 4).max(1);
                    tracing::info!("📊 No usage in stream; estimating {} tokens from {} chars", estimated, tracker.content_chars);
                    (0, estimated, estimated)
                } else {
                    (0, 0, 0)
                }
            };

            if total_tokens > 0 {
                tracing::info!("📊 Recording usage for user {}: {} tokens", uid, total_tokens);

                let usage_record = UsageRecord {
                    user_id: uid,
                    license_key: license_key.clone(),
                    model: model_id.clone().unwrap_or_else(|| model.clone()),
                    provider: provider.to_string(),
                    prompt_tokens,
                    completion_tokens,
                    conversation_id: None,
                    request_duration_ms: None,
                };

                match state.usage_service.record_usage_from_client(usage_record).await {
                    Ok(_) => {
                        tracing::info!("✅ Usage recorded successfully");
                    }
                    Err(e) => {
                        tracing::error!("❌ Failed to record usage: {}", e);
                    }
                }
            } else {
                tracing::warn!("⚠️ No token usage captured from stream and no content to estimate");
            }
        }
    };

    axum::response::Sse::new(event_stream).into_response()
}

// ============================================
// HELPER STRUCTURES
// ============================================

struct UsageTracker {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
    /// Accumulated completion content length (chars) for fallback estimation when usage is not in stream
    content_chars: i32,
}

// ============================================
// HELPER FUNCTIONS
// ============================================

/// Get user_id from license_key
async fn get_user_id_from_license(
    state: &AppState,
    license_key: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT user_id FROM licenses WHERE license_key = $1 AND status = 'active'"
    )
    .bind(license_key)
    .fetch_optional(&state.pool)
    .await?;

    Ok(result.flatten())
}
