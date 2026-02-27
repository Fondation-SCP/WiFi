use axum::extract::Path;
use super::prelude::*;
use crate::db_structs::{Message, Thread};
use super::messages::{build_answers_hierarchy, FullMessage};
use crate::tools::TryFutureIterator;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct ThreadQuery {
    title: Option<String>,
    author: Option<String>,
    site_prefix: Option<String>,
    site_url: Option<String>,
    category_id: Option<i32>,
    category_name: Option<String>,
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
    order: Option<Order>,
    page: Option<i32>
}

pub(super) async fn list(
    State(state): State<Pool<MySql>>,
    Query(params): Query<ThreadQuery>,
) -> Result<Json<Vec<Thread>>, ApiError> {

    let rows: Vec<_> = sqlx::query_as!(Thread,
        r#"
        SELECT threads.*
        FROM threads
        LEFT JOIN categories on threads.category_id = categories.id
        WHERE (? IS NULL OR threads.title LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL OR threads.author_username LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL AND ? IS NULL OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        AND (? IS NULL OR threads.creation_date >= ?)
        AND (? IS NULL OR threads.creation_date <= ?)
        AND (? IS NULL OR categories.id = ?)
        AND (? IS NULL OR categories.name LIKE CONCAT('%', ?, '%'))
        ORDER BY
            CASE ?
                WHEN 'Date' THEN threads.creation_date
                WHEN 'ID' THEN threads.id
                WHEN 'Alpha' THEN threads.title
                ELSE threads.creation_date
            END
        LIMIT ?
        OFFSET ?
        "#,
        params.title,
        params.title,
        params.author,
        params.author,
        params.site_prefix,
        params.site_url,
        params.site_prefix,
        params.site_url.as_ref().and_then(|url| url.split(".").next().map(String::from)),
        params.after,
        params.after,
        params.before,
        params.before,
        params.category_id,
        params.category_id,
        params.category_name,
        params.category_name,
        params.order.unwrap_or_default().to_string(),
        MAX_PER_PAGE,
        params.page.unwrap_or_default() * MAX_PER_PAGE
    )
        .fetch_all(&state)
        .await?;

    Ok(Json(rows))
}

pub(super) async fn count(
    State(state): State<Pool<MySql>>,
    Query(params): Query<ThreadQuery>,
) -> Result<Json<i64>, ApiError> {

    let thread_nb = sqlx::query!(
        r#"
        SELECT COUNT(*) as thread_nb
        FROM threads
        LEFT JOIN categories on threads.category_id = categories.id
        WHERE (? IS NULL OR threads.title LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL OR threads.author_username LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL AND ? IS NULL OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        AND (? IS NULL OR threads.creation_date >= ?)
        AND (? IS NULL OR threads.creation_date <= ?)
        AND (? IS NULL OR categories.id = ?)
        AND (? IS NULL OR categories.name LIKE CONCAT('%', ?, '%'))
        "#,
        params.title,
        params.title,
        params.author,
        params.author,
        params.site_prefix,
        params.site_url,
        params.site_prefix,
        params.site_url.as_ref().and_then(|url| url.split(".").next().map(String::from)),
        params.after,
        params.after,
        params.before,
        params.before,
        params.category_id,
        params.category_id,
        params.category_name,
        params.category_name
    )
        .fetch_one(&state)
        .await?;

    Ok(Json(thread_nb.thread_nb))
}


#[derive(Debug, Serialize, Deserialize)]
pub(super) struct FullThread {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub creation_date: Option<NaiveDateTime>,
    pub author_username: Option<String>,
    pub messages: Vec<FullMessage>
}

impl FullThread {
    fn build(thread: Thread, messages: Vec<FullMessage>) -> Self {
        Self {
            id: thread.id,
            title: thread.title,
            description: thread.description,
            creation_date: thread.creation_date,
            author_username: thread.author_username,
            messages
        }
    }
}

pub(super) async fn get(
    state: State<Pool<MySql>>,
    Path(id): Path<i32>
) -> Result<Json<FullThread>, ApiError> {
    let thread: Thread = sqlx::query_as!(Thread,
        r#"
            select * from threads where id = ?
        "#,
        id
    ).fetch_one(&state.0).await?;

    let messages: Vec<_> = sqlx::query_as!(Message,
        r#"
            select * from messages where thread_id = ? order by publication_date
        "#,
        thread.id
    ).fetch_all(&state.0).await?
        .into_iter().map(|message| build_answers_hierarchy(state.clone(), message)).try_join_all().await?;

    Ok(Json(FullThread::build(thread, messages)))
}