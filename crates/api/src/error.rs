use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use application::ApplicationError;
use domain::DomainError;

// ============================================================================
// API Error Response
// ============================================================================

/// Standardized error response body following REST API best practices.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Serialize)]
pub struct ErrorBody {
    /// Machine-readable error code (e.g., "NOT_FOUND", "VALIDATION_ERROR")
    pub code: String,
    /// Human-readable error message
    pub message: String,
}

/// API-level error that automatically converts to HTTP responses.
/// 
/// This follows the pattern used by major Rust projects:
/// - Axum's error handling
/// - GraphQL error responses
/// - REST API best practices
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, "CONFLICT", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code,
                message: self.message,
            },
        };

        (self.status, Json(body)).into_response()
    }
}

// ============================================================================
// Error Conversions
// ============================================================================

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        match &err {
            DomainError::NotFound { .. } => ApiError::not_found(err.to_string()),
            DomainError::Validation(_) => ApiError::bad_request(err.to_string()),
            DomainError::Conflict(_) => ApiError::conflict(err.to_string()),
            DomainError::Internal(_) => ApiError::internal(err.to_string()),
        }
    }
}

impl From<ApplicationError> for ApiError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::Domain(domain_err) => domain_err.into(),
            ApplicationError::UseCase(msg) => ApiError::bad_request(msg),
        }
    }
}
