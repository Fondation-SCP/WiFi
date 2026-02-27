use crate::Query;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{query, FromRow};

pub trait Insertable {
    fn query_insert(&'_ self) -> Query<'_>;
    const CREATE_TABLE: &str;
    const DELETE_TABLE: &str;
    fn query_create_table() -> Query<'static> {
        sqlx::query(Self::CREATE_TABLE)
    }
    fn query_delete_table() -> Query<'static> {
        sqlx::query(Self::DELETE_TABLE)
    }
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Site {
    pub url: String,
    pub name: Option<String>
}

impl PartialEq for Site {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}
impl Eq for Site {}

impl Insertable for Site {
    fn query_insert(&'_ self) -> Query<'_> {
        query!(
            "insert into sites (url, name) values (?, ?)",
            self.url,
            self.name
        )
    }

    const CREATE_TABLE: &str = "
    create table if not exists sites (
        url varchar(64) unique not null,
        name varchar(255),
        primary key(url)
    )
    ";

    const DELETE_TABLE: &str = "drop table if exists sites";
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i32,
    pub name: Option<String>,
    pub site_url: String
}

impl PartialEq for Category {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Category {}

impl Insertable for Category {
    fn query_insert(&'_ self) -> Query<'_> {
        query!(
            "insert into categories (id, name, site_url) values (?, ?, ?)",
            self.id,
            self.name,
            self.site_url
        )
    }

    const CREATE_TABLE: &str = "
    create table if not exists categories (
        id int unique not null,
        name varchar(1024),
        site_url varchar(64) not null,
        foreign key(site_url)
            references sites(url),
        primary key(id)
    )
";
    const DELETE_TABLE: &str = "drop table if exists categories";
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Author {
    pub username: String,
}

impl PartialEq for Author {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
    }
}

impl Eq for Author {}

impl Insertable for Author {
    fn query_insert(&'_ self) -> Query<'_> {
        query!(
            "insert into authors (username) values (?)",
            self.username
        )
    }

    const CREATE_TABLE: &str = "
    create table if not exists authors (
        username varchar(255) unique not null,
        primary key(username)
    )
";
    const DELETE_TABLE: &str = "drop table if exists authors";
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: i32,
    pub title: Option<String>,
    pub description: Option<String>,
    pub creation_date: Option<NaiveDateTime>,
    pub author_username: Option<String>,
    pub category_id: i32
}

impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Thread {}

impl Insertable for Thread {
    fn query_insert(&'_ self) -> Query<'_> {
        query!(
            "insert into threads (id, title, description, creation_date, author_username, category_id) values (?, ?, ?, ?, ?, ?)",
            self.id,
            self.title,
            self.description,
            self.creation_date,
            self.author_username,
            self.category_id
        )
    }

    const CREATE_TABLE: &str = "
    create table if not exists threads (
        id int unique not null,
        title varchar(1024),
        description text,
        creation_date datetime,
        author_username varchar(255),
        category_id int not null,
        foreign key(author_username)
            references authors(username),
        foreign key(category_id)
            references categories(id),
        primary key(id)
    )
";
    const DELETE_TABLE: &str = "drop table if exists threads";
}

#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i32,
    pub title: Option<String>,
    pub content: Option<String>,
    pub author_username: Option<String>,
    pub publication_date: Option<NaiveDateTime>,
    pub thread_id: i32,
    pub answers_to: Option<i32>
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Message {}

impl Insertable for Message {
    fn query_insert(&'_ self) -> Query<'_> {
        query!(
            "insert into messages (id, title, content, author_username, publication_date, thread_id, answers_to) values (?, ?, ?, ?, ?, ?, ?)",
            self.id,
            self.title,
            self.content,
            self.author_username,
            self.publication_date,
            self.thread_id,
            self.answers_to
        )
    }
    const CREATE_TABLE: &str = "
    create table if not exists messages (
        id int unique not null,
        title varchar(1024),
        content longtext,
        author varchar(255),
        publication_date datetime,
        thread_id int not null,
        answers_to int,
        foreign key(author)
            references authors(username),
        foreign key(thread_id)
            references threads(id),
        foreign key(answers_to)
            references messages(id),
        primary key(id)
    )
";

    const DELETE_TABLE: &str = "drop table if exists messages";
}