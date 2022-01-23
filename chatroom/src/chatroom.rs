use async_recursion::async_recursion;
use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use futures::SinkExt;
use log::error;
use shared::{Chat, ServerToClientMessage};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub enum ClientToServerEvent {
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

pub struct Chatroom {
    chatroom_id: u32,
    channel: UnboundedSender<ClientToServerEvent>,
    count: AtomicU32,
}

impl Chatroom {
    pub fn new(chatroom_id: u32) -> Arc<Chatroom> {
        let (sender, receiver) = unbounded_channel::<ClientToServerEvent>();

        let chatroom = Chatroom {
            chatroom_id,
            channel: sender,
            count: AtomicU32::new(0),
        };

        let chatroom = Arc::new(chatroom);
        tokio::spawn(Self::handle_events(chatroom.clone(), receiver));
        chatroom
    }

    pub fn get_user_count(&self) -> u32 {
        self.count.load(Ordering::SeqCst)
    }

    pub fn send_event(&self, event: ClientToServerEvent) {
        let result = self.channel.send(event);

        if let Err(error) = result {
            error!(
                "An event was send to a channel that is no longer accepting events - {}",
                error
            )
        }
    }

    async fn handle_events(
        chatroom: Arc<Chatroom>,
        mut receiver: UnboundedReceiver<ClientToServerEvent>,
    ) {
        let mut connections: HashMap<u32, SplitSink<WebSocket, Message>> = HashMap::new();
        let mut messages: Vec<Chat> = Vec::new();

        loop {
            while let Some(event) = receiver.recv().await {
                match event {
                    ClientToServerEvent::NewMessage(chat) => {
                        messages.push(chat.clone());
                        let message = ServerToClientMessage::NewMessage(chat);
                        Self::broadcast_message(&mut connections, message).await;
                    }

                    ClientToServerEvent::RangeRequest { limit, offset } => {
                        let slice = &messages[offset..offset + limit];
                        let message = ServerToClientMessage::RangeResponse {
                            messages: slice.to_vec(),
                        };
                        Self::broadcast_message(&mut connections, message).await;
                    }

                    ClientToServerEvent::Connect {
                        user_id,
                        connection,
                    } => {
                        chatroom.count.fetch_add(1, Ordering::SeqCst);

                        let message = ServerToClientMessage::Joined {
                            chatroom_id: chatroom.chatroom_id,
                        };
                        Self::send_message(&mut connections, user_id, message);

                        let message = ServerToClientMessage::NewUser { user_id };
                        Self::broadcast_message(&mut connections, message).await;

                        connections.insert(user_id, connection);
                    }

                    ClientToServerEvent::Disconnect { user_id } => {
                        chatroom.count.fetch_sub(1, Ordering::SeqCst);
                        connections.remove(&user_id);

                        let message = ServerToClientMessage::UserDisconnected { user_id };
                        Self::broadcast_message(&mut connections, message).await;
                    }
                }
            }
        }
    }

    #[async_recursion]
    async fn send_message(
        connections: &mut HashMap<u32, SplitSink<WebSocket, Message>>,
        user_id: u32,
        message: ServerToClientMessage,
    ) {
        let message = serde_json::to_string(&message).unwrap();
        let message = Message::Text(message);

        let connection = connections.get_mut(&user_id).unwrap();
        let result = connection.send(message.clone()).await;

        if result.is_err() {
            connections.remove(&user_id);

            Self::broadcast_message(
                connections,
                ServerToClientMessage::UserDisconnected { user_id },
            )
            .await;
        }
    }

    #[async_recursion]
    async fn broadcast_message(
        connections: &mut HashMap<u32, SplitSink<WebSocket, Message>>,
        message: ServerToClientMessage,
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
                connections,
                ServerToClientMessage::UserDisconnected { user_id },
            )
            .await;
        }
    }
}
