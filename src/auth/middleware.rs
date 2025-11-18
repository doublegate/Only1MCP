//! Authentication middleware for Axum with JWT validation
//!
//! Provides request authentication by extracting and validating JWT tokens
//! from the Authorization header. Attaches user claims to request extensions
//! for downstream handlers to access.

use axum::{
    body::Body,
    extract::Request,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::jwt::{Claims, JwtManager};

/// Authentication error responses
#[derive(Debug)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    RevokedToken,
    ExpiredToken,
    InsufficientPermissions,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, "Missing authorization token"),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, "Invalid authorization token"),
            AuthError::RevokedToken => (StatusCode::UNAUTHORIZED, "Token has been revoked"),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token has expired"),
            AuthError::InsufficientPermissions => {
                (StatusCode::FORBIDDEN, "Insufficient permissions")
            },
        };

        let body = axum::Json(serde_json::json!({
            "error": message,
            "code": status.as_u16(),
        }));

        (status, body).into_response()
    }
}

/// Extract JWT from Authorization header
fn extract_token(headers: &HeaderMap) -> Result<String, AuthError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?
        .to_str()
        .map_err(|_| AuthError::InvalidToken)?;

    // Support both "Bearer <token>" and raw token
    let token = if let Some(bearer) = auth_header.strip_prefix("Bearer ") {
        bearer.to_string()
    } else if let Some(bearer) = auth_header.strip_prefix("bearer ") {
        bearer.to_string()
    } else {
        auth_header.to_string()
    };

    if token.is_empty() {
        return Err(AuthError::MissingToken);
    }

    Ok(token)
}

/// Middleware function for JWT authentication
///
/// Extracts JWT from Authorization header, validates it, and attaches
/// claims to request extensions. Returns 401 for invalid/missing tokens.
///
/// # Usage
///
/// ```rust
/// use axum::Router;
/// use axum::middleware::from_fn_with_state;
///
/// let jwt_manager = Arc::new(JwtManager::new(config, secret)?);
///
/// Router::new()
///     .route("/protected", get(handler))
///     .layer(from_fn_with_state(jwt_manager.clone(), jwt_auth_middleware))
/// ```
pub async fn jwt_auth_middleware(
    jwt_manager: Arc<JwtManager>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // Extract token from headers
    let token = extract_token(request.headers())?;

    // Validate token
    let claims = jwt_manager
        .validate_token(&token)
        .await
        .map_err(|e| match e {
            super::jwt::Error::ExpiredToken => AuthError::ExpiredToken,
            super::jwt::Error::RevokedToken => AuthError::RevokedToken,
            _ => AuthError::InvalidToken,
        })?;

    // Attach claims to request extensions for handlers to access
    request.extensions_mut().insert(claims);

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Optional authentication middleware (doesn't fail on missing token)
///
/// Similar to jwt_auth_middleware but doesn't return an error if no token
/// is present. If a token exists, it validates and attaches claims.
/// Useful for endpoints that support both authenticated and anonymous access.
pub async fn jwt_auth_optional_middleware(
    jwt_manager: Arc<JwtManager>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Try to extract token (don't fail if missing)
    if let Ok(token) = extract_token(request.headers()) {
        // If token exists, validate it
        if let Ok(claims) = jwt_manager.validate_token(&token).await {
            request.extensions_mut().insert(claims);
        }
    }

    next.run(request).await
}

/// Extension trait for extracting claims from request
pub trait ClaimsExt {
    /// Get claims from request extensions
    fn claims(&self) -> Option<&Claims>;
}

impl ClaimsExt for Request<Body> {
    fn claims(&self) -> Option<&Claims> {
        self.extensions().get::<Claims>()
    }
}

/// Helper to check if user has specific permission
pub fn has_permission(claims: &Claims, permission: &str) -> bool {
    claims.permissions.iter().any(|p| p == permission || p == &format!("{}:*", permission.split(':').next().unwrap_or("")))
}

/// Helper to check if user has any of the specified roles
pub fn has_any_role(claims: &Claims, roles: &[&str]) -> bool {
    claims.roles.iter().any(|r| roles.contains(&r.as_str()))
}

/// Require specific permission middleware
///
/// Returns 403 if user doesn't have the required permission.
/// Assumes jwt_auth_middleware has already run and attached claims.
pub async fn require_permission(
    permission: String,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(AuthError::InvalidToken)?;

    if !has_permission(claims, &permission) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

/// Require admin role middleware
pub async fn require_admin(request: Request<Body>, next: Next) -> Result<Response, AuthError> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(AuthError::InvalidToken)?;

    if !has_any_role(claims, &["admin"]) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".parse().unwrap(),
        );

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_extract_token_lowercase_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            "bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".parse().unwrap(),
        );

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_extract_token_raw() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".parse().unwrap(),
        );

        let token = extract_token(&headers).unwrap();
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }

    #[test]
    fn test_extract_token_missing() {
        let headers = HeaderMap::new();
        let result = extract_token(&headers);
        assert!(matches!(result, Err(AuthError::MissingToken)));
    }

    #[test]
    fn test_has_permission() {
        let claims = Claims {
            sub: "user123".to_string(),
            iat: 0,
            exp: 9999999999,
            nbf: 0,
            jti: "jti123".to_string(),
            iss: "test".to_string(),
            aud: vec!["test".to_string()],
            roles: vec!["admin".to_string()],
            permissions: vec!["read:*".to_string(), "write:servers".to_string()],
            mfa_verified: false,
            sid: None,
            client_id: None,
        };

        assert!(has_permission(&claims, "read:servers"));
        assert!(has_permission(&claims, "write:servers"));
        assert!(!has_permission(&claims, "delete:servers"));
    }

    #[test]
    fn test_has_any_role() {
        let claims = Claims {
            sub: "user123".to_string(),
            iat: 0,
            exp: 9999999999,
            nbf: 0,
            jti: "jti123".to_string(),
            iss: "test".to_string(),
            aud: vec!["test".to_string()],
            roles: vec!["developer".to_string()],
            permissions: vec![],
            mfa_verified: false,
            sid: None,
            client_id: None,
        };

        assert!(has_any_role(&claims, &["admin", "developer"]));
        assert!(!has_any_role(&claims, &["admin", "viewer"]));
    }
}
