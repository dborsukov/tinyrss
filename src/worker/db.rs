use crate::worker::utils;
use sqlx::{query, query_as, FromRow, Result};
use sqlx::{Connection, SqliteConnection};

async fn establish_connection() -> Result<SqliteConnection> {
    let app_dir = utils::get_app_dir();
    SqliteConnection::connect(app_dir.join("tinyrss.db").to_str().unwrap()).await
}

pub async fn create_tables() -> Result<()> {
    let mut conn = establish_connection().await?;
    query(
        "
        CREATE TABLE IF NOT EXISTS channels (
            id VARCHAR NOT NULL UNIQUE PRIMARY KEY,
            kind VARCHAR NOT NULL,
            link VARCHAR NOT NULL UNIQUE,
            title VARCHAR,
            description VARCHAR
        );
        CREATE TABLE IF NOT EXISTS items (
            id VARCHAR NOT NULL UNIQUE PRIMARY KEY,
            link VARCHAR NOT NULL,
            title VARCHAR,
            summary VARCHAR,
            published INTEGER,
            dismissed BOOLEAN NOT NULL,
            channel_title VARCHAR,
            channel VARCHAR NOT NULL,
            FOREIGN KEY (channel) REFERENCES channels (id) ON DELETE CASCADE
        );
    ",
    )
    .execute(&mut conn)
    .await?;
    Ok(())
}

#[derive(Debug, Default, FromRow)]
pub struct Channel {
    pub id: String,
    pub kind: String,
    pub link: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Default, FromRow)]
pub struct Item {
    pub id: String,
    pub link: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub published: i64,
    pub dismissed: bool,
    pub channel_title: Option<String>,
    pub channel: String,
}

pub async fn add_channel(channel: Channel) -> Result<()> {
    let mut conn = establish_connection().await?;

    query("INSERT OR IGNORE INTO channels (id, kind, link, title, description) VALUES (?, ?, ?, ?, ?)")
        .bind(channel.id)
        .bind(channel.kind)
        .bind(channel.link)
        .bind(channel.title)
        .bind(channel.description)
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub async fn get_all_channels() -> Result<Vec<Channel>> {
    let mut conn = establish_connection().await?;

    let channels =
        query_as::<_, Channel>("SELECT id, kind, link, title, description FROM channels")
            .fetch_all(&mut conn)
            .await?;

    Ok(channels)
}

pub async fn add_items(items: Vec<Item>) -> Result<()> {
    let mut conn = establish_connection().await?;

    let mut tz = conn.begin().await?;

    for item in items {
        query("INSERT OR IGNORE INTO items (id, link, title, summary, published, dismissed, channel_title, channel) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(item.id)
            .bind(item.link)
            .bind(item.title)
            .bind(item.summary)
            .bind(item.published)
            .bind(item.dismissed)
            .bind(item.channel_title)
            .bind(item.channel)
            .execute(&mut tz)
            .await?;
    }

    tz.commit().await?;

    Ok(())
}

pub async fn get_all_items() -> Result<Vec<Item>> {
    let mut conn = establish_connection().await?;

    let items = query_as::<_, Item>(
        "SELECT id, link, title, summary, published, dismissed, channel_title, channel FROM items ORDER BY published DESC",
    )
    .fetch_all(&mut conn)
    .await?;

    Ok(items)
}

pub async fn set_dismissed(id: &str, dismissed: bool) -> Result<()> {
    let mut conn = establish_connection().await?;

    query("UPDATE items SET dismissed = ? WHERE id = ?")
        .bind(dismissed)
        .bind(id)
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub async fn dismiss_all() -> Result<()> {
    let mut conn = establish_connection().await?;

    query("UPDATE items SET dismissed = True")
        .execute(&mut conn)
        .await?;

    Ok(())
}

pub async fn unsubscribe(id: &str) -> Result<()> {
    let mut conn = establish_connection().await?;

    query("DELETE FROM channels WHERE id = ?")
        .bind(id)
        .execute(&mut conn)
        .await?;

    Ok(())
}
