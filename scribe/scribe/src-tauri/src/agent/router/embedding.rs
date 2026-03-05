//! Embedding for intent routing: HTTP API calls to the backend embedding service.
//!
//! **Configuration:** Set `EMBEDDING_SERVICE_URL` in `src-tauri/.env` (copy from `env.example`).
//! The app loads `.env` at startup (see lib.rs). Never commit `.env` or real API keys.
//!
//! **Contract:** POST to `{EMBEDDING_SERVICE_URL}/embed` with `{"texts": ["..."]}`;
//! response must include `{"embeddings": [[...floats...]]}`. Dimension is backend-defined.

use std::sync::{Mutex, OnceLock};

/// Global embedder: HTTP client for the backend embedding service.
static EMBEDDER: OnceLock<Option<Mutex<BackendEmbedder>>> = OnceLock::new();

/// Dedicated runtime for HTTP so we can block_on without deadlocking with Tauri's runtime.
static API_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn api_runtime() -> &'static tokio::runtime::Runtime {
    API_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("embedder API runtime")
    })
}

/// Ensure .env is loaded so EMBEDDING_SERVICE_URL is visible (e.g. if router runs before main .env load).
fn ensure_env_loaded() {
    if std::env::var("EMBEDDING_SERVICE_URL").is_ok() {
        return;
    }
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join("src-tauri").join(".env"),
            cwd.join(".env"),
        ];
        for p in &candidates {
            if p.exists() {
                let _ = dotenv::from_path(p);
                break;
            }
        }
    }
    let _ = dotenv::dotenv();
}

/// Load embedder from env: backend URL only.
fn load_embedder() -> Option<BackendEmbedder> {
    ensure_env_loaded();

    let raw = std::env::var("EMBEDDING_SERVICE_URL").ok()?;
    let base_url = raw.trim().trim_end_matches('/').to_string();
    if base_url.is_empty() {
        tracing::warn!("[router] EMBEDDING_SERVICE_URL is empty after trim");
        return None;
    }
    BackendEmbedder::new(base_url.clone()).map(|e| {
        tracing::info!("[router] embedding backend: API ({})", base_url);
        e
    })
}

/// Get the global embedder. Initializes on first call.
fn get_embedder() -> Option<&'static Mutex<BackendEmbedder>> {
    let opt = EMBEDDER.get_or_init(|| {
        match load_embedder() {
            Some(backend) => {
                tracing::info!("[router] embedding backend loaded");
                Some(Mutex::new(backend))
            }
            None => {
                tracing::warn!(
                    "[router] no embedding backend: set EMBEDDING_SERVICE_URL to your backend embedding API in .env"
                );
                None
            }
        }
    });
    opt.as_ref()
}

/// Embed a single string. Returns a vector of f32 from the backend (dimension is backend-defined).
pub fn embed(text: &str) -> Result<Vec<f32>, String> {
    let guard = match get_embedder() {
        Some(m) => m,
        None => return Err("Embedder not available".to_string()),
    };
    let mut e = guard.lock().map_err(|_| "Embedder lock poisoned".to_string())?;
    e.embed(text)
}

// ----- Backend HTTP embedder -----

/// HTTP client for the backend embedding service. POST /embed with {"texts": ["..."]}.
struct BackendEmbedder {
    client: reqwest::Client,
    embed_url: String,
}

impl BackendEmbedder {
    fn new(base_url: String) -> Option<Self> {
        if base_url.is_empty() {
            return None;
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .ok()?;
        let embed_url = format!("{}/embed", base_url);
        Some(Self { client, embed_url })
    }

    fn embed(&mut self, text: &str) -> Result<Vec<f32>, String> {
        let body = serde_json::json!({ "texts": [text] });
        let req = self
            .client
            .post(&self.embed_url)
            .header("Content-Type", "application/json")
            .json(&body);
        let res = api_runtime().block_on(async { req.send().await }).map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            let status = res.status();
            let body = api_runtime().block_on(async { res.text().await }).unwrap_or_default();
            return Err(format!("Embedding service error {}: {}", status, body));
        }
        let json: serde_json::Value = api_runtime()
            .block_on(async { res.json().await })
            .map_err(|e| e.to_string())?;
        let embeddings = json
            .get("embeddings")
            .and_then(|e| e.as_array())
            .ok_or("Embedding service response missing 'embeddings'")?;
        let first = embeddings
            .first()
            .and_then(|v| v.as_array())
            .ok_or("Embedding service response: no first embedding")?;
        let vec: Vec<f32> = first
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();
        if vec.is_empty() {
            return Err("Embedding service returned empty vector".to_string());
        }
        Ok(vec)
    }
}
