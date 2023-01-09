use std::fs;

use crate::types::{Autocomplete, Emote, ParsedMessage, User, Users};
use serde_json::Result as JSON_Result;
use std::borrow::Cow::{Borrowed, Owned};
use textwrap::Options;
use tui::{
    style::{Color, Modifier, Style},
    text::{Span, Spans},
};

use std::str;

use hyper::body;
use hyper::Client;
use hyper_tls::HttpsConnector;

use viuer::Config;

pub fn get_emotenames() -> Vec<String> {
    let paths = fs::read_dir("./src/emotes_resized/").unwrap();
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

// pub async fn get_emotenames() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
//     let https = HttpsConnector::new();
//     let client = Client::builder().build::<_, hyper::Body>(https);

//     // Parse an `http::Uri`...
//     let uri = "https://cdn.destiny.gg/emotes/emotes.json".parse()?;

//     // Await the response...
//     let resp = client.get(uri).await?;

//     // println!("Response: {}", resp.status());

//     let bytes = body::to_bytes(resp.into_body()).await?;
//     // let json_str: &str = str::from_utf8(&bytes).unwrap();
//     // println!("Hey: {}", json_str);

//     let emote_json: Vec<Emote> = serde_json::from_slice(&bytes)?;

//     let mut names: Vec<String> = Vec::new();

//     for emote in emote_json {
//         names.push(emote.name.to_owned())
//     }
//     Ok(names)
// }

pub fn parse_message(msg: &str) -> JSON_Result<ParsedMessage> {
    let json: ParsedMessage = serde_json::from_str(msg)?;
    return Ok(json);
}

pub fn parse_emote_json(json_data: &str) -> JSON_Result<Emote> {
    let json: Emote = serde_json::from_str(json_data)?;
    return Ok(json);
}

pub async fn get_history() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let uri = "https://www.destiny.gg/api/chat/history".parse()?;

    let resp = client.get(uri).await?;

    let bytes = body::to_bytes(resp.into_body()).await?;

    let history_json: Vec<String> = serde_json::from_slice(&bytes)?;
    Ok(history_json)
}

pub fn parse_users(msg: &str) -> JSON_Result<Users> {
    let json: Users = serde_json::from_str(msg)?;
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
    let lines: Vec<String> = wrap_message(&msg, width - 5 - msg.nick.len() as u16).to_vec();

    let mut message_lines: Vec<Spans> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let mut words: Vec<Span> = Vec::new();

        if i == 0 {
            words.push(
                Span::styled(
                    format!("<{}> ", msg.nick),
                    Style::default()
                        .fg(Color::from_tier(get_tier(msg.features.to_owned())))
                        .add_modifier(Modifier::BOLD),
                )
                .to_owned(),
            )
        }

        for word in line.split(" ") {
            let mut word_style: Style = Style::default();
            if word.contains("http") {
                word_style = word_style.add_modifier(Modifier::UNDERLINED);
                if msg.data.to_lowercase().contains("nsfl") {
                    word_style = word_style.fg(Color::Yellow)
                } else if msg.data.to_lowercase().contains("nsfw") {
                    word_style = word_style.fg(Color::Red)
                }
            }
            words.push(Span::styled(word.to_owned(), word_style));
            words.push(Span::styled(" ", Style::default()))
        }

        message_lines.push(Spans::from(words))
    }

    message_lines
}

pub fn format_user(user: &User) -> Spans<'static> {
    Spans::from(Span::styled(
        format!("{} ", user.nick),
        Style::default()
            .fg(Color::from_tier(get_tier(user.features.to_owned())))
            .add_modifier(Modifier::BOLD),
    ))
}

pub fn wrap_message(msg: &ParsedMessage, width: u16) -> Vec<String> {
    let cloned_msg: String = msg.data.to_owned();

    let wrap_options = Options::new(width as usize)
        .break_words(false)
        .word_splitter(textwrap::WordSplitter::NoHyphenation)
        .word_separator(textwrap::WordSeparator::AsciiSpace);
    let cow_lines = textwrap::wrap(&cloned_msg, wrap_options);

    let lines = cow_lines
        .iter()
        .map(|line| match line {
            Borrowed(text) => text.to_string(),
            Owned(text) => text.to_string(),
        })
        .collect::<Vec<String>>();

    lines
}

pub fn get_users(names: String) -> Users {
    let users_plain: JSON_Result<Users> = parse_users(&names[5..]);
    let users: Users = users_plain.unwrap();
    users
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

pub fn get_suggestions(
    input: String,
    mut autocomplete: Autocomplete,
    users: Users,
    emotes: Vec<String>,
) -> Autocomplete {
    let mut last_word: String = String::new();

    match input.split(' ').last() {
        Some(input_last) => last_word = input_last.to_string(),
        None => last_word = "".to_string(),
    }

    if last_word.len() > 0 {
        let mut names: Vec<String> = Vec::new();
        let mut matching_emotes: Vec<String> = Vec::new();

        for user in users.users.to_owned() {
            if user
                .nick
                .to_lowercase()
                .starts_with(&last_word.to_lowercase())
            {
                names.push(user.nick)
            }
        }

        for emote in emotes.to_owned() {
            if emote.to_lowercase().starts_with(&last_word.to_lowercase()) {
                matching_emotes.push(emote)
            }
        }

        let mut suggestions_vec: Vec<String> = Vec::new();
        suggestions_vec.append(&mut matching_emotes);
        suggestions_vec.append(&mut names);
        autocomplete.suggestions = suggestions_vec;
    } else {
        autocomplete.suggestions = Vec::new();
    }
    autocomplete.last_word = last_word;
    autocomplete
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
