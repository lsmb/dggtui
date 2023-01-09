use bytes::Bytes;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};

use tui::{backend::CrosstermBackend, Terminal};

use serde_json::Result as JSON_Result;

use websocket_lite::Result;

use tokio::sync::watch;

mod config;
mod irender;
mod threads;
mod types;
mod ui;
mod utils;
use crate::config::Config;
use types::{EmoteData, InternalMessage, InternalMessageType};

#[tokio::main]
async fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let conf = Config::init().await;
    let mut app = types::App::default();

    if conf.is_ok() {
        app.config = conf.to_owned().unwrap();
    }

    let (tx, rx) = watch::channel("".to_string());
    let (etx, erx) = watch::channel(EmoteData {
        term_size: 0,
        messages: vec![],
        message_pos: 0,
    });
    let (mtx, mrx) = watch::channel("".to_string());
    let (itx, irx) = watch::channel(InternalMessage {
        message_type: InternalMessageType::COMMAND,
        message: "Initialize".to_string(),
        data: Bytes::new(),
    });

    tokio::spawn(async move {
        threads::run_ws2(tx, mrx, itx, irx, conf.to_owned().unwrap())
            .await
            .unwrap_or_else(|e| {
                eprintln!("{}", e);
            })
    });

    // Emote thread
    if app.config.emotes {
        tokio::spawn(async move {
            threads::run_emotes(erx).await.unwrap_or_else(|e| {
                eprintln!("{}", e);
            })
        });
    }

    for msg in utils::get_history().await? {
        app.message_list.items.push((msg.to_owned(), 1));
    }

    let tick_rate = Duration::from_millis(5);

    // create app and run it
    let res = threads::run_app(&mut terminal, app, rx, etx, mtx, tick_rate);

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
