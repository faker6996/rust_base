use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use tracing::{info_span, Instrument};

use application::TokenService;
use domain::Claims;
use crate::AppState;
use crate::error::ApiError;

// ============================================================================
// Request ID Extension
// ============================================================================

/// Request ID for tracing and debugging
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Middleware to generate and inject request ID
pub async fn request_id(
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    request.extensions_mut().insert(RequestId(request_id.clone()));
    
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );
    response
}

// ============================================================================
// JWT Authentication Middleware
// ============================================================================

/// Production-ready JWT authentication middleware.
/// - Validates JWT from Authorization header
/// - Returns proper JSON error responses
/// - Adds Claims and creates tracing span with user context
pub async fn jwt_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract request ID for error responses
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_default();

    // Get Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        Some(_) => {
            return Err(ApiError::unauthorized("Invalid Authorization header format. Use: Bearer <token>"));
        }
        None => {
            return Err(ApiError::unauthorized("Missing Authorization header"));
        }
    };

    // Validate token
    let claims = state
        .token_service
        .validate(token)
        .map_err(|e| ApiError::unauthorized(e.to_string()))?;

    // Add claims to request extensions
    let user_id = claims.sub.clone();
    let user_email = claims.email.clone();
    request.extensions_mut().insert(claims);

    // Create tracing span with user context
    let span = info_span!(
        "authenticated_request",
        user_id = %user_id,
        user_email = %user_email,
        request_id = %request_id,
    );

    Ok(next.run(request).instrument(span).await)
}

// ============================================================================
// Role-Based Access Control Middleware
// ============================================================================

/// Middleware factory for role-based access control.
/// Use with `axum::middleware::from_fn_with_state`.
/// 
/// Example:
/// ```rust
/// .route_layer(axum::middleware::from_fn(require_role("admin")))
/// ```
pub fn require_role(required_role: &'static str) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |request: Request, next: Next| {
        Box::pin(async move {
            let claims = request
                .extensions()
                .get::<Claims>()
                .ok_or_else(|| ApiError::unauthorized("Authentication required"))?;

            if !claims.roles.contains(&required_role.to_string()) {
                return Err(ApiError::new(
                    StatusCode::FORBIDDEN,
                    "FORBIDDEN",
                    format!("Required role '{}' not found", required_role),
                ));
            }

            Ok(next.run(request).await)
        })
    }
}

// ============================================================================
// AuthUser Extractor
// ============================================================================

/// Extractor to get authenticated user claims in handlers.
/// 
/// Example:
/// ```rust
/// async fn protected_handler(AuthUser(claims): AuthUser) -> impl IntoResponse {
///     format!("Hello, user {}", claims.sub)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            parts
                .extensions
                .get::<Claims>()
                .cloned()
                .map(AuthUser)
                .ok_or_else(|| ApiError::unauthorized("Authentication required"))
        })
    }
}

// ============================================================================
// Optional AuthUser Extractor
// ============================================================================

/// Extractor for optional authentication.
/// Returns None if not authenticated, Some(claims) if authenticated.
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<Claims>);

impl<S> axum::extract::FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            Ok(OptionalAuthUser(parts.extensions.get::<Claims>().cloned()))
        })
    }
}

// ============================================================================
// Request ID Extractor
// ============================================================================

impl<S> axum::extract::FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut axum::http::request::Parts,
        _state: &'life1 S,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            Ok(parts
                .extensions
                .get::<RequestId>()
                .cloned()
                .unwrap_or_else(|| RequestId("unknown".to_string())))
        })
    }
}
