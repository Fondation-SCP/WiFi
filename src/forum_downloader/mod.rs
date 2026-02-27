use crate::db_structs::{Author, Category, Insertable, Message, Site, Thread};
use crate::tools::{download_html, FutureIterator, TryFutureIterator};
use chrono::NaiveDateTime;
use futures_util::{StreamExt, TryStreamExt};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use sqlx::{Executor, MySql, Pool};
use std::error::Error;
use std::iter;
use std::sync::Arc;
use tokio::sync::Mutex;


lazy_static!(
    static ref FDL_SEL_GROUP: Selector = Selector::parse("div.forum-group").unwrap();
    static ref FDL_SEL_TR: Selector = Selector::parse("tr").unwrap();
    static ref CAT_SEL_TITLE: Selector = Selector::parse("td.name div.title a").unwrap();
    static ref CAT_SEL_THREADS: Selector = Selector::parse(".threads").unwrap();
    static ref CAT_SEL_POSTS: Selector = Selector::parse(".posts").unwrap();
    static ref SITE_REGEX: Regex = Regex::new(r#"https?:\/\/\w+.\w+.?\w*\/"#).unwrap();
    static ref CAT_ID_REGEX: Regex = Regex::new(r#"c-(\d+)"#).unwrap();
    static ref GT_SEL_TR: Selector = Selector::parse(".table tr").unwrap();
    static ref THD_SEL_TITLE: Selector = Selector::parse(".name .title a").unwrap();
    static ref THD_SEL_DESC: Selector = Selector::parse(".name .description").unwrap();
    static ref THD_SEL_DATE: Selector = Selector::parse(".started .odate").unwrap();
    static ref THD_SEL_POSTS: Selector = Selector::parse(".posts").unwrap();
    static ref THD_SEL_AUTHOR: Selector = Selector::parse(".started .printuser a").unwrap();
    static ref THD_ID_REGEX: Regex = Regex::new(r#"t-(\d+)"#).unwrap();
    static ref PM_SEL_POST: Selector = Selector::parse(".post").unwrap();
    static ref PM_SEL_CONTAINERS: Selector = Selector::parse(".post-container").unwrap();
    static ref PM_SEL_TITLE: Selector = Selector::parse(".long .head .title").unwrap();
    static ref PM_SEL_DATE: Selector = Selector::parse(".long .head .info .odate").unwrap();
    static ref PM_SEL_AUTHOR: Selector = Selector::parse(".long .head .info .printuser a").unwrap();
    static ref PM_SEL_CONTENT: Selector = Selector::parse(".long .content").unwrap();
    static ref GM_SEL_THREAD_CONTAINER_POSTS: Selector = Selector::parse("#thread-container-posts").unwrap();
);

pub(crate) struct ForumDownloader {
    authors: Arc<Mutex<Vec<Author>>>,
    messages: Arc<Mutex<Vec<Message>>>,
    client: Arc<reqwest::Client>,
    site: Arc<Site>,
    db: Arc<Pool<MySql>>
}

impl ForumDownloader {
    pub fn new(db: Arc<Pool<MySql>>, site: Arc<Site>) -> Self {
        Self {
            db,
            site,
            authors: Arc::new(Mutex::new(Vec::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            client: Arc::new(reqwest::Client::new()),
        }
    }

    pub async fn download(self, forum_url: &str) -> Result<(), Box<dyn Error>> {
        let doc = download_html(&self.client, format!("http://{}.wikidot.com/{forum_url}", self.site.url), 5)
            .await
            .expect("Too many failed attempts");

        let groups = doc.select(&FDL_SEL_GROUP);
        let categories = groups
            .flat_map(|group| {
                group.select(&FDL_SEL_TR).skip(1).map(|html| self.parse_category(html))
            }).join_all().await;

        let threads = categories.iter()
            .map(|(category, cat_addr)| self.download_threads(cat_addr.as_str(), category.id))
            .join_all().await.into_iter().flatten().collect::<Box<[_]>>();

        threads.iter().map(|(thread, thread_addr)| self.get_messages(thread_addr.as_str(), thread.id))
            .into_future_iter().buffer_unordered(crate::THREADS).collect::<Vec<_>>().await;

        println!("Found {} categories, {} threads, {} messages from {} authors.", categories.len(), threads.len(), self.messages.lock().await.len(), self.authors.lock().await.len());

        let mut trx = self.db.begin().await?;

        for category in categories.into_iter().map(|(c, _)| c) {
            category.query_insert().execute(&mut *trx).await.ok();
        }

        for author in self.authors.lock().await.iter() {
            author.query_insert().execute(&mut *trx).await.ok();
        }

        for thread in threads.into_iter().map(|(t, _)| t) {
            thread.query_insert().execute(&mut *trx).await.ok();
        }

        for message in self.messages.lock().await.iter() {
            message.query_insert().execute(&mut *trx).await.ok();
        }

        trx.commit().await?;

        Ok(())
    }

    async fn add_author(&self, author_username: &String) {
        let mut authors = self.authors.lock().await;
        if !authors.iter().any(|author| *author.username == *author_username) {
            authors.push(Author {username: author_username.clone() });
        }
    }

    async fn parse_category(&self, tr: ElementRef<'_>) -> (Category, String) {
        let url = tr.select(&CAT_SEL_TITLE).next()
            .and_then(|title| title.attr("href"))
            .unwrap_or_else(|| panic!("Can't find title for a category: {}", tr.inner_html()))
            .strip_prefix("/")
            .expect("Category URL is not relative (but it should be).")
            .rsplit_once('/')
            .unwrap()
            .0;
        (
            Category {
                id: CAT_ID_REGEX.find(url).unwrap().as_str().split_once("-").unwrap().1.parse().unwrap(),
                name: tr
                    .select(&CAT_SEL_TITLE)
                    .next().map(|title| title.inner_html()),
                site_url: self.site.url.clone()
            },
            url.to_string()
        )
    }

    async fn download_threads(&self, category_path: &str, category_id: i32) -> Box<[(Thread, String)]> {
        let category_url = format!("http://{}.wikidot.com/{category_path}", self.site.url);
        let doc = download_html(&self.client, category_url.as_str(), 5)
            .await
            .expect("Too many failed attempts");
        let pages_nb = _get_page_nb(&doc);

        (1..=pages_nb)
            .map(|i| format!("{category_url}/p/{i}"))
            .map(|page| download_html(self.client.clone(), page, 5))
            .into_future_iter().buffer_unordered(4)
            .try_collect::<Vec<_>>().await
            .expect("Too many failed attempts")
            .iter()
            .flat_map(|page| page.select(&GT_SEL_TR).skip(1).map(|tr| self.parse_thread(tr, category_id) ))
            .into_future_iter().buffer_unordered(crate::THREADS).collect::<Vec<_>>()
            .await.into_boxed_slice()
    }

    async fn parse_thread(&self, thread: ElementRef<'_>, category_id: i32) -> (Thread, String) {
        let title = thread.select(&THD_SEL_TITLE).next();
        let url = title
            .and_then(|link| link.attr("href"))
            .expect("No url for a forum thread")
            .strip_prefix("/")
            .expect("Thread URL is not relative (but it should be).")
            .rsplit_once('/')
            .unwrap()
            .0;


        let author_username = thread
            .select(&THD_SEL_AUTHOR).nth(1)
            .map(|author| author.inner_html().trim().to_string());

        if let Some(author_username) = author_username.as_ref() {
            self.add_author(author_username).await;
        }

        (
            Thread {
                id: THD_ID_REGEX.find(url).unwrap().as_str().split_once("-").unwrap().1.parse().unwrap(),
                title: title.map(|link| link.inner_html().trim().to_string()),
                description: thread
                    .select(&THD_SEL_DESC)
                    .next().as_ref()
                    .map(|e| e.inner_html().trim().to_string()),
                creation_date: thread
                    .select(&THD_SEL_DATE)
                    .next().as_ref().map(ElementRef::inner_html)
                    .and_then(|date| NaiveDateTime::parse_from_str(date.as_str(), WIKIDOT_DATE_FORMAT).ok()),
                author_username,
                category_id
            },
            url.to_string()
        )
    }

    async fn get_messages(&self, thread_path: &str, thread_id: i32) {
        let thread_url = format!("http://{}.wikidot.com/{thread_path}", self.site.url);
        let doc = download_html(&self.client, &thread_url, 5)
            .await
            .expect("Too many failed attempts");
        let pages_nb = _get_page_nb(&doc);

        let full_doc = Html::parse_fragment(
            iter::once(doc)
                .chain(
                    (2..=pages_nb)
                        .map(|i| format!("{thread_url}/p/{i}"))
                        .map(|url| download_html(&self.client, url, 5))
                        .try_join_all().await.expect("Too many failed attempts")
                )
                .map(|doc|
                    doc.select(&GM_SEL_THREAD_CONTAINER_POSTS)
                        .map(|doc| doc.inner_html())
                        .join("\n")
                ).join("\n").as_str()
        );


        let messages = full_doc
            .select(&PM_SEL_CONTAINERS)
            .flat_map(|message| self.parse_message(message, thread_id, None))
            .collect::<Box<[_]>>();

        {
            let mut authors = self.authors.lock().await;
            messages.iter().filter_map(|message| message.author_username.as_ref())
                .for_each(|author_username|
                if !authors.iter().any(|author| author.username == *author_username) {
                    authors.push(Author { username: author_username.clone() });
                }
            )
        }

        {
            let mut messages_vec = self.messages.lock().await;
            messages.into_iter().for_each(|message| messages_vec.push(message));
        }
    }

    fn parse_message<'a>(&'a self, post_container: ElementRef<'a>, thread_id: i32, answers_to: Option<i32>) -> Box<[Message]> {
        let mut children = post_container.child_elements();

        let Some(message) = children.next() else {
            eprintln!("No post in a post container.");
            return Box::default();
        };

        let author_username = message
            .select(&PM_SEL_AUTHOR).nth(1)
            .map(|author| author.inner_html().trim().to_string());

        let id = message.attr("id").unwrap().split_once("-").unwrap().1.parse().unwrap();


        iter::once(Message {
            id,
            title: message
                .select(&PM_SEL_TITLE)
                .next().map(|title| title.inner_html().trim().to_string()),
            publication_date: message
                .select(&PM_SEL_DATE)
                .next().as_ref().map(ElementRef::inner_html)
                .and_then(|date| NaiveDateTime::parse_from_str(date.as_str(), WIKIDOT_DATE_FORMAT).ok()),
            author_username,
            content: message
                .select(&PM_SEL_CONTENT)
                .next().map(|title| title.inner_html().trim().to_string()),
            thread_id,
            answers_to
        }).chain(children.flat_map(|post_container|
            self.parse_message(post_container, thread_id, Some(id))
        )).collect()

    }
}

const WIKIDOT_DATE_FORMAT: &str = "%d %b %Y %H:%M";


fn _get_page_nb(doc: &Html) -> i32 {
    let sel_pager = Selector::parse(".pager span").unwrap();

    doc.select(&sel_pager)
        .next()
        .and_then(|span| {
            span.inner_html()
                .split(" ")
                .last()
                .and_then(|page_str| page_str.parse::<i32>().ok())
        })
        .unwrap_or(1)


}



