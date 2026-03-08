use super::prelude::*;
use crate::objects::Author;


#[derive(Debug, Serialize, Deserialize)]
pub(super) struct AuthorQuery {
    username: Option<String>,
    page: Option<i32>
}

pub(super) async fn list(
    State(state): State<Pool<MySql>>,
    Query(params): Query<AuthorQuery>,
) -> Result<Json<Vec<Author>>, ApiError> {
    let rows: Vec<_> = sqlx::query_as!(Author,
        r#"
        SELECT *
        FROM authors
        WHERE (? IS NULL OR username LIKE CONCAT('%', ?, '%'))
        LIMIT ?
        OFFSET ?
        "#,
        params.username,
        params.username,
        MAX_PER_PAGE,
        params.page.unwrap_or_default() * MAX_PER_PAGE
    )
        .fetch_all(&state)
        .await?;

    Ok(Json(rows))
}

pub(super) async fn count(
    State(state): State<Pool<MySql>>,
    Query(params): Query<AuthorQuery>,
) -> Result<Json<i64>, ApiError> {
    let rows= sqlx::query!(
        r#"
        SELECT COUNT(*) as nb_authors
        FROM authors
        WHERE (? IS NULL OR username LIKE CONCAT('%', ?, '%'))
        "#,
        params.username,
        params.username,
    )
        .fetch_one(&state)
        .await?;

    Ok(Json(rows.nb_authors))
}