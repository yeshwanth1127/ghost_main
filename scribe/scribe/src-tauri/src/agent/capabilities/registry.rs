// Capability registry - central lookup for all capabilities

use super::{Capability, CapabilityDescriptor};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry of all available capabilities
pub struct CapabilityRegistry {
    capabilities: Arc<RwLock<HashMap<String, Arc<dyn Capability>>>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a capability
    pub async fn register(&self, capability: Arc<dyn Capability>) {
        let mut caps = self.capabilities.write().await;
        caps.insert(capability.name().to_string(), capability);
    }

    /// Get a capability by name
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Capability>> {
        let caps = self.capabilities.read().await;
        caps.get(name).map(|c| Arc::clone(c))
    }

    /// List all registered capabilities
    pub async fn list(&self) -> Vec<String> {
        let caps = self.capabilities.read().await;
        caps.keys().cloned().collect()
    }

    /// Get descriptors for all capabilities (used by LLM for planning)
    pub async fn get_all_descriptors(&self) -> Vec<CapabilityDescriptor> {
        let caps = self.capabilities.read().await;
        caps.values()
            .map(|c| c.descriptor())
            .collect()
    }

    /// Get descriptor for a specific capability
    pub async fn get_descriptor(&self, name: &str) -> Option<CapabilityDescriptor> {
        let caps = self.capabilities.read().await;
        caps.get(name).map(|c| c.descriptor())
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
