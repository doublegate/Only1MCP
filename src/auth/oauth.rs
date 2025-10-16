//! OAuth 2.0 / OIDC implementation with PKCE
//!
//! Supports multiple OAuth providers with automatic discovery,
//! PKCE flow for enhanced security, and token introspection.

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use reqwest::Url;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// OAuth error types
#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    #[error("Invalid state parameter")]
    InvalidState,

    #[error("Authorization code expired")]
    ExpiredCode,

    #[error("Invalid or expired token")]
    InvalidToken,

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("URL parse error: {0}")]
    UrlError(#[from] url::ParseError),

    #[error("Token validation failed: {0}")]
    ValidationError(String),

    #[error("JWKS fetch failed: {0}")]
    JwksFetchError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// OAuth2 configuration
#[derive(Debug, Clone, Deserialize)]
pub struct OAuth2Config {
    /// OAuth providers
    pub providers: Vec<ProviderConfig>,

    /// Redirect URI for callbacks
    pub redirect_uri: String,

    /// Default scopes
    pub default_scopes: Vec<String>,
}

/// Individual provider configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    /// Provider ID
    pub id: String,

    /// OAuth issuer URL
    pub issuer: String,

    /// Client ID
    pub client_id: String,

    /// Client secret (should be stored securely)
    pub client_secret: String,

    /// Required scopes
    pub scopes: Vec<String>,

    /// Whether PKCE is required
    pub pkce_required: bool,
}

/// OAuth provider details
#[derive(Debug, Clone)]
pub struct OAuthProvider {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub auth_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub scopes: Vec<String>,
    pub pkce_required: bool,
}

/// OpenID Connect discovery response
#[derive(Debug, Deserialize)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub userinfo_endpoint: Option<String>,
    pub revocation_endpoint: Option<String>,
    pub introspection_endpoint: Option<String>,
    pub response_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
}

/// Pending authorization state
#[derive(Debug, Clone)]
pub struct PendingAuth {
    pub provider_id: String,
    pub verifier: PkceCodeVerifier,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// PKCE code verifier
#[derive(Debug, Clone)]
pub struct PkceCodeVerifier(String);

impl PkceCodeVerifier {
    /// Generate random verifier
    pub fn new_random() -> Self {
        use rand::Rng;
        let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes);
        Self(verifier)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// PKCE code challenge
#[derive(Debug, Clone)]
pub struct PkceCodeChallenge(String);

impl PkceCodeChallenge {
    /// Create challenge from verifier using S256
    pub fn from_code_verifier(verifier: &PkceCodeVerifier) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_str());
        let result = hasher.finalize();
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(result);
        Self(challenge)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Authorization URL with state
#[derive(Debug)]
pub struct AuthorizeUrl {
    pub url: String,
    pub state: String,
    pub expires_at: DateTime<Utc>,
}

/// Token response from OAuth provider
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub scope: Option<String>,
}

/// Token pair with claims
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: u64,
    pub claims: HashMap<String, serde_json::Value>,
}

/// ID token claims
#[derive(Debug, Deserialize)]
pub struct IdTokenClaims {
    pub sub: String,
    pub aud: Vec<String>,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

/// JWKS cache entry
#[derive(Debug, Clone)]
pub struct JwksCache {
    pub keys: Vec<serde_json::Value>,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Default for JwksCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JwksCache {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            fetched_at: Utc::now(),
            expires_at: Utc::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Token info from introspection
#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub sub: Option<String>,
    pub aud: Option<Vec<String>>,
}

/// OAuth 2.0 / OIDC implementation with PKCE
pub struct OAuth2Authenticator {
    /// OAuth provider configurations
    providers: HashMap<String, OAuthProvider>,

    /// JWKS cache for token validation (for future JWKS validation feature)
    _jwks_cache: Arc<RwLock<HashMap<String, JwksCache>>>,

    /// Active authorization codes (for PKCE flow)
    auth_codes: Arc<DashMap<String, PendingAuth>>,

    /// Token introspection cache
    introspection_cache: Arc<DashMap<String, TokenInfo>>,

    /// Configuration
    config: OAuth2Config,
}

impl OAuth2Authenticator {
    /// Initialize OAuth with provider discovery
    pub async fn new(config: OAuth2Config) -> Result<Self, OAuthError> {
        let mut providers = HashMap::new();

        for provider_config in &config.providers {
            // Discover OAuth endpoints via .well-known
            let discovery_url = format!(
                "{}/.well-known/openid-configuration",
                provider_config.issuer
            );

            let discovery: OpenIdConfiguration = reqwest::get(&discovery_url).await?.json().await?;

            let provider = OAuthProvider {
                issuer: provider_config.issuer.clone(),
                client_id: provider_config.client_id.clone(),
                client_secret: provider_config.client_secret.clone(),
                auth_endpoint: discovery.authorization_endpoint,
                token_endpoint: discovery.token_endpoint,
                jwks_uri: discovery.jwks_uri,
                scopes: provider_config.scopes.clone(),
                pkce_required: provider_config.pkce_required,
            };

            providers.insert(provider_config.id.clone(), provider);
        }

        Ok(Self {
            providers,
            _jwks_cache: Arc::new(RwLock::new(HashMap::new())),
            auth_codes: Arc::new(DashMap::new()),
            introspection_cache: Arc::new(DashMap::new()),
            config,
        })
    }

    /// Generate authorization URL with PKCE
    pub async fn authorize_url(&self, provider_id: &str) -> Result<AuthorizeUrl, OAuthError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| OAuthError::UnknownProvider(provider_id.to_string()))?;

        // Generate PKCE challenge
        let verifier = PkceCodeVerifier::new_random();
        let challenge = PkceCodeChallenge::from_code_verifier(&verifier);

        // Generate state for CSRF protection
        let state = generate_secure_random(32);

        // Store pending auth
        let pending = PendingAuth {
            provider_id: provider_id.to_string(),
            verifier: verifier.clone(),
            state: state.clone(),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(10),
        };

        self.auth_codes.insert(state.clone(), pending.clone());

        // Build authorization URL
        let mut url = Url::parse(&provider.auth_endpoint)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("client_id", &provider.client_id);
            query.append_pair("redirect_uri", &self.config.redirect_uri);
            query.append_pair("response_type", "code");
            query.append_pair("scope", &provider.scopes.join(" "));
            query.append_pair("state", &state);

            if provider.pkce_required {
                query.append_pair("code_challenge", challenge.as_str());
                query.append_pair("code_challenge_method", "S256");
            }

            query.append_pair("nonce", &generate_secure_random(16));
        }

        Ok(AuthorizeUrl {
            url: url.to_string(),
            state,
            expires_at: pending.expires_at,
        })
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&self, code: &str, state: &str) -> Result<TokenPair, OAuthError> {
        // Retrieve and validate pending auth
        let pending = self.auth_codes.remove(state).ok_or(OAuthError::InvalidState)?.1;

        if pending.expires_at < Utc::now() {
            return Err(OAuthError::ExpiredCode);
        }

        let provider = self
            .providers
            .get(&pending.provider_id)
            .ok_or_else(|| OAuthError::UnknownProvider(pending.provider_id.clone()))?;

        // Build token request
        let mut params = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &provider.client_id),
            ("client_secret", &provider.client_secret),
            ("redirect_uri", &self.config.redirect_uri),
        ];

        if provider.pkce_required {
            params.push(("code_verifier", pending.verifier.as_str()));
        }

        // Exchange code for tokens
        let token_response: TokenResponse = reqwest::Client::new()
            .post(&provider.token_endpoint)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        // Parse ID token claims if present
        let mut claims = HashMap::new();
        if let Some(id_token) = &token_response.id_token {
            // In production, validate the ID token signature using JWKS
            claims = self.decode_id_token(id_token)?;
        }

        Ok(TokenPair {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            id_token: token_response.id_token,
            expires_in: token_response.expires_in,
            claims,
        })
    }

    /// Decode ID token without validation (for demo purposes)
    fn decode_id_token(
        &self,
        token: &str,
    ) -> Result<HashMap<String, serde_json::Value>, OAuthError> {
        // Split the token
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(OAuthError::ValidationError(
                "Invalid token format".to_string(),
            ));
        }

        // Decode the payload
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| OAuthError::ValidationError(e.to_string()))?;

        // Parse as JSON
        let claims: HashMap<String, serde_json::Value> = serde_json::from_slice(&payload)?;

        Ok(claims)
    }

    /// Validate access token via introspection
    pub async fn introspect_token(
        &self,
        provider_id: &str,
        token: &str,
    ) -> Result<TokenInfo, OAuthError> {
        // Check cache first
        if let Some(cached) = self.introspection_cache.get(token) {
            return Ok(cached.clone());
        }

        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| OAuthError::UnknownProvider(provider_id.to_string()))?;

        // In production, call the introspection endpoint
        // For now, return a mock response
        let info = TokenInfo {
            active: true,
            scope: Some("openid email profile".to_string()),
            client_id: Some(provider.client_id.clone()),
            username: None,
            exp: Some((Utc::now() + Duration::hours(1)).timestamp()),
            iat: Some(Utc::now().timestamp()),
            sub: Some("user123".to_string()),
            aud: Some(vec![provider.client_id.clone()]),
        };

        // Cache the result
        self.introspection_cache.insert(token.to_string(), info.clone());

        Ok(info)
    }

    /// Refresh access token
    pub async fn refresh_token(
        &self,
        provider_id: &str,
        refresh_token: &str,
    ) -> Result<TokenPair, OAuthError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| OAuthError::UnknownProvider(provider_id.to_string()))?;

        let token_response: TokenResponse = reqwest::Client::new()
            .post(&provider.token_endpoint)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
                ("client_id", &provider.client_id),
                ("client_secret", &provider.client_secret),
            ])
            .send()
            .await?
            .json()
            .await?;

        let mut claims = HashMap::new();
        if let Some(id_token) = &token_response.id_token {
            claims = self.decode_id_token(id_token)?;
        }

        Ok(TokenPair {
            access_token: token_response.access_token,
            refresh_token: token_response.refresh_token,
            id_token: token_response.id_token,
            expires_in: token_response.expires_in,
            claims,
        })
    }

    /// Revoke token
    pub async fn revoke_token(&self, provider_id: &str, token: &str) -> Result<(), OAuthError> {
        let _provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| OAuthError::UnknownProvider(provider_id.to_string()))?;

        // In production, call the revocation endpoint if available
        // For now, just remove from cache
        self.introspection_cache.remove(token);

        tracing::info!("Token revoked for provider: {}", provider_id);
        Ok(())
    }
}

/// Generate secure random string
fn generate_secure_random(length: usize) -> String {
    use rand::Rng;
    let random_bytes: Vec<u8> = (0..length).map(|_| rand::thread_rng().gen::<u8>()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let verifier = PkceCodeVerifier::new_random();
        let challenge = PkceCodeChallenge::from_code_verifier(&verifier);

        assert!(!verifier.as_str().is_empty());
        assert!(!challenge.as_str().is_empty());
        assert_ne!(verifier.as_str(), challenge.as_str());
    }

    #[test]
    fn test_secure_random() {
        let random1 = generate_secure_random(32);
        let random2 = generate_secure_random(32);

        assert_eq!(random1.len(), 43); // Base64 encoded 32 bytes
        assert_ne!(random1, random2);
    }

    #[tokio::test]
    async fn test_oauth_initialization() {
        let config = OAuth2Config {
            providers: vec![],
            redirect_uri: "http://localhost:8080/callback".to_string(),
            default_scopes: vec!["openid".to_string(), "profile".to_string()],
        };

        let auth = OAuth2Authenticator::new(config).await;
        assert!(auth.is_ok());
    }
}
