//! Shared state for gateway WebSocket: desktop connection and pending agent/permission requests.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub type PushSender = mpsc::Sender<String>;

struct GatewayStateInner {
    pub desktop_tx: Option<PushSender>,
    pub pending_agent_requests: HashMap<String, PushSender>,
    /// ticket_id -> run_id for tool.permission.reply routing (Moltbot-aligned)
    pub pending_permission_run_id: HashMap<String, String>,
    /// input_request_id -> run_id for tool.input.reply routing
    pub pending_input_run_id: HashMap<String, String>,
}

#[derive(Clone)]
pub struct GatewayState(Arc<RwLock<GatewayStateInner>>);

impl GatewayState {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(GatewayStateInner {
            desktop_tx: None,
            pending_agent_requests: HashMap::new(),
            pending_permission_run_id: HashMap::new(),
            pending_input_run_id: HashMap::new(),
        })))
    }

    pub async fn register_desktop(&self, tx: PushSender) {
        let mut g = self.0.write().await;
        g.desktop_tx = Some(tx);
    }

    pub async fn unregister_desktop(&self) {
        let mut g = self.0.write().await;
        g.desktop_tx = None;
    }

    pub async fn register_agent_request(&self, request_id: &str, channels_tx: PushSender) {
        let mut g = self.0.write().await;
        g.pending_agent_requests
            .insert(request_id.to_string(), channels_tx);
    }

    pub async fn take_channels_for_agent_result(&self, request_id: &str) -> Option<PushSender> {
        let mut g = self.0.write().await;
        g.pending_agent_requests.remove(request_id)
    }

    /// Get channels tx for a request (e.g. to forward tool.permission.requested) without removing.
    pub async fn get_channels_tx_for_agent_request(&self, request_id: &str) -> Option<PushSender> {
        let g = self.0.read().await;
        g.pending_agent_requests.get(request_id).cloned()
    }

    pub async fn get_desktop_tx(&self) -> Option<PushSender> {
        let g = self.0.read().await;
        g.desktop_tx.clone()
    }

    /// Store run_id for ticket_id so tool.permission.reply(ticketId, granted) can be routed to desktop.
    pub async fn store_permission_run_id(&self, ticket_id: String, run_id: String) {
        let mut g = self.0.write().await;
        g.pending_permission_run_id.insert(ticket_id, run_id);
    }

    pub async fn take_run_id_for_permission(&self, ticket_id: &str) -> Option<String> {
        let mut g = self.0.write().await;
        g.pending_permission_run_id.remove(ticket_id)
    }

    /// Store run_id for input_request_id so tool.input.reply can be routed to desktop.
    pub async fn store_input_run_id(&self, input_request_id: String, run_id: String) {
        let mut g = self.0.write().await;
        g.pending_input_run_id.insert(input_request_id, run_id);
    }

    pub async fn take_run_id_for_input(&self, input_request_id: &str) -> Option<String> {
        let mut g = self.0.write().await;
        g.pending_input_run_id.remove(input_request_id)
    }
}
