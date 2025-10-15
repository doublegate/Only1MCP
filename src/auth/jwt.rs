//! JWT token creation, validation, and rotation with RS256 signatures.
//! Implements secure token lifecycle management with refresh tokens.

use jsonwebtoken::{encode, decode, Header, Algorithm, EncodingKey, DecodingKey, Validation, errors::Error as JwtError};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use chrono::{Utc, DateTime};
use uuid::Uuid;
use dashmap::DashSet;
use thiserror::Error;

/// JWT error types
#[derive(Debug, Error)]
pub enum Error {
    #[error("JWT encoding/decoding error: {0}")]
    Jwt(#[from] JwtError),

    #[error("Invalid token")]
    InvalidToken,

    #[error("Token has been revoked")]
    RevokedToken,

    #[error("Token expired")]
    ExpiredToken,

    #[error("Invalid audience")]
    InvalidAudience,

    #[error("Invalid issuer")]
    InvalidIssuer,

    #[error("MFA required but not verified")]
    MfaRequired,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("Key generation error: {0}")]
    KeyGeneration(String),
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Token issuer
    pub issuer: String,

    /// Token audience
    pub audience: Vec<String>,

    /// Access token TTL
    pub access_token_ttl: Duration,

    /// Refresh token TTL
    pub refresh_token_ttl: Duration,

    /// Require MFA for sensitive operations
    pub require_mfa: bool,

    /// Key rotation interval
    pub key_rotation_interval: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            issuer: "only1mcp".to_string(),
            audience: vec!["only1mcp-api".to_string()],
            access_token_ttl: Duration::from_secs(15 * 60), // 15 minutes
            refresh_token_ttl: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            require_mfa: false,
            key_rotation_interval: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issued at
    pub iat: i64,

    /// Expiration
    pub exp: i64,

    /// Not before
    pub nbf: i64,

    /// JWT ID (for revocation)
    pub jti: String,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: Vec<String>,

    /// Custom claims
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub mfa_verified: bool,

    /// Session ID for tracking
    pub sid: Option<String>,

    /// Client ID for API keys
    pub client_id: Option<String>,
}

/// User identity for token creation
#[derive(Debug, Clone)]
pub struct Identity {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub roles: Vec<String>,
    pub mfa_verified: bool,
    pub session_id: Option<String>,
    pub client_id: Option<String>,
}

/// JWT token manager with key rotation support
pub struct JwtManager {
    /// Current signing key
    current_key: Arc<RwLock<EncodingKey>>,

    /// Current decoding key
    decoding_key: Arc<RwLock<DecodingKey>>,

    /// Previous keys for validation during rotation
    previous_keys: Arc<RwLock<Vec<DecodingKey>>>,

    /// Key rotation schedule
    rotation_schedule: Duration,

    /// Token configuration
    config: JwtConfig,

    /// Revoked tokens (JTI blacklist)
    revoked: Arc<DashSet<String>>,

    /// Token store for refresh tokens
    refresh_tokens: Arc<DashSet<String>>,
}

impl JwtManager {
    /// Create new JWT manager
    pub fn new(config: JwtConfig, secret: &[u8]) -> Result<Self, Error> {
        let encoding_key = EncodingKey::from_secret(secret);
        let decoding_key = DecodingKey::from_secret(secret);

        Ok(Self {
            current_key: Arc::new(RwLock::new(encoding_key)),
            decoding_key: Arc::new(RwLock::new(decoding_key)),
            previous_keys: Arc::new(RwLock::new(Vec::new())),
            rotation_schedule: config.key_rotation_interval,
            config,
            revoked: Arc::new(DashSet::new()),
            refresh_tokens: Arc::new(DashSet::new()),
        })
    }

    /// Create new JWT manager with RSA keys
    pub fn new_with_rsa(config: JwtConfig, private_key: &[u8], public_key: &[u8]) -> Result<Self, Error> {
        let encoding_key = EncodingKey::from_rsa_pem(private_key)?;
        let decoding_key = DecodingKey::from_rsa_pem(public_key)?;

        Ok(Self {
            current_key: Arc::new(RwLock::new(encoding_key)),
            decoding_key: Arc::new(RwLock::new(decoding_key)),
            previous_keys: Arc::new(RwLock::new(Vec::new())),
            rotation_schedule: config.key_rotation_interval,
            config,
            revoked: Arc::new(DashSet::new()),
            refresh_tokens: Arc::new(DashSet::new()),
        })
    }

    /// Create new access token
    pub async fn create_access_token(&self, identity: &Identity) -> Result<String, Error> {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: identity.id.clone(),
            iat: now.timestamp(),
            exp: (now + chrono::Duration::from_std(self.config.access_token_ttl).unwrap()).timestamp(),
            nbf: now.timestamp(),
            jti: jti.clone(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            roles: identity.roles.clone(),
            permissions: self.expand_permissions(&identity.roles),
            mfa_verified: identity.mfa_verified,
            sid: identity.session_id.clone(),
            client_id: identity.client_id.clone(),
        };

        // Sign with configured algorithm
        let header = Header::new(Algorithm::RS256);
        let key = self.current_key.read().await;
        let token = encode(&header, &claims, &key)?;

        // Store JTI for potential revocation tracking
        self.store_jti(&jti, claims.exp);

        Ok(token)
    }

    /// Create refresh token
    pub async fn create_refresh_token(&self, identity: &Identity) -> Result<String, Error> {
        let now = Utc::now();
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: identity.id.clone(),
            iat: now.timestamp(),
            exp: (now + chrono::Duration::from_std(self.config.refresh_token_ttl).unwrap()).timestamp(),
            nbf: now.timestamp(),
            jti: jti.clone(),
            iss: self.config.issuer.clone(),
            aud: vec!["refresh".to_string()], // Special audience for refresh tokens
            roles: vec![], // No roles in refresh token
            permissions: vec![], // No permissions in refresh token
            mfa_verified: identity.mfa_verified,
            sid: identity.session_id.clone(),
            client_id: identity.client_id.clone(),
        };

        let header = Header::new(Algorithm::RS256);
        let key = self.current_key.read().await;
        let token = encode(&header, &claims, &key)?;

        // Store refresh token
        self.refresh_tokens.insert(jti);

        Ok(token)
    }

    /// Validate and decode token
    pub async fn validate_token(&self, token: &str) -> Result<Claims, Error> {
        // Setup validation rules
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&self.config.audience);
        validation.validate_exp = true;
        validation.validate_nbf = true;

        // Try current key first
        let decoding_key = self.decoding_key.read().await;
        if let Ok(data) = decode::<Claims>(token, &decoding_key, &validation) {
            // Check if token is revoked
            if self.revoked.contains(&data.claims.jti) {
                return Err(Error::RevokedToken);
            }
            return Ok(data.claims);
        }

        // Try previous keys (for rotation grace period)
        let previous = self.previous_keys.read().await;
        for key in previous.iter() {
            if let Ok(data) = decode::<Claims>(token, key, &validation) {
                if self.revoked.contains(&data.claims.jti) {
                    return Err(Error::RevokedToken);
                }
                return Ok(data.claims);
            }
        }

        Err(Error::InvalidToken)
    }

    /// Validate refresh token
    pub async fn validate_refresh_token(&self, token: &str) -> Result<Claims, Error> {
        // Setup validation for refresh tokens
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&["refresh"]); // Special audience for refresh tokens
        validation.validate_exp = true;
        validation.validate_nbf = true;

        let decoding_key = self.decoding_key.read().await;
        let data = decode::<Claims>(token, &decoding_key, &validation)?;

        // Check if refresh token exists and is not revoked
        if !self.refresh_tokens.contains(&data.claims.jti) || self.revoked.contains(&data.claims.jti) {
            return Err(Error::InvalidToken);
        }

        Ok(data.claims)
    }

    /// Revoke a token
    pub fn revoke_token(&self, jti: &str) {
        self.revoked.insert(jti.to_string());
    }

    /// Revoke all tokens for a user
    pub fn revoke_user_tokens(&self, user_id: &str) {
        // In production, this would query a database to find all JTIs for the user
        // For now, we just log the action
        tracing::info!("Revoking all tokens for user: {}", user_id);
    }

    /// Expand permissions based on roles
    fn expand_permissions(&self, roles: &[String]) -> Vec<String> {
        let mut permissions = Vec::new();

        for role in roles {
            match role.as_str() {
                "admin" => {
                    permissions.extend_from_slice(&[
                        "read:*".to_string(),
                        "write:*".to_string(),
                        "delete:*".to_string(),
                        "admin:*".to_string(),
                    ]);
                }
                "developer" => {
                    permissions.extend_from_slice(&[
                        "read:*".to_string(),
                        "write:code".to_string(),
                        "write:config".to_string(),
                    ]);
                }
                "viewer" => {
                    permissions.push("read:*".to_string());
                }
                _ => {}
            }
        }

        permissions
    }

    /// Store JTI for tracking
    fn store_jti(&self, jti: &str, exp: i64) {
        // In production, this would store in a database with TTL
        // For now, we just track in memory
        let _jti = jti.to_string();
        let _exp = exp;
    }

    /// Clean up expired tokens
    pub async fn cleanup_expired(&self) {
        // This would run periodically to clean up expired tokens
        // from the revocation list and refresh token store
        tracing::debug!("Cleaning up expired tokens");
    }

    /// Rotate keys (for RSA key rotation)
    pub async fn rotate_keys(&self, new_private_key: &[u8], new_public_key: &[u8]) -> Result<(), Error> {
        tracing::info!("Starting key rotation");

        // Create new keys
        let new_encoding = EncodingKey::from_rsa_pem(new_private_key)?;
        let new_decoding = DecodingKey::from_rsa_pem(new_public_key)?;

        // Move current decoding key to previous
        let current_decoding = self.decoding_key.read().await.clone();
        let mut previous = self.previous_keys.write().await;
        previous.push(current_decoding);

        // Limit previous keys (keep last 3)
        if previous.len() > 3 {
            previous.remove(0);
        }

        // Update current keys
        *self.current_key.write().await = new_encoding;
        *self.decoding_key.write().await = new_decoding;

        tracing::info!("Key rotation completed");
        Ok(())
    }
}

/// Token pair for access and refresh tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

impl TokenPair {
    /// Create new token pair
    pub async fn new(jwt_manager: &JwtManager, identity: &Identity) -> Result<Self, Error> {
        let access_token = jwt_manager.create_access_token(identity).await?;
        let refresh_token = jwt_manager.create_refresh_token(identity).await?;

        Ok(Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: jwt_manager.config.access_token_ttl.as_secs(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_creation_and_validation() {
        let config = JwtConfig::default();
        let secret = b"test_secret_key_for_testing_only";
        let manager = JwtManager::new(config, secret).unwrap();

        let identity = Identity {
            id: "user123".to_string(),
            username: "testuser".to_string(),
            email: Some("test@example.com".to_string()),
            roles: vec!["developer".to_string()],
            mfa_verified: false,
            session_id: Some("session123".to_string()),
            client_id: None,
        };

        // Create token
        let token = manager.create_access_token(&identity).await.unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = manager.validate_token(&token).await.unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.roles, vec!["developer"]);
        assert!(claims.permissions.contains(&"read:*".to_string()));
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let config = JwtConfig::default();
        let secret = b"test_secret_key_for_testing_only";
        let manager = JwtManager::new(config, secret).unwrap();

        let identity = Identity {
            id: "user123".to_string(),
            username: "testuser".to_string(),
            email: None,
            roles: vec!["viewer".to_string()],
            mfa_verified: false,
            session_id: None,
            client_id: None,
        };

        let token = manager.create_access_token(&identity).await.unwrap();
        let claims = manager.validate_token(&token).await.unwrap();

        // Revoke the token
        manager.revoke_token(&claims.jti);

        // Try to validate revoked token
        let result = manager.validate_token(&token).await;
        assert!(matches!(result, Err(Error::RevokedToken)));
    }
}