//! WebSocket Support for Real-time GUI Updates
//!
//! Provides WebSocket-based event streaming for real-time dashboard updates.
//! Clients can subscribe to events and receive live updates.

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info};

use super::GuiEvent;
use crate::error::Result;

/// WebSocket event broadcaster
pub struct GuiEventBroadcaster {
    /// Broadcast channel for GUI events
    tx: broadcast::Sender<GuiEvent>,

    /// Active subscriptions
    subscriptions: Arc<RwLock<HashSet<String>>>,
}

impl GuiEventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);

        Self {
            tx,
            subscriptions: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: GuiEvent) {
        match self.tx.send(event.clone()) {
            Ok(subscribers) => {
                if subscribers > 0 {
                    debug!("Broadcast event to {} subscribers: {:?}", subscribers, event);
                }
            }
            Err(e) => {
                error!("Failed to broadcast event: {}", e);
            }
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<GuiEvent> {
        self.tx.subscribe()
    }

    /// Get number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Add a subscription ID
    pub async fn add_subscription(&self, id: String) {
        let mut subs = self.subscriptions.write().await;
        subs.insert(id.clone());
        info!("Added subscription: {}", id);
    }

    /// Remove a subscription ID
    pub async fn remove_subscription(&self, id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(id);
        info!("Removed subscription: {}", id);
    }

    /// Get all active subscription IDs
    pub async fn get_subscriptions(&self) -> Vec<String> {
        let subs = self.subscriptions.read().await;
        subs.iter().cloned().collect()
    }
}

impl Clone for GuiEventBroadcaster {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            subscriptions: self.subscriptions.clone(),
        }
    }
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Subscribe to event types
    Subscribe { event_types: Vec<String> },

    /// Unsubscribe from event types
    Unsubscribe { event_types: Vec<String> },

    /// Ping message
    Ping,

    /// Pong response
    Pong,

    /// Event notification
    Event { event: GuiEvent },

    /// Error message
    Error { message: String },
}

/// WebSocket connection handler
pub struct WsConnection {
    /// Connection ID
    pub id: String,

    /// Event receiver
    rx: broadcast::Receiver<GuiEvent>,

    /// Subscribed event types (empty means all)
    subscribed_events: Arc<RwLock<HashSet<String>>>,
}

impl WsConnection {
    pub fn new(id: String, rx: broadcast::Receiver<GuiEvent>) -> Self {
        Self {
            id,
            rx,
            subscribed_events: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Subscribe to specific event types
    pub async fn subscribe(&self, event_types: Vec<String>) {
        let mut subscribed = self.subscribed_events.write().await;
        for event_type in event_types {
            subscribed.insert(event_type);
        }
        debug!("Connection {} subscribed to events", self.id);
    }

    /// Unsubscribe from specific event types
    pub async fn unsubscribe(&self, event_types: Vec<String>) {
        let mut subscribed = self.subscribed_events.write().await;
        for event_type in event_types {
            subscribed.remove(&event_type);
        }
        debug!("Connection {} unsubscribed from events", self.id);
    }

    /// Check if connection is subscribed to an event
    pub async fn is_subscribed(&self, event: &GuiEvent) -> bool {
        let subscribed = self.subscribed_events.read().await;

        // If no specific subscriptions, receive all events
        if subscribed.is_empty() {
            return true;
        }

        // Check if event type is subscribed
        let event_type = Self::get_event_type(event);
        subscribed.contains(&event_type)
    }

    /// Get event type as string
    fn get_event_type(event: &GuiEvent) -> String {
        match event {
            GuiEvent::ServerHealthChanged { .. } => "server_health_changed".to_string(),
            GuiEvent::MetricUpdated { .. } => "metric_updated".to_string(),
            GuiEvent::AlertTriggered { .. } => "alert_triggered".to_string(),
            GuiEvent::AuditEvent { .. } => "audit_event".to_string(),
            GuiEvent::ConfigReloaded => "config_reloaded".to_string(),
            GuiEvent::ConnectionCountChanged { .. } => "connection_count_changed".to_string(),
        }
    }

    /// Receive next event (filtered by subscription)
    pub async fn recv(&mut self) -> Option<GuiEvent> {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    if self.is_subscribed(&event).await {
                        return Some(event);
                    }
                    // Event not subscribed, continue to next
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    error!(
                        "Connection {} lagged, skipped {} events",
                        self.id, skipped
                    );
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Connection {} event channel closed", self.id);
                    return None;
                }
            }
        }
    }
}

/// WebSocket session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsStats {
    pub active_connections: usize,
    pub total_events_sent: u64,
    pub total_subscriptions: usize,
}

/// WebSocket session manager
pub struct WsSessionManager {
    broadcaster: GuiEventBroadcaster,
    total_events_sent: Arc<RwLock<u64>>,
}

impl WsSessionManager {
    pub fn new(broadcaster: GuiEventBroadcaster) -> Self {
        Self {
            broadcaster,
            total_events_sent: Arc::new(RwLock::new(0)),
        }
    }

    /// Create a new WebSocket connection
    pub async fn create_connection(&self, id: String) -> WsConnection {
        let rx = self.broadcaster.subscribe();
        self.broadcaster.add_subscription(id.clone()).await;
        WsConnection::new(id, rx)
    }

    /// Close a WebSocket connection
    pub async fn close_connection(&self, id: &str) {
        self.broadcaster.remove_subscription(id).await;
    }

    /// Broadcast an event
    pub async fn broadcast(&self, event: GuiEvent) {
        self.broadcaster.broadcast(event);
        let mut count = self.total_events_sent.write().await;
        *count += 1;
    }

    /// Get session statistics
    pub async fn get_stats(&self) -> WsStats {
        let total_events_sent = *self.total_events_sent.read().await;
        let subscriptions = self.broadcaster.get_subscriptions().await;

        WsStats {
            active_connections: self.broadcaster.subscriber_count(),
            total_events_sent,
            total_subscriptions: subscriptions.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthStatus;

    #[tokio::test]
    async fn test_broadcaster() {
        let broadcaster = GuiEventBroadcaster::new(100);

        let event = GuiEvent::ServerHealthChanged {
            server_id: "test".to_string(),
            health: HealthStatus::Healthy,
        };

        // No subscribers yet
        assert_eq!(broadcaster.subscriber_count(), 0);

        let _rx = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);

        broadcaster.broadcast(event);
    }

    #[tokio::test]
    async fn test_connection_subscription() {
        let broadcaster = GuiEventBroadcaster::new(100);
        let rx = broadcaster.subscribe();
        let conn = WsConnection::new("test-conn".to_string(), rx);

        // Subscribe to specific events
        conn.subscribe(vec!["server_health_changed".to_string()])
            .await;

        let event1 = GuiEvent::ServerHealthChanged {
            server_id: "test".to_string(),
            health: HealthStatus::Healthy,
        };
        assert!(conn.is_subscribed(&event1).await);

        let event2 = GuiEvent::ConfigReloaded;
        assert!(!conn.is_subscribed(&event2).await);
    }

    #[tokio::test]
    async fn test_session_manager() {
        let broadcaster = GuiEventBroadcaster::new(100);
        let manager = WsSessionManager::new(broadcaster);

        let conn1 = manager.create_connection("conn1".to_string()).await;
        let conn2 = manager.create_connection("conn2".to_string()).await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_subscriptions, 2);

        manager.close_connection("conn1").await;

        let stats = manager.get_stats().await;
        assert_eq!(stats.total_subscriptions, 1);
    }
}
