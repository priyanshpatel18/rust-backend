use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

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
