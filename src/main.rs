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
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};
// use termion::{clear, color, cursor, cursor::DetectCursorPos, raw::IntoRawMode, style};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use unicode_width::UnicodeWidthStr;

use futures::sink::SinkExt;
use futures::stream::StreamExt;
use websocket_lite::{Message, Opcode, Result};

use tokio::sync::mpsc;

use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

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
    items: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
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
                    self.items.len() - 1
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
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,

    items: StatefulList<(String, usize)>,
}

impl<'a> Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            items: StatefulList::with_items(vec![]),
        }
    }
}

impl<'a> App {
    fn on_tick(&mut self) {
        // let event = self.events.remove(0);
        // self.events.push(event);
        self.items.items.push(("yooo".to_string(), 1));
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

    // let (tx, mut rx) = watch::channel("hello");
    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move {
        run_ws(tx).await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    });

    let tick_rate = Duration::from_millis(250);
    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app, rx, tick_rate);

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

async fn run_ws(tx: tokio::sync::mpsc::Sender<String>) -> Result<()> {
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
                tx.send(msg).await;
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

fn parse_message(msg: &str) -> JSON_Result<ParsedMessage> {
    let json: ParsedMessage = serde_json::from_str(msg)?;
    return Ok(json);
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut rx: tokio::sync::mpsc::Receiver<String>,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }

        match rx.try_recv() {
            Ok(msg) => {
                if msg.starts_with("MSG ") {
                    // let parsed_output: JSON_Result<ParsedMessage>;
                    // parsed_output = parse_message(&msg[4..]);
                    app.items.items.push((msg.to_owned(), 1));
                }
            }
            _ => (),
        }
        terminal.draw(|f| ui(f, &mut app))?;

        if crossterm::event::poll(tick_rate).unwrap() {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('e') => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Down => app.items.next(),
                        KeyCode::Up => app.items.previous(),
                        KeyCode::Left => app.items.unselect(),
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Enter => {
                            let message: String = app.input.drain(..).collect();
                            app.items.items.push((message.to_owned(), 1));
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
            0 => return Color::White,
            1 => return Color::Blue,
            2 => return Color::Green,
            3 => return Color::Cyan,
            4 => return Color::Magenta,
            6 => return Color::Yellow,
            _ => return Color::White,
        }
    }
}

fn format_message(msg: ParsedMessage) -> Spans<'static> {
    Spans::from(vec![
        Span::styled(
            format!("<{}> ", msg.nick),
            Style::default()
                .fg(Color::from_tier(get_tier(msg.features)))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(msg.data, Style::default().fg(Color::White)),
    ])
}
fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(1),
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
    f.render_widget(help_message, chunks[0]);

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

    // let messages: Vec<ListItem> = app
    //     .messages
    //     .iter()
    //     .enumerate()
    //     .map(|(i, m)| {
    //         let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
    //         ListItem::new(content)
    //     })
    //     .collect();
    // let messages =
    //     List::new(messages).block(Block::default().borders(Borders::ALL).title("Messages"));

    let items: Vec<ListItem> = app
        .items
        .items
        .iter()
        .map(|i| {
            let mut lines = vec![Spans::from(i.0.as_str())];
            // for _ in 0..i.1 {
            //     lines.push(Spans::from(Span::styled(
            //         "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            //         Style::default().add_modifier(Modifier::ITALIC),
            //     )));
            // }

            let parsed_output: JSON_Result<ParsedMessage>;
            parsed_output = parse_message(&i.0.as_str()[4..]);
            let formattedMessage: Spans = format_message(parsed_output.unwrap());
            lines.push(formattedMessage);
            ListItem::new(lines).style(Style::default().fg(Color::White).bg(Color::Black))
        })
        .collect();

    // Create a List from all list items and highlight the currently selected one
    let items = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // f.render_widget(messages, chunks[2]);
    f.render_stateful_widget(items, chunks[2], &mut app.items.state);
}
