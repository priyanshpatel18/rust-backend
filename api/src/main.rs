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
    Router,
    routing::{get, post},
};
use dashmap::DashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

pub mod auth;
pub mod dto;
pub mod errors;
pub mod models;
pub mod routes;
pub mod states;

pub use models::{Post, User};
pub use routes::*;
pub use states::*;

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
