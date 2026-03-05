// Preflight / Command Router: fast path for simple goals without LLM.

mod exec;
mod intent_embeddings;
mod parse;
mod route;
mod types;

#[cfg(feature = "embedding-router")]
mod embedding;

pub use exec::run_direct_path;
pub use parse::ParseError;
pub use route::{direct_command_to_execution, route_goal, route_goal_with_result};

#[cfg(feature = "embedding-router")]
pub use route::ensure_intent_centroids_loaded;

pub use types::{Clarification, DirectCommand, EnvContext, RouteDecision, RouterResult, WriteMode};
