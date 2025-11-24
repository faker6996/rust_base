use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{UserService, UserServiceImpl};
use infrastructure::PostgresUserRepository;


struct AppState {
    user_service: Arc<dyn UserService>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load config (mocking for now or loading from env)
    // let config = Config::from_env()?; 
    // For this template, we'll assume some defaults or env vars are set if we were running it.
    // But to make it runnable without setup, we will skip actual DB connection if not present?
    // No, let's try to connect and fail if not present, standard behavior.
    
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let pool = sqlx::PgPool::connect(&database_url).await?;
    
    let user_repository = Arc::new(PostgresUserRepository::new(pool));
    let user_service = Arc::new(UserServiceImpl::new(user_repository));
    
    let state = Arc::new(AppState { user_service });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .with_state(state);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username: String,
    email: String,
}

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    email: String,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let user = state
        .user_service
        .create_user(payload.username, payload.email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(UserResponse {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
    }))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let user = state
        .user_service
        .get_user(id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    match user {
        Some(u) => Ok(Json(UserResponse {
            id: u.id.to_string(),
            username: u.username,
            email: u.email,
        })),
        None => Err((StatusCode::NOT_FOUND, "User not found".to_string())),
    }
}
