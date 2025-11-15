use axum::Json;
use std::time::{SystemTime, UNIX_EPOCH};

/// GET /health
/// Response: 200 OK with JSON
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
      "status": "healthy",
      "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }))
}
