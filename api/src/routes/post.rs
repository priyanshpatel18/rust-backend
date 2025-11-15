use crate::{
    AppState,
    auth::validate_token,
    dto::{CreatePostRequest, PaginatedResponse, PaginationParams},
    errors::ApiError,
    models::Post,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

/// POST /posts
/// Headers: Authorization: Bearer <token>
/// Body: { "title": "...", "content": "..." }
pub async fn create_post(
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
pub async fn get_posts(
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
pub async fn get_post(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Post>, ApiError> {
    let post = state.posts.get(&id).ok_or(ApiError::NotFound)?;

    Ok(Json(post.clone()))
}

/// DELETE /posts/:id
/// Headers: Authorization: Bearer <token>
pub async fn delete_post(
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
