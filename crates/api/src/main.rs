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
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use application::{AuthService, AuthServiceImpl, TokenService, UserService, UserServiceImpl};
use domain::PaginationParams;
use infrastructure::{ArgonPasswordHasher, JwtConfig, JwtTokenService, PostgresUserRepository};
use error::ApiError;
use middleware::{AuthUser, RequestId};

// Re-export auth types for OpenAPI
use auth::{RegisterRequest, LoginRequest, AuthResponse, TokenResponse, UserDto};

// ============================================================================
// OpenAPI Documentation
// ============================================================================

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rust Base API",
        version = "1.0.0",
        description = "A production-ready Rust backend API with Clean Architecture",
        contact(name = "API Support", email = "support@example.com"),
        license(name = "MIT")
    ),
    paths(
        auth::register,
        auth::login,
        list_users,
        get_user,
        get_current_user,
        health_check,
    ),
    components(schemas(
        RegisterRequest,
        LoginRequest,
        AuthResponse,
        TokenResponse,
        UserDto,
        UserResponse,
        PaginatedUserResponse,
        HealthResponse,
    )),
    tags(
        (name = "Authentication", description = "User registration and login"),
        (name = "Users", description = "User management endpoints"),
        (name = "Health", description = "Health check endpoints")
    )
)]
struct ApiDoc;

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
    // Load .env file
    dotenvy::dotenv().ok();

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
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(public_routes)
        .merge(protected_routes)
        .layer(TraceLayer::new_for_http())
        .layer(axum_mw::from_fn(middleware::request_id))
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    let listener = TcpListener::bind(addr).await?;
    tracing::info!("🚀 Server listening on {}", addr);
    tracing::info!("📖 Swagger UI: http://{}/swagger-ui/", addr);
    tracing::info!("📄 OpenAPI JSON: http://{}/api-docs/openapi.json", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// Health Check
// ============================================================================

/// Health check response
#[derive(Serialize, ToSchema)]
struct HealthResponse {
    /// API status
    #[schema(example = "ok")]
    status: String,
    /// Request tracking ID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    request_id: String,
}

/// Check API health status
#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    responses(
        (status = 200, description = "API is healthy", body = HealthResponse)
    )
)]
async fn health_check(request_id: RequestId) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        request_id: request_id.0,
    })
}

// ============================================================================
// Request/Response DTOs
// ============================================================================

/// User response object
#[derive(Serialize, ToSchema)]
struct UserResponse {
    /// User UUID
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    id: String,
    /// Username
    #[schema(example = "john_doe")]
    username: String,
    /// Email address
    #[schema(example = "john@example.com")]
    email: String,
}

/// Paginated response wrapper for users
#[derive(Serialize, ToSchema)]
struct PaginatedUserResponse {
    /// List of users
    items: Vec<UserResponse>,
    /// Total number of users
    #[schema(example = 100)]
    total: u64,
    /// Current page number
    #[schema(example = 1)]
    page: u32,
    /// Items per page
    #[schema(example = 20)]
    per_page: u32,
    /// Total number of pages
    #[schema(example = 5)]
    total_pages: u32,
}

// ============================================================================
// Public Handlers
// ============================================================================

/// List all users with pagination
#[utoipa::path(
    get,
    path = "/users",
    tag = "Users",
    params(
        ("page" = Option<u32>, Query, description = "Page number (default: 1)"),
        ("per_page" = Option<u32>, Query, description = "Items per page (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "List of users", body = PaginatedUserResponse)
    )
)]
async fn list_users(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedUserResponse>, ApiError> {
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

    Ok(Json(PaginatedUserResponse {
        items,
        total: page.total,
        page: page.page,
        per_page: page.per_page,
        total_pages: page.total_pages,
    }))
}

/// Get a user by ID
#[utoipa::path(
    get,
    path = "/users/{id}",
    tag = "Users",
    params(
        ("id" = String, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User found", body = UserResponse),
        (status = 404, description = "User not found")
    )
)]
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

/// Get current authenticated user
#[utoipa::path(
    get,
    path = "/me",
    tag = "Users",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = UserResponse),
        (status = 401, description = "Unauthorized")
    )
)]
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



