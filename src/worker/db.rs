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
            link VARCHAR NOT NULL,
            title VARCHAR,
            description VARCHAR
        );
        CREATE TABLE IF NOT EXISTS items (
            id VARCHAR NOT NULL UNIQUE PRIMARY KEY,
            link VARCHAR NOT NULL,
            title VARCHAR,
            summary VARCHAR,
            published INTEGER,
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
pub struct Item {}

pub async fn add_channel(channel: Channel) -> Result<()> {
    let mut conn = establish_connection().await?;

    query("INSERT INTO channels (id, kind, link, title, description) VALUES (?, ?, ?, ?, ?)")
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
