use crate::api::prelude::{MySql, Pool};
use crate::api::WriteToken;
use crate::errors::ApiError;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use sqlx::query;
use crate::init_db_schema;

pub(super) async fn reset(
    State(db): State<Pool<MySql>>,
    Query(token): Query<WriteToken>
) -> Result<StatusCode, ApiError> {
    token.auth()?;

    query("drop table if exists messages, threads, categories, authors, sites").execute(&db).await?;
    init_db_schema(&db).await?;

    Ok(StatusCode::RESET_CONTENT)
}