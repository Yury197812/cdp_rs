// orchestrator/dashboard/ws.rs - WebSocket server
use tokio::sync::broadcast;

pub struct WebSocketServer {
    broadcaster: broadcast::Sender<String>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        let (broadcaster, _) = broadcast::channel(100);
        WebSocketServer { broadcaster }
    }
    
    /// Broadcast message to all connected clients
    pub fn broadcast(&self, message: &str) {
        let _ = self.broadcaster.send(message.to_string());
    }
    
    /// Get receiver for subscribed clients
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcaster.subscribe()
    }
    
    /// Get sender for external use
    pub fn sender(&self) -> broadcast::Sender<String> {
        self.broadcaster.clone()
    }
}
