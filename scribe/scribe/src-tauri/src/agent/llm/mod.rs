// LLM Integration Module
//
// This module provides the ONLY LLM integration point in the system.
// Only the Planner calls the LLM.
//
// Rules:
// - LLM never executes, retries, evaluates, or mutates state
// - LLM output must be strict JSON
// - All LLM output is validated before use
// - All consequences happen via events

pub mod ollama;

pub use ollama::{check_ollama, OllamaCheck};
