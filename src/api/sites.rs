use super::prelude::*;
use crate::api::WriteToken;
use crate::objects::{Insertable, Site};
use crate::forum_downloader::ForumDownloader;
use axum::http::StatusCode;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SiteQuery {
    prefix: Option<String>,
    url: Option<String>,
    name: Option<String>,
    page: Option<i32>
}

pub(super) async fn list(
    State(state): State<Api>,
    Query(params): Query<SiteQuery>,
) -> Result<Json<Vec<Site>>, ApiError> {
    let rows: Vec<_> = sqlx::query_as!(Site,
        r#"
        SELECT *
        FROM sites
        WHERE (? IS NULL OR name LIKE CONCAT('%', ?, '%'))
        OR url = ?
        OR url = ?
        LIMIT ?
        OFFSET ?
        "#,
        params.name,
        params.name,
        params.prefix,
        params.url.and_then(|s| s.split(".").next().map(String::from)),
        MAX_PER_PAGE,
        params.page.unwrap_or_default() * MAX_PER_PAGE
    )
        .fetch_all(&state.db)
        .await?;

    Ok(Json(rows))
}

pub(super) async fn count(
    State(state): State<Api>,
    Query(params): Query<SiteQuery>,
) -> Result<Json<i64>, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT COUNT(*) as nb_sites
        FROM sites
        WHERE (? IS NULL OR name LIKE CONCAT('%', ?, '%'))
        OR url = ?
        OR url = ?
        "#,
        params.name,
        params.name,
        params.prefix,
        params.url.and_then(|s| s.split(".").next().map(String::from)),
    )
        .fetch_one(&state.db)
        .await?;

    Ok(Json(rows.nb_sites))
}

pub(super) async fn post(
    State(state): State<Api>,
    Query(token): Query<WriteToken>,
    Json(site): Json<Site>
) -> Result<StatusCode, ApiError> {
    state.cfg.validate_token(token)?;

    site.query_insert().execute(&state.db).await.map_err(|e| match e {
        sqlx::Error::Database(e) if e.code().is_some_and(|code| code == "23000") => ApiError::Duplicate,
        _ => ApiError::Sqlx(e)
    })?;

    ForumDownloader::new(state.db, state.cfg, site).download("forum:start".to_string()).await?;

    Ok(StatusCode::CREATED)
}