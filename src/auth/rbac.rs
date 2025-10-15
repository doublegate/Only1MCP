//! Fine-grained RBAC system with hierarchical roles and dynamic policies.
//! Supports role inheritance, attribute-based access control (ABAC),
//! and policy-based access control (PBAC).

use async_trait::async_trait;
use chrono::{DateTime, Local, Timelike, Utc, Weekday};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

// Helper macro for creating permission sets (defined at end of file)
macro_rules! hashset {
    ( $( $x:expr ),* $(,)? ) => {
        {
            let mut set = HashSet::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}

/// RBAC error types
#[derive(Debug, Error)]
pub enum AuthzError {
    #[error("User not found: {0}")]
    UserNotFound(String),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Role inheritance cycle detected: {0}")]
    RoleCycle(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Invalid context: {0}")]
    InvalidContext(String),

    #[error("Policy evaluation failed: {0}")]
    PolicyError(String),
}

/// Permission represents an action that can be performed
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    // Server permissions
    ServerCreate,
    ServerRead,
    ServerUpdate,
    ServerDelete,
    ServerExecute,

    // Tool permissions (can be wildcarded)
    ToolExecute(String), // e.g., "db_query", "*" for all
    ToolRead(String),
    ToolModify(String),

    // Admin permissions
    AdminUserManage,
    AdminRoleManage,
    AdminSystemConfig,
    AdminAuditRead,

    // Cost permissions
    CostView,
    CostAllocate,
    CostBudgetSet,

    // Special permissions
    BypassRateLimit,
    BypassCache,
    EmergencyAccess,
}

/// Role definition with inheritance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: HashSet<Permission>,
    pub inherits_from: Vec<String>, // Role inheritance
    pub constraints: RoleConstraints,
}

/// Dynamic constraints on role usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleConstraints {
    /// Time-based access (e.g., business hours only)
    pub time_restrictions: Option<TimeRestriction>,

    /// IP-based access control
    pub ip_allowlist: Option<Vec<IpNetwork>>,

    /// MFA requirement
    pub require_mfa: bool,

    /// Maximum session duration
    pub max_session_duration: Duration,

    /// Resource quotas
    pub quotas: ResourceQuotas,
}

impl Default for RoleConstraints {
    fn default() -> Self {
        Self {
            time_restrictions: None,
            ip_allowlist: None,
            require_mfa: false,
            max_session_duration: Duration::from_secs(8 * 3600), // 8 hours
            quotas: ResourceQuotas::default(),
        }
    }
}

/// Time-based access restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestriction {
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub allowed_days: Vec<Weekday>,
    pub timezone: String,
}

/// Resource quotas for rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuotas {
    pub max_requests_per_hour: Option<u32>,
    pub max_tokens_per_day: Option<u64>,
    pub max_cost_per_month: Option<f64>,
}

impl Default for ResourceQuotas {
    fn default() -> Self {
        Self {
            max_requests_per_hour: None,
            max_tokens_per_day: None,
            max_cost_per_month: None,
        }
    }
}

/// Authorization context for policy evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzContext {
    pub request_id: String,
    pub ip_address: Option<IpAddr>,
    pub user_agent: Option<String>,
    pub mfa_verified: bool,
    pub session_age: Duration,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Authorization decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny,
}

/// Authorization event for audit logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzEvent {
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub permission: Permission,
    pub context: AuthzContext,
    pub decision: Decision,
    pub reason: String,
}

/// Simple in-memory cache implementation
pub struct Cache<K, V> {
    data: Arc<RwLock<HashMap<K, V>>>,
    max_size: usize,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }

    pub async fn get(&self, key: &K) -> Option<V> {
        let cache = self.data.read().await;
        cache.get(key).cloned()
    }

    pub async fn insert(&self, key: K, value: V) {
        let mut cache = self.data.write().await;

        // Simple eviction: remove oldest if at capacity
        if cache.len() >= self.max_size {
            if let Some(first_key) = cache.keys().next().cloned() {
                cache.remove(&first_key);
            }
        }

        cache.insert(key, value);
    }
}

/// Audit logger interface
#[async_trait]
pub trait AuditLogger: Send + Sync {
    async fn log_authorization(&self, event: AuthzEvent) -> Result<(), AuthzError>;
}

/// Simple console audit logger
pub struct ConsoleAuditLogger;

#[async_trait]
impl AuditLogger for ConsoleAuditLogger {
    async fn log_authorization(&self, event: AuthzEvent) -> Result<(), AuthzError> {
        tracing::info!(
            "Authorization: user={}, permission={:?}, decision={:?}, reason={}",
            event.user_id,
            event.permission,
            event.decision,
            event.reason
        );
        Ok(())
    }
}

/// Authorization engine with caching and audit
pub struct AuthorizationEngine {
    /// Role definitions
    roles: Arc<RwLock<HashMap<String, Role>>>,

    /// User-role assignments
    assignments: Arc<RwLock<HashMap<String, Vec<String>>>>,

    /// Permission cache for performance
    permission_cache: Arc<Cache<(String, Permission), bool>>,

    /// Policy engine for dynamic rules
    policy_engine: Arc<PolicyEngine>,

    /// Audit logger
    audit: Arc<dyn AuditLogger>,
}

impl AuthorizationEngine {
    /// Create new authorization engine
    pub fn new(audit: Arc<dyn AuditLogger>) -> Self {
        Self {
            roles: Arc::new(RwLock::new(HashMap::new())),
            assignments: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(Cache::new(1000)),
            policy_engine: Arc::new(PolicyEngine::new()),
            audit,
        }
    }

    /// Add a role definition
    pub async fn add_role(&self, role: Role) -> Result<(), AuthzError> {
        let mut roles = self.roles.write().await;
        roles.insert(role.id.clone(), role);
        Ok(())
    }

    /// Assign role to user
    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> Result<(), AuthzError> {
        let roles = self.roles.read().await;
        if !roles.contains_key(role_id) {
            return Err(AuthzError::RoleNotFound(role_id.to_string()));
        }

        let mut assignments = self.assignments.write().await;
        assignments
            .entry(user_id.to_string())
            .or_insert_with(Vec::new)
            .push(role_id.to_string());

        Ok(())
    }

    /// Check if user has permission
    pub async fn authorize(
        &self,
        user_id: &str,
        permission: &Permission,
        context: &AuthzContext,
    ) -> Result<bool, AuthzError> {
        // Check cache first
        let cache_key = (user_id.to_string(), permission.clone());
        if let Some(cached) = self.permission_cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Get user's roles
        let assignments = self.assignments.read().await;
        let user_roles = assignments
            .get(user_id)
            .ok_or_else(|| AuthzError::UserNotFound(user_id.to_string()))?;

        // Check each role (including inherited)
        let roles = self.roles.read().await;
        let mut effective_permissions = HashSet::new();

        for role_id in user_roles {
            self.collect_permissions(
                &roles,
                role_id,
                &mut effective_permissions,
                &mut HashSet::new(), // Cycle detection
            )?;
        }

        // Check if permission is granted
        let mut authorized = effective_permissions.contains(permission);

        // Check wildcard permissions
        if !authorized {
            match permission {
                Permission::ToolExecute(tool) => {
                    authorized =
                        effective_permissions.contains(&Permission::ToolExecute("*".to_string()));
                },
                Permission::ToolRead(tool) => {
                    authorized =
                        effective_permissions.contains(&Permission::ToolRead("*".to_string()));
                },
                Permission::ToolModify(tool) => {
                    authorized =
                        effective_permissions.contains(&Permission::ToolModify("*".to_string()));
                },
                _ => {},
            }
        }

        // Apply dynamic policies
        if authorized {
            authorized = self
                .policy_engine
                .evaluate(user_id, permission, context, &effective_permissions)
                .await?;
        }

        // Audit the decision
        let reason = self.policy_engine.last_evaluation_reason().await;
        self.audit
            .log_authorization(AuthzEvent {
                timestamp: Utc::now(),
                user_id: user_id.to_string(),
                permission: permission.clone(),
                context: context.clone(),
                decision: if authorized { Decision::Allow } else { Decision::Deny },
                reason,
            })
            .await?;

        // Cache the result
        self.permission_cache.insert(cache_key, authorized).await;

        Ok(authorized)
    }

    /// Recursively collect permissions with inheritance
    fn collect_permissions(
        &self,
        roles: &HashMap<String, Role>,
        role_id: &str,
        permissions: &mut HashSet<Permission>,
        visited: &mut HashSet<String>,
    ) -> Result<(), AuthzError> {
        // Cycle detection
        if !visited.insert(role_id.to_string()) {
            return Err(AuthzError::RoleCycle(role_id.to_string()));
        }

        let role = roles
            .get(role_id)
            .ok_or_else(|| AuthzError::RoleNotFound(role_id.to_string()))?;

        // Add direct permissions
        permissions.extend(role.permissions.iter().cloned());

        // Process inherited roles
        for parent_id in &role.inherits_from {
            self.collect_permissions(roles, parent_id, permissions, visited)?;
        }

        Ok(())
    }

    /// Get all permissions for a user
    pub async fn get_user_permissions(
        &self,
        user_id: &str,
    ) -> Result<HashSet<Permission>, AuthzError> {
        let assignments = self.assignments.read().await;
        let user_roles = assignments
            .get(user_id)
            .ok_or_else(|| AuthzError::UserNotFound(user_id.to_string()))?;

        let roles = self.roles.read().await;
        let mut permissions = HashSet::new();

        for role_id in user_roles {
            self.collect_permissions(&roles, role_id, &mut permissions, &mut HashSet::new())?;
        }

        Ok(permissions)
    }
}

/// Policy decision
#[derive(Debug, Clone)]
pub enum PolicyDecision {
    Allow,
    Deny(String),
}

/// Policy engine for dynamic authorization rules
pub struct PolicyEngine {
    policies: Vec<Box<dyn Policy>>,
    last_reason: Arc<RwLock<String>>,
}

impl PolicyEngine {
    /// Create new policy engine
    pub fn new() -> Self {
        Self {
            policies: vec![
                Box::new(TimeBasedPolicy::new()),
                Box::new(IpAllowlistPolicy::new()),
                Box::new(MfaPolicy::new()),
            ],
            last_reason: Arc::new(RwLock::new("No evaluation performed".to_string())),
        }
    }

    /// Evaluate all policies
    pub async fn evaluate(
        &self,
        user_id: &str,
        permission: &Permission,
        context: &AuthzContext,
        user_permissions: &HashSet<Permission>,
    ) -> Result<bool, AuthzError> {
        let mut reason = String::from("Allowed by permissions");

        for policy in &self.policies {
            match policy.evaluate(user_id, permission, context, user_permissions).await? {
                PolicyDecision::Allow => continue,
                PolicyDecision::Deny(policy_reason) => {
                    reason = policy_reason;
                    *self.last_reason.write().await = reason.clone();
                    return Ok(false);
                },
            }
        }

        *self.last_reason.write().await = reason;
        Ok(true)
    }

    /// Get the reason for the last evaluation
    pub async fn last_evaluation_reason(&self) -> String {
        self.last_reason.read().await.clone()
    }
}

#[async_trait]
pub trait Policy: Send + Sync {
    async fn evaluate(
        &self,
        user_id: &str,
        permission: &Permission,
        context: &AuthzContext,
        user_permissions: &HashSet<Permission>,
    ) -> Result<PolicyDecision, AuthzError>;
}

/// Time-based access policy
pub struct TimeBasedPolicy {
    restrictions: HashMap<String, TimeRestriction>,
}

impl TimeBasedPolicy {
    pub fn new() -> Self {
        Self {
            restrictions: HashMap::new(),
        }
    }
}

#[async_trait]
impl Policy for TimeBasedPolicy {
    async fn evaluate(
        &self,
        user_id: &str,
        _permission: &Permission,
        _context: &AuthzContext,
        _user_permissions: &HashSet<Permission>,
    ) -> Result<PolicyDecision, AuthzError> {
        if let Some(restriction) = self.restrictions.get(user_id) {
            let now = Local::now();
            let current_time = now.time();
            let current_day = now.weekday();

            // Check business hours
            if current_time < restriction.start_time || current_time > restriction.end_time {
                return Ok(PolicyDecision::Deny(
                    "Access outside business hours".to_string(),
                ));
            }

            // Check weekdays
            if !restriction.allowed_days.contains(&current_day) {
                return Ok(PolicyDecision::Deny(
                    "Access not allowed on this day".to_string(),
                ));
            }
        }

        Ok(PolicyDecision::Allow)
    }
}

/// IP allowlist policy
pub struct IpAllowlistPolicy;

impl IpAllowlistPolicy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Policy for IpAllowlistPolicy {
    async fn evaluate(
        &self,
        _user_id: &str,
        _permission: &Permission,
        context: &AuthzContext,
        _user_permissions: &HashSet<Permission>,
    ) -> Result<PolicyDecision, AuthzError> {
        // Check if IP restrictions apply
        if let Some(_ip) = context.ip_address {
            // Implementation would check against allowlist
            // For now, allow all
        }

        Ok(PolicyDecision::Allow)
    }
}

/// MFA requirement policy
pub struct MfaPolicy;

impl MfaPolicy {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Policy for MfaPolicy {
    async fn evaluate(
        &self,
        _user_id: &str,
        permission: &Permission,
        context: &AuthzContext,
        _user_permissions: &HashSet<Permission>,
    ) -> Result<PolicyDecision, AuthzError> {
        // Check if permission requires MFA
        let requires_mfa = matches!(
            permission,
            Permission::AdminUserManage
                | Permission::AdminRoleManage
                | Permission::AdminSystemConfig
                | Permission::EmergencyAccess
        );

        if requires_mfa && !context.mfa_verified {
            return Ok(PolicyDecision::Deny(
                "MFA verification required for this action".to_string(),
            ));
        }

        Ok(PolicyDecision::Allow)
    }
}

/// Create default roles
pub fn create_default_roles() -> Vec<Role> {
    vec![
        Role {
            id: "admin".to_string(),
            name: "Administrator".to_string(),
            description: "Full system access".to_string(),
            permissions: hashset![
                Permission::ServerCreate,
                Permission::ServerRead,
                Permission::ServerUpdate,
                Permission::ServerDelete,
                Permission::ServerExecute,
                Permission::ToolExecute("*".to_string()),
                Permission::ToolRead("*".to_string()),
                Permission::ToolModify("*".to_string()),
                Permission::AdminUserManage,
                Permission::AdminRoleManage,
                Permission::AdminSystemConfig,
                Permission::AdminAuditRead,
                Permission::CostView,
                Permission::CostAllocate,
                Permission::CostBudgetSet,
                Permission::BypassRateLimit,
                Permission::BypassCache,
                Permission::EmergencyAccess,
            ],
            inherits_from: vec![],
            constraints: RoleConstraints {
                require_mfa: true,
                ..Default::default()
            },
        },
        Role {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: "Development access".to_string(),
            permissions: hashset![
                Permission::ServerRead,
                Permission::ServerExecute,
                Permission::ToolExecute("*".to_string()),
                Permission::ToolRead("*".to_string()),
                Permission::CostView,
            ],
            inherits_from: vec!["viewer".to_string()],
            constraints: RoleConstraints::default(),
        },
        Role {
            id: "viewer".to_string(),
            name: "Viewer".to_string(),
            description: "Read-only access".to_string(),
            permissions: hashset![
                Permission::ServerRead,
                Permission::ToolRead("*".to_string()),
            ],
            inherits_from: vec![],
            constraints: RoleConstraints::default(),
        },
    ]
}

// Helper macro for creating permission sets
#[macro_export]
macro_rules! hashset {
    ( $( $x:expr ),* ) => {
        {
            let mut set = HashSet::new();
            $(
                set.insert($x);
            )*
            set
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rbac_basic() {
        let audit = Arc::new(ConsoleAuditLogger);
        let engine = AuthorizationEngine::new(audit);

        // Add admin role
        let admin_role = Role {
            id: "admin".to_string(),
            name: "Admin".to_string(),
            description: "Administrator role".to_string(),
            permissions: hashset![Permission::ServerCreate, Permission::ServerDelete],
            inherits_from: vec![],
            constraints: RoleConstraints::default(),
        };

        engine.add_role(admin_role).await.unwrap();
        engine.assign_role("user1", "admin").await.unwrap();

        let context = AuthzContext {
            request_id: "req1".to_string(),
            ip_address: None,
            user_agent: None,
            mfa_verified: false,
            session_age: Duration::from_secs(100),
            resource: None,
            action: None,
            metadata: HashMap::new(),
        };

        // Test authorization
        let result = engine.authorize("user1", &Permission::ServerCreate, &context).await;
        assert!(result.unwrap());

        let result = engine.authorize("user1", &Permission::ServerRead, &context).await;
        assert!(!result.unwrap()); // Should fail as ServerRead is not in permissions
    }

    #[tokio::test]
    async fn test_role_inheritance() {
        let audit = Arc::new(ConsoleAuditLogger);
        let engine = AuthorizationEngine::new(audit);

        // Create base role
        let viewer_role = Role {
            id: "viewer".to_string(),
            name: "Viewer".to_string(),
            description: "View-only access".to_string(),
            permissions: hashset![Permission::ServerRead],
            inherits_from: vec![],
            constraints: RoleConstraints::default(),
        };

        // Create developer role that inherits from viewer
        let dev_role = Role {
            id: "developer".to_string(),
            name: "Developer".to_string(),
            description: "Developer access".to_string(),
            permissions: hashset![Permission::ServerExecute],
            inherits_from: vec!["viewer".to_string()],
            constraints: RoleConstraints::default(),
        };

        engine.add_role(viewer_role).await.unwrap();
        engine.add_role(dev_role).await.unwrap();
        engine.assign_role("user2", "developer").await.unwrap();

        let context = AuthzContext {
            request_id: "req2".to_string(),
            ip_address: None,
            user_agent: None,
            mfa_verified: false,
            session_age: Duration::from_secs(100),
            resource: None,
            action: None,
            metadata: HashMap::new(),
        };

        // Should have both ServerRead (inherited) and ServerExecute (direct)
        let permissions = engine.get_user_permissions("user2").await.unwrap();
        assert!(permissions.contains(&Permission::ServerRead));
        assert!(permissions.contains(&Permission::ServerExecute));
    }
}
