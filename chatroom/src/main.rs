#![feature(try_blocks)]

// This is a basic implementation for a server that can handle an arbitrary number of chatrooms.
// All chatrooms are assigned a unique id by external services and the associated chatroom is
// allocated when the first client connects to the server.

mod chatroom;
mod model;

use crate::chatroom::{Chatroom, ClientToServerEvent};
use crate::model::Model;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use log::{error, info};
use shared::discovery::{PingRequest, PingResponse, PingResult, RegisterRequest, RegisterResponse};
use shared::{get_channel_id, initialize_logger, ClientToServerMessage};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::SocketAddrV4;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::RwLock;

type BoxError = Box<dyn Error + Send + Sync>;
type BoxResult<T> = Result<T, BoxError>;

struct State {
    model: Arc<Model>,
    chatrooms: HashMap<i32, Arc<Chatroom>>,
}

impl State {
    async fn get_channel(&mut self, chatroom_id: i32) -> Arc<Chatroom> {
        let chatroom = self.chatrooms.get_mut(&chatroom_id);

        if let Some(chatroom) = chatroom {
            chatroom.clone()
        } else {
            info!("New channel requested.");

            let chatroom = Chatroom::new(self.model.clone(), chatroom_id);
            self.chatrooms.insert(chatroom_id, chatroom.clone());
            chatroom
        }
    }
}

async fn chatrooms_handler(
    state: Arc<RwLock<State>>,
    Json(terms): Json<Vec<String>>,
) -> Json<HashMap<String, (i32, u32)>> {
    let mut counts = HashMap::new();

    for term in terms {
        let channel_id = get_channel_id(&term);

        let mut state = state.write().await;
        let channel = state.get_channel(channel_id).await;

        counts.insert(term, (channel_id, channel.get_user_count()));
    }

    Json(counts)
}

async fn ws_handler(state: Arc<RwLock<State>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket_messages(state, socket))
}

fn parse_message(message: Message) -> Option<ClientToServerMessage> {
    match message {
        Message::Text(text) => serde_json::from_str::<ClientToServerMessage>(&text).ok(),
        _ => None,
    }
}

async fn handle_socket_messages(state: Arc<RwLock<State>>, socket: WebSocket) {
    let (sink, mut stream) = socket.split();

    // Generate unique id for this connection.
    let user_id = rand::random::<i32>();

    info!("New websocket connection with id {}.", user_id);

    spawn(async move {
        if let Some(Ok(message)) = stream.next().await {
            let message = parse_message(message);
            if let Some(ClientToServerMessage::Join {
                chatroom_id: channel_id,
            }) = message
            {
                let chatroom = {
                    let mut state = state.write().await;
                    state.get_channel(channel_id).await
                };

                chatroom.send_event(
                    user_id,
                    ClientToServerEvent::Connect {
                        user_id,
                        connection: sink,
                    },
                );

                info!("User {} joined a server.", user_id);

                while let Some(Ok(message)) = stream.next().await {
                    let message = parse_message(message);

                    if let Some(message) = message {
                        info!("Message received from user {} - {:?}", user_id, message);

                        match message {
                            ClientToServerMessage::Join { .. } => {
                                // Joining is unsupported once in a chatroom.
                            }
                            ClientToServerMessage::NewMessage { content } => {
                                chatroom
                                    .send_event(user_id, ClientToServerEvent::NewMessage(content));
                            }
                            ClientToServerMessage::ChatsFromTodayRequest => {
                                chatroom.send_event(
                                    user_id,
                                    ClientToServerEvent::ChatsFromTodayRequest,
                                );
                            }
                        }
                    }
                }
                chatroom.send_event(user_id, ClientToServerEvent::Disconnect { user_id });
            } else {
                error!("Expected a join message from new client.");
            }
        } else {
            error!("Expected a join message from new client.");
        }
    });
}

fn main() -> BoxResult<()> {
    initialize_logger()?;
    dotenv::dotenv().ok();

    let runtime = Runtime::new()?;
    runtime.block_on(async_main())?;
    Ok(())
}

async fn async_main() -> BoxResult<()> {
    let listen_address = env::var("LISTEN_ADDRESS").expect("LISTEN_ADDRESS not defined.");
    let listen_address = SocketAddrV4::from_str(&listen_address)
        .expect("LISTEN_ADDRESS is not valid socket address");

    let discovery_address = env::var("DISCOVERY_ADDRESS").expect("DISCOVERY_ADDRESS not defined.");

    let chatrooms = Arc::new(RwLock::new(State {
        model: Arc::new(Model::new().await?),
        chatrooms: HashMap::new(),
    }));

    let ws_state = chatrooms.clone();
    let chatrooms_state = chatrooms.clone();

    let app = Router::new()
        .route("/ws", get(move |ws| ws_handler(ws_state, ws)))
        .route(
            "/chatrooms",
            post(move |terms| chatrooms_handler(chatrooms_state, terms)),
        );

    let client = reqwest::Client::new();
    let registration = client
        .post(discovery_address.clone() + "/register")
        .json(&RegisterRequest { listen_address })
        .send()
        .await
        .expect("Failed to contact discovery service.")
        .json::<RegisterResponse>()
        .await
        .expect("Invalid response from discovery service.");

    let instance_id = registration.instance_id;

    tokio::spawn(async move {
        // Continuously ping the discovery to let it know this instance is active.
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;

            let response: BoxResult<PingResponse> = try {
                client
                    .post(discovery_address.clone() + "/ping")
                    .json(&PingRequest {
                        listen_address,
                        instance_id,
                    })
                    .send()
                    .await?
                    .json::<PingResponse>()
                    .await?
            };

            match response {
                Ok(response) => {
                    if let PingResult::NoLongerActive = response.ping_result {
                        error!("Registration expired. Exiting.");
                        std::process::exit(-1);
                    }
                }
                Err(error) => {
                    error!(
                        "The discovery service could not be reached because of an error - {:?}",
                        error
                    );
                }
            }
        }
    });

    axum::Server::bind(&listen_address.into())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
