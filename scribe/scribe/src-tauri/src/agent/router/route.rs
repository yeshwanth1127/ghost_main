// Route a user goal to direct execution or planner.
// Order: cache -> parse/shortcuts -> embedding (so parseable goals never pay embed cost).

use super::parse::{parse_cli_shortcuts, parse_goal};
use super::types::{DirectCommand, EnvContext, RouteDecision, RouterResult};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use once_cell::sync::Lazy;

/// Threshold for direct path: only high-similarity matches go direct. Env ROUTER_FAST_PATH_THRESHOLD overrides.
const DEFAULT_CONFIDENCE_FAST_PATH: f32 = 0.82;
const CONFIDENCE_HINT: f32 = 0.55;

fn confidence_fast_path() -> f32 {
    std::env::var("ROUTER_FAST_PATH_THRESHOLD")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CONFIDENCE_FAST_PATH)
}

fn route_cache_capacity() -> usize {
    std::env::var("ROUTER_CACHE_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(500)
}

fn route_cache_key(normalized_goal: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    normalized_goal.hash(&mut hasher);
    hasher.finish()
}

struct RouteCache {
    map: HashMap<u64, RouterResult>,
    order: VecDeque<u64>,
    capacity: usize,
}

impl RouteCache {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity: route_cache_capacity(),
        }
    }
    fn get(&self, key: u64) -> Option<RouterResult> {
        self.map.get(&key).cloned()
    }
    fn insert(&mut self, key: u64, value: RouterResult) {
        if self.capacity == 0 {
            return;
        }
        if self.map.contains_key(&key) {
            self.map.insert(key, value);
            return;
        }
        while self.map.len() >= self.capacity {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            } else {
                break;
            }
        }
        self.map.insert(key, value);
        self.order.push_back(key);
    }
}

static ROUTE_CACHE: Lazy<RwLock<RouteCache>> = Lazy::new(|| RwLock::new(RouteCache::new()));

/// Route the goal and return full result (decision + hint + metadata for logging).
pub fn route_goal_with_result(goal: &str, _context: &EnvContext) -> RouterResult {
    let goal = goal.trim();
    if goal.is_empty() {
        return RouterResult {
            decision: RouteDecision::DeferToPlanner,
            intent_hint: None,
            predicted_intent: String::new(),
            confidence: 0.0,
            final_route: "llm".to_string(),
            fallback_reason: None,
        };
    }

    // Step 1: Check cache (hash of normalized goal).
    let normalized = goal.to_lowercase();
    let cache_key = route_cache_key(&normalized);
    {
        let cache = ROUTE_CACHE.read().unwrap();
        if let Some(cached) = cache.get(cache_key) {
            return cached;
        }
    }

    // Step 2: Try structured parse (and CLI shortcuts) before any embedding.
    let cmd = parse_cli_shortcuts(goal).or_else(|| parse_goal(goal).ok());
    if let Some(cmd) = cmd {
        if direct_command_to_execution(&cmd).is_ok() {
            let result = RouterResult {
                decision: RouteDecision::Direct(cmd),
                intent_hint: None,
                predicted_intent: String::new(),
                confidence: 1.0,
                final_route: "direct".to_string(),
                fallback_reason: None,
            };
            ROUTE_CACHE.write().unwrap().insert(cache_key, result.clone());
            return result;
        }
    }

    #[cfg(feature = "embedding-router")]
    let result = {
        match try_embedding_route(goal) {
            Ok(r) => {
                ROUTE_CACHE.write().unwrap().insert(cache_key, r.clone());
                r
            }
            Err(_) => {
                let r = RouterResult {
                    decision: RouteDecision::DeferToPlanner,
                    intent_hint: None,
                    predicted_intent: String::new(),
                    confidence: 0.0,
                    final_route: "llm".to_string(),
                    fallback_reason: Some("embedding_unavailable".to_string()),
                };
                ROUTE_CACHE.write().unwrap().insert(cache_key, r.clone());
                r
            }
        }
    };

    #[cfg(not(feature = "embedding-router"))]
    let result = {
        let r = RouterResult {
            decision: RouteDecision::DeferToPlanner,
            intent_hint: None,
            predicted_intent: String::new(),
            confidence: 0.0,
            final_route: "llm".to_string(),
            fallback_reason: Some("feature_disabled".to_string()),
        };
        ROUTE_CACHE.write().unwrap().insert(cache_key, r.clone());
        r
    };

    result
}

#[cfg(feature = "embedding-router")]
fn intent_prototype_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("INTENT_PROTOTYPES_PATH") {
        return std::path::PathBuf::from(p);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let p = cwd.join("intent_prototypes.json");
        if p.exists() {
            return p;
        }
        let p2 = cwd.join("src-tauri").join("intent_prototypes.json");
        if p2.exists() {
            return p2;
        }
    }
    std::path::PathBuf::from("intent_prototypes.json")
}

#[cfg(feature = "embedding-router")]
static INTENT_PROTOTYPES: Lazy<RwLock<Option<std::collections::HashMap<String, Vec<String>>>>> =
    Lazy::new(|| RwLock::new(None));
#[cfg(feature = "embedding-router")]
static INTENT_CENTROIDS: Lazy<RwLock<Option<std::collections::HashMap<String, Vec<f32>>>>> =
    Lazy::new(|| RwLock::new(None));

/// Precompute intent centroids at startup so the first user request does not pay embedding cost.
/// Call from app setup (e.g. spawn_blocking). Safe to call multiple times; idempotent.
#[cfg(feature = "embedding-router")]
pub fn ensure_intent_centroids_loaded() -> Result<(), String> {
    use super::embedding::embed;
    use super::intent_embeddings::{build_centroids_from_prototypes, load_prototypes_from_file};
    use std::collections::HashMap;

    {
        let mut p_guard = INTENT_PROTOTYPES.write().unwrap();
        if p_guard.is_none() {
            let path = intent_prototype_path();
            *p_guard = load_prototypes_from_file(&path).ok();
        }
    }
    let prototypes = INTENT_PROTOTYPES
        .read()
        .unwrap()
        .clone()
        .ok_or("Prototypes not loaded (check INTENT_PROTOTYPES_PATH or intent_prototypes.json)")?;

    {
        let mut c_guard = INTENT_CENTROIDS.write().unwrap();
        if c_guard.is_none() {
            let embed_fn = |s: &str| embed(s);
            *c_guard = build_centroids_from_prototypes(&prototypes, &embed_fn).ok();
        }
    }
    Ok(())
}

#[cfg(feature = "embedding-router")]
fn try_embedding_route(goal: &str) -> Result<RouterResult, String> {
    use super::embedding::embed;
    use super::intent_embeddings::{build_centroids_from_prototypes, cosine_similarity, fast_path_intents, load_prototypes_from_file};
    use std::collections::HashMap;

    {
        let mut p_guard = INTENT_PROTOTYPES.write().unwrap();
        if p_guard.is_none() {
            let path = intent_prototype_path();
            *p_guard = load_prototypes_from_file(&path).ok();
        }
    }
    let prototypes = INTENT_PROTOTYPES.read().unwrap().clone().ok_or(
        "Prototypes not loaded (check INTENT_PROTOTYPES_PATH or intent_prototypes.json)".to_string(),
    )?;

    {
        let mut c_guard = INTENT_CENTROIDS.write().unwrap();
        if c_guard.is_none() {
            let embed_fn = |s: &str| embed(s);
            *c_guard = build_centroids_from_prototypes(&prototypes, &embed_fn).ok();
        }
    }
    let guard = INTENT_CENTROIDS.read().unwrap();
    let centroids = guard.as_ref().ok_or("Centroids not available (embedding backend or prototype file missing)".to_string())?;

    let goal_embedding = embed(goal)?;
    let mut best_intent = "";
    let mut best_score = -1.0_f32;
    for (intent_name, centroid) in centroids {
        let score = cosine_similarity(&goal_embedding, centroid);
        if score > best_score {
            best_score = score;
            best_intent = intent_name;
        }
    }
    if best_intent.is_empty() {
        return Err("No intents".to_string());
    }

    let confidence = best_score;
    let fast_path_threshold = confidence_fast_path();
    let intent_hint = if confidence >= CONFIDENCE_HINT && confidence < fast_path_threshold {
        Some(best_intent.to_string())
    } else {
        None
    };

    if confidence >= fast_path_threshold && fast_path_intents().contains(&best_intent) {
        match parse_goal(goal) {
            Ok(cmd) => {
                return Ok(RouterResult {
                    decision: RouteDecision::Direct(cmd),
                    intent_hint: None,
                    predicted_intent: best_intent.to_string(),
                    confidence,
                    final_route: "direct".to_string(),
                    fallback_reason: None,
                });
            }
            Err(_) => {
                // Parse failed: defer to planner, optionally with hint
                return Ok(RouterResult {
                    decision: RouteDecision::DeferToPlanner,
                    intent_hint,
                    predicted_intent: best_intent.to_string(),
                    confidence,
                    final_route: "llm".to_string(),
                    fallback_reason: None,
                });
            }
        }
    }

    let final_route = if intent_hint.is_some() {
        "llm_with_hint"
    } else {
        "llm"
    };
    Ok(RouterResult {
        decision: RouteDecision::DeferToPlanner,
        intent_hint,
        predicted_intent: best_intent.to_string(),
        confidence,
        final_route: final_route.to_string(),
        fallback_reason: None,
    })
}

/// Route the goal. Public API: returns only the decision.
pub fn route_goal(goal: &str, context: &EnvContext) -> RouteDecision {
    route_goal_with_result(goal, context).decision
}

/// Map DirectCommand to (capability_name, inputs) for the Executor.
/// Returns Err for commands that are not yet supported (e.g. ListFiles without filesystem.list).
pub fn direct_command_to_execution(cmd: &DirectCommand) -> Result<(String, Value), String> {
    use super::types::DirectCommand as DC;

    match cmd {
        DC::WriteFile { path, content, .. } => Ok((
            "filesystem.write".to_string(),
            json!({ "path": path, "content": content }),
        )),
        DC::ReadFile { path } => Ok((
            "filesystem.read".to_string(),
            json!({ "path": path }),
        )),
        DC::CreateFile { path } => Ok((
            "filesystem.write".to_string(),
            json!({ "path": path, "content": "" }),
        )),
        DC::ListFiles { .. } => Err("filesystem.list not implemented; use planner".to_string()),
        DC::RunCommand { cmd: program, args } => Ok((
            "process.spawn".to_string(),
            json!({ "program": program, "args": args }),
        )),
    }
}
