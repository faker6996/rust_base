use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use validator::Validate;

use application::AuthService;
use crate::error::ApiError;
use crate::AppState;

// ============================================================================
// Validated JSON Extractor
// ============================================================================

/// Custom extractor that validates JSON payload using validator crate
pub struct ValidatedJson<T>(pub T);

impl<S, T> axum::extract::FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Validate + Send,
{
    type Rejection = ApiError;

    fn from_request<'life0, 'async_trait>(
        req: Request,
        state: &'life0 S,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let bytes = axum::body::Bytes::from_request(req, state)
                .await
                .map_err(|e| ApiError::bad_request(format!("Failed to read body: {}", e)))?;

            let value: T = serde_json::from_slice(&bytes)
                .map_err(|e| ApiError::bad_request(format!("Invalid JSON: {}", e)))?;

            value.validate().map_err(|e| {
                let errors: Vec<String> = e
                    .field_errors()
                    .into_iter()
                    .flat_map(|(field, errors)| {
                        errors.iter().map(move |err| {
                            format!("{}: {}", field, err.message.clone().unwrap_or_default())
                        })
                    })
                    .collect();
                ApiError::bad_request(errors.join(", "))
            })?;

            Ok(ValidatedJson(value))
        })
    }
}

// ============================================================================
// Request/Response DTOs with Validation
// ============================================================================

/// Request body for user registration
#[derive(Deserialize, Validate, ToSchema)]
pub struct RegisterRequest {
    /// Username (3-50 characters)
    #[validate(length(min = 3, max = 50, message = "must be 3-50 characters"))]
    #[schema(example = "john_doe", min_length = 3, max_length = 50)]
    pub username: String,
    /// Valid email address
    #[validate(email(message = "must be a valid email"))]
    #[schema(example = "john@example.com")]
    pub email: String,
    /// Password (8-128 characters)
    #[validate(length(min = 8, max = 128, message = "must be 8-128 characters"))]
    #[schema(example = "securepassword123", min_length = 8)]
    pub password: String,
}

/// Request body for user login
#[derive(Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    /// Valid email address
    #[validate(email(message = "must be a valid email"))]
    #[schema(example = "john@example.com")]
    pub email: String,
    /// User password
    #[validate(length(min = 1, message = "cannot be empty"))]
    #[schema(example = "securepassword123")]
    pub password: String,
}

/// Response after successful registration
#[derive(Serialize, ToSchema)]
pub struct AuthResponse {
    /// Registered user details
    pub user: UserDto,
}

/// JWT token response after login
#[derive(Serialize, ToSchema)]
pub struct TokenResponse {
    /// JWT access token
    #[schema(example = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...")]
    pub access_token: String,
    /// Token type (always "Bearer")
    #[schema(example = "Bearer")]
    pub token_type: String,
    /// Token expiration time in seconds
    #[schema(example = 86400)]
    pub expires_in: i64,
}

/// User data transfer object
#[derive(Serialize, ToSchema)]
pub struct UserDto {
    /// User UUID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub id: String,
    /// Username
    #[schema(example = "john_doe")]
    pub username: String,
    /// Email address
    #[schema(example = "john@example.com")]
    pub email: String,
}

// ============================================================================
// Routes
// ============================================================================

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

// ============================================================================
// Handlers
// ============================================================================

/// Register a new user
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "Authentication",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered successfully", body = AuthResponse),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Email already registered")
    )
)]
pub async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), ApiError> {
    let user = state
        .auth_service
        .register(payload.username, payload.email, payload.password)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user: UserDto {
                id: user.id.to_string(),
                username: user.username,
                email: user.email,
            },
        }),
    ))
}

/// Login and get JWT token
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "Authentication",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = TokenResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn login(
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> Result<Json<TokenResponse>, ApiError> {
    let token = state
        .auth_service
        .login(payload.email, payload.password)
        .await?;

    Ok(Json(TokenResponse {
        access_token: token.access_token,
        token_type: token.token_type,
        expires_in: token.expires_in,
    }))
}


