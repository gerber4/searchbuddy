// This is a basic implementation for a server that can handle an arbitrary number of chatrooms.
// All chatrooms are assigned a unique id by external services and the associated chatroom is
// allocated when the first client connects to the server. All messages are saved until midnight.
// At midnight a bulk job deletes all existing messages.

mod model;

use async_recursion::async_recursion;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::{sink::SinkExt, stream::SplitSink, StreamExt};
use shared::{get_channel_id, Chat, ClientToServerMessage, ServerToClientMessage};
use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::spawn;
use tokio::sync::RwLock;

enum ClientToServerEvent {
    NewMessage(Chat),
    RangeRequest {
        limit: usize,
        offset: usize,
    },
    Connect {
        user_id: u32,
        connection: SplitSink<WebSocket, Message>,
    },
    Disconnect {
        user_id: u32,
    },
}

fn parse_message(message: Message) -> Option<ClientToServerMessage> {
    match message {
        Message::Text(text) => serde_json::from_str::<ClientToServerMessage>(&text).ok(),
        _ => None,
    }
}

struct State {
    chatrooms: HashMap<u32, Arc<Chatroom>>,
}

struct Chatroom {
    channel: UnboundedSender<ClientToServerEvent>,
    count: AtomicU32,
}

impl State {
    #[async_recursion]
    async fn send_message(
        user_id: u32,
        message: ServerToClientMessage,
        connections: &mut HashMap<u32, SplitSink<WebSocket, Message>>,
    ) {
        let message = serde_json::to_string(&message).unwrap();
        let message = Message::Text(message);

        let connection = connections.get_mut(&user_id).unwrap();
        let result = connection.send(message.clone()).await;

        if result.is_err() {
            connections.remove(&user_id);

            Self::broadcast_message(
                ServerToClientMessage::UserDisconnected { user_id },
                connections,
            )
            .await;
        }
    }

    #[async_recursion]
    async fn broadcast_message(
        message: ServerToClientMessage,
        connections: &mut HashMap<u32, SplitSink<WebSocket, Message>>,
    ) {
        let message = serde_json::to_string(&message).unwrap();
        let message = Message::Text(message);

        let mut disconnected = Vec::new();

        for (id, connection) in connections.into_iter() {
            let result = connection.send(message.clone()).await;

            if result.is_err() {
                disconnected.push(*id);
            }
        }

        for user_id in disconnected {
            connections.remove(&user_id);

            Self::broadcast_message(
                ServerToClientMessage::UserDisconnected { user_id },
                connections,
            )
            .await;
        }
    }

    async fn get_channel(&mut self, chatroom_id: u32) -> Arc<Chatroom> {
        let chatroom = self.chatrooms.get_mut(&chatroom_id);

        if let Some(chatroom) = chatroom {
            chatroom.clone()
        } else {
            let (sender, mut receiver) = unbounded::<ClientToServerEvent>();

            let chatroom = Arc::new(Chatroom {
                channel: sender.clone(),
                count: AtomicU32::new(0),
            });

            self.chatrooms.insert(chatroom_id, chatroom.clone());

            let clone = chatroom.clone();

            // Spawn a task to process incoming events from clients.
            spawn(async move {
                let channel_id = chatroom_id;
                let mut connections: HashMap<u32, SplitSink<WebSocket, Message>> = HashMap::new();
                let mut messages: Vec<Chat> = Vec::new();

                loop {
                    let result = receiver.next().await;
                    match result {
                        Some(event) => match event {
                            ClientToServerEvent::NewMessage(chat) => {
                                messages.push(chat.clone());
                                let message = ServerToClientMessage::NewMessage(chat);
                                Self::broadcast_message(message, &mut connections).await;
                            }
                            ClientToServerEvent::RangeRequest { limit, offset } => {
                                let slice = &messages[offset..offset + limit];
                                let message = ServerToClientMessage::RangeResponse {
                                    messages: slice.to_vec(),
                                };
                                Self::broadcast_message(message, &mut connections).await;
                            }
                            ClientToServerEvent::Connect {
                                user_id,
                                connection,
                            } => {
                                chatroom.count.fetch_add(1, Ordering::SeqCst);

                                let message = ServerToClientMessage::Joined { channel_id };
                                Self::send_message(user_id, message, &mut connections);

                                let message = ServerToClientMessage::NewUser { user_id };
                                Self::broadcast_message(message, &mut connections).await;

                                connections.insert(user_id, connection);
                            }
                            ClientToServerEvent::Disconnect { user_id } => {
                                chatroom.count.fetch_sub(1, Ordering::SeqCst);
                                connections.remove(&user_id);

                                let message = ServerToClientMessage::UserDisconnected { user_id };
                                Self::broadcast_message(message, &mut connections).await;
                            }
                        },
                        None => {
                            // Once all handles to the channel are dropped this chatroom becomes
                            // unreachable. Exit the loop to release memory.
                            return;
                        }
                    }
                }
            });

            clone
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
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

async fn chatrooms_handler(
    state: Arc<RwLock<State>>,
    Json(terms): Json<Vec<String>>,
) -> Json<HashMap<String, (u32, u32)>> {
    let mut counts = HashMap::new();

    for term in terms {
        let channel_id = get_channel_id(&term);

        let mut state = state.write().await;
        let channel = state.get_channel(channel_id).await;

        counts.insert(term, (channel_id, channel.count.load(Ordering::SeqCst)));
    }

    Json(counts)
}

async fn ws_handler(state: Arc<RwLock<State>>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

async fn handle_socket(state: Arc<RwLock<State>>, socket: WebSocket) {
    let (sink, mut stream) = socket.split();

    // Generate unique id for this connection.
    let user_id = rand::random::<u32>();

    spawn(async move {
        let item = stream.next().await;
        if let Some(item) = item {
            let item = item?;
            let message = parse_message(item);
            if let Some(message) = message {
                if let ClientToServerMessage::Join { channel_id } = message {
                    let chatroom = {
                        let mut state = state.write().await;
                        state.get_channel(channel_id).await
                    };

                    let mut channel = &chatroom.channel;

                    channel
                        .send(ClientToServerEvent::Connect {
                            user_id,
                            connection: sink,
                        })
                        .await
                        .unwrap();

                    loop {
                        let item = stream.next().await;
                        if let Some(item) = item {
                            let message = parse_message(item?);
                            if let Some(message) = message {
                                match message {
                                    ClientToServerMessage::Join { .. } => {
                                        // Joining is unsupported once in a chatroom.
                                    }
                                    ClientToServerMessage::NewMessage(chat) => {
                                        channel
                                            .send(ClientToServerEvent::NewMessage(chat))
                                            .await
                                            .unwrap();
                                    }
                                    ClientToServerMessage::RangeRequest {
                                        limit: start_timestamp,
                                        offset: end_timestamp,
                                    } => {
                                        channel
                                            .send(ClientToServerEvent::RangeRequest {
                                                limit: start_timestamp,
                                                offset: end_timestamp,
                                            })
                                            .await
                                            .unwrap();
                                    }
                                }
                            }
                        } else {
                            let message = ClientToServerEvent::Disconnect { user_id };
                            channel.send(message).await.unwrap();
                            break;
                        }
                    }
                } else {
                    // Unsupported message. Waiting for join message.
                }
            }
        }

        let result: Result<(), Box<dyn Error + Send + Sync>> = Ok(());
        result
    });
}
