use super::prelude::*;
use crate::objects::Category;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CategoryQuery {
    name: Option<String>,
    page: Option<i32>,
    site_url: Option<String>,
    site_prefix: Option<String>
}

pub(super) async fn list(
    State(state): State<Api>,
    Query(params): Query<CategoryQuery>,
) -> Result<Json<Vec<Category>>, ApiError> {
    let rows: Vec<_> = sqlx::query_as!(Category,
        r#"
        SELECT *
        FROM categories
        WHERE (? IS NULL OR name LIKE CONCAT('%', ?, '%'))
        AND ((? IS NULL AND ? IS NULL) OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        LIMIT ?
        OFFSET ?
        "#,
        params.name,
        params.name,
        params.site_url,
        params.site_prefix,
        params.site_url,
        params.site_prefix,
        MAX_PER_PAGE,
        params.page.unwrap_or_default() * MAX_PER_PAGE
    )
        .fetch_all(&state.db)
        .await?;

    Ok(Json(rows))
}

pub(super) async fn count(
    State(state): State<Api>,
    Query(params): Query<CategoryQuery>,
) -> Result<Json<i64>, ApiError> {
    let rows = sqlx::query!(
        r#"
        SELECT COUNT(*) as nb_categories
        FROM categories
        WHERE (? IS NULL OR name LIKE CONCAT('%', ?, '%'))
        AND ((? IS NULL AND ? IS NULL) OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        ORDER BY categories.name
        "#,
        params.name,
        params.name,
        params.site_url,
        params.site_prefix,
        params.site_url,
        params.site_prefix
    )
        .fetch_one(&state.db)
        .await?;

    Ok(Json(rows.nb_categories))
}


pub(super) async fn get(
    State(state): State<Api>,
    Path(id): Path<i32>
) -> Result<Json<Category>, ApiError> {
    Ok(Json(sqlx::query_as!(Category, "select * from categories where id = ?", id).fetch_one(&state.db).await?))
}