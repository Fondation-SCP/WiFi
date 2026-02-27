use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use crate::db_structs::{Author, Category, Message, Site, Thread};
use crate::errors::ApiError;
use crate::tools::TryFutureIterator;
use axum::extract::{Path, Query, State};
use axum::{routing::get, Json, Router};
use chrono::{NaiveDate, NaiveDateTime};
use futures_util::future::BoxFuture;
use futures_util::FutureExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
enum Order {
    ID,
    Alpha,
    #[default]
    Date,
}

impl Display for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ID => "ID",
            Self::Alpha => "Alpha",
            Self::Date => "Date"
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ThreadQuery {
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

const MAX_PER_PAGE: i32 = 25;

async fn list_threads(
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

async fn count_threads(
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
struct MessageQuery {
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

async fn list_messages(
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

async fn count_messages(
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
struct CategoryQuery {
    name: Option<String>,
    page: Option<i32>,
    site_url: Option<String>,
    site_prefix: Option<String>
}

async fn list_categories(
    State(state): State<Pool<MySql>>,
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
        .fetch_all(&state)
        .await?;

    Ok(Json(rows))
}

async fn count_categories(
    State(state): State<Pool<MySql>>,
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
        .fetch_one(&state)
        .await?;

    Ok(Json(rows.nb_categories))
}

#[derive(Debug, Serialize, Deserialize)]
struct SiteQuery {
    prefix: Option<String>,
    url: Option<String>,
    name: Option<String>,
    page: Option<i32>
}

async fn list_sites(
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

async fn count_sites(
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

#[derive(Debug, Serialize, Deserialize)]
struct AuthorQuery {
    username: Option<String>,
    page: Option<i32>
}

async fn list_authors(
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

async fn count_authors(
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

#[derive(Debug, Serialize, Deserialize)]
struct FullMessage {
    pub id: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub author_username: Option<String>,
    pub publication_date: Option<NaiveDateTime>,
    pub thread_id: i32,
    pub answers: Vec<FullMessage>
}

impl FullMessage {
    fn build(message: Message, answers: Vec<FullMessage>) -> Self {
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


fn build_answers_hierarchy(state: State<Pool<MySql>>, message: Message) -> BoxFuture<'static, Result<FullMessage, ApiError>> {
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

async fn get_message(
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

#[derive(Debug, Serialize, Deserialize)]
struct FullThread {
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

async fn get_thread(
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

async fn get_category(
    State(state): State<Pool<MySql>>,
    Path(id): Path<i32>
) -> Result<Json<Category>, ApiError> {
    Ok(Json(sqlx::query_as!(Category, "select * from categories where id = ?", id).fetch_one(&state).await?))
}



pub fn create_router(db: Pool<MySql>) -> Router {
    Router::new()
        .route("/", get(async || "ok"))
        .route("/threads", get(list_threads))
        .route("/count/threads", get(count_threads))
        .route("/threads/{id}", get(get_thread))
        .route("/messages", get(list_messages))
        .route("/count/messages", get(count_messages))
        .route("/messages/{id}", get(get_message))
        .route("/categories", get(list_categories))
        .route("/count/categories", get(count_categories))
        .route("/categories/{id}", get(get_category))
        .route("/sites", get(list_sites))
        .route("/count/sites", get(count_sites))
        .route("/authors", get(list_authors))
        .route("/count/authors", get(count_authors))
        .with_state(db)
}