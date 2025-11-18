//! In-memory storage for audit events with efficient querying

use super::{AuditEvent, AuditEventType, AuditFilter, Severity};
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum events to keep in memory (circular buffer)
const MAX_MEMORY_EVENTS: usize = 10000;

/// In-memory audit event storage with circular buffer
pub struct AuditStorage {
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    log_path: PathBuf,
}

impl AuditStorage {
    pub fn new(log_path: PathBuf) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_MEMORY_EVENTS))),
            log_path,
        }
    }

    /// Store an event (maintains circular buffer)
    pub async fn store(&self, event: AuditEvent) {
        let mut events = self.events.write().await;

        // Remove oldest if at capacity
        if events.len() >= MAX_MEMORY_EVENTS {
            events.pop_front();
        }

        events.push_back(event);
    }

    /// Query events with filter
    pub async fn query(&self, filter: AuditFilter) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        let mut results: Vec<AuditEvent> = Vec::new();

        for event in events.iter() {
            // Apply filters
            if let Some(event_type) = &filter.event_type {
                if event.event_type != *event_type {
                    continue;
                }
            }

            if let Some(user_id) = &filter.user_id {
                if event.user_id.as_ref() != Some(user_id) {
                    continue;
                }
            }

            if let Some(min_severity) = filter.min_severity {
                if event.severity < min_severity {
                    continue;
                }
            }

            if let Some(start_time) = filter.start_time {
                if event.timestamp < start_time {
                    continue;
                }
            }

            if let Some(end_time) = filter.end_time {
                if event.timestamp > end_time {
                    continue;
                }
            }

            results.push(event.clone());

            // Check limit
            if let Some(limit) = filter.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        results
    }

    /// Get total event count
    pub async fn count(&self) -> usize {
        let events = self.events.read().await;
        events.len()
    }

    /// Clear all events
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        events.clear();
    }

    /// Get recent events (last N)
    pub async fn recent(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        let start = events.len().saturating_sub(count);
        events.iter().skip(start).cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditEventType, EventResult, Severity};

    #[tokio::test]
    async fn test_storage_circular_buffer() {
        let storage = AuditStorage::new(PathBuf::from("/tmp/test"));

        // Add more than max capacity
        for i in 0..MAX_MEMORY_EVENTS + 100 {
            let event = AuditEvent {
                id: format!("event-{}", i),
                timestamp: Utc::now(),
                event_type: AuditEventType::ToolExecute,
                severity: Severity::Info,
                user_id: Some(format!("user-{}", i)),
                session_id: None,
                source_ip: None,
                user_agent: None,
                resource: None,
                action: None,
                result: EventResult::Success,
                metadata: serde_json::json!({}),
                error: None,
                duration_ms: None,
                request_id: None,
                signature: None,
            };
            storage.store(event).await;
        }

        // Should maintain max capacity
        assert_eq!(storage.count().await, MAX_MEMORY_EVENTS);
    }

    #[tokio::test]
    async fn test_query_filtering() {
        let storage = AuditStorage::new(PathBuf::from("/tmp/test"));

        // Add test events
        for i in 0..10 {
            let event = AuditEvent {
                id: format!("event-{}", i),
                timestamp: Utc::now(),
                event_type: if i % 2 == 0 {
                    AuditEventType::AuthLogin
                } else {
                    AuditEventType::ToolExecute
                },
                severity: Severity::Info,
                user_id: Some("testuser".to_string()),
                session_id: None,
                source_ip: None,
                user_agent: None,
                resource: None,
                action: None,
                result: EventResult::Success,
                metadata: serde_json::json!({}),
                error: None,
                duration_ms: None,
                request_id: None,
                signature: None,
            };
            storage.store(event).await;
        }

        // Query for auth events only
        let filter = AuditFilter {
            event_type: Some(AuditEventType::AuthLogin),
            ..Default::default()
        };

        let results = storage.query(filter).await;
        assert_eq!(results.len(), 5); // Half of the events

        // All results should be AuthLogin
        assert!(results
            .iter()
            .all(|e| e.event_type == AuditEventType::AuthLogin));
    }
}
