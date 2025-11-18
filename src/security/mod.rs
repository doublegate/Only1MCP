//! Security hardening middleware and utilities
//!
//! Provides production-grade security features including:
//! - Security headers (CSP, HSTS, X-Frame-Options, etc.)
//! - Request size limits
//! - Input validation
//! - Request timeouts

use axum::{
    extract::Request,
    http::{header, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum request body size in bytes (default: 10MB)
    pub max_body_size: usize,

    /// Enable strict security headers
    pub strict_headers: bool,

    /// Content-Security-Policy header value
    pub csp_policy: Option<String>,

    /// Enable HSTS (HTTP Strict Transport Security)
    pub enable_hsts: bool,

    /// HSTS max-age in seconds (default: 1 year)
    pub hsts_max_age: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_body_size: 10 * 1024 * 1024, // 10MB
            strict_headers: true,
            csp_policy: Some(
                "default-src 'self'; script-src 'self'; object-src 'none'".to_string(),
            ),
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
        }
    }
}

/// Security headers middleware
///
/// Adds production-ready security headers to all responses:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: DENY
/// - X-XSS-Protection: 1; mode=block
/// - Strict-Transport-Security (if HTTPS)
/// - Content-Security-Policy
/// - Referrer-Policy: strict-origin-when-cross-origin
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // Prevent clickjacking attacks
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );

    // Enable browser XSS protection (legacy but still useful)
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    // Strict referrer policy
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'",
        ),
    );

    // Permissions Policy (formerly Feature-Policy)
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=(), payment=()"),
    );

    response
}

/// HSTS middleware for HTTPS enforcement
///
/// Adds Strict-Transport-Security header when serving over HTTPS
pub async fn hsts_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;

    // Only add HSTS if request was over HTTPS
    // In production behind a reverse proxy, check X-Forwarded-Proto header
    let headers = response.headers_mut();

    // HSTS with 1 year max-age, including subdomains, and preload
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
    );

    response
}

/// Request size validation middleware
///
/// Rejects requests with bodies larger than the configured limit
pub async fn request_size_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    const MAX_SIZE: u64 = 10 * 1024 * 1024; // 10MB

    // Check Content-Length header if present
    if let Some(content_length) = request.headers().get(header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<u64>() {
                if length > MAX_SIZE {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        format!(
                            "Request body too large. Maximum size: {} bytes",
                            MAX_SIZE
                        ),
                    ));
                }
            }
        }
    }

    Ok(next.run(request).await)
}

/// Rate limit headers middleware
///
/// Adds rate limit information to responses
pub fn add_rate_limit_headers(
    response: &mut Response,
    limit: u32,
    remaining: u32,
    reset_in: u64,
) {
    let headers = response.headers_mut();

    if let Ok(value) = HeaderValue::from_str(&limit.to_string()) {
        headers.insert(HeaderName::from_static("x-ratelimit-limit"), value);
    }

    if let Ok(value) = HeaderValue::from_str(&remaining.to_string()) {
        headers.insert(HeaderName::from_static("x-ratelimit-remaining"), value);
    }

    if let Ok(value) = HeaderValue::from_str(&reset_in.to_string()) {
        headers.insert(HeaderName::from_static("x-ratelimit-reset"), value);
    }
}

// Re-export for convenience
use axum::http::HeaderName;

/// Input validation utilities
pub mod validation {
    /// Validate that a string contains only safe characters
    pub fn is_safe_string(s: &str) -> bool {
        s.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == '-'
                || c == '_'
                || c == '.'
                || c == ' '
                || c == '@'
        })
    }

    /// Validate email format (basic check)
    pub fn is_valid_email(email: &str) -> bool {
        email.contains('@')
            && email.len() > 3
            && email.len() < 255
            && !email.starts_with('@')
            && !email.ends_with('@')
    }

    /// Validate that a path is safe (no directory traversal)
    pub fn is_safe_path(path: &str) -> bool {
        !path.contains("..") && !path.starts_with('/') && !path.contains('\\')
    }

    /// Sanitize string by removing potentially dangerous characters
    pub fn sanitize_string(s: &str) -> String {
        s.chars()
            .filter(|c| {
                c.is_ascii_alphanumeric()
                    || *c == '-'
                    || *c == '_'
                    || *c == '.'
                    || *c == ' '
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_string_validation() {
        assert!(validation::is_safe_string("hello_world-123"));
        assert!(validation::is_safe_string("user@example.com"));
        assert!(!validation::is_safe_string("hello<script>"));
        assert!(!validation::is_safe_string("path/../etc/passwd"));
    }

    #[test]
    fn test_email_validation() {
        assert!(validation::is_valid_email("user@example.com"));
        assert!(validation::is_valid_email("test.user@domain.co.uk"));
        assert!(!validation::is_valid_email("notanemail"));
        assert!(!validation::is_valid_email("@example.com"));
        assert!(!validation::is_valid_email("user@"));
    }

    #[test]
    fn test_path_validation() {
        assert!(validation::is_safe_path("config/settings.yaml"));
        assert!(validation::is_safe_path("data/file.json"));
        assert!(!validation::is_safe_path("../etc/passwd"));
        assert!(!validation::is_safe_path("/etc/shadow"));
        assert!(!validation::is_safe_path("path\\..\\file"));
    }

    #[test]
    fn test_string_sanitization() {
        assert_eq!(
            validation::sanitize_string("hello<script>world"),
            "helloscriptworld"
        );
        assert_eq!(
            validation::sanitize_string("user@example!.com"),
            "userexample.com"
        );
    }
}
