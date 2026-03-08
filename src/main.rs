//! # WiFi: the Wikidot Forum indexer.
//! Downloads Wikidot forums into a database and exposes an API to search through it.

use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool};
use std::error::Error;

mod forum_downloader;
mod tools;
mod objects;
mod api;
mod errors;

const PARALLEL_DOWNLOADS: usize = 16;
const DB_URL: &str = "mariadb://cyrielle@localhost:3306/wifi_test";
const FATAL_ERROR: &str = "If you see this, please post an issue on Github: https://github.com/Fondation-SCP/WiFi/issues";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>>{
    let db = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(DB_URL)
        .await?;

    init_db_schema(&db).await?;

    let app = api::create_router(db);

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:5000").await?,
        app
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