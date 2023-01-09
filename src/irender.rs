use crossterm::cursor::MoveTo;
use crossterm::execute;
use std::io::stdout;

// use serde::{Deserialize, Serialize};
use serde_json::Result as JSON_Result;

use crate::types;
use crate::utils;
use types::{EmoteData, ParsedMessage};

pub fn clear_all() {
    print!("\x1b_Ga=d\x1b\\")
}

pub fn print_emote(x: u16, y: u16) -> Result<(), crossterm::ErrorKind> {
    print!("\x1b_Ga=d,y={}\x1b\\", y + 1);
    execute!(stdout(), MoveTo(x, y + 2))?;
    print!("\x1b_Ga=p,i={}\x1b\\", 1);

    Ok(())
}

pub fn print_one(_path: &str) -> Result<(), crossterm::ErrorKind> {
    execute!(stdout(), MoveTo(10, 10))?;
    print!("\x1b_Ga=p,i={}\x1b\\", 1);
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
                final_emotes.push((
                    i as u16,
                    pos.0 as u16 + msg.nick.len() as u16 + 3,
                    pos.1.to_string(),
                ));
            }
        }
    }

    if final_emotes.len() > 0 {
        for pos in final_emotes.to_owned() {
            utils::print_emote(pos.0, pos.1, pos.2.as_str());
        }
    }

    Ok(())
}
