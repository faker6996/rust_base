mod error;

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
use error::ApiError;

// ============================================================================
// Application State
// ============================================================================

struct AppState {
    user_service: Arc<dyn UserService>,
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

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

// ============================================================================
// Health Check
// ============================================================================

async fn health_check() -> &'static str {
    "OK"
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

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

// ============================================================================
// Handlers
// ============================================================================

async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), ApiError> {
    let user = state
        .user_service
        .create_user(payload.username, payload.email)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id: user.id.to_string(),
            username: user.username,
            email: user.email,
        }),
    ))
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<UserResponse>, ApiError> {
    let user = state
        .user_service
        .get_user(id)
        .await?
        .ok_or_else(|| ApiError::not_found(format!("User with id {} not found", id)))?;

    Ok(Json(UserResponse {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
    }))
}
