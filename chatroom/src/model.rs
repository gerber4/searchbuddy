use crate::BoxResult;
use chrono::Local;
use futures::StreamExt;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::env;

pub struct Model {
    pool: Pool<Postgres>,
}

impl Model {
    pub async fn new() -> BoxResult<Self> {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not defined.");
        let database_url = database_url.trim();
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Model { pool })
    }

    pub async fn insert_chat(&self, chatroom_id: i32, content: &str) -> BoxResult<()> {
        let mut conn = self.pool.acquire().await?;

        let now = Local::now();

        sqlx::query(
            r#"
            INSERT INTO chat (chatroom_id, ts, content)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(chatroom_id as i64)
        .bind(&now)
        .bind(content)
        .execute(&mut conn)
        .await?;

        Ok(())
    }

    pub async fn get_chats_from_today(&self, chatroom_id: i32) -> BoxResult<Vec<String>> {
        let mut conn = self.pool.acquire().await?;

        let now = Local::now();
        let date = now.date().and_hms(0, 0, 0);

        let mut chats_stream =
            sqlx::query("SELECT content FROM chat WHERE chatroom_id = $1 AND ts > $2")
                .bind(chatroom_id as i64)
                .bind(&date)
                .fetch(&mut conn);

        let mut chats: Vec<String> = Vec::new();

        while let Some(chat) = chats_stream.next().await {
            if let Ok(chat) = chat {
                chats.push(chat.get(0))
            }
        }

        Ok(chats)
    }
}
