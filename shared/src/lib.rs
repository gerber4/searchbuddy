use byteorder::{LittleEndian, ReadBytesExt};
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientToServerMessage {
    Join { channel_id: u32 },
    NewMessage(Chat),
    RangeRequest { limit: usize, offset: usize },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerToClientMessage {
    Joined { channel_id: u32 },
    NewUser { user_id: u32 },
    UserDisconnected { user_id: u32 },
    NewMessage(Chat),
    RangeResponse { messages: Vec<Chat> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chat {
    pub idempotency: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chatroom {
    pub term: String,
    pub chatroom_id: u32,
    pub num_users: u32,
    pub url: String,
}

pub fn get_channel_id(term: &str) -> u32 {
    let mut context = Context::new(&SHA256);
    context.update(term.as_bytes());
    let hash = context.finish();
    let bytes = hash.as_ref();
    let mut cursor = Cursor::new(bytes);
    return cursor.read_u32::<LittleEndian>().unwrap();
}
