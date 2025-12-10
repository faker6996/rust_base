use async_trait::async_trait;
use domain::{User, UserRepository, DomainError};
use std::sync::Arc;

// ============================================================================
// Application Errors
// ============================================================================

/// Application-level errors that wrap domain errors and add use-case context.
#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    /// Domain layer errors (propagated with full context)
    #[error(transparent)]
    Domain(#[from] DomainError),

    /// Use case specific errors
    #[error("Use case error: {0}")]
    UseCase(String),
}

impl ApplicationError {
    /// Create a use case specific error
    pub fn use_case<T: Into<String>>(message: T) -> Self {
        Self::UseCase(message.into())
    }
}

// ============================================================================
// Service Traits (Use Cases)
// ============================================================================

#[async_trait]
pub trait UserService: Send + Sync {
    async fn create_user(&self, username: String, email: String) -> Result<User, ApplicationError>;
    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, ApplicationError>;
}

// ============================================================================
// Service Implementations
// ============================================================================

pub struct UserServiceImpl {
    repository: Arc<dyn UserRepository>,
}

impl UserServiceImpl {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn create_user(&self, username: String, email: String) -> Result<User, ApplicationError> {
        // Validation could be done here
        if username.is_empty() {
            return Err(ApplicationError::Domain(DomainError::validation("Username cannot be empty")));
        }
        if email.is_empty() {
            return Err(ApplicationError::Domain(DomainError::validation("Email cannot be empty")));
        }

        let user = User::new(username, email);
        Ok(self.repository.create(&user).await?)
    }

    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, ApplicationError> {
        Ok(self.repository.find_by_id(id).await?)
    }
}
