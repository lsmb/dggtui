use crate::{config::Config, utils};
use serde::{Deserialize, Serialize};
use tui::widgets::ListState;

/// App holds the state of the application
pub struct App {
    /// Current value of the input box
    pub input: String,
    /// Current input mode
    pub input_mode: InputMode,
    /// History of recorded messages
    pub message_list: StatefulList<(String, usize)>,
    pub show_suggestion: bool,
    pub users: Users,
    pub emotes: Vec<String>,
    pub autocomplete: Autocomplete,
    pub config: Config,
}

impl<'a> Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            message_list: StatefulList::with_items(vec![]),
            show_suggestion: false,
            users: Users::from(Users::default()),
            emotes: { utils::get_emotenames() },
            autocomplete: Autocomplete::from(Autocomplete::default()),
            config: Config::default(),
        }
    }
}

impl<'a> App {
    // fn on_tick(&mut self) {
    //     // let event = self.events.remove(0);
    //     // self.events.push(event);
    //     self.message_list.messages.push(("yooo".to_string(), 1));
    //     self.input.push('h');
    // }
}

pub enum InputMode {
    Normal,
    Editing,
}

pub struct StatefulList<T> {
    pub state: ListState,
    pub messages: Vec<T>,
}

impl<T> StatefulList<T> {
    fn with_items(messages: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            messages,
        }
    }

    pub fn next(&mut self) {
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

    pub fn previous(&mut self) {
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

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn bottom(&mut self) {
        self.state.select(Some(self.messages.len() - 1));
    }
}

#[derive(Debug)]
pub struct EmoteData {
    pub term_size: u16,
    pub messages: Vec<(String, usize)>,
    pub message_pos: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ParsedMessage {
    pub nick: String,
    pub features: Vec<String>,
    pub timestamp: u64,
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HistoryJSON {
    String(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Emote {
    #[serde(alias = "name")]
    pub prefix: String,
    pub twitch: bool,
    pub theme: u16,
    pub image: Vec<EmoteImage>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EmoteImage {
    pub url: String,
    pub name: String,
    pub mime: String,
    pub height: u16,
    pub width: u16,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Autocomplete {
    pub last_word: String,
    pub tabbing: bool,
    pub suggestions: Vec<String>,
    pub selected: Option<usize>,
}

impl Autocomplete {
    pub fn next(&mut self) {
        if let Some(selected) = self.selected {
            if selected + 1 < self.suggestions.len() {
                self.selected = Some(selected + 1)
            } else {
                self.selected = Some(0)
            }
        } else {
            if self.suggestions.len() > 0 {
                self.selected = Some(0)
            } else {
                self.selected = None
            }
        }
    }

    pub fn previous(&mut self) {
        if let Some(selected) = self.selected {
            if selected > 0 {
                self.selected = Some(selected - 1)
            } else {
                self.selected = Some(self.suggestions.len() - 1)
            }
        }
    }

    pub fn unselect(&mut self) {
        self.selected = None;
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct User {
    pub nick: String,
    pub features: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Users {
    pub connectioncount: u16,
    pub users: Vec<User>,
}
