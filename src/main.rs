/// A simple example demonstrating how to handle user input. This is
/// a bit out of the scope of the library as it does not provide any
/// input handling out of the box. However, it may helps some to get
/// started.
///
/// This is a very simple example:
///   * A input box always focused. Every character you type is registered
///   here
///   * Pressing Backspace erases a character
///   * Pressing Enter pushes the current input in the history of previous
///   messages
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEvent},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use std::io::Write;
// use termion::{clear, color, cursor, cursor::DetectCursorPos, raw::IntoRawMode, style};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use image as imageutil;
use image::GenericImageView;
use imageutil::DynamicImage;
use std::fs;
use std::path::{Path, PathBuf};
use viuer::Config;

use unicode_width::UnicodeWidthStr;

use futures::stream::StreamExt;
use futures::{sink::SinkExt, TryFutureExt};
use websocket_lite::{Message, Opcode, Result};

use tokio::sync::watch;
use tokio::{fs::write, sync::mpsc};

use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

use rand::Rng;

#[derive(Serialize, Deserialize, Debug)]
struct ParsedMessage {
    nick: String,
    features: Vec<String>,
    timestamp: u64,
    data: String,
}

enum InputMode {
    Normal,
    Editing,
}

struct StatefulList<T> {
    state: ListState,
    messages: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(messages: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            messages,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    self.messages.len() - 1
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn unselect(&mut self) {
        self.state.select(None);
    }

    fn bottom(&mut self) {
        self.state.select(Some(self.messages.len() - 1));
    }
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    message_list: StatefulList<(String, usize)>,
}

impl<'a> Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            message_list: StatefulList::with_items(vec![]),
        }
    }
}

impl<'a> App {
    fn on_tick(&mut self) {
        // let event = self.events.remove(0);
        // self.events.push(event);
        self.message_list.messages.push(("yooo".to_string(), 1));
        self.input.push('h');
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (mut tx, rx) = watch::channel("".to_string());
    let (etx, mut erx) = watch::channel(EmoteData {
        term_size: 0,
        messages: vec![],
        message_pos: 0,
    });

    tokio::spawn(async move {
        run_ws(tx).await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    });

    tokio::spawn(async move {
        run_emotes(erx).await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    });

    let tick_rate = Duration::from_millis(50);
    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app, rx, etx, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn get_emotenames() -> Vec<String> {
    let paths = fs::read_dir("./src/emotes/").unwrap();
    let mut names: Vec<String> = vec![];
    for path in paths {
        let pathstr: String = path
            .unwrap()
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        names.push(format!("{}", &pathstr[0..pathstr.len() - 4],));
    }

    names
}

fn print_emote(voffset: u16, xoffset: u16, emote_name: &str) {
    let conf = Config {
        width: Some(0),
        height: Some(0),
        absolute_offset: true,
        ..Default::default()
    };

    viuer::print_from_file(
        format!("./src/emotes_resized/{}.png", emote_name),
        &Config {
            y: 2 + voffset as i16,
            x: xoffset + 2,
            ..conf
        },
    )
    .expect("Imge printing failed.");
}

async fn run_emotes(mut erx: tokio::sync::watch::Receiver<EmoteData>) -> Result<()> {
    // for i in 0..messages.len() {
    while erx.changed().await.is_ok() {
        let res = &*erx.borrow();
        // println!("Hei: {}", res.term_size);

        // println!("Pos: {}", res.message_pos);

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
            let emote_names: Vec<String> = get_emotenames();
            let mut emote_pos: Vec<(usize, &str)> = vec![];

            let parsed_output: JSON_Result<ParsedMessage>;
            parsed_output = parse_message(&message.0.as_str()[4..]);
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
                    print_emote(i as u16, pos.0 as u16 + msg.nick.len() as u16 + 3, pos.1)
                }
            }
        }
    }

    Ok(())
}

async fn run_ws(tx: tokio::sync::watch::Sender<String>) -> Result<()> {
    let url = "wss://chat.destiny.gg/ws".to_owned();
    let builder = websocket_lite::ClientBuilder::new(&url)?;

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

struct EmoteData {
    term_size: u16,
    messages: Vec<(String, usize)>,
    message_pos: usize,
}

fn parse_message(msg: &str) -> JSON_Result<ParsedMessage> {
    let json: ParsedMessage = serde_json::from_str(msg)?;
    return Ok(json);
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut rx: tokio::sync::watch::Receiver<String>,
    etx: tokio::sync::watch::Sender<EmoteData>,
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
                // let parsed_output: JSON_Result<ParsedMessage>;
                // parsed_output = parse_message(&msg[4..]);
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
                });

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
                            });
                        }
                        KeyCode::Down => {
                            app.message_list.next();
                            etx.send(EmoteData {
                                term_size: terminal.size().unwrap().height,
                                messages: app.message_list.messages.clone(),
                                message_pos: app.message_list.state.selected().unwrap(),
                            });
                        }
                        KeyCode::Up => app.message_list.previous(),
                        KeyCode::Left => app.message_list.unselect(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let message: String = app.input.drain(..).collect();
                            app.message_list.messages.push((message.to_owned(), 1));
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

fn get_tier(features: Vec<String>) -> i8 {
    let mut tiers: Vec<u8> = vec![0];

    for feature in features {
        let temp_f: &str = &feature;
        match temp_f {
            "subscriber" => tiers.push(1),
            "flair3" => tiers.push(2),
            "flair1" => tiers.push(3),
            "flair8" => tiers.push(4),
            "admin" => tiers.push(6),
            _ => (),
        }
    }
    *tiers.iter().max().unwrap() as i8
}

trait FromTier {
    fn from_tier(tier: i8) -> Color;
}

impl FromTier for Color {
    fn from_tier(tier: i8) -> Color {
        match tier {
            0 => Color::White,
            1 => Color::Blue,
            2 => Color::Green,
            3 => Color::Cyan,
            4 => Color::Magenta,
            6 => Color::Yellow,
            _ => Color::White,
        }
    }
}

fn format_message(msg: ParsedMessage, width: u16) -> Vec<Spans<'static>> {
    let message: Vec<Spans> = vec![
        Spans::from(vec![
            Span::styled(
                format!("<{}> ", msg.nick),
                Style::default()
                    .fg(Color::from_tier(get_tier(msg.features)))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(msg.data, Style::default().fg(Color::White)),
        ]),
        /* Spans::from("") */
    ];

    // println!("Message width: {}", message.width());

    message
}
fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = match app.input_mode {
        InputMode::Normal => (
            vec![
                Span::raw("Press "),
                Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to exit, "),
                Span::styled("e", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to start editing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop editing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to record the message"),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[2]);

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                chunks[1].x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                chunks[1].y + 1,
            )
        }
    }

    let messages: Vec<ListItem> = app
        .message_list
        .messages
        .iter()
        .map(|i| {
            let mut lines = vec![/* Spans::from(i.0.as_str()) */];
            // for _ in 0..i.1 {
            //     lines.push(Spans::from(Span::styled(
            //         "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            //         Style::default().add_modifier(Modifier::ITALIC),
            //     )));
            // }

            let parsed_output: JSON_Result<ParsedMessage>;
            parsed_output = parse_message(&i.0.as_str()[4..]);
            let formatted_message: Vec<Spans> =
                format_message(parsed_output.unwrap(), chunks[0].width);
            for line in formatted_message {
                lines.push(line)
            }

            ListItem::new(lines).style(Style::default().fg(Color::White).bg(Color::Black))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one

    let messages = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .highlight_style(match app.input_mode {
            InputMode::Normal => Style::default(),
            // Style::default()
            // .bg(Color::LightGreen)
            // .add_modifier(Modifier::BOLD),
            InputMode::Editing => Style::default(),
        })
        .highlight_symbol(match app.input_mode {
            InputMode::Normal => "",
            InputMode::Editing => "",
        });

    // println!("Size is: {}", f.size().width);

    // println!("{}", chunks[0].y);
    // f.render_widget(messages, chunks[2]);

    f.render_stateful_widget(messages, chunks[0], &mut app.message_list.state);
}
