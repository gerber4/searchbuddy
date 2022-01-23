#![feature(try_blocks)]

// This is a basic implementation for a server that can handle an arbitrary number of chatrooms.
// All chatrooms are assigned a unique id by external services and the associated chatroom is
// allocated when the first client connects to the server. All messages are saved until midnight.
// At midnight a bulk job deletes all existing messages.

mod chatroom;
mod model;

use crate::chatroom::{Chatroom, ClientToServerEvent};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use log::error;
use shared::{get_channel_id, ClientToServerMessage, initialize_logger};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::RwLock;

struct State {
    chatrooms: HashMap<u32, Arc<Chatroom>>,
}

impl State {
    async fn get_channel(&mut self, chatroom_id: u32) -> Arc<Chatroom> {
        let chatroom = self.chatrooms.get_mut(&chatroom_id);

        if let Some(chatroom) = chatroom {
            chatroom.clone()
        } else {
            Chatroom::new(chatroom_id)
        }
    }
}

async fn chatrooms_handler(
    state: Arc<RwLock<State>>,
    Json(terms): Json<Vec<String>>,
) -> Json<HashMap<String, (u32, u32)>> {
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
    let user_id = rand::random::<u32>();

    spawn(async move {
        if let Some(Ok(message)) = stream.next().await {
            let message = parse_message(message);
            if let Some(ClientToServerMessage::Join { channel_id }) = message {
                let chatroom = {
                    let mut state = state.write().await;
                    state.get_channel(channel_id).await
                };

                chatroom.send_event(ClientToServerEvent::Connect {
                    user_id,
                    connection: sink,
                });

                while let Some(Ok(message)) = stream.next().await {
                    let message = parse_message(message);

                    if let Some(message) = message {
                        match message {
                            ClientToServerMessage::Join { .. } => {
                                // Joining is unsupported once in a chatroom.
                            }
                            ClientToServerMessage::NewMessage(chat) => {
                                chatroom.send_event(ClientToServerEvent::NewMessage(chat));
                            }
                            ClientToServerMessage::RangeRequest {
                                limit: start_timestamp,
                                offset: end_timestamp,
                            } => {
                                chatroom.send_event(ClientToServerEvent::RangeRequest {
                                    limit: start_timestamp,
                                    offset: end_timestamp,
                                });
                            }
                        }
                    }
                }
                chatroom.send_event(ClientToServerEvent::Disconnect { user_id });
            } else {
                error!("Expected a join message from new client.");
            }
        } else {
            error!("Expected a join message from new client.");
        }
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    initialize_logger()?;

    let runtime = Runtime::new()?;

    let chatrooms = Arc::new(RwLock::new(State {
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

    runtime.block_on(async {
        axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    Ok(())
}
