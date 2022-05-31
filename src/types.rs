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
