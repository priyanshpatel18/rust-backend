use crate::{
    AppState,
    auth::{create_token, validate_token},
    dto::{AuthResponse, LoginRequest, SignupRequest, UserResponse},
    errors::ApiError,
    models::User,
};
use axum::{Json, extract::State, http::HeaderMap};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Utc;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

/// POST /auth/signup
/// Body: { "email": "...", "username": "...", "password": "..." }
pub async fn signup(
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
pub async fn login(
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
pub async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<UserResponse>, ApiError> {
    let claims = validate_token(&headers, &state.jwt_secret)?;
    let user_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Unauthorized)?;

    let user = state.users.get(&user_id).ok_or(ApiError::NotFound)?;

    Ok(Json(user.clone().into()))
}
