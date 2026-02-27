use std::fmt::{Display, Formatter};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error")]
    Sqlx(#[from] sqlx::Error),
    #[error("Not found")]
    NotFound,
    #[error("Missing parameter")]
    MissingParam(&'static str),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            ApiError::NotFound => StatusCode::NOT_FOUND,
            ApiError::Sqlx(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::MissingParam(_) => StatusCode::BAD_REQUEST,
        };

        (status, self.to_string()).into_response()
    }
}