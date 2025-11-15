// ============================================================================
// HIGH-PERFORMANCE REST API WITH AUTHENTICATION
// ============================================================================

// - User signup/login with password hashing
// - JWT authentication & authorization
// - Pagination and filtering
// - CORS configuration
// - Input validation
// - Proper error handling
// - Structured logging

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use uuid::Uuid;
use validator::Validate;

// ============================================================================
// MODELS - The User and Post models
// ============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub hashed_password: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub content: String,
    pub created_at: i64,
}

// ============================================================================
// REQUEST/RESPONSE DTOs (Data Transfer Objects)
// ============================================================================
// `Validate` trait: Axum will automatically check these rules before
// the handler runs. If validation fails, returns 400 Bad Request.
#[derive(Debug, Validate, Deserialize)]
pub struct SignupRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 3, max = 20, message = "Username must be 3-20 characters"))]
    pub username: String,
    #[validate(length(min = 8, max = 100, message = "Password must be 8-100 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub created_at: i64,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    #[validate(length(min = 1, max = 5000))]
    pub content: String,
}

/// Pagination query parameters
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_page() -> usize {
    1
}
fn default_limit() -> usize {
    10
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub page: usize,
    pub limit: usize,
    pub total: usize,
}

// ============================================================================
// JWT - What we encode in the authentication token
// ============================================================================
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID)
    pub email: String,
    pub exp: usize,
}

// ============================================================================
// APPLICATION STATE - Shared data across all requests
// ============================================================================
/// `Arc` = Atomic Reference Counter
/// - Allows multiple threads to share ownership safely
/// - When last reference drops, data is cleaned up
///
/// `DashMap` = Thread-safe HashMap
/// - Can be read/written from multiple threads simultaneously
/// - No need for Mutex locks (handles it internally)
#[derive(Clone)]
pub struct AppState {
    pub users: Arc<DashMap<Uuid, User>>,
    pub posts: Arc<DashMap<Uuid, Post>>,
    pub email_index: Arc<DashMap<String, Uuid>>, // Quick Lookup by Email
    pub jwt_secret: String,
}

// ============================================================================
// ERROR HANDLING - Custom error types for clean responses
// ============================================================================
#[derive(Debug)]
pub enum ApiError {
    InvalidCredentials,
    UserAlreadyExists,
    Unauthorized,
    NotFound,
    ValidationError(String),
    InternalError(String),
}

/// Convert our custom errors to HTTP responses
///
/// `IntoResponse` trait: Axum calls this to convert errors to responses
/// This is how we control what users see when errors occur
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials"),
            ApiError::UserAlreadyExists => (StatusCode::CONFLICT, "User already exists"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not Found"),
            ApiError::ValidationError(msg) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                      "error": msg
                    })),
                )
                    .into_response();
            }
            ApiError::InternalError(msg) => {
                error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        (
            status,
            Json(serde_json::json!({
              "error": message
            })),
        )
            .into_response()
    }
}

// ============================================================================
// JWT UTILITIES - Token creation and validation
// ============================================================================

pub fn create_token(user_id: &Uuid, email: &str, secret: &str) -> Result<String, ApiError> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .ok_or_else(|| ApiError::InternalError("Failed to calculate expiration".into()))?
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("Token Creation failed: {}", e)))
}

pub fn validate_token(headers: &HeaderMap, secret: &str) -> Result<Claims, ApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    // Check for "Bearer " prefix
    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::Unauthorized);
    }

    let token = &auth_header[7..];

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|_| ApiError::Unauthorized)
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /health
/// Response: 200 OK with JSON
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
      "status": "healthy",
      "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}

/// POST /auth/signup
/// Body: { "email": "...", "username": "...", "password": "..." }
async fn signup(
    State(state): State<AppState>,
    Json(payload): Json<SignupRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    if state.email_index.contains_key(&payload.email) {
        return Err(ApiError::UserAlreadyExists);
    }

    let hashed_password = hash(&payload.password, DEFAULT_COST)
        .map_err(|e| ApiError::InternalError(format!("Password hashing failed: {}", e)));

    let user = User {
        id: Uuid::new_v4(),
        email: payload.email,
        username: payload.username,
        hashed_password: hashed_password?,
        created_at: Utc::now().timestamp(),
    };

    let token = create_token(&user.id, &user.email, &state.jwt_secret)?;

    state.email_index.insert(user.email.clone(), user.id);
    state.users.insert(user.id, user.clone());

    info!("New user registered: {}", user.email);

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

/// POST /auth/login
/// Body: { "email": "...", "password": "..." }
async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    // Find user by email
    let user_id = state
        .email_index
        .get(&payload.email)
        .ok_or(ApiError::InvalidCredentials)?;

    let user = state
        .users
        .get(&*user_id)
        .ok_or(ApiError::InvalidCredentials)?;

    // Verify password
    let valid = verify(&payload.password, &user.hashed_password)
        .map_err(|e| ApiError::InternalError(format!("Password verification failed: {}", e)))?;

    if !valid {
        return Err(ApiError::InvalidCredentials);
    }

    // Generate token
    let token = create_token(&user.id, &user.email, &state.jwt_secret)?;

    info!("User logged in: {}", user.email);

    Ok(Json(AuthResponse {
        token,
        user: user.clone().into(),
    }))
}

/// GET /users/me
/// Headers: Authorization: Bearer <token>
async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, ApiError> {
    let claims = validate_token(&headers, &state.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;

    let user = state.users.get(&user_id).ok_or(ApiError::NotFound)?;

    Ok(Json(user.clone().into()))
}

/// POST /posts
/// Headers: Authorization: Bearer <token>
/// Body: { "title": "...", "content": "..." }
async fn create_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<Post>), ApiError> {
    payload
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    let claims = validate_token(&headers, &state.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;

    let post = Post {
        id: Uuid::new_v4(),
        user_id,
        title: payload.title,
        content: payload.content,
        created_at: Utc::now().timestamp(),
    };

    state.posts.insert(post.id, post.clone());

    info!("Post created: {} by user {}", post.id, user_id);

    Ok((StatusCode::CREATED, Json(post)))
}

/// GET /posts?page=1&limit=10
async fn get_posts(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Json<PaginatedResponse<Post>> {
    let mut posts: Vec<Post> = state
        .posts
        .iter()
        .map(|entry| entry.value().clone())
        .collect();

    // Sort by creation date (newest first)
    posts.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let total = posts.len();
    let start = (params.page.saturating_sub(1)) * params.limit;
    let end = (start + params.limit).min(total);

    let paginated_posts = if start < total {
        posts[start..end].to_vec()
    } else {
        vec![]
    };

    Json(PaginatedResponse {
        data: paginated_posts,
        page: params.page,
        limit: params.limit,
        total,
    })
}

/// GET /posts/:id
async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Post>, ApiError> {
    let post = state.posts.get(&id).ok_or(ApiError::NotFound)?;

    Ok(Json(post.clone()))
}

/// DELETE /posts/:id
/// Headers: Authorization: Bearer <token>
async fn delete_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let claims = validate_token(&headers, &state.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;

    let post = state.posts.get(&id).ok_or(ApiError::NotFound)?;

    // Check ownership
    if post.user_id != user_id {
        return Err(ApiError::Unauthorized);
    }

    state.posts.remove(&id);

    info!("Post deleted: {} by user {}", id, user_id);

    Ok(StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    dotenvy::dotenv().ok();

    // JWT Secret
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set!");

    // Create application state
    let state = AppState {
        users: Arc::new(DashMap::new()),
        posts: Arc::new(DashMap::new()),
        email_index: Arc::new(DashMap::new()),
        jwt_secret,
    };

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build the router
    let app = Router::new()
        // Public routes (no auth required)
        .route("/health", get(health_check))
        .route("/auth/signup", post(signup))
        .route("/auth/login", post(login))
        // Protected routes (auth required)
        .route("/users/me", get(get_current_user))
        .route("/posts", post(create_post).get(get_posts))
        .route("/posts/{id}", get(get_post).delete(delete_post))
        // Add state and middleware
        .with_state(state)
        .layer(cors);

    // Start server
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Server running on http://{}", addr);
    info!("API Endpoints:");
    info!("  GET    /health           - Health check");
    info!("  POST   /auth/signup      - Create account");
    info!("  POST   /auth/login       - Login");
    info!("  GET    /users/me         - Get current user (auth)");
    info!("  POST   /posts            - Create post (auth)");
    info!("  GET    /posts            - List posts (paginated)");
    info!("  GET    /posts/:id        - Get specific post");
    info!("  DELETE /posts/:id        - Delete post (auth, owner only)");

    axum::serve(listener, app).await.unwrap();
}
