use std::fs;

use crate::types::ParsedMessage;
use serde_json::Result as JSON_Result;
use std::borrow::Cow::{Borrowed, Owned};
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
};

use viuer::Config;

pub fn get_emotenames() -> Vec<String> {
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

pub fn parse_message(msg: &str) -> JSON_Result<ParsedMessage> {
    let json: ParsedMessage = serde_json::from_str(msg)?;
    return Ok(json);
}

pub fn print_emote(voffset: u16, xoffset: u16, emote_name: &str) {
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

pub fn format_message(msg: ParsedMessage, width: u16) -> Vec<Spans<'static>> {
    let lines: Vec<String> = wrap_message(&msg, width - 4 - msg.nick.len() as u16).to_vec();

    let mut message_lines: Vec<Spans> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            message_lines.push(Spans::from(vec![
                Span::styled(
                    format!("<{}> ", msg.nick),
                    Style::default()
                        .fg(Color::from_tier(get_tier(msg.features.to_owned())))
                        .add_modifier(Modifier::BOLD),
                )
                .to_owned(),
                Span::styled(line.to_owned(), Style::default().fg(Color::White)),
            ]))
        } else {
            message_lines.push(Spans::from(vec![Span::styled(
                line.to_owned(),
                Style::default().fg(Color::White),
            )]))
        }
    }

    message_lines
}

pub fn wrap_message(msg: &ParsedMessage, width: u16) -> Vec<String> {
    let cloned_msg: String = msg.data.to_owned();
    let cow_lines = textwrap::wrap(&cloned_msg, width as usize);

    let lines = cow_lines
        .iter()
        .map(|line| match line {
            Borrowed(text) => text.to_string(),
            Owned(text) => text.to_string(),
        })
        .collect::<Vec<String>>();

    lines
}

pub fn get_tier(features: Vec<String>) -> i8 {
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
