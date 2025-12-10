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

    /// Authentication/Authorization errors
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
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

    /// Create an unauthorized error
    pub fn unauthorized<T: Into<String>>(message: T) -> Self {
        Self::Unauthorized(message.into())
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
    #[serde(skip_serializing)] // Never expose password hash in responses
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new(username: String, email: String, password_hash: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            created_at: Utc::now(),
        }
    }
}

// ============================================================================
// Authentication Types
// ============================================================================

/// User credentials for login
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

/// Token pair returned after successful authentication
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl TokenPair {
    pub fn new(access_token: String, expires_in: i64) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in,
        }
    }
}

/// JWT Claims structure with role-based access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,           // User ID
    pub email: String,
    pub roles: Vec<String>,    // User roles for RBAC
    pub exp: i64,              // Expiration timestamp
    pub iat: i64,              // Issued at timestamp
}

// ============================================================================
// Pagination Types
// ============================================================================

/// Pagination parameters for list queries
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page (max 100)
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 { 1 }
fn default_per_page() -> u32 { 20 }

impl PaginationParams {
    pub fn new(page: u32, per_page: u32) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.clamp(1, 100),
        }
    }

    /// Calculate offset for SQL queries
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    /// Get limit for SQL queries
    pub fn limit(&self) -> u32 {
        self.per_page.min(100)
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize)]
pub struct Page<T> {
    /// Items for current page
    pub items: Vec<T>,
    /// Total number of items
    pub total: u64,
    /// Current page number
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total number of pages
    pub total_pages: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, params: &PaginationParams) -> Self {
        let total_pages = ((total as f64) / (params.per_page as f64)).ceil() as u32;
        Self {
            items,
            total,
            page: params.page,
            per_page: params.per_page,
            total_pages,
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
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, DomainError>;
    async fn list(&self, params: &PaginationParams) -> Result<Page<User>, DomainError>;
    async fn count(&self) -> Result<u64, DomainError>;
}


