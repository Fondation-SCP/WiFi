use super::prelude::*;
use crate::db_structs::Site;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SiteQuery {
    prefix: Option<String>,
    url: Option<String>,
    name: Option<String>,
    page: Option<i32>
}

pub(super) async fn list(
    State(state): State<Pool<MySql>>,
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
        .fetch_all(&state)
        .await?;

    Ok(Json(rows))
}

pub(super) async fn count(
    State(state): State<Pool<MySql>>,
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
        .fetch_one(&state)
        .await?;

    Ok(Json(rows.nb_sites))
}