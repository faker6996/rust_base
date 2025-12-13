use async_trait::async_trait;
use domain::{User, UserRepository, DomainError, TokenPair, Claims, PaginationParams, Page};
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
// Infrastructure Traits (Ports for DI)
// ============================================================================

/// Password hashing service trait for dependency injection
#[async_trait]
pub trait PasswordHasher: Send + Sync {
    fn hash(&self, password: &str) -> Result<String, DomainError>;
    fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError>;
}

/// JWT token service trait for dependency injection
#[async_trait]
pub trait TokenService: Send + Sync {
    fn generate(&self, user: &User) -> Result<TokenPair, DomainError>;
    fn validate(&self, token: &str) -> Result<Claims, DomainError>;
}

// ============================================================================
// Service Traits (Use Cases)
// ============================================================================

#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, ApplicationError>;
    async fn list_users(&self, params: &PaginationParams) -> Result<Page<User>, ApplicationError>;
}

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn register(&self, username: String, email: String, password: String) -> Result<User, ApplicationError>;
    async fn login(&self, email: String, password: String) -> Result<TokenPair, ApplicationError>;
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
    async fn get_user(&self, id: uuid::Uuid) -> Result<Option<User>, ApplicationError> {
        Ok(self.repository.find_by_id(id).await?)
    }

    async fn list_users(&self, params: &PaginationParams) -> Result<Page<User>, ApplicationError> {
        Ok(self.repository.find_all(params).await?)
    }
}

// ============================================================================
// Auth Service Implementation
// ============================================================================

pub struct AuthServiceImpl {
    repository: Arc<dyn UserRepository>,
    password_hasher: Arc<dyn PasswordHasher>,
    token_service: Arc<dyn TokenService>,
}

impl AuthServiceImpl {
    pub fn new(
        repository: Arc<dyn UserRepository>,
        password_hasher: Arc<dyn PasswordHasher>,
        token_service: Arc<dyn TokenService>,
    ) -> Self {
        Self {
            repository,
            password_hasher,
            token_service,
        }
    }
}

#[async_trait]
impl AuthService for AuthServiceImpl {
    async fn register(&self, username: String, email: String, password: String) -> Result<User, ApplicationError> {
        // Validation
        if username.is_empty() {
            return Err(ApplicationError::Domain(DomainError::validation("Username cannot be empty")));
        }
        if email.is_empty() {
            return Err(ApplicationError::Domain(DomainError::validation("Email cannot be empty")));
        }
        if password.len() < 8 {
            return Err(ApplicationError::Domain(DomainError::validation("Password must be at least 8 characters")));
        }

        // Check if user already exists
        if self.repository.find_by_email(&email).await?.is_some() {
            return Err(ApplicationError::Domain(DomainError::conflict("Email already registered")));
        }

        // Hash password and create user
        let password_hash = self.password_hasher.hash(&password)?;
        let user = User::new(username, email, password_hash);
        
        Ok(self.repository.create(&user).await?)
    }

    async fn login(&self, email: String, password: String) -> Result<TokenPair, ApplicationError> {
        // Find user by email
        let user = self.repository
            .find_by_email(&email)
            .await?
            .ok_or_else(|| ApplicationError::Domain(DomainError::unauthorized("Invalid credentials")))?;

        // Verify password
        let valid = self.password_hasher.verify(&password, &user.password_hash)?;
        if !valid {
            return Err(ApplicationError::Domain(DomainError::unauthorized("Invalid credentials")));
        }

        // Generate JWT token
        let token = self.token_service.generate(&user)?;
        Ok(token)
    }
}

