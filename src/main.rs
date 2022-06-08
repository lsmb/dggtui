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
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, time::Duration};

use tui::{backend::CrosstermBackend, Terminal};

use serde_json::Result as JSON_Result;

use websocket_lite::Result;

use tokio::sync::watch;
// use tokio::{fs::write, sync::mpsc};

mod threads;
mod types;
mod ui;
mod utils;
use types::EmoteData;

#[tokio::main]
async fn main() -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, rx) = watch::channel("".to_string());
    let (etx, erx) = watch::channel(EmoteData {
        term_size: 0,
        messages: vec![],
        message_pos: 0,
    });
    let (mtx, mrx) = watch::channel("".to_string());

    tokio::spawn(async move {
        threads::run_ws2(tx, mrx).await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    });

    // tokio::spawn(async move {
    //     threads::run_ws_sender(mrx).await.unwrap_or_else(|e| {
    //         eprintln!("{}", e);
    //     })
    // });

    // Emote thread
    tokio::spawn(async move {
        threads::run_emotes(erx).await.unwrap_or_else(|e| {
            eprintln!("{}", e);
        })
    });

    let tick_rate = Duration::from_millis(0);
    // create app and run it
    let app = types::App::default();
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

// async fn run_ws_sender(mrx: tokio::sync::watch::Receiver<String>) -> Result<()> {
//     let url = "wss://chat.destiny.gg/ws".to_owned();
//     let mut builder = websocket_lite::ClientBuilder::new(&url)?;

//     builder.add_header(
//         "Cookie".to_string(),
//         "authtoken=7uooLJ8yxtTmCBnjaloirWpXXpbRNgOWJ0ZJLsyvjX8xoTavppOf7OdL1hbCtfVm".to_string(),
//     );

//     let mut ws_stream = builder.async_connect().await?;

//     let mut message: String = String::new();
//     let mut changed: bool = false;

//     // loop {
//     // if mrx.has_changed().unwrap() {
//     //     let msg = &*mrx.borrow_and_update();

//     //     if msg.to_string() == "quit".to_string() {
//     //         break;
//     //     }

//     //     message = msg.to_string();
//     //     // message_to_send = msg.to_string();
//     //     // changed = true;
//     // }

//     // if changed == true {
//     ws_stream.send(Message::text("test".to_string())).await?;
//     //     changed = false;
//     // }
//     // }

//     // if changed == true {
//     //     println!("{}", message_to_send.to_owned());
//     //     match ws_stream.send(Message::new(Opcode::Text, "hello")?).await {
//     //         Ok(sk) => {
//     //             // ... use sk ...
//     //         }
//     //         Err(e) => {
//     //             println!("Error is: {}", e)
//     //         }
//     //     }
//     //     changed = false;
//     //     ws_stream.next().await;
//     // }

//     Ok(())
// }
