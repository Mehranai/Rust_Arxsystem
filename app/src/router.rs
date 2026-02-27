use axum::{Router, routing::get};
use crate::handlers::{health, status};

pub fn build_router() -> Router {
    Router::new()
        .route("/health", get(health::health_check))
        .route("/status", get(status::status))
}
