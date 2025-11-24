use async_trait::async_trait;
use domain::{User, UserRepository};
use sqlx::PgPool;
use uuid::Uuid;

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

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn create(&self, user: &User) -> Result<User, String> {
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
        .map_err(|e| e.to_string())?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, String> {
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
        .map_err(|e| e.to_string())?;

        Ok(row.map(Into::into))
    }
}
