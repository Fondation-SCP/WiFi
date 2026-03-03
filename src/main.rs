use crate::db_structs::{Author, Category, Insertable, Message, Site, Thread};
use sqlx::mysql::{MySqlArguments, MySqlPoolOptions};
use sqlx::{MySql, Pool};
use std::error::Error;

mod forum_downloader;
use crate::api::create_router;

mod tools;
mod db_structs;
mod api;
pub mod errors;

pub type Query<'a> = sqlx::query::Query<'a, MySql, MySqlArguments>;

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

    let app = create_router(db);

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:5000").await?,
        app
    ).await?;

    Ok(())

}

async fn init_db_schema(db: &Pool<MySql>) -> Result<(), sqlx::Error> {

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