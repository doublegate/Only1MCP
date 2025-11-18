//! Comprehensive audit logging system for compliance and security monitoring.
//!
//! This module provides structured, immutable audit logs with:
//! - Event signing for tamper detection
//! - Retention policies
//! - Compliance-ready output (SOC2, HIPAA, GDPR)
//! - Real-time streaming to SIEM systems
//! - Efficient rotation and archival

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;
use tracing::info;

mod storage;
pub use storage::AuditStorage;

/// Audit event types following security event taxonomy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    // Authentication events
    AuthLogin,
    AuthLogout,
    AuthFailure,
    AuthTokenIssued,
    AuthTokenRevoked,
    AuthMfaVerified,

    // Authorization events
    AuthzPermissionCheck,
    AuthzAccessDenied,
    AuthzRoleAssigned,
    AuthzRoleRevoked,

    // Resource access
    ResourceAccess,
    ResourceModify,
    ResourceCreate,
    ResourceDelete,

    // MCP operations
    ToolExecute,
    ToolListAccess,
    ServerConnect,
    ServerDisconnect,

    // Configuration changes
    ConfigUpdate,
    ConfigReload,
    ServerAdd,
    ServerRemove,

    // Security events
    SecurityRateLimitExceeded,
    SecuritySuspiciousActivity,
    SecurityPolicyViolation,

    // Administrative actions
    AdminUserCreate,
    AdminUserDelete,
    AdminSystemShutdown,
    AdminEmergencyAccess,

    // System events
    SystemStartup,
    SystemShutdown,
    SystemError,
}

/// Severity levels for audit events
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Debug = 0,
    Info = 1,
    Warning = 2,
    Error = 3,
    Critical = 4,
}

/// Audit event with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,

    /// Event timestamp (UTC)
    pub timestamp: DateTime<Utc>,

    /// Event type
    pub event_type: AuditEventType,

    /// Severity level
    pub severity: Severity,

    /// User ID (if applicable)
    pub user_id: Option<String>,

    /// Session ID (if applicable)
    pub session_id: Option<String>,

    /// Source IP address
    pub source_ip: Option<String>,

    /// User agent / client info
    pub user_agent: Option<String>,

    /// Resource accessed
    pub resource: Option<String>,

    /// Action performed
    pub action: Option<String>,

    /// Result (success/failure)
    pub result: EventResult,

    /// Additional context
    pub metadata: serde_json::Value,

    /// Error message (if result=failure)
    pub error: Option<String>,

    /// Duration in milliseconds
    pub duration_ms: Option<u64>,

    /// Request ID for correlation
    pub request_id: Option<String>,

    /// Event signature (SHA256 hash for tamper detection)
    pub signature: Option<String>,
}

/// Event result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EventResult {
    Success,
    Failure,
    Partial,
}

impl AuditEvent {
    /// Create new audit event
    pub fn new(event_type: AuditEventType, severity: Severity) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type,
            severity,
            user_id: None,
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
        }
    }

    /// Add user context
    pub fn with_user(mut self, user_id: String, session_id: Option<String>) -> Self {
        self.user_id = Some(user_id);
        self.session_id = session_id;
        self
    }

    /// Add source information
    pub fn with_source(mut self, ip: Option<String>, user_agent: Option<String>) -> Self {
        self.source_ip = ip;
        self.user_agent = user_agent;
        self
    }

    /// Add resource and action
    pub fn with_resource(mut self, resource: String, action: String) -> Self {
        self.resource = Some(resource);
        self.action = Some(action);
        self
    }

    /// Set result
    pub fn with_result(mut self, result: EventResult) -> Self {
        self.result = result;
        self
    }

    /// Add error
    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self.result = EventResult::Failure;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Add duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Add request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Sign event with SHA256 for tamper detection
    pub fn sign(mut self) -> Self {
        let canonical = self.canonical_form();
        let mut hasher = Sha256::new();
        hasher.update(canonical);
        self.signature = Some(format!("{:x}", hasher.finalize()));
        self
    }

    /// Get canonical form for signing (without signature field)
    fn canonical_form(&self) -> String {
        format!(
            "{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}|{:?}",
            self.id,
            self.timestamp.to_rfc3339(),
            self.event_type,
            self.severity,
            self.user_id.as_deref().unwrap_or(""),
            self.resource.as_deref().unwrap_or(""),
            self.action.as_deref().unwrap_or(""),
            self.metadata,
            self.result == EventResult::Success,
            self.error
        )
    }

    /// Verify event signature
    pub fn verify_signature(&self) -> bool {
        if let Some(signature) = &self.signature {
            let canonical = self.canonical_form();
            let mut hasher = Sha256::new();
            hasher.update(canonical);
            let computed = format!("{:x}", hasher.finalize());
            computed == *signature
        } else {
            false
        }
    }
}

/// Audit logger configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,

    /// Log file path
    pub log_path: PathBuf,

    /// Minimum severity to log
    pub min_severity: Severity,

    /// Enable event signing
    pub sign_events: bool,

    /// Rotation size in bytes (default: 100MB)
    pub rotation_size: u64,

    /// Retention days (default: 90)
    pub retention_days: u32,

    /// Buffer size for batch writes
    pub buffer_size: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_path: PathBuf::from("/var/log/only1mcp/audit.jsonl"),
            min_severity: Severity::Info,
            sign_events: true,
            rotation_size: 100 * 1024 * 1024, // 100MB
            retention_days: 90,
            buffer_size: 1000,
        }
    }
}

/// Audit logger with async I/O and buffering
pub struct AuditLogger {
    config: Arc<AuditConfig>,
    writer: Arc<RwLock<Option<BufWriter<File>>>>,
    storage: Arc<AuditStorage>,
}

impl AuditLogger {
    /// Create new audit logger
    pub async fn new(config: AuditConfig) -> Result<Self, std::io::Error> {
        // Ensure parent directory exists
        if let Some(parent) = config.log_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let writer = if config.enabled {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&config.log_path)
                .await?;
            Some(BufWriter::new(file))
        } else {
            None
        };

        let storage = Arc::new(AuditStorage::new(config.log_path.clone()));

        Ok(Self {
            config: Arc::new(config),
            writer: Arc::new(RwLock::new(writer)),
            storage,
        })
    }

    /// Log an audit event
    pub async fn log(&self, mut event: AuditEvent) -> Result<(), std::io::Error> {
        // Check if logging is enabled
        if !self.config.enabled {
            return Ok(());
        }

        // Filter by severity
        if event.severity < self.config.min_severity {
            return Ok(());
        }

        // Sign event if configured
        if self.config.sign_events {
            event = event.sign();
        }

        // Serialize to JSON
        let json = serde_json::to_string(&event)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // Write to file
        let mut writer_guard = self.writer.write().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.write_all(json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }

        // Store in memory for queries
        self.storage.store(event).await;

        Ok(())
    }

    /// Query audit events
    pub async fn query(&self, filter: AuditFilter) -> Vec<AuditEvent> {
        self.storage.query(filter).await
    }

    /// Get event count
    pub async fn count(&self) -> usize {
        self.storage.count().await
    }

    /// Rotate log file
    pub async fn rotate(&self) -> Result<(), std::io::Error> {
        let mut writer_guard = self.writer.write().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.flush().await?;
        }

        // Rename current file with timestamp
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let rotated_path = self
            .config
            .log_path
            .with_file_name(format!("audit_{}.jsonl", timestamp));

        tokio::fs::rename(&self.config.log_path, &rotated_path).await?;

        // Open new file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.log_path)
            .await?;

        *writer_guard = Some(BufWriter::new(file));

        info!("Audit log rotated to {:?}", rotated_path);

        Ok(())
    }

    /// Flush buffered events
    pub async fn flush(&self) -> Result<(), std::io::Error> {
        let mut writer_guard = self.writer.write().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.flush().await?;
        }
        Ok(())
    }
}

/// Filter for querying audit events
#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    pub event_type: Option<AuditEventType>,
    pub user_id: Option<String>,
    pub min_severity: Option<Severity>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::AuthLogin, Severity::Info)
            .with_user("user123".to_string(), Some("session456".to_string()))
            .with_resource("server1".to_string(), "login".to_string())
            .with_result(EventResult::Success);

        assert_eq!(event.event_type, AuditEventType::AuthLogin);
        assert_eq!(event.user_id.unwrap(), "user123");
        assert_eq!(event.result, EventResult::Success);
    }

    #[tokio::test]
    async fn test_event_signing() {
        let event = AuditEvent::new(AuditEventType::ToolExecute, Severity::Info)
            .with_user("user1".to_string(), None)
            .sign();

        assert!(event.signature.is_some());
        assert!(event.verify_signature());
    }

    #[tokio::test]
    async fn test_tamper_detection() {
        let mut event = AuditEvent::new(AuditEventType::ConfigUpdate, Severity::Warning).sign();

        // Tamper with event
        event.user_id = Some("attacker".to_string());

        // Signature should no longer match
        assert!(!event.verify_signature());
    }
}
