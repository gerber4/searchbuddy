use crate::BoxResult;
use chrono::Local;
use log::error;
use scylla::{IntoTypedRows, Session, SessionBuilder};
use std::env;

pub struct Model {
    session: Session,
}

impl Model {
    pub async fn new() -> BoxResult<Self> {
        let scylla_urls = env::var("SCYLLA_URL")?;
        let scylla_urls: Vec<&str> = scylla_urls.split_whitespace().collect();

        // Generate all tables in the database.
        let session = SessionBuilder::new()
            .known_nodes(&scylla_urls)
            .build()
            .await?;

        session
            .query(
                r#"
                CREATE KEYSPACE IF NOT EXISTS searchbuddy
                WITH REPLICATION = {
                    'class': 'SimpleStrategy',
                    'replication_factor': 1
                };
                "#,
                (),
            )
            .await?;

        session.use_keyspace("searchbuddy", false).await?;

        session
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS chat (
                    chatroom_id int,
                    ts timestamp,
                    content text,
                    PRIMARY KEY(chatroom_id, ts),
                );
                "#,
                (),
            )
            .await?;

        Ok(Model { session })
    }

    pub async fn insert_chat(&self, chatroom_id: i32, content: &str) -> BoxResult<()> {
        let now = Local::now();

        self.session
            .query(
                r#"
                INSERT INTO chat (chatroom_id, ts, content)
                VALUES (?, ?, ?);
                "#,
                (&chatroom_id, now.timestamp_millis(), content),
            )
            .await?;

        Ok(())
    }

    pub async fn get_chats_from_today(&self, chatroom_id: i32) -> BoxResult<Vec<String>> {
        let now = Local::now();
        let date = now.date().and_hms(0, 0, 0);

        let rows = self
            .session
            .query(
                r#"SELECT content FROM chat WHERE chatroom_id = ? AND ts > ?"#,
                (&chatroom_id, date.timestamp_millis()),
            )
            .await?
            .rows
            .expect("Expected row response.")
            .into_typed::<(String,)>();

        let mut chats = Vec::new();

        for row in rows {
            match row {
                Ok((content,)) => {
                    chats.push(content);
                }
                Err(error) => {
                    error!("Invalid row in data found - {:?}", error);
                }
            }
        }

        Ok(chats)
    }
}
