#![feature(try_blocks)]

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use log::{error, info};
use serde::Deserialize;
use shared::discovery::{ChatroomRequest, ChatroomResponse};
use shared::{initialize_logger, Chatroom};
use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddrV4;
use tokio::runtime::Runtime;

type BoxError = Box<dyn Error + Send + Sync>;
type BoxResult<T> = Result<T, BoxError>;

#[derive(Debug, Deserialize)]
struct ChatroomQuery {
    search: String,
}

async fn get_chatrooms(Query(query): Query<ChatroomQuery>) -> Json<Vec<Chatroom>> {
    info!("GET /chatrooms: {:?}", query);

    let terms: Vec<&str> = query.search.split(" ").collect();

    let instances = locate_instances(&terms).await;

    let client = reqwest::Client::new();

    let mut chatrooms = Vec::new();

    for (address, terms) in instances {
        let response: BoxResult<HashMap<String, (i32, u32)>> = try {
            client
                .post(format!("http://{}/chatrooms", address))
                .json(&terms)
                .send()
                .await?
                .json::<HashMap<String, (i32, u32)>>()
                .await?
        };

        match response {
            Ok(response) => {
                for (term, (chatroom_id, count)) in response {
                    chatrooms.push(Chatroom {
                        term,
                        online: true,
                        chatroom_id,
                        num_users: count,
                        url: format!("ws://{}/ws", address),
                    });
                }
            }
            Err(error) => {
                error!("Error occurred while querying chatroom instance {}", error);
            }
        }
    }

    Json(chatrooms)
}

async fn locate_instances(terms: &[&str]) -> HashMap<SocketAddrV4, Vec<String>> {
    let mut locations: HashMap<SocketAddrV4, Vec<String>> = HashMap::new();

    let client = reqwest::Client::new();

    for term in terms {
        let response = client
            // .post("http://localhost:8081/chatroom")
            .post("http://discovery.gerber.website:8081/chatroom")
            .json(&ChatroomRequest {
                term: term.to_string(),
            })
            .send()
            .await
            .expect("Failed to contact discovery service.")
            .json::<ChatroomResponse>()
            .await
            .expect("Invalid response from discovery service.");

        match response.instance {
            Some(instance) => {
                let terms = locations.get_mut(&instance.address);

                match terms {
                    Some(terms) => terms.push(term.to_string()),
                    None => {
                        locations.insert(instance.address, vec![term.to_string()]);
                    }
                };
            }
            None => {
                error!("A mapping could not be found for {}.", term);
            }
        }
    }

    locations
}

fn main() -> BoxResult<()> {
    initialize_logger()?;
    dotenv::dotenv().ok();

    let runtime = Runtime::new()?;

    let app = Router::new().route("/chatrooms", get(get_chatrooms));

    runtime.block_on(async {
        axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    Ok(())
}
