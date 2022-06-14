use bytes::Bytes;
use futures::future::{self, select, Either, FutureExt};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use websocket_lite::{Message, Opcode, Result};

// use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

use crate::config::Config;
use crate::irender;
use crate::types::{self, Autocomplete, InternalMessage, InternalMessageType};
use crate::ui::ui;
use crate::utils;
use types::{App, EmoteData, InputMode, ParsedMessage};

use tui::{backend::Backend, Terminal};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEvent};

use std::thread;
use std::{
    io,
    time::{Duration, Instant},
};

pub async fn run_ws2(
    tx: tokio::sync::watch::Sender<String>,
    mut mrx: tokio::sync::watch::Receiver<String>,
    itx: tokio::sync::watch::Sender<InternalMessage>,
    mut irx: tokio::sync::watch::Receiver<InternalMessage>,
    config: Config,
) -> Result<()> {
    let url = "wss://chat.destiny.gg/ws".to_owned();
    let mut builder = websocket_lite::ClientBuilder::new(&url)?;

    if let Some(token) = config.token.to_owned() {
        builder.add_header(
            "Cookie".to_string(),
            format!("authtoken={}", token).to_string(),
        )
    }

    let client = builder.async_connect().await?;
    let (sink, stream) = client.split();

    let send_loop = async {
        let mut sink = sink;
        let mut message: String = String::new();
        let mut message_changed = false;

        let mut ping_data: Bytes = Bytes::new();
        let mut do_ping = false;

        while message != "/quit" {
            // let res = select(irx.changed().boxed(), mrx.changed().boxed()).await;

            tokio::select! {
                val = irx.changed() => {
                    if val.is_ok() {
                        let ping_msg: &InternalMessage = &*irx.borrow();
                        ping_data = ping_msg.data.to_owned();
                        do_ping = true;
                    }
                }
                val = mrx.changed() => {
                    if val.is_ok() {
                        let msg = &*mrx.borrow();
                        message = msg.to_string();
                        message_changed = true;
                    }
                }
            };

            if message_changed {
                let message_data = Message::new(
                    Opcode::Text,
                    format!("MSG {{ \"data\": \"{}\" }}", &message),
                )?;
                sink.send(message_data).await?;
                message_changed = false;
            } else if do_ping {
                sink.send(Message::pong(ping_data.to_owned())).await?;
                do_ping = false;
            }
        }

        Ok(())
    };

    let recv_loop = async {
        let mut stream_mut = stream;

        loop {
            let (msg, stream) = stream_mut.into_future().await;

            let msg = if let Some(msg) = msg {
                msg?
            } else {
                stream_mut = stream;
                break;
            };

            if let Opcode::Text = msg.opcode() {
                if let Some(text) = msg.as_text() {
                    let msg_text: String = text.to_string();
                    if msg_text.contains("/quit") {
                        break;
                    }
                    let res = tx.send(msg_text);
                }
            }

            if let Opcode::Ping = msg.opcode() {
                itx.send(InternalMessage {
                    message_type: types::InternalMessageType::PING,
                    message: "WS_PING".to_string(),
                    data: msg.into_data(),
                })?;
            }

            stream_mut = stream;
        }
        println!("Connection closed");

        Ok(()) as Result<()>
    };

    let result = future::select(send_loop.boxed(), recv_loop.boxed())
        .await
        .into_inner()
        .0;

    Ok(())
}

pub async fn run_ws(
    tx: tokio::sync::watch::Sender<String>,
    mut mrx: tokio::sync::watch::Receiver<String>,
) -> Result<()> {
    let url = "wss://chat.destiny.gg/ws".to_owned();
    let mut builder = websocket_lite::ClientBuilder::new(&url)?;

    builder.add_header(
        "Cookie".to_string(),
        "authtoken=7uooLJ8yxtTmCBnjaloirWpXXpbRNgOWJ0ZJLsyvjX8xoTavppOf7OdL1hbCtfVm".to_string(),
    );

    let mut ws_stream = builder.async_connect().await?;

    loop {
        let mut message: String = String::new();
        let mut changed: bool = false;

        if mrx.has_changed().unwrap() {
            let channel_message = &*mrx.borrow_and_update();

            if channel_message.to_string() == "quit".to_string() {
                break;
            }

            message = channel_message.to_string();
            // message_to_send = msg.to_string();
            changed = true;
        }

        if changed == true {
            ws_stream
                .send(Message::text(format!(
                    "MSG {{ \"data\": \"{}\" }}",
                    &message
                )))
                .await?;
            // ws_stream.send(message);
            // println!("Hello test: {}", message);
            ws_stream.next().await;
            changed = false;
        } else {
            let msg: Option<Result<Message>> = ws_stream.next().await;

            let msg = if let Some(msg) = msg {
                msg
            } else {
                break;
            };

            let msg = if let Ok(msg) = msg {
                msg
            } else {
                let _ = ws_stream.send(Message::close(None)).await;
                break;
            };

            match msg.opcode() {
                Opcode::Text => {
                    // println!("{}", msg.as_text().unwrap());

                    // ws_stream.send(msg).await?
                    let msg: String = msg.as_text().unwrap().to_string();
                    tx.send(msg)?;
                }
                Opcode::Binary => ws_stream.send(msg).await?,
                Opcode::Ping => ws_stream.send(Message::pong(msg.into_data())).await?,
                Opcode::Close => {
                    let _ = ws_stream.send(Message::close(None)).await;
                    break;
                }
                Opcode::Pong => {}
            }
        }
    }

    Ok(())
}

pub async fn run_emotes(mut erx: tokio::sync::watch::Receiver<EmoteData>) -> Result<()> {
    while erx.changed().await.is_ok() {
        let res = &*erx.borrow();

        let max: isize = res.term_size as isize - 9;

        let floor = |_: usize| -> usize {
            if res.message_pos as isize - max <= 0 {
                0
            } else {
                (res.message_pos as isize - max) as usize
            }
        };

        let mut final_emotes: Vec<(u16, u16, String)> = Vec::new();

        for (i, message) in res.messages[floor(0)..res.message_pos + 1]
            .iter()
            .enumerate()
        {
            let emote_names: Vec<String> = utils::get_emotenames();
            let mut emote_pos: Vec<(usize, &str)> = vec![];

            let parsed_output: JSON_Result<ParsedMessage>;
            parsed_output = utils::parse_message(&message.0.as_str()[4..]);
            let msg: ParsedMessage = parsed_output.unwrap();

            for name in emote_names {
                let mut pos: Vec<_> = msg
                    .data
                    .match_indices(&format!("{}", &name).to_string())
                    .collect();
                emote_pos.append(&mut pos);
            }

            if emote_pos.len() > 0 {
                for pos in emote_pos.to_owned() {
                    // utils::print_emote(i as u16, pos.0 as u16 + msg.nick.len() as u16 + 3, pos.1);
                    final_emotes.push((
                        i as u16,
                        pos.0 as u16 + msg.nick.len() as u16 + 3,
                        pos.1.to_string(),
                    ));
                }
            }
        }

        // irender::clear_all();
        if final_emotes.len() > 0 {
            for pos in final_emotes.to_owned() {
                irender::print_emote(pos.0, pos.1)?;
            }
        }
    }

    Ok(())
}

pub fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut rx: tokio::sync::watch::Receiver<String>,
    etx: tokio::sync::watch::Sender<EmoteData>,
    mtx: tokio::sync::watch::Sender<String>,
    tick_rate: Duration,
) -> Result<()> {
    let mut last_tick = Instant::now();

    app.emotes = utils::get_emotenames();

    loop {
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }

        let rx_res = match rx.has_changed() {
            Ok(rx_res) => {
                if rx_res {
                    let msg = &*rx.borrow_and_update();
                    if msg.starts_with("MSG ") {
                        let hi: String = msg.to_string();
                        app.message_list.items.push((hi.to_owned(), 1));
                        match app.input_mode {
                            InputMode::Normal => app.message_list.bottom(),
                            InputMode::Editing => app.message_list.bottom(),
                        }

                        let term_height: u16 = terminal.size().unwrap().height;
                        // etx.send(EmoteData {
                        //     term_size: term_height,
                        //     messages: app.message_list.messages.clone(),
                        //     message_pos: app.message_list.state.selected().unwrap(),
                        // })
                        // .expect("Error sending EmoteData to etx");

                        if app.config.emotes {
                            irender::clear_all();
                            irender::emote_meme(EmoteData {
                                term_size: term_height,
                                messages: app.message_list.items.clone(),
                                message_pos: app.message_list.state.selected().unwrap(),
                            })?;
                        }
                    } else if msg.starts_with("NAMES ") {
                        app.users = utils::get_users(msg.to_string());
                        for user in app.users.users.to_owned() {
                            app.user_list.items.push(user)
                        }

                        // for user in app.users.users {

                        // }
                    }
                }
            }
            Err(e) => {}
        };

        terminal.draw(|f| ui(f, &mut app))?;

        if crossterm::event::poll(tick_rate).unwrap() {
            // if let Event::Mouse(event) = event::read()? {
            //     match event.kind {
            //         event::MouseEventKind::Drag(event::MouseButton::Left) => {
            //             // println!("{}", event.row)
            //         }
            //         _ => (),
            //     }
            // }

            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Char('c') => irender::clear_all(),
                        KeyCode::Char('r') => {
                            print!("hello");
                            terminal.clear()?;
                            // println!(utils::emotes_remote)::emotes_remote();
                        }
                        KeyCode::Char('t') => {}
                        KeyCode::Char('p') => {
                            irender::print_one(
                                "/Users/lsmb/Dev/dggtui/src/emotes_resized/OOOO.png",
                            )?
                            // terminal.clear()?;
                        }
                        KeyCode::Char('g') => match app.tab_index {
                            0 => {
                                app.message_list.bottom();
                                etx.send(EmoteData {
                                    term_size: terminal.size().unwrap().height,
                                    messages: app.message_list.items.clone(),
                                    message_pos: app.message_list.state.selected().unwrap(),
                                })
                                .expect("Error sending EmoteData to etx");
                            }
                            1 => {
                                app.user_list.bottom();
                            }
                            _ => unreachable!(),
                        },
                        KeyCode::Char('G') => match app.tab_index {
                            0 => {
                                app.message_list.top();
                                etx.send(EmoteData {
                                    term_size: terminal.size().unwrap().height,
                                    messages: app.message_list.items.clone(),
                                    message_pos: app.message_list.state.selected().unwrap(),
                                })
                                .expect("Error sending EmoteData to etx");
                            }
                            1 => {
                                app.user_list.top();
                            }
                            _ => unreachable!(),
                        },

                        KeyCode::Down => match app.tab_index {
                            0 => {
                                app.message_list.next();
                                etx.send(EmoteData {
                                    term_size: terminal.size().unwrap().height,
                                    messages: app.message_list.items.clone(),
                                    message_pos: app.message_list.state.selected().unwrap(),
                                })
                                .expect("Error sending EmoteData to etx");
                            }
                            1 => app.user_list.next(),
                            _ => unreachable!(),
                        },
                        KeyCode::Up => match app.tab_index {
                            0 => app.message_list.previous(),
                            1 => app.user_list.previous(),
                            _ => unreachable!(),
                        },
                        KeyCode::Left => app.message_list.unselect(),
                        KeyCode::Tab => app.next_tab(),
                        KeyCode::BackTab => app.prev_tab(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let message: String = app.input.drain(..).collect();
                            // println!("Message: {}", message);
                            mtx.send(message.to_string());
                            // app.message_list.messages.push((message.to_owned(), 1));
                        }
                        KeyCode::Char(c) => {
                            if app.autocomplete.tabbing == true {
                                // app.input.push(' ');
                                app.autocomplete.unselect();
                                app.autocomplete.tabbing = false
                            }
                            app.input.push(c);
                            // app.autocomplete.last_word = app.input.split(' ').last();
                            let autocomplete: Autocomplete = utils::get_suggestions(
                                app.input.to_owned(),
                                app.autocomplete.to_owned(),
                                app.users.to_owned(),
                                app.emotes.to_owned(),
                            );

                            app.autocomplete = autocomplete;
                            app.autocomplete.unselect();
                        }
                        KeyCode::Backspace => {
                            if key.modifiers == KeyModifiers::ALT {
                                let mut split_input: Vec<&str> = app.input.split(' ').collect();
                                split_input.pop();
                                app.input = split_input.join(" ");
                            } else {
                                app.input.pop();
                            }

                            let autocomplete: Autocomplete = utils::get_suggestions(
                                app.input.to_owned(),
                                app.autocomplete.to_owned(),
                                app.users.to_owned(),
                                app.emotes.to_owned(),
                            );

                            app.autocomplete = autocomplete;
                            app.autocomplete.unselect();
                            app.autocomplete.tabbing = false;
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Tab => {
                            app.autocomplete.next();
                            if let Some(state) = app.autocomplete.selected {
                                let mut split_input: Vec<&str> = app.input.split(' ').collect();
                                if app.autocomplete.tabbing {
                                    split_input.pop();
                                }
                                if split_input.len() > 0
                                    && state < app.autocomplete.suggestions.len()
                                {
                                    split_input.pop();
                                    split_input.push(&app.autocomplete.suggestions[state]);
                                    app.input = split_input.join(" ");
                                    app.input.push(' ');
                                }
                            }
                            app.autocomplete.tabbing = true;

                            // what man
                        }
                        KeyCode::BackTab => {
                            app.autocomplete.previous();
                            if let Some(state) = app.autocomplete.selected {
                                let mut split_input: Vec<&str> = app.input.split(' ').collect();
                                if app.autocomplete.tabbing {
                                    split_input.pop();
                                }

                                if split_input.len() > 0 {
                                    split_input.pop();
                                    split_input.push(&app.autocomplete.suggestions[state]);
                                    app.input = split_input.join(" ");
                                    app.input.push(' ')
                                }
                            }
                            app.autocomplete.tabbing = true;
                            // what man
                        }

                        _ => {}
                    },
                }
            }
        }
    }
}
