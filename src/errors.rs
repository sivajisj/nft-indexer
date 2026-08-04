use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    InvalidSignature,
    NonceExpired,
    NonceMismatch,
    UserNotFound,
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::InvalidSignature => (StatusCode::UNAUTHORIZED, "Invalid signature".to_string()),
            AppError::NonceExpired => (StatusCode::UNAUTHORIZED, "Nonce expired, request a new one".to_string()),
            AppError::NonceMismatch => (StatusCode::UNAUTHORIZED, "Nonce does not match".to_string()),
            AppError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}