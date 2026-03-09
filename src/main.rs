//! # WiFi: the Wikidot Forum indexer.
//! Downloads Wikidot forums into a database and exposes an API to search through it.

use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool};
use std::error::Error;
use crate::config::Config;

mod forum_downloader;
mod tools;
mod objects;
mod api;
mod errors;
mod config;

const FATAL_ERROR: &str = "If you see this error message, please post an issue on Github: https://github.com/Fondation-SCP/WiFi/issues";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>>{

    let config = Config::try_new()?;

    let db = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(config.database.get_url().as_str())
        .await?;

    init_db_schema(&db).await?;

    axum::serve(
        tokio::net::TcpListener::bind(config.get_bind_addr()).await?,
        api::create_router(db, config)
    ).await?;

    Ok(())
}

async fn init_db_schema(db: &Pool<MySql>) -> Result<(), sqlx::Error> {
    use crate::objects::*;
    let tables = [
        Site::query_create_table(),
        Category::query_create_table(),
        Author::query_create_table(),
        Thread::query_create_table(),
        Message::query_create_table()
    ];

    for table in tables {
        table.execute(db).await?;
    }

    Ok(())
}