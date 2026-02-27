use crate::db_structs::{Author, Category, Insertable, Message, Site, Thread};
use sqlx::mysql::{MySqlArguments, MySqlPoolOptions};
use sqlx::{MySql, MySqlPool, Pool};
use std::error::Error;
use std::sync::Arc;

mod forum_downloader;
use forum_downloader::ForumDownloader;
use crate::api::create_router;

mod tools;
mod db_structs;
mod api;
pub mod errors;

pub type Query<'a> = sqlx::query::Query<'a, MySql, MySqlArguments>;

const THREADS: usize = 4;
const DB_URL: &str = "mariadb://cyrielle@localhost:3306/wifi_test";

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>>{
    let site = Site {
        url: "commandementO5".to_string(),
        name: Some("Commandement O5".to_string())
    };

    let db = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(DB_URL)
        .await?;

    init_db_schema(&db).await?;
    site.query_insert().execute(&db).await.ok();

    /*
    let arc_db = Arc::new(db);
    ForumDownloader::new(arc_db.clone(), Arc::new(site)).download("forum:start").await?;
    let db = Arc::<Pool<MySql>>::into_inner(arc_db).unwrap();
    */

    let app = create_router(db);

    axum::serve(
        tokio::net::TcpListener::bind("0.0.0.0:5000").await?,
        app
    ).await?;

    Ok(())

}

async fn init_db_schema(db: &Pool<MySql>) -> Result<(), Box<dyn Error>> {

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