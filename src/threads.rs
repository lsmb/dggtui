use futures::sink::SinkExt;
use futures::stream::StreamExt;
use websocket_lite::{Message, Opcode, Result};

// use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

use crate::types;
use crate::ui::ui;
use crate::utils;
use types::{App, EmoteData, InputMode, ParsedMessage};

use tui::{backend::Backend, Terminal};

use crossterm::event::{self, Event, KeyCode, MouseEvent};

use std::{
    io,
    time::{Duration, Instant},
};

pub async fn run_ws(tx: tokio::sync::watch::Sender<String>) -> Result<()> {
    let url = "wss://chat.destiny.gg/ws".to_owned();
    let mut builder = websocket_lite::ClientBuilder::new(&url)?;

    builder.add_header(
        "Cookie".to_string(),
        "authtoken=7uooLJ8yxtTmCBnjaloirWpXXpbRNgOWJ0ZJLsyvjX8xoTavppOf7OdL1hbCtfVm".to_string(),
    );

    let mut ws_stream = builder.async_connect().await?;

    loop {
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

        print!("\x1b_Ga=d;\x1b\\");

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
                    utils::print_emote(i as u16, pos.0 as u16 + msg.nick.len() as u16 + 3, pos.1)
                }
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
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }

        if rx.has_changed().unwrap() {
            let msg = &*rx.borrow_and_update();
            if msg.starts_with("MSG ") {
                let hi: String = msg.to_string();
                app.message_list.messages.push((hi.to_owned(), 1));
                match app.input_mode {
                    InputMode::Normal => app.message_list.bottom(),
                    InputMode::Editing => app.message_list.bottom(),
                }

                let term_height: u16 = terminal.size().unwrap().height;
                etx.send(EmoteData {
                    term_size: term_height,
                    messages: app.message_list.messages.clone(),
                    message_pos: app.message_list.state.selected().unwrap(),
                })
                .expect("Error sending EmoteData to etx");

                // print!("\x1b_Gi=31,a=d;\x1b\\")
            }
        }

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
                        KeyCode::Char('c') => print!("\x1b_Gi=31,a=d;\x1b\\"),
                        KeyCode::Char('r') => {
                            print!("hello");
                            terminal.clear()?;
                        }
                        KeyCode::Char('g') => {
                            app.message_list.bottom();
                            etx.send(EmoteData {
                                term_size: terminal.size().unwrap().height,
                                messages: app.message_list.messages.clone(),
                                message_pos: app.message_list.state.selected().unwrap(),
                            })
                            .expect("Error sending EmoteData to etx");
                        }
                        KeyCode::Down => {
                            app.message_list.next();
                            etx.send(EmoteData {
                                term_size: terminal.size().unwrap().height,
                                messages: app.message_list.messages.clone(),
                                message_pos: app.message_list.state.selected().unwrap(),
                            })
                            .expect("Error sending EmoteData to etx");
                        }
                        KeyCode::Up => app.message_list.previous(),
                        KeyCode::Left => app.message_list.unselect(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let message: String = app.input.drain(..).collect();
                            mtx.send(message).expect("Error sending message to mtx");
                            // app.message_list.messages.push((message.to_owned(), 1));
                        }
                        KeyCode::Char(c) => {
                            app.input.push(c);
                        }
                        KeyCode::Backspace => {
                            app.input.pop();
                        }
                        KeyCode::Esc => {
                            app.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}