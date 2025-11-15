use crate::{Post, User};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

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
