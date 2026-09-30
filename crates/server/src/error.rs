//! The `server`-wide error type for request handlers.
//!
//! Handlers return `Result<_, AppError>`. Internals are logged via
//! `tracing`; the client only ever sees a generic message so we never leak
//! implementation details (query text, file paths, ...) in a response body.

use axum::{http::StatusCode, response::IntoResponse, response::Response};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!(error = %self, "request failed");
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}
