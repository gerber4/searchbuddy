#![feature(try_blocks)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::ClearType;
use crossterm::{execute, queue};
use futures::{Sink, SinkExt, Stream, StreamExt};
use log::error;
use shared::{initialize_logger, Chatroom, ClientToServerMessage, ServerToClientMessage};
use std::error::Error;
use std::io::{stdout, Write};
use std::pin::Pin;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::tungstenite::error::Error as WsError;
use tokio_tungstenite::tungstenite::Message as WsMessage;

type BoxError = Box<dyn Error + Send + Sync>;
type BoxResult<T> = Result<T, BoxError>;

#[derive(Debug)]
enum Event {
    Keyboard(KeyEvent),
    Disconnect,
    Joined,
    NewUser { user_id: i32 },
    UserDisconnected { user_id: i32 },
    NewMessage(String),
    ChatsFromTodayResponse { messages: Vec<String> },
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
        sink: Pin<Box<dyn Sink<WsMessage, Error = WsError> + Send + Sync>>,
        messages: Vec<String>,
        input: String,
    },
    Error {
        error: BoxError,
    },
    Break,
}

fn view(model: &Model) -> BoxResult<()> {
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
                        let chatrooms: BoxResult<Vec<Chatroom>> = try {
                            client
                                .get("http://searchbuddy.gerber.website:8080/chatrooms")
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
                                model.state = State::Error { error };
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
        State::SelectChatroom { chatrooms, index } => match event {
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

                    let result: BoxResult<()> = try {
                        let (socket, _response) =
                            tokio_tungstenite::connect_async(&chatroom.url).await?;

                        let (mut send, receive) = socket.split();

                        let message = serde_json::to_string(&ClientToServerMessage::Join {
                            channel_id: chatroom.chatroom_id,
                        })?;
                        send.send(WsMessage::Text(message)).await?;

                        let message =
                            serde_json::to_string(&ClientToServerMessage::ChatsFromTodayRequest)?;
                        send.send(WsMessage::Text(message)).await?;

                        let channel = model.event_sender.clone();

                        tokio::spawn(handle_messages(channel, Box::pin(receive)));

                        model.state = State::InChatroom {
                            sink: Box::pin(send),
                            messages: Vec::new(),
                            input: "".to_string(),
                        };
                    };

                    if let Err(error) = result {
                        error!(
                            "An error occurred while connecting to the provided chatroom instance."
                        );
                        model.state = State::Error { error };
                    }
                }
                _ => {}
            },
            _ => {}
        },
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
                        let message = serde_json::to_string(&ClientToServerMessage::NewMessage(
                            input.clone(),
                        ))
                        .unwrap();
                        let result = sink.send(WsMessage::Text(message)).await;
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
                messages.push(chat);
            }
            Event::ChatsFromTodayResponse {
                messages: mut new_messages,
            } => {
                messages.append(&mut new_messages);
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

async fn handle_messages(
    channel: UnboundedSender<Event>,
    mut receive: Pin<Box<impl Stream<Item = Result<WsMessage, WsError>>>>,
) {
    let result: BoxResult<()> = try {
        while let Some(message) = receive.next().await {
            match message {
                Ok(message) => {
                    if let WsMessage::Text(message) = message {
                        let message = serde_json::from_str::<ServerToClientMessage>(&message)?;

                        match message {
                            ServerToClientMessage::Joined { .. } => {
                                channel.send(Event::Joined)?;
                            }
                            ServerToClientMessage::NewUser { user_id } => {
                                channel.send(Event::NewUser { user_id })?;
                            }
                            ServerToClientMessage::UserDisconnected { user_id } => {
                                channel.send(Event::UserDisconnected { user_id })?;
                            }
                            ServerToClientMessage::NewMessage(chat) => {
                                channel.send(Event::NewMessage(chat))?;
                            }
                            ServerToClientMessage::ChatsFromTodayResponse { messages } => {
                                channel.send(Event::ChatsFromTodayResponse { messages })?;
                            }
                        }
                    }
                }
                Err(error) => {
                    error!(
                        "An error occurred while processing messages from chatroom instance - {:?}",
                        error
                    );
                    channel.send(Event::Disconnect)?;
                }
            }
        }
    };

    if let Err(error) = result {
        error!(
            "An error occurred while processing messages from chatroom instance - {:?}",
            error
        );
    }
}

fn main() -> BoxResult<()> {
    initialize_logger()?;

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
