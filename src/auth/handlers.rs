//! Authentication endpoint handlers for token management
//!
//! Provides HTTP endpoints for token issuance, refresh, and revocation.
//! Integrates with JWT manager and RBAC system.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::jwt::{Identity, JwtManager, TokenPair};

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Username or email
    pub username: String,
    /// Password (in production, use proper authentication)
    pub password: String,
    /// Optional MFA token
    pub mfa_token: Option<String>,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

/// Refresh request
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Logout request
#[derive(Debug, Deserialize)]
pub struct LogoutRequest {
    /// Access token to revoke
    pub token: String,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: u16,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

/// POST /auth/login - Issue access and refresh tokens
///
/// In production, this would:
/// 1. Validate credentials against a user database
/// 2. Verify MFA if required
/// 3. Check account status (locked, suspended, etc.)
/// 4. Rate limit login attempts
///
/// For MVP, this is a simplified demonstration endpoint.
pub async fn login_handler(
    State(jwt_manager): State<Arc<JwtManager>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ErrorResponse> {
    // DEMO: Simplified authentication (in production, verify against user DB)
    // For now, accept any non-empty credentials
    if req.username.is_empty() || req.password.is_empty() {
        return Err(ErrorResponse {
            error: "Invalid credentials".to_string(),
            code: 401,
        });
    }

    // DEMO: Assign roles based on username (in production, fetch from DB)
    let roles = match req.username.as_str() {
        "admin" => vec!["admin".to_string()],
        "developer" => vec!["developer".to_string()],
        _ => vec!["viewer".to_string()],
    };

    // Create identity
    let identity = Identity {
        id: format!("user_{}", req.username),
        username: req.username.clone(),
        email: Some(format!("{}@example.com", req.username)),
        roles,
        mfa_verified: req.mfa_token.is_some(),
        session_id: Some(uuid::Uuid::new_v4().to_string()),
        client_id: None,
    };

    // Issue token pair
    let tokens = TokenPair::new(&jwt_manager, &identity)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Failed to create tokens: {}", e),
            code: 500,
        })?;

    Ok(Json(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
    }))
}

/// POST /auth/refresh - Refresh access token using refresh token
pub async fn refresh_handler(
    State(jwt_manager): State<Arc<JwtManager>>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<LoginResponse>, ErrorResponse> {
    // Validate refresh token
    let claims = jwt_manager
        .validate_refresh_token(&req.refresh_token)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Invalid refresh token: {}", e),
            code: 401,
        })?;

    // Create new identity from claims
    let identity = Identity {
        id: claims.sub.clone(),
        username: claims.sub.clone(),
        email: None,
        roles: vec![], // Roles will be fetched from DB in production
        mfa_verified: claims.mfa_verified,
        session_id: claims.sid.clone(),
        client_id: claims.client_id.clone(),
    };

    // Issue new token pair
    let tokens = TokenPair::new(&jwt_manager, &identity)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Failed to create tokens: {}", e),
            code: 500,
        })?;

    Ok(Json(LoginResponse {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: tokens.token_type,
        expires_in: tokens.expires_in,
    }))
}

/// POST /auth/logout - Revoke token
pub async fn logout_handler(
    State(jwt_manager): State<Arc<JwtManager>>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<SuccessResponse>, ErrorResponse> {
    // Validate and extract JTI from token
    let claims = jwt_manager
        .validate_token(&req.token)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Invalid token: {}", e),
            code: 401,
        })?;

    // Revoke the token
    jwt_manager.revoke_token(&claims.jti);

    Ok(Json(SuccessResponse {
        message: "Token revoked successfully".to_string(),
    }))
}

/// GET /auth/verify - Verify token validity
///
/// Returns user information if token is valid
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub expires_at: i64,
}

pub async fn verify_handler(
    State(jwt_manager): State<Arc<JwtManager>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<VerifyResponse>, ErrorResponse> {
    // Extract token from Authorization header
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer ").or(h.strip_prefix("bearer ")))
        .or_else(|| {
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|h| h.to_str().ok())
        })
        .ok_or_else(|| ErrorResponse {
            error: "Missing authorization header".to_string(),
            code: 401,
        })?;

    // Validate token
    let claims = jwt_manager
        .validate_token(token)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Invalid token: {}", e),
            code: 401,
        })?;

    Ok(Json(VerifyResponse {
        valid: true,
        user_id: claims.sub,
        roles: claims.roles,
        permissions: claims.permissions,
        expires_at: claims.exp,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::JwtConfig;

    #[tokio::test]
    async fn test_login_handler() {
        let config = JwtConfig::default();
        let secret = b"test_secret_key_for_testing_only_32_bytes_long!!!";
        let jwt_manager = Arc::new(JwtManager::new(config, secret).unwrap());

        let req = LoginRequest {
            username: "testuser".to_string(),
            password: "password123".to_string(),
            mfa_token: None,
        };

        let result = login_handler(State(jwt_manager.clone()), Json(req))
            .await
            .unwrap();

        assert!(!result.access_token.is_empty());
        assert!(!result.refresh_token.is_empty());
        assert_eq!(result.token_type, "Bearer");
    }

    #[tokio::test]
    async fn test_login_empty_credentials() {
        let config = JwtConfig::default();
        let secret = b"test_secret_key_for_testing_only_32_bytes_long!!!";
        let jwt_manager = Arc::new(JwtManager::new(config, secret).unwrap());

        let req = LoginRequest {
            username: "".to_string(),
            password: "".to_string(),
            mfa_token: None,
        };

        let result = login_handler(State(jwt_manager.clone()), Json(req)).await;
        assert!(result.is_err());
    }
}
