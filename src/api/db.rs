use super::{Api, WriteToken};
use crate::errors::ApiError;
use crate::init_db_schema;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use sqlx::query;

pub(super) async fn reset(
    State(state): State<Api>,
    Query(token): Query<WriteToken>
) -> Result<StatusCode, ApiError> {
    state.cfg.validate_token(token)?;

    query("drop table if exists messages, threads, categories, authors, sites")
        .execute(&state.db).await?;
    init_db_schema(&state.db).await?;

    Ok(StatusCode::RESET_CONTENT)
}