use crate::{config::Config, utils};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use tui::widgets::{ListItem, ListState};

/// App holds the state of the application
pub struct App<'a> {
    /// Current value of the input box
    pub input: String,
    /// Current input mode
    pub input_mode: InputMode,
    pub tab_titles: Vec<String>,
    pub tab_index: usize,
    /// History of recorded messages
    pub message_list: MessageList<(String, usize)>,
    pub message_spans: MessageList<ListItem<'a>>,
    pub user_list: UserList<User>,
    pub show_suggestion: bool,
    pub users: Users,
    pub emotes: Vec<String>,
    pub autocomplete: Autocomplete,
    pub config: Config,
}

impl<'a> Default for App<'a> {
    fn default() -> App<'a> {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            tab_titles: vec!["Chat".to_string(), "Users".to_string()],
            tab_index: 0,
            message_list: MessageList::with_items(vec![]),
            message_spans: MessageList::with_items(vec![]),
            user_list: UserList::with_items(vec![]),
            show_suggestion: false,
            users: Users::from(Users::default()),
            emotes: { utils::get_emotenames() },
            autocomplete: Autocomplete::from(Autocomplete::default()),
            config: Config::default(),
        }
    }
}

impl<'a> App<'a> {
    // fn on_tick(&mut self) {
    //     // let event = self.events.remove(0);
    //     // self.events.push(event);
    //     self.message_list.messages.push(("yooo".to_string(), 1));
    //     self.input.push('h');
    // }

    pub fn next_tab(&mut self) {
        self.tab_index = (self.tab_index + 1) % self.tab_titles.len();
    }

    pub fn prev_tab(&mut self) {
        if self.tab_index > 0 {
            self.tab_index -= 1;
        } else {
            self.tab_index = self.tab_titles.len() - 1;
        }
    }
}

pub enum InputMode {
    Normal,
    Editing,
}

pub struct MessageList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> MessageList<T> {
    fn with_items(items: Vec<T>) -> MessageList<T> {
        MessageList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.items.len() - 1
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
        self.state.select(Some(self.items.len() - 1));
    }

    pub fn top(&mut self) {
        self.state.select(Some(0));
    }
}

pub struct UserList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> UserList<T> {
    fn with_items(items: Vec<T>) -> UserList<T> {
        UserList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    self.items.len() - 1
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
        self.state.select(Some(self.items.len() - 1));
    }

    pub fn top(&mut self) {
        self.state.select(Some(0));
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
    #[serde(alias = "prefix")]
    pub name: String,
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

#[derive(Debug, Clone)]
pub enum InternalMessageType {
    UPDATE,
    ERROR,
    COMMAND,
    PING,
}

#[derive(Debug, Clone)]
pub struct InternalMessage {
    pub message_type: InternalMessageType,
    pub message: String,
    pub data: Bytes,
}
