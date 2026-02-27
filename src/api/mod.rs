mod categories;
mod prelude;
pub mod threads;
pub mod messages;
pub mod sites;
mod authors;

use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};
use std::fmt::{Display, Formatter};

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






pub fn create_router(db: Pool<MySql>) -> Router {
    Router::new()
        .route("/", get(async || "ok"))
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
        .route("/count/sites", get(sites::count))
        .route("/authors", get(authors::list))
        .route("/count/authors", get(authors::count))
        .with_state(db)
}