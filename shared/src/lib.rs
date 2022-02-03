use byteorder::{LittleEndian, ReadBytesExt};
use log::LevelFilter;
use log4rs::append::rolling_file::policy::compound::roll::delete::DeleteRoller;
use log4rs::append::rolling_file::policy::compound::trigger::size::SizeTrigger;
use log4rs::append::rolling_file::policy::compound::CompoundPolicy;
use log4rs::append::rolling_file::RollingFileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use ring::digest::{Context, SHA256};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Cursor;

pub mod discovery;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ClientToServerMessage {
    Join { chatroom_id: i32 },
    NewMessage { content: String },
    ChatsFromTodayRequest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ServerToClientMessage {
    Joined { chatroom_id: i32 },
    NewUser { user_id: i32 },
    UserDisconnected { user_id: i32 },
    NewMessage { content: String },
    ChatsFromTodayResponse { messages: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chatroom {
    pub chatroom_id: i32,
    pub num_users: u32,
    pub online: bool,
    pub term: String,
    pub url: String,
}

pub fn get_channel_id(term: &str) -> i32 {
    let mut context = Context::new(&SHA256);
    context.update(term.as_bytes());
    let hash = context.finish();
    let bytes = hash.as_ref();
    let mut cursor = Cursor::new(bytes);
    return cursor.read_i32::<LittleEndian>().unwrap();
}

pub fn initialize_logger() -> Result<(), Box<dyn Error + Send + Sync>> {
    let roller = Box::new(DeleteRoller::new());
    let trigger = Box::new(SizeTrigger::new(1_000_000));
    let policy = Box::new(CompoundPolicy::new(trigger, roller));

    let logfile = RollingFileAppender::builder()
        .append(true)
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log", policy)?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ClientToServerMessage;

    #[test]
    fn serialize_and_deserialize() {
        let message = ClientToServerMessage::Join { chatroom_id: 6969 };
        let serialized = serde_json::to_string(&message).unwrap();
        assert_eq!(serialized, r#"{"type":"Join","chatroom_id":6969}"#);
        let _deserialize: ClientToServerMessage = serde_json::from_str(&serialized).unwrap();
    }
}
