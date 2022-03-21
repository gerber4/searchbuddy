use crate::BoxResult;
use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use shared::discovery::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres, Row};
use std::env;
use std::iter::Iterator;
use std::net::SocketAddrV4;

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

    pub async fn register_instance(&self, address: &SocketAddrV4) -> BoxResult<i32> {
        let instance_id = rand::random::<i32>();
        let now = Utc::now();

        let mut conn = self.pool.acquire().await?;

        sqlx::query(
            r#"
            INSERT INTO instance (region, address, instance_id, last_accessed)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&"USA1")
        .bind(&format!("{}", address))
        .bind(instance_id as i64)
        .bind(now)
        .execute(&mut conn)
        .await?;

        Ok(instance_id)
    }

    pub async fn ping_instance(
        &self,
        address: &SocketAddrV4,
        instance_id: i32,
    ) -> BoxResult<PingResult> {
        let now = Utc::now();
        let threshold = now.checked_sub_signed(Duration::seconds(10)).unwrap();

        let mut conn = self.pool.acquire().await?;

        let instance = sqlx::query(
            r#"
            SELECT instance_id
            FROM instance
            WHERE region = $1 and address = $2 and instance_id = $3 and last_accessed >= $4
            "#,
        )
        .bind(&"USA1")
        .bind(&format!("{}", address))
        .bind(instance_id as i64)
        .bind(threshold)
        .fetch_optional(&mut conn)
        .await?;

        if let Some(_instance) = instance {
            // If we are pinged by an active instance, then update the last_accessed field.
            let now = Utc::now();

            sqlx::query(
                r#"
                UPDATE instance
                SET last_accessed = $1
                WHERE region = $2 and address = $3
                "#,
            )
            .bind(now)
            .bind(&"USA1")
            .bind(&format!("{}", address))
            .execute(&mut conn)
            .await?;

            Ok(PingResult::Ok)
        } else {
            Ok(PingResult::NoLongerActive)
        }
    }

    #[async_recursion]
    pub async fn get_chatroom(&self, term: &str) -> BoxResult<Option<Instance>> {
        let active_instances = self.get_instances().await?;

        let mut conn = self.pool.acquire().await?;

        let chatroom = sqlx::query(
            r#"
            SELECT instance_id
            FROM chatroom
            WHERE term = $1
            "#,
        )
        .bind(term)
        .fetch_optional(&mut conn)
        .await?;

        if let Some(chatroom) = chatroom
            && let Some(instance) = active_instances.iter().find(|instance| instance.instance_id as i64 == chatroom.get::<i64, &str>("instance_id")) {
            // If a chatroom exists and it is associated with an active instance, then return the active instance.
            Ok(Some(*instance))
        } else {
            // Either there is no associated instance or the associated instance is no longer
            // valid. Choose a new instance.

            let new_instance = active_instances.iter().choose(&mut thread_rng());

            if let Some(instance) = new_instance {
                sqlx::query(
                    r#"
                    INSERT INTO chatroom (term, address, instance_id)
                    VALUES ($1, $2, $3)
                    "#)
                    .bind(term)
                    .bind(&format!("{}", instance.address))
                    .bind(instance.instance_id as i64)
                    .execute(&mut conn)
                    .await?;

                Ok(Some(*instance))
            } else {
                Ok(None)
            }
        }
    }

    async fn get_instances(&self) -> BoxResult<Vec<Instance>> {
        let mut conn = self.pool.acquire().await?;

        let now = Utc::now();
        let threshold = now.checked_sub_signed(Duration::seconds(10)).unwrap();

        let instances = sqlx::query(
            r#"
            SELECT instance_id, address
            FROM instance
            WHERE region = $1 and last_accessed >= $2
            "#,
        )
        .bind(&"USA1")
        .bind(threshold)
        .fetch_all(&mut conn)
        .await?;

        let instances = instances
            .into_iter()
            .filter_map(|row| {
                let instance_id: i64 = row.get("instance_id");
                let address: SocketAddrV4 = row.get::<&str, &str>("address").parse().ok()?;

                Some(Instance {
                    instance_id: instance_id as i32,
                    address,
                })
            })
            .collect();

        Ok(instances)
    }
}
