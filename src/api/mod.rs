mod categories;
mod prelude;
pub mod threads;
pub mod messages;
pub mod sites;
mod authors;
mod db;

use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};
use std::fmt::{Display, Formatter};
use axum::routing::{delete, post};
use crate::config::Config;

#[derive(Clone)]
pub(crate) struct Api {
    db: Pool<MySql>,
    cfg: Config
}

#[derive(Debug, Serialize, Deserialize, Default, Clone, Copy)]
pub enum Order {
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy)]
pub struct WriteToken {
    token: u64
}

impl From<WriteToken> for u64 {
    fn from(value: WriteToken) -> Self {
        value.token
    }
}

pub fn create_router(db: Pool<MySql>, cfg: Config) -> Router {
    Router::new()
        .route("/", get(async || "Connected!"))
        .route("/", delete(db::reset))
        .route("/threads", get(threads::list))
        .route("/count/threads", get(threads::count))
        .route("/threads/{id}", get(threads::get))
        .route("/messages", get(messages::list))
        .route("/count/messages", get(messages::count))
        .route("/messages/{id}", get(messages::get))
        .route("/categories", get(categories::list))
        .route("/count/categories", get(categories::count))
        .route("/categories/{id}", get(categories::get))
        .route("/sites", get(sites::list))
        .route("/sites", post(sites::post))
        .route("/count/sites", get(sites::count))
        .route("/authors", get(authors::list))
        .route("/count/authors", get(authors::count))
        .with_state(Api {
            db,
            cfg
        })
}