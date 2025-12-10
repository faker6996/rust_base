mod auth;
mod error;
mod middleware;

use axum::{
    extract::{Path, Query, State},
    middleware as axum_mw,
    routing::get,
    Json, Router,
};
use http::Method;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use application::{AuthService, AuthServiceImpl, TokenService, UserService, UserServiceImpl};
use domain::PaginationParams;
use infrastructure::{ArgonPasswordHasher, JwtConfig, JwtTokenService, PostgresUserRepository};
use error::ApiError;
use middleware::{AuthUser, RequestId};

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    pub user_service: Arc<dyn UserService>,
    pub auth_service: Arc<dyn AuthService>,
    pub token_service: Arc<dyn TokenService>,
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    let pool = sqlx::PgPool::connect(&database_url).await?;
    
    // Create shared dependencies
    let user_repository = Arc::new(PostgresUserRepository::new(pool));
    let password_hasher = Arc::new(ArgonPasswordHasher::new());
    let jwt_config = JwtConfig::from_env();
    let token_service: Arc<dyn TokenService> = Arc::new(JwtTokenService::new(jwt_config));
    
    // Create services
    let user_service = Arc::new(UserServiceImpl::new(user_repository.clone()));
    let auth_service = Arc::new(AuthServiceImpl::new(
        user_repository,
        password_hasher,
        token_service.clone(),
    ));
    
    let state = Arc::new(AppState {
        user_service,
        auth_service,
        token_service,
    });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any)
        .max_age(Duration::from_secs(3600));

    // Protected routes (require authentication)
    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route_layer(axum_mw::from_fn_with_state(state.clone(), middleware::jwt_auth));

    // Public routes
    let public_routes = Router::new()
        .route("/health", get(health_check))
        .route("/users", get(list_users))
        .route("/users/:id", get(get_user))
        .nest("/auth", auth::auth_routes());

    // Combine all routes with global middlewares
    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(axum_mw::from_fn(middleware::request_id))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📖 Endpoints:");
    tracing::info!("   POST /auth/register - Register new user");
    tracing::info!("   POST /auth/login    - Login and get JWT");
    tracing::info!("   GET  /users         - List users (paginated)");
    tracing::info!("   GET  /users/:id     - Get user by ID");
    tracing::info!("   GET  /me            - Get current user (protected)");
    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// Health Check
// ============================================================================

async fn health_check(request_id: RequestId) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "request_id": request_id.0
    }))
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

#[derive(Serialize)]
struct UserResponse {
    id: String,
    username: String,
    email: String,
}

/// Paginated response wrapper for API responses
#[derive(Serialize)]
struct PaginatedResponse<T> {
    items: Vec<T>,
    total: u64,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

// ============================================================================
// Public Handlers
// ============================================================================

/// List users with pagination
async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<UserResponse>>, ApiError> {
    let page = state
        .user_service
        .list_users(&params)
        .await?;

    let items: Vec<UserResponse> = page
        .items
        .into_iter()
        .map(|u| UserResponse {
            id: u.id.to_string(),
            username: u.username,
            email: u.email,
        })
        .collect();

    Ok(Json(PaginatedResponse {
        items,
        total: page.total,
        page: page.page,
        per_page: page.per_page,
        total_pages: page.total_pages,
    }))
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

// ============================================================================
// Protected Handlers
// ============================================================================

/// Handler that requires authentication - demonstrates AuthUser extractor
async fn get_current_user(
    State(state): State<Arc<AppState>>,
    AuthUser(claims): AuthUser,
) -> Result<Json<UserResponse>, ApiError> {
    let user_id = claims.sub.parse::<uuid::Uuid>()
        .map_err(|_| ApiError::internal("Invalid user ID in token"))?;
    
    let user = state
        .user_service
        .get_user(user_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Current user not found"))?;

    Ok(Json(UserResponse {
        id: user.id.to_string(),
        username: user.username,
        email: user.email,
    }))
}


