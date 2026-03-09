//! Downloads a Wikidot forum and adds its data into the database

use crate::objects::{Author, Category, Insertable, Message, Site, Thread};
use crate::errors::ApiError;
use crate::tools::{download, FutureIterator};
use chrono::NaiveDateTime;
use itertools::Itertools;
use scraper::{ElementRef, Html};
use sqlx::{MySql, Pool};
use std::{iter, mem};
use std::sync::Arc;
use futures_util::{StreamExt, TryStreamExt};
use tokio::sync::RwLock;

/// Static variables ([Selectors](scraper::Selector) and [Regexes](regex::Regex)) used for scrapping.
mod statics {
    use lazy_static::lazy_static;
    use regex::Regex;
    use scraper::Selector;

    lazy_static!(
        /// Selects a [Category](crate::objects::Category) group on the forum.
        ///
        /// Parsed from `div.forum-group`.
        pub(super) static ref FDL_SEL_GROUP: Selector = Selector::parse("div.forum-group").unwrap();
        /// Selects `<tr>`.
        pub(super) static ref FDL_SEL_TR: Selector = Selector::parse("tr").unwrap();
        /// Selects the [name of a Category](crate::objects::Category::name).
        ///
        /// Parsed from `td.name div.title a`.
        pub(super) static ref CAT_SEL_TITLE: Selector = Selector::parse("td.name div.title a").unwrap();
        /// Selects the number of threads in a [Category](crate::objects::Category).
        ///
        /// Parsed from `.threads`.
        pub(super) static ref CAT_SEL_THREADS: Selector = Selector::parse(".threads").unwrap();
        /// Selects the number of messages in a [Category](crate::objects::Category).
        ///
        /// Parsed from `.posts`.
        pub(super) static ref CAT_SEL_POSTS: Selector = Selector::parse(".posts").unwrap();
        /// Matches the domain name of a URL:
        ///
        /// Parsed from `https?:\/\/\w+.\w+.?\w*\/`.
        pub(super) static ref SITE_REGEX: Regex = Regex::new(r#"https?:\/\/\w+.\w+.?\w*\/"#).unwrap();
        /// Matches the [ID of a Category](crate::objects::Category::id) in the URL.
        ///
        /// Parsed from `c-(\d+)`.
        pub(super) static ref CAT_ID_REGEX: Regex = Regex::new(r#"c-(\d+)"#).unwrap();
        /// Selects a line of the list of [Threads](crate::objects::Thread) in a [Category](crate::objects::Category).
        ///
        /// Parsed from `.table tr`.
        pub(super) static ref GT_SEL_TR: Selector = Selector::parse(".table tr").unwrap();
        /// Selects the [title of a Thread](crate::objects::Thread::title).
        ///
        /// Parsed from `.name .title a`.
        pub(super) static ref THD_SEL_TITLE: Selector = Selector::parse(".name .title a").unwrap();
        /// Selects the [description of a Thread](crate::objects::Thread::description).
        ///
        /// Parsed from `.name .description`.
        pub(super) static ref THD_SEL_DESC: Selector = Selector::parse(".name .description").unwrap();
        /// Selects the [creation date of a Thread](crate::objects::Thread::creation_date).
        ///
        /// Parsed from `.started .odate`.
        pub(super) static ref THD_SEL_DATE: Selector = Selector::parse(".started .odate").unwrap();
        /// Selects the number of messages in a [Thread](crate::objects::Thread).
        ///
        /// Parsed from `.posts`.
        pub(super) static ref THD_SEL_POSTS: Selector = Selector::parse(".posts").unwrap();
        /// Selects the [username of the author of the Thread](crate::objects::Thread::author_username).
        ///
        /// Parsed from `.started .printuser a`.
        pub(super) static ref THD_SEL_AUTHOR: Selector = Selector::parse(".started .printuser a").unwrap();
        /// Matches the [ID of a Thread](crate::objects::Thread::id) in the URL.
        ///
        /// Parsed from `t-(\d+)`.
        pub(super) static ref THD_ID_REGEX: Regex = Regex::new(r#"t-(\d+)"#).unwrap();
        /// Selects a [Message](crate::objects::Message) in a [Thread](crate::objects::Thread)
        ///
        /// Parsed from `.post`.
        pub(super) static ref PM_SEL_POST: Selector = Selector::parse(".post").unwrap();
        /// Selects a post container in a [Thread](crate::objects::Thread).
        ///
        /// Parsed from `.post-container`.
        pub(super) static ref PM_SEL_CONTAINERS: Selector = Selector::parse(".post-container").unwrap();
        /// Selects the [title of a Message](crate::objects::Message::title).
        ///
        /// Parsed from `.long .head .title`.
        pub(super) static ref PM_SEL_TITLE: Selector = Selector::parse(".long .head .title").unwrap();
        /// Selects the [publication date of a Message](crate::objects::Message::publication_date).
        ///
        /// Parsed from `.long .head .info .odate`.
        pub(super) static ref PM_SEL_DATE: Selector = Selector::parse(".long .head .info .odate").unwrap();
        /// Selects the [username of the author of a Message](crate::objects::Message::author_username).
        ///
        /// Parsed from `.long .head .info .printuser a`.
        pub(super) static ref PM_SEL_AUTHOR: Selector = Selector::parse(".long .head .info .printuser a").unwrap();
        /// Selects the [content of a Message](crate::objects::Message::content).
        ///
        /// Parsed from `.long .content`.
        pub(super) static ref PM_SEL_CONTENT: Selector = Selector::parse(".long .content").unwrap();
        /// Selects the thread container posts, that contains all [Messages](crate::objects::Message) of a [Thread](crate::objects::Thread).
        ///
        /// Parsed from `#thread-container-posts`.
        pub(super) static ref GM_SEL_THREAD_CONTAINER_POSTS: Selector = Selector::parse("#thread-container-posts").unwrap();
        /// Selects the pager for pages with multiples pages.
        ///
        /// Parsed from `.pager span`.
        pub(super) static ref SEL_PAGER: Selector = Selector::parse(".pager span").unwrap();
    );
}

use statics::*;
use crate::{FATAL_ERROR};
use crate::config::Config;

pub(crate) struct ForumDownloader {
    authors: RwLock<Vec<Author>>,
    messages: RwLock<Vec<Message>>,
    client: Arc<reqwest::Client>,
    site: Site,
    db: Pool<MySql>,
    cfg: Config
}

impl ForumDownloader {
    pub fn new(db: Pool<MySql>, cfg: Config, site: Site) -> Arc<Self> {
        Self {
            db,
            site,
            authors: RwLock::default(),
            messages: RwLock::default(),
            client: Arc::new(reqwest::Client::new()),
            cfg
        }.into()
    }

    pub async fn download(self: Arc<Self>, forum_url: String) -> Result<(), ApiError> {
        let categories: Box<[_]> = {
            let doc = download(self.client.clone(), format!("http://{}.wikidot.com/{forum_url}", self.site.url), 5).await?;

            Html::parse_document(doc.as_str()).select(&FDL_SEL_GROUP)
                .flat_map(|group| {
                    group.select(&FDL_SEL_TR).skip(1).map(|html| self.clone().parse_category(html))
                }).collect::<Result<_, _>>()?
        };

        let (download_threads, categories): (Vec<_>, Vec<_>) = categories.into_iter()
            .map(|(category, cat_addr)| ((category.id, cat_addr), category)).unzip();

        let (get_messages, threads): (Vec<_>, Vec<_>) = download_threads.into_iter()
            .map(|(category, cat_addr)| self.clone().download_threads(cat_addr.as_str().into(), category))
            .into_future_iter().buffer_unordered(self.cfg.parallel_tasks).try_collect::<Vec<_>>().await?.into_iter().flatten()
            .map(|(thread, thread_addr)| ((thread.id, thread_addr), thread)).unzip();

        get_messages.into_iter().map(|(thread, thread_addr)| self.clone().get_messages(thread_addr.as_str().into(), thread))
            .into_future_iter().buffer_unordered(self.cfg.parallel_tasks).try_collect::<Vec<_>>().await?;

        let mut this = Arc::into_inner(self).expect(FATAL_ERROR);
        let authors = mem::take(&mut this.authors).into_inner();
        let messages = mem::take(&mut this.messages).into_inner();

        println!("Found {} categories, {} threads, {} messages from {} authors.", categories.len(), threads.len(), messages.len(), authors.len());

        let mut trx = this.db.begin().await?;

        for category in categories {
            category.query_insert().execute(&mut *trx).await.ok();
        }

        for author in authors {
            author.query_insert().execute(&mut *trx).await.ok();
        }

        for thread in threads {
            thread.query_insert().execute(&mut *trx).await.ok();
        }

        for message in messages {
            message.query_insert().execute(&mut *trx).await.ok();
        }

        trx.commit().await?;

        Ok(())
    }

    async fn add_author(self: Arc<Self>, author_username: &str) {
        if !self.authors.read().await.iter().any(|author| *author.username == *author_username) {
            self.authors.write().await.push(Author {username: author_username.to_string() });
        }
    }

    fn parse_category(self: Arc<Self>, tr: ElementRef<'_>) -> Result<(Category, String), ApiError> {
        let url = tr.select(&CAT_SEL_TITLE).next()
            .and_then(|title| title.attr("href"))
            .ok_or_else(|| ApiError::ParsingError {
                details: "Category: can't find link",
                at: tr.inner_html()
            })?
            .strip_prefix("/")
            .ok_or_else(|| ApiError::ParsingError {
                details: "Category: url not prefixed by '/'",
                at: tr.inner_html()
            })?
            .rsplit_once('/')
            .ok_or_else(|| ApiError::ParsingError {
                details: "Category: url does not contain '/'",
                at: tr.inner_html()
            })?
            .0;
        Ok((
            Category {
                id: CAT_ID_REGEX.find(url).and_then(|m| m.as_str().split_once("-"))
                    .ok_or_else(|| ApiError::ParsingError {
                        details: "Category: could not find id",
                        at: url.to_string(),
                    }).and_then(|(_, id)| id.parse().map_err(|_| ApiError::ParsingError {
                        details: "Category: could not parse id",
                        at: id.to_string()
                    }))?,
                name: tr
                    .select(&CAT_SEL_TITLE)
                    .next().map(|title| title.inner_html()),
                site_url: self.site.url.clone()
            },
            url.to_string()
        ))
    }

    async fn download_threads(self: Arc<Self>, category_path: Arc<str>, category_id: i32) -> Result<Box<[(Thread, String)]>, ApiError> {
        let category_url = format!("http://{}.wikidot.com/{category_path}", self.site.url);
        let pages_nb = _get_page_nb(
            &Html::parse_document(download(self.client.clone(), category_url.as_str(), 5).await?.as_str())
        );

        let threads: Box<_> = (1..=pages_nb)
            .map(|i| format!("{category_url}/p/{i}"))
            .map(|page| download(self.client.clone(), page, 5))
            .into_future_iter().buffer_unordered(self.cfg.parallel_tasks).try_collect::<Vec<_>>().await?
            .iter()
            .map(String::as_str).map(Html::parse_document)
            .flat_map(|page| page
                .select(&GT_SEL_TR)
                .skip(1)
                .map(|tr| self.clone().parse_thread(tr, category_id))
                .collect::<Box<[_]>>()
            ).collect::<Result<_, _>>()?;

        for author_username in threads.iter().filter_map(|(thread, _)| thread.author_username.as_ref()) {
            self.clone().add_author(author_username).await;
        }

        Ok(threads)
    }

    fn parse_thread(self: Arc<Self>, thread: ElementRef<'_>, category_id: i32) -> Result<(Thread, String), ApiError> {
        let title = thread.select(&THD_SEL_TITLE).next();
        let url = title
            .and_then(|link| link.attr("href"))
            .ok_or_else(|| ApiError::ParsingError {
                details: "Thread: can't find link",
                at: thread.inner_html()
            })?
            .strip_prefix("/")
            .ok_or_else(|| ApiError::ParsingError {
                details: "Thread: url not prefixed by '/'",
                at: thread.inner_html()
            })?
            .rsplit_once('/')
            .ok_or_else(|| ApiError::ParsingError {
                details: "Thread: url does not contain '/'",
                at: thread.inner_html()
            })?
            .0;


        let author_username = thread
            .select(&THD_SEL_AUTHOR).nth(1)
            .map(|author| author.inner_html().trim().to_string());

        Ok((
            Thread {
                id: THD_ID_REGEX.find(url)
                    .and_then(|m| m.as_str().split_once("-"))
                    .ok_or_else(|| ApiError::ParsingError {
                        details: "Thread: could not find id",
                        at: url.to_string(),
                    }).and_then(|(_, id)| id.parse().map_err(|_| ApiError::ParsingError {
                        details: "Thread: could not parse id",
                        at: id.to_string()
                    }))?,
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
        ))
    }

    async fn get_messages(self: Arc<Self>, thread_path: Arc<str>, thread_id: i32) -> Result<(), ApiError> {
        let thread_url = format!("http://{}.wikidot.com/{thread_path}", self.site.url);
        let pages_nb = _get_page_nb(
            &Html::parse_document(
                download(
                    self.client.clone(),
                    thread_url.as_str(),
                    5
                ).await?.as_str()
            )
        );

        let messages = {
            let full_doc = Html::parse_fragment(
                (1..=pages_nb)
                    .map(|i| format!("{thread_url}/p/{i}"))
                    .map(|url| download(self.client.clone(), url, 5))
                    .into_future_iter().buffered(self.cfg.parallel_tasks).try_collect::<Vec<_>>().await?
                    .iter()
                    .map(String::as_str)
                    .map(Html::parse_document)
                    .map(|doc|
                        doc.select(&GM_SEL_THREAD_CONTAINER_POSTS)
                            .map(|doc| doc.inner_html())
                            .join("\n")
                    ).join("\n").as_str()
            );

            full_doc
                .select(&PM_SEL_CONTAINERS)
                .map(|message| self.parse_message(message, thread_id, None))
                .flatten_ok()
                .collect::<Result<Box<[_]>, _>>()?
        };

        for author_username in messages.iter().filter_map(|message| message.author_username.as_ref()) {
            self.clone().add_author(author_username).await;
        }

        for message in messages {
            self.clone().messages.write().await.push(message)
        }

        Ok(())
    }

    fn parse_message<'a>(&'a self, post_container: ElementRef<'a>, thread_id: i32, answers_to: Option<i32>) -> Result<Box<[Message]>, ApiError> {
        let mut children = post_container.child_elements();

        let message = children.next().ok_or_else(|| ApiError::ParsingError {
            details: "Message: empty post-container",
            at: post_container.inner_html()
        })?;

        let author_username = message
            .select(&PM_SEL_AUTHOR).nth(1)
            .map(|author| author.inner_html().trim().to_string());

        let id = message.attr("id")
            .and_then(|s| s.split_once("-"))
            .ok_or_else(|| ApiError::ParsingError {
                details: "Message: could not find id",
                at: message.inner_html(),
            }).and_then(|(_, id)| id.parse().map_err(|_| ApiError::ParsingError {
                details: "Message: could not parse id",
                at: id.to_string()
            }))?;


        Ok(iter::once(Message {
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
        }).chain(
            children.map(|post_container|
                self.parse_message(post_container, thread_id, Some(id))
            ).flatten_ok().collect::<Result<Box<[_]>, _>>()?
        ).collect())

    }
}

const WIKIDOT_DATE_FORMAT: &str = "%d %b %Y %H:%M";


fn _get_page_nb(doc: &Html) -> i32 {
    doc.select(&SEL_PAGER)
        .next()
        .and_then(|span| {
            span.inner_html()
                .split(" ")
                .last()
                .and_then(|page_str| page_str.parse::<i32>().ok())
        })
        .unwrap_or(1)


}



