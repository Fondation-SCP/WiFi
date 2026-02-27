use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use super::prelude::*;
use crate::db_structs::Message;
use crate::tools::TryFutureIterator;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct MessageQuery {
    title: Option<String>,
    author: Option<String>,
    site_prefix: Option<String>,
    site_url: Option<String>,
    thread_id: Option<i32>,
    thread_name: Option<String>,
    category_id: Option<i32>,
    category_name: Option<String>,
    before: Option<NaiveDate>,
    after: Option<NaiveDate>,
    order: Option<Order>,
    contains: Option<String>,
    page: Option<i32>
}

pub(super) async fn list(
    State(state): State<Pool<MySql>>,
    Query(params): Query<MessageQuery>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let rows: Vec<_> = sqlx::query_as!(Message,
        r#"
        SELECT messages.*
        FROM messages
        LEFT JOIN threads ON messages.thread_id = threads.id
        LEFT JOIN categories ON threads.category_id = categories.id
        WHERE (? IS NULL OR messages.title LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL OR messages.author_username LIKE CONCAT('%', ?, '%'))
        AND ((? IS NULL AND ? IS NULL) OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        AND (? IS NULL OR (threads.id = ?))
        AND (? IS NULL OR (threads.title LIKE CONCAT('%', ?, '%')))
        AND (? IS NULL OR (messages.publication_date >= ?))
        AND (? IS NULL OR (messages.publication_date <= ?))
        AND (? IS NULL OR categories.id = ?)
        AND (? IS NULL OR categories.name LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL OR messages.content LIKE CONCAT('%', ?, '%'))
        ORDER BY
            CASE ?
                WHEN 'Date' THEN messages.publication_date
                WHEN 'ID' THEN messages.id
                WHEN 'Alpha' THEN messages.title
                ELSE messages.publication_date
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
        params.thread_id,
        params.thread_id,
        params.thread_name,
        params.thread_name,
        params.after,
        params.after,
        params.before,
        params.before,
        params.category_id,
        params.category_id,
        params.category_name,
        params.category_name,
        params.contains,
        params.contains,
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
    Query(params): Query<MessageQuery>,
) -> Result<Json<i64>, ApiError> {
    let messages_nb = sqlx::query!(
        r#"
        SELECT COUNT(*) as messages_nb
        FROM messages
        LEFT JOIN threads ON messages.thread_id = threads.id
        LEFT JOIN categories ON threads.category_id = categories.id
        WHERE (? IS NULL OR messages.title LIKE CONCAT('%', ?, '%'))
        AND (? IS NULL OR messages.author_username LIKE CONCAT('%', ?, '%'))
        AND ((? IS NULL AND ? IS NULL) OR ((categories.site_url = ?) OR (categories.site_url = ?)))
        AND (? IS NULL OR (threads.id = ?))
        AND (? IS NULL OR (threads.title LIKE CONCAT('%', ?, '%')))
        AND (? IS NULL OR (messages.publication_date >= ?))
        AND (? IS NULL OR (messages.publication_date <= ?))
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
        params.thread_id,
        params.thread_id,
        params.thread_name,
        params.thread_name,
        params.after,
        params.after,
        params.before,
        params.before,
        params.category_id,
        params.category_id,
        params.category_name,
        params.category_name,
    )
        .fetch_one(&state)
        .await?;

    Ok(Json(messages_nb.messages_nb))
}


#[derive(Debug, Serialize, Deserialize)]
pub(super) struct FullMessage {
    pub id: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub author_username: Option<String>,
    pub publication_date: Option<NaiveDateTime>,
    pub thread_id: i32,
    pub answers: Vec<FullMessage>
}

impl FullMessage {
    pub fn build(message: Message, answers: Vec<FullMessage>) -> Self {
        Self {
            id: message.id,
            title: message.title,
            content: message.content,
            author_username: message.author_username,
            publication_date: message.publication_date,
            thread_id: message.thread_id,
            answers
        }
    }
}


pub(super) fn build_answers_hierarchy(state: State<Pool<MySql>>, message: Message) -> BoxFuture<'static, Result<FullMessage, ApiError>> {
    async move {
        let answers: Vec<Message> = sqlx::query_as!(Message,
            r#"
                select * from messages where answers_to = ?
            "#,
            message.id
        ).fetch_all(&state.0).await?;

        let full_answers = answers.into_iter()
            .map(|message| build_answers_hierarchy(state.clone(), message))
            .try_join_all().await?;

        Ok(FullMessage::build(message, full_answers))
    }.boxed()
}

pub(super) async fn get(
    state: State<Pool<MySql>>,
    Path(id): Path<i32>
) -> Result<Json<FullMessage>, ApiError> {
    let message: Message = sqlx::query_as!(Message,
        r#"
            select * from messages where id = ?
        "#,
        id
    ).fetch_one(&state.0).await?;

    Ok(Json(build_answers_hierarchy(state, message).await?))
}