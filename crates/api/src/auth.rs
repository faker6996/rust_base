use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50, message = "must be 3-50 characters"))]
    pub username: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, max = 128, message = "must be 8-128 characters"))]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "cannot be empty"))]
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserDto,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

#[derive(Serialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
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

async fn register(
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

async fn login(
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

