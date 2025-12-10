use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher as Argon2Hasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use domain::{Claims, DomainError, TokenPair, User};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use application::{PasswordHasher, TokenService};

// ============================================================================
// Argon2 Password Hasher
// ============================================================================

pub struct ArgonPasswordHasher;

impl ArgonPasswordHasher {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ArgonPasswordHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PasswordHasher for ArgonPasswordHasher {
    fn hash(&self, password: &str) -> Result<String, DomainError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| DomainError::internal(format!("Password hashing failed: {}", e)))?
            .to_string();
        
        Ok(password_hash)
    }

    fn verify(&self, password: &str, hash: &str) -> Result<bool, DomainError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| DomainError::internal(format!("Invalid password hash format: {}", e)))?;
        
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

// ============================================================================
// JWT Token Service
// ============================================================================

pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl JwtConfig {
    pub fn new(secret: String, expiration_hours: i64) -> Self {
        Self { secret, expiration_hours }
    }

    pub fn from_env() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "super-secret-key-change-in-production".to_string()),
            expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24),
        }
    }
}

pub struct JwtTokenService {
    config: JwtConfig,
}

impl JwtTokenService {
    pub fn new(config: JwtConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl TokenService for JwtTokenService {
    fn generate(&self, user: &User) -> Result<TokenPair, DomainError> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(self.config.expiration_hours);
        
        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            roles: vec!["user".to_string()], // Default role, can be extended
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.secret.as_bytes()),
        )
        .map_err(|e| DomainError::internal(format!("Token generation failed: {}", e)))?;

        Ok(TokenPair::new(token, self.config.expiration_hours * 3600))
    }

    fn validate(&self, token: &str) -> Result<Claims, DomainError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| DomainError::unauthorized(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }
}
