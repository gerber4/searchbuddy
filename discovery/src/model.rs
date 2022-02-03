use crate::BoxResult;
use async_recursion::async_recursion;
use chrono::{Duration, Utc};
use log::error;
use rand::prelude::IteratorRandom;
use rand::thread_rng;
use scylla::{IntoTypedRows, Session, SessionBuilder};
use shared::discovery::*;
use std::env;
use std::iter::Iterator;
use std::net::SocketAddrV4;

pub struct Model {
    pub session: Session,
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
                CREATE TABLE IF NOT EXISTS instance (
                    region text,
                    address text,
                    instance_id int,
                    last_accessed bigint,
                    PRIMARY KEY(region, address),
                );
                "#,
                (),
            )
            .await?;

        session
            .query(
                r#"
                CREATE TABLE IF NOT EXISTS chatroom (
                    term text,
                    address text,
                    instance_id int,
                    PRIMARY KEY(term),
                );
                "#,
                (),
            )
            .await?;

        Ok(Model { session })
    }

    pub async fn register_instance(&self, address: &SocketAddrV4) -> BoxResult<i32> {
        let instance_id = rand::random::<i32>();
        let now = Utc::now();

        self.session
            .query(
                r#"
                INSERT INTO instance (region, address, instance_id, last_accessed)
                VALUES (?, ?, ?, ?);
                "#,
                (
                    &"US1",
                    &format!("{}", address),
                    &instance_id,
                    now.timestamp_millis(),
                ),
            )
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

        let mut rows = self
            .session
            .query(
                r#"
                SELECT address, instance_id
                FROM instance
                WHERE region = ? and address = ? and instance_id = ? and last_accessed >= ?
                ALLOW FILTERING"#,
                (
                    &"US1",
                    &format!("{}", address),
                    &instance_id,
                    threshold.timestamp_millis(),
                ),
            )
            .await?
            .rows
            .expect("Expected row response.")
            .into_typed::<(String, i32)>();

        if let Some(row) = rows.next() {
            let (address, _instance_id) = row?;

            let now = Utc::now();

            self.session
                .query(
                    r#"
                    UPDATE instance
                    SET last_accessed = ?
                    WHERE region = ? and address = ?"#,
                    (now.timestamp_millis(), &"US1", &address),
                )
                .await?;

            Ok(PingResult::Ok)
        } else {
            Ok(PingResult::NoLongerActive)
        }
    }

    #[async_recursion]
    pub async fn get_chatroom(&self, term: &str) -> BoxResult<Option<Instance>> {
        let active_instances = self.get_instances().await?;

        let mut rows = self
            .session
            .query(
                r#"
                SELECT term, address, instance_id
                FROM chatroom
                WHERE term = ?"#,
                (term,),
            )
            .await?
            .rows
            .expect("Expected row response.")
            .into_typed::<(String, String, i32)>();

        let mut instance: Option<Instance> = None;
        if let Some(row) = rows.next() {
            let (_term, _address, instance_id) = row?;
            if let Some(active_instance) = active_instances
                .iter()
                .find(|instance| instance.instance_id == instance_id)
            {
                instance = Some(*active_instance);
            }
        }

        return match instance {
            Some(instance) => Ok(Some(instance)),
            None => {
                // Either there is no associated instance or the associated instance is no longer
                // valid. Choose a new instance.

                let new_instance = active_instances.iter().choose(&mut thread_rng());

                match new_instance {
                    Some(new_instance) => {
                        self.session
                            .query(
                                r#"
                                INSERT INTO chatroom (term, address, instance_id)
                                VALUES (?, ?, ?)
                                "#,
                                (
                                    term,
                                    &format!("{}", new_instance.address),
                                    &new_instance.instance_id,
                                ),
                            )
                            .await?;

                        self.get_chatroom(term).await
                    }

                    None => Ok(None),
                }
            }
        };
    }

    async fn get_instances(&self) -> BoxResult<Vec<Instance>> {
        let now = Utc::now();
        let threshold = now.checked_sub_signed(Duration::seconds(10)).unwrap();

        let rows = self
            .session
            .query(
                r#"
                SELECT address, instance_id
                FROM instance
                WHERE region = ? and last_accessed >= ?
                ALLOW FILTERING"#,
                (&"US1", threshold.timestamp_millis()),
            )
            .await?
            .rows
            .expect("Expected row response.")
            .into_typed::<(String, i32)>();

        let mut instances = Vec::new();

        for row in rows {
            match row {
                Ok((address, instance_id)) => {
                    let address: SocketAddrV4 = address
                        .parse()
                        .expect("Invalid address stored in database.");

                    instances.push(Instance {
                        instance_id,
                        address,
                    });
                }
                Err(error) => {
                    error!("Invalid row in data found - {:?}", error);
                }
            }
        }

        Ok(instances)
    }
}
