//! Intent canonicalization - structured ToolIntent for permissions, UI, auditing.

pub mod canonicalizer;

pub use canonicalizer::canonicalize_tool_intent;
