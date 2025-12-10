use async_trait::async_trait;
use domain::{User, UserRepository, DomainError};
use sqlx::PgPool;
use uuid::Uuid;

// ============================================================================
// Repository Implementations (Adapters)
// ============================================================================

pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    email: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            username: row.username,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

// ============================================================================
// SQLx Error Mapping
// ============================================================================

/// Helper to detect unique constraint violations from PostgreSQL
fn is_unique_violation(err: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = err {
        // PostgreSQL unique violation error code is "23505"
        return db_err.code().map(|c| c == "23505").unwrap_or(false);
    }
    false
}

/// Map SQLx errors to domain errors with proper context
fn map_sqlx_error(err: sqlx::Error, entity: &'static str) -> DomainError {
    if is_unique_violation(&err) {
        return DomainError::conflict(format!("{} already exists", entity));
    }

    match err {
        sqlx::Error::RowNotFound => DomainError::not_found(entity, "unknown"),
        _ => DomainError::internal(err.to_string()),
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: &User) -> Result<User, DomainError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (id, username, email, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id, username, email, created_at
            "#,
        )
        .bind(user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(user.created_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "User"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, DomainError> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, username, email, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "User"))?;

        Ok(row.map(Into::into))
    }
}
