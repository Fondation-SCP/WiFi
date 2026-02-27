create table authors
(
    username varchar(255) not null,
    primary key (username),
    constraint username
        unique (username)
);

create table sites
(
    url  varchar(64)  not null,
    name varchar(255) null,
    primary key (url),
    constraint url
        unique (url)
);

create table categories
(
    id       int           not null,
    name     varchar(1024) null,
    site_url varchar(64)   not null,
    primary key (id),
    constraint id
        unique (id),
    constraint categories_ibfk_1
        foreign key (site_url) references sites (url)
);

create index site_url
    on categories (site_url);

create table threads
(
    id              int           not null,
    title           varchar(1024) null,
    description     text          null,
    creation_date   datetime      null,
    author_username varchar(255)  null,
    category_id     int           not null,
    primary key (id),
    constraint id
        unique (id),
    constraint threads_ibfk_1
        foreign key (author_username) references authors (username),
    constraint threads_ibfk_2
        foreign key (category_id) references categories (id)
);

create table messages
(
    id               int           not null,
    title            varchar(1024) null,
    content          longtext      null,
    author_username  varchar(255)  null,
    publication_date datetime      null,
    thread_id        int           not null,
    answers_to       int           null,
    primary key (id),
    constraint id
        unique (id),
    constraint messages_ibfk_1
        foreign key (author_username) references authors (username),
    constraint messages_ibfk_2
        foreign key (thread_id) references threads (id),
    constraint messages_ibfk_3
        foreign key (answers_to) references messages (id)
);

create index answers_to
    on messages (answers_to);

create index author
    on messages (author_username);

create index thread_id
    on messages (thread_id);

create index author_username
    on threads (author_username);

create index category_id
    on threads (category_id);


