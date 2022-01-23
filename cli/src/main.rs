#![feature(try_blocks)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::{execute, queue};
use futures::{Sink, SinkExt, StreamExt};
use log::{error, LevelFilter};
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use shared::{Chat, Chatroom, ClientToServerMessage, ServerToClientMessage};
use std::error::Error;
use std::io::{stdout, Write};
use std::pin::Pin;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug)]
enum Event {
    Keyboard(KeyEvent),
    Disconnect,
    Joined,
    NewUser { user_id: u32 },
    UserDisconnected { user_id: u32 },
    NewMessage(Chat),
}

struct Model {
    event_sender: UnboundedSender<Event>,
    state: State,
}

enum State {
    Initial {
        search: String,
    },
    SelectChatroom {
        chatrooms: Vec<Chatroom>,
        index: isize,
    },
    InChatroom {
        sink: Pin<
            Box<
                dyn Sink<Message, Error = tokio_tungstenite::tungstenite::error::Error>
                    + Send
                    + Sync,
            >,
        >,
        messages: Vec<String>,
        input: String,
    },
    Error {
        error: Box<dyn Error + Send + Sync>,
    },
    Break,
}

fn view(model: &Model) -> Result<(), Box<dyn Error>> {
    let (_width, height) = crossterm::terminal::size()?;

    let mut stdout = stdout();
    queue!(stdout, crossterm::terminal::Clear(ClearType::All))?;

    match &model.state {
        State::Initial { search } => {
            queue!(stdout, crossterm::cursor::MoveTo(0, 0))?;
            queue!(
                stdout,
                crossterm::style::Print(format!("Enter query: {}", search))
            )?;
        }
        State::SelectChatroom { chatrooms, index } => {
            let index = index.rem_euclid(chatrooms.len() as isize) as usize;

            for i in 0..chatrooms.len() {
                let chatroom = &chatrooms[i];

                queue!(stdout, crossterm::cursor::MoveTo(0, i as u16))?;
                if i == index {
                    queue!(stdout, crossterm::style::Print("> "))?;
                } else {
                    queue!(stdout, crossterm::style::Print("  "))?;
                }

                queue!(
                    stdout,
                    crossterm::style::Print(format!(
                        "Chatroom {} - {} users",
                        chatroom.term, chatroom.num_users
                    ))
                )?;
            }
        }
        State::InChatroom {
            messages, input, ..
        } => {
            queue!(stdout, crossterm::cursor::MoveTo(0, height))?;
            queue!(stdout, crossterm::style::Print(format!("> {}", input)))?;

            for i in 0..=(height - 1) {
                if i as usize > messages.len() {
                    break;
                }

                let message = messages.get(messages.len() - i as usize);
                match message {
                    Some(message) => {
                        queue!(stdout, crossterm::cursor::MoveTo(0, (height - 1) - i))?;
                        queue!(stdout, crossterm::style::Print(format!("{}", message)))?;
                    }

                    None => {}
                }
            }
        }
        State::Error { error } => {
            queue!(
                stdout,
                crossterm::style::Print(format!("An error occurred: {:#?}", error))
            )?;
        }
        State::Break => {
            // ignored
        }
    }

    stdout.flush()?;

    Ok(())
}

async fn update(model: &mut Model, event: Event) {
    match &mut model.state {
        State::Initial { search } => match event {
            Event::Keyboard(key_event) => match key_event.code {
                KeyCode::Esc => model.state = State::Break,
                KeyCode::Backspace => {
                    search.pop();
                }
                KeyCode::Enter => {
                    if !search.is_empty() {
                        let client = reqwest::Client::new();
                        let chatrooms: Result<Vec<Chatroom>, reqwest::Error> = try {
                            client
                                .get("http://localhost:8080/chatrooms")
                                .query(&[("search", &search)])
                                .send()
                                .await?
                                .json::<Vec<Chatroom>>()
                                .await?
                        };

                        match chatrooms {
                            Ok(chatrooms) => {
                                model.state = State::SelectChatroom {
                                    chatrooms,
                                    index: 0,
                                };
                            }
                            Err(error) => {
                                error!(
                                    "An error occurred while accessing the server = {:?}",
                                    error
                                );
                                model.state = State::Error {
                                    error: Box::new(error),
                                };
                            }
                        }
                    }
                }
                KeyCode::Char(char) => {
                    if char == 'u' && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        search.clear();
                    } else {
                        search.push(char);
                    }
                }
                _ => {}
            },
            _ => {}
        },
        State::SelectChatroom { chatrooms, index } => {
            match event {
                Event::Keyboard(key_event) => match key_event.code {
                    KeyCode::Esc => {
                        model.state = State::Initial {
                            search: "".to_string(),
                        }
                    }
                    KeyCode::Up => {
                        *index = *index - 1;
                    }
                    KeyCode::Down => {
                        *index = *index + 1;
                    }
                    KeyCode::Enter => {
                        let index = index.rem_euclid(chatrooms.len() as isize) as usize;
                        let chatroom = &chatrooms[index];

                        let result: Result<(), Box<dyn Error + Sync + Send>> = try {
                            let (socket, _response) =
                                tokio_tungstenite::connect_async(&chatroom.url).await?;

                            let (mut send, mut receive) = socket.split();

                            let message = serde_json::to_string(&ClientToServerMessage::Join {
                                channel_id: chatroom.chatroom_id,
                            })?;

                            send.send(Message::Text(message)).await?;

                            let channel = model.event_sender.clone();

                            tokio::spawn(async move {
                                let result: Result<(), Box<dyn Error + Sync + Send>> = try {
                                    while let Some(message) = receive.next().await {
                                        match message {
                                            Ok(message) => {
                                                if let Message::Text(message) = message {
                                                    let message = serde_json::from_str::<
                                                        ServerToClientMessage,
                                                    >(
                                                        &message
                                                    )?;

                                                    match message {
                                                            ServerToClientMessage::Joined { .. } => {
                                                                channel.send(Event::Joined)?;
                                                            }
                                                            ServerToClientMessage::NewUser { user_id } => {
                                                                channel.send(Event::NewUser { user_id })?;
                                                            }
                                                            ServerToClientMessage::UserDisconnected { user_id } => {
                                                                channel
                                                                    .send(Event::UserDisconnected { user_id })?;
                                                            }
                                                            ServerToClientMessage::NewMessage(chat) => {
                                                                channel.send(Event::NewMessage(chat))?;
                                                            }
                                                            ServerToClientMessage::RangeResponse { .. } => {}
                                                        }
                                                }
                                            }
                                            Err(error) => {
                                                error!("An error occurred while processing messages from chatroom instance - {:?}", error);
                                                channel.send(Event::Disconnect)?;
                                            }
                                        }
                                    }
                                };

                                if let Err(error) = result {
                                    error!("An error occurred while processing messages from chatroom instance - {:?}", error);
                                }
                            });

                            model.state = State::InChatroom {
                                sink: Box::pin(send),
                                messages: Vec::new(),
                                input: "".to_string(),
                            };
                        };

                        if let Err(error) = result {
                            error!("An error occurred while connecting to the provided chatroom instance.");
                            model.state = State::Error { error };
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        State::InChatroom {
            sink,
            messages,
            input,
        } => match event {
            Event::Keyboard(key_event) => match key_event.code {
                KeyCode::Esc => model.state = State::Break,
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    if !input.is_empty() {
                        let message =
                            serde_json::to_string(&ClientToServerMessage::NewMessage(Chat {
                                text: input.clone(),
                                idempotency: "ligma".to_string(),
                            }))
                            .unwrap();
                        let result = sink.send(Message::Text(message)).await;
                        match result {
                            Ok(()) => {
                                input.clear();
                            }
                            Err(error) => {
                                error!("An error occurred while sending a message - {:?}", error);
                                model.state = State::Error {
                                    error: Box::new(error),
                                };
                            }
                        }
                    }
                }
                KeyCode::Char(char) => {
                    if char == 'u' && key_event.modifiers.contains(KeyModifiers::CONTROL) {
                        input.clear();
                    } else {
                        input.push(char);
                    }
                }
                _ => {}
            },
            Event::Disconnect => {
                model.state = State::Initial {
                    search: "".to_string(),
                };
            }
            Event::Joined => {
                messages.push("Joined chatroom!".to_string());
            }
            Event::NewUser { user_id } => {
                messages.push(format!("User with id {} joined chatroom!", user_id));
            }
            Event::UserDisconnected { user_id } => {
                messages.push(format!("User with id {} left chatroom!", user_id));
            }
            Event::NewMessage(chat) => {
                messages.push(chat.text);
            }
        },
        State::Error { .. } => match event {
            Event::Keyboard(key_event) => match key_event.code {
                KeyCode::Esc => model.state = State::Break,
                _ => {}
            },
            _ => {}
        },
        State::Break => {}
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log")?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    let runtime = Runtime::new()?;

    let mut stdout = stdout();
    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    execute!(stdout, crossterm::cursor::Hide)?;
    crossterm::terminal::enable_raw_mode()?;

    let (send, mut receive) = unbounded_channel::<Event>();
    let send_clone = send.clone();

    let handle = runtime.spawn(async move {
        let mut model = Model {
            event_sender: send_clone,
            state: State::Initial {
                search: "".to_string(),
            },
        };

        {
            let result = view(&model);
            if result.is_err() {
                error!("Failed to draw to terminal on startup.");
                return;
            }
        }

        while let Some(event) = receive.recv().await {
            update(&mut model, event).await;

            if let State::Break = model.state {
                break;
            }

            let result = view(&model);
            if result.is_err() {
                error!("Failed to draw to terminal after startup.");
                return;
            }
        }
    });

    thread::spawn(move || {
        while let Ok(crossterm::event::Event::Key(event)) = crossterm::event::read() {
            let result = send.send(Event::Keyboard(event));

            if result.is_err() {
                error!("Failed to send keyboard event.");
            }
        }
    });

    let result = runtime.block_on(handle);

    if result.is_err() {
        error!("Main future failed to run to completion.");
    }

    crossterm::terminal::disable_raw_mode()?;
    execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}
