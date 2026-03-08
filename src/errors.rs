//! Defines [ApiError].
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

/// Errors that can occur during the execution of the API.
#[derive(Error, Debug)]
pub enum ApiError {
    /// Database error.
    #[error("Database error: {0:?}")]
    Sqlx(#[from] sqlx::Error),
    /// Object not found.
    #[error("Not found")]
    NotFound,
    /// Unauthorized token.
    #[error("Access forbiden")]
    AccessForbidden,
    /// Key already exists in the database.
    #[error("Duplicate key")]
    Duplicate,
    /// Error while parsing during scraping a forum.
    #[error("Error while parsing during scraping:\n{details}\nAt: {at}")]
    ParsingError {
        details: &'static str,
        at: String
    },
    /// Error while downloading during scraping a forum.
    #[error("Error while downloading during scraping: {0:?}")]
    ConnectionError(#[from] reqwest::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::AccessForbidden => StatusCode::UNAUTHORIZED,
            ApiError::Duplicate => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        };

        (status, self.to_string()).into_response()
    }
}