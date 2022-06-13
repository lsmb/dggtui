use base64::{decode, encode};
use crossterm::cursor::{MoveRight, MoveTo, MoveToPreviousLine};
use crossterm::execute;
use std::io::{stdout, Write};
use std::{fs, io};

use futures::stream::StreamExt;

// use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

use crate::config::Config;
use crate::irender;
use crate::types::{self, Autocomplete};
use crate::ui::ui;
use crate::utils;
use types::{App, EmoteData, InputMode, ParsedMessage};

use tui::{backend::Backend, Terminal};

use crossterm::event::{self, Event, KeyCode, MouseEvent};

use crate::utils::parse_emote_json;

pub fn clear_all() {
    print!("\x1b_Ga=d\x1b\\")
}

pub async fn transmit_all(app: &App) {
    let paths = fs::read_dir("/Users/lsmb/Dev/dggtui/src/emotes_resized/").unwrap();
    let mut names: Vec<String> = vec![];
    for path in paths {
        let pathstr: String = path.unwrap().path().to_str().unwrap().to_string();

        names.push(format!("{}", &pathstr));
    }

    let mut esc_msg: Vec<String> = Vec::new();

    for (i, path) in names.iter().enumerate() {
        // println!("Path {}: {}", i + 1, path);
        esc_msg.push(format!(
            "\x1b_Gq=2,f=100,t=f,i={};{}\x1b\\",
            i + 1,
            encode(path)
        ));
        // io::stdout()::write(dst, src)
        //     .write(format!("\x1b_Gf=100,t=f,i={};{}\x1b\\", i + 1, encode(path)).as_bytes())
        //     .await?;
        // // stdout().write();
    }

    // print!("\x1b_Gf=100,t=f,i={};{}\x1b\\", 1, encode(&names[0]));

    print!("{}", esc_msg.join(""));
    // println!("All transmitted");
}

pub fn print_emote(x: u16, y: u16) -> Result<(), crossterm::ErrorKind> {
    // println!("Hello. {}", path);

    print!("\x1b_Ga=d,y={}\x1b\\", y + 1);
    execute!(stdout(), MoveTo(x, y + 2))?;
    print!("\x1b_Ga=p,i={}\x1b\\", 1);

    // print!(
    //     "\x1b_Gf=100,t=f,a=T;{}\x1b\\\x1b_Gf=100,t=f,X=4,Y=4,a=T;{}\x1b\\",
    //     encode(path),
    //     encode(path)
    // )
    Ok(())
}

pub fn print_one(path: &str) -> Result<(), crossterm::ErrorKind> {
    // println!("Hello. {}", path);

    execute!(stdout(), MoveTo(10, 10))?;
    print!("\x1b_Ga=p,i={}\x1b\\", 1);

    // print!(
    //     "\x1b_Gf=100,t=f,a=T;{}\x1b\\\x1b_Gf=100,t=f,X=4,Y=4,a=T;{}\x1b\\",
    //     encode(path),
    //     encode(path)
    // )
    Ok(())
}

pub fn emote_meme(res: EmoteData) -> Result<(), crossterm::ErrorKind> {
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
            utils::print_emote(pos.0, pos.1, pos.2.as_str());
        }
    }

    Ok(())
}
