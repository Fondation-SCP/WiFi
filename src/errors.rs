use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0:?}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Not found")]
    NotFound,
    #[error("Access forbiden")]
    AccessForbidden,
    #[error("Duplicate key")]
    Duplicate,
    #[error("Error while parsing during scraping:\n{details}\nAt: {at}")]
    ParsingError {
        details: &'static str,
        at: String
    },
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