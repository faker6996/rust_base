use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================================
// Domain Errors
// ============================================================================

/// Domain-level errors following professional Rust patterns.
/// Used across all layers with proper error chaining.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Entity not found in the system
    #[error("Entity not found: {entity} with id {id}")]
    NotFound { entity: &'static str, id: String },

    /// Validation errors (invalid input, business rule violations)
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Conflict errors (duplicate entries, concurrent modifications)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal/unexpected errors (database failures, etc.)
    #[error("Internal error: {0}")]
    Internal(String),
}

impl DomainError {
    /// Create a NotFound error for a specific entity type
    pub fn not_found<T: AsRef<str>>(entity: &'static str, id: T) -> Self {
        Self::NotFound {
            entity,
            id: id.as_ref().to_string(),
        }
    }

    /// Create a validation error
    pub fn validation<T: Into<String>>(message: T) -> Self {
        Self::Validation(message.into())
    }

    /// Create a conflict error (e.g., duplicate username)
    pub fn conflict<T: Into<String>>(message: T) -> Self {
        Self::Conflict(message.into())
    }

    /// Create an internal error
    pub fn internal<T: Into<String>>(message: T) -> Self {
        Self::Internal(message.into())
    }
}

// ============================================================================
// Domain Entities
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, email: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Repository Traits (Ports)
// ============================================================================

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, user: &User) -> Result<User, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError>;
}
