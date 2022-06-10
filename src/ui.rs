use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use unicode_width::UnicodeWidthStr;

use crate::types::{App, InputMode, ParsedMessage};
use crate::utils::{format_message, parse_message};
use crate::JSON_Result;

pub fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Min(1),
                Constraint::Length(3),
                Constraint::Length(1),
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
                Span::raw(" to start typing."),
            ],
            Style::default().add_modifier(Modifier::RAPID_BLINK),
        ),
        InputMode::Editing => (
            vec![
                Span::raw("Press "),
                Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to stop typing, "),
                Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" to send the message"),
            ],
            Style::default(),
        ),
    };
    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text);

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL));
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

    let messages: Vec<ListItem> = app
        .message_list
        .messages
        .iter()
        .map(|i| {
            let mut lines = vec![];

            let parsed_output: JSON_Result<ParsedMessage>;
            parsed_output = parse_message(&i.0.as_str()[4..]);
            let parsed_message = parsed_output.unwrap();

            let mut list_style: Style = Style::default();

            if parsed_message.nick == "Keah" {
                list_style = list_style.bg(Color::Rgb(25, 25, 25))
            }

            if parsed_message.data.to_lowercase().contains("keah") && parsed_message.nick != "Keah"
            {
                list_style = list_style.fg(Color::Blue).add_modifier(Modifier::BOLD)
            }
            if parsed_message.data.starts_with(">") {
                list_style = list_style.fg(Color::LightGreen)
            }

            let formatted_message: Vec<Spans> = format_message(parsed_message, chunks[0].width);
            for line in formatted_message {
                lines.push(line)
            }

            ListItem::new(lines).style(list_style)
        })
        .collect();

    // Create a List from all messages and manage highlighting based on state
    let messages = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .highlight_style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default(),
        })
        .highlight_symbol(match app.input_mode {
            InputMode::Normal => "",
            InputMode::Editing => "",
        });

    f.render_stateful_widget(messages, chunks[0], &mut app.message_list.state);
    f.render_widget(help_message, chunks[2]);
    f.render_widget(input, chunks[1]);

    if app.input.len() > 0 {
        let area = suggestion_rect(f.size());
        let block = Block::default()/* .borders(Borders::ALL) */;

        if app.autocomplete.suggestions.len() > 0 {
            let suggestions = Paragraph::new(app.autocomplete.suggestions.join(" ").to_string())
                .style(Style::default().fg(Color::Blue))
                .block(block);

            f.render_widget(Clear, area); //this clears out the background
            f.render_widget(suggestions, area);
        }
    }
}

fn get_suggestions() -> Option<String> {
    None
}

fn suggestion_rect(r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(r.height - 6),
                Constraint::Length(1),
                Constraint::Length(20),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Length(2),
                Constraint::Percentage(80),
                Constraint::Percentage(60),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
