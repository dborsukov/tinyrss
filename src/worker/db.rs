use crate::worker::utils;
use sqlx::Result;
use sqlx::{Connection, SqliteConnection};

async fn establish_connection() -> Result<SqliteConnection> {
    let app_dir = utils::get_app_dir();
    SqliteConnection::connect(app_dir.join("tinyrss.db").to_str().unwrap()).await
}

pub async fn create_tables() -> Result<()> {
    todo!("Initialize database with tables");
}
