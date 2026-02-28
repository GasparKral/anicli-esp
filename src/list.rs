use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::config::CONFIG;

// Por ahora lo dejo hardcodeado, no se si agregarlo a la configuración del usuario o simplemente
// dejarlo con un valor fijo

const SEARCH_TIMEOUT: Duration = Duration::from_millis(300); // 300 milisegundos suele ser un
                                                             // estandar

#[derive(Default, PartialEq, Eq)]
enum ListMode {
    #[default]
    Navigating,
    Searching,
}

#[derive(Default)]
pub struct OptionsList<'list_contents> {
    mode: ListMode,
    contents: Vec<ListItem<'list_contents>>,
    contents_texts: Vec<&'list_contents str>,
    list_state: ListState,
    search_query: Vec<String>,
    focus: bool,
    filter_last_input: Option<Instant>,
}

impl<'list_contents> OptionsList<'list_contents> {
    pub fn focus(&mut self) {
        self.focus = true;
    }

    pub fn defocus(&mut self) {
        self.focus = false;
    }

    fn toggle_mode(&mut self) {
        if self.mode == ListMode::Navigating {
            self.mode = ListMode::Searching;
            self.search_query.clear();
        } else {
            self.mode = ListMode::Navigating;
            self.search_query.clear();
        }
    }

    pub fn tick(&mut self) {
        if let Some(last_input) = self.filter_last_input {
            if last_input.elapsed() >= SEARCH_TIMEOUT {
                self.reset_filter();
            }
        }
    }

    fn reset_filter(&mut self) {
        self.search_query.clear();
        self.filter_last_input = None;
        self.mode = ListMode::Navigating;
    }

    pub fn set_contents(&mut self, contents: Vec<String>) {
        let contents = contents
            .into_iter()
            .map(|mut tittle| {
                if CONFIG.read().unwrap().get_liked_animes().contains(&tittle) {
                    tittle.push_str(" ★");
                }

                // Luego borro los viejos
                &*Box::leak(tittle.into_boxed_str())
            })
            .collect();

        // Borro los viejos
        for tittle in self.contents_texts.iter() {
            unsafe {
                let boxed_str: Box<str> = Box::from_raw(*tittle as *const str as *mut str);
                drop(boxed_str);
            }
        }

        self.contents_texts = contents;
        self.contents = self
            .contents_texts
            .iter()
            .map(|tittle| ListItem::new(*tittle))
            .collect();
        if !self.contents.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.mode {
            ListMode::Navigating => match key_event.code {
                KeyCode::Char('g' /*Go to*/) => self.toggle_mode(),
                KeyCode::Up => self.up(),
                KeyCode::Down => self.down(),
                _ => (),
            },
            ListMode::Searching => match key_event.code {
                KeyCode::Esc => self.toggle_mode(),
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.search();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c.to_string());
                    self.search();
                }
                _ => (),
            },
        }
    }

    fn search(&mut self) {
        let mut matches = self
            .get_contents()
            .iter()
            .enumerate()
            .filter(|(_, v)| v.contains(&self.search_query.join("")))
            .map(|(i, _)| i)
            .collect::<Vec<usize>>();

        if let Some(current) = self.current() {
            //que el numero introducido sea valido
            if matches.len() > 0 {
                matches.sort_by(|a, b| b.cmp(a));
                // moverse al primer match que coincida o al siguiente si ya estamos en esa
                // posición
                if let Some(pos_in_matches) = matches.iter().position(|i| *i == current) {
                    self.select(Some(matches[(pos_in_matches + 1) % matches.len()]))
                } else {
                    self.select(Some(matches[0]));
                }
            }
        }
    }

    fn up(&mut self) {
        self.list_state.select_previous();
    }

    fn down(&mut self) {
        self.list_state.select_next();
    }

    pub fn current(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn select(&mut self, idx: Option<usize>) {
        self.list_state.select(idx);
    }

    pub fn current_value(&self) -> Option<String> {
        if let Some(idx) = self.list_state.selected() {
            let contents = self.contents_texts[idx]
                .strip_suffix(" ★")
                .unwrap_or(&self.contents_texts[idx]);
            // La referencia puede ser dropeada antes
            return Some(contents.to_owned());
        }
        None
    }

    pub fn get_contents(&self) -> Vec<String> {
        self.contents_texts
            .iter()
            .map(|entry| entry.strip_suffix(" ★").unwrap_or(entry).to_owned())
            .collect()
    }

    fn build_highlighted_item<'a>(text: &'a str, query: &str) -> ListItem<'a> {
        if query.is_empty() {
            return ListItem::new(text);
        }

        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();

        let Some(match_start) = lower_text.find(&lower_query) else {
            return ListItem::new(text);
        };

        let match_end = match_start + lower_query.len();

        let before = &text[..match_start];
        let matched = &text[match_start..match_end];
        let after = &text[match_end..];

        let spans = vec![
            Span::raw(before),
            Span::styled(
                matched,
                Style::new()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(after),
        ];

        ListItem::new(Line::from(spans))
    }
    fn get_query(&self) -> String {
        self.search_query.join("")
    }
}

impl Widget for &mut OptionsList<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mode_color = if self.mode == ListMode::Navigating {
            Color::Yellow
        } else {
            Color::Green
        };

        let items: Vec<ListItem<'_>> = self
            .contents_texts
            .iter()
            .map(|v| OptionsList::build_highlighted_item(v, &self.get_query()))
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_style(match self.mode {
                ListMode::Navigating => Style::new().fg(mode_color).add_modifier(Modifier::BOLD),
                ListMode::Searching => Style::new().bg(mode_color).add_modifier(Modifier::BOLD),
            })
            .block(
                Block::new()
                    .borders(Borders::RIGHT)
                    .border_style(Style::new().fg(match self.focus {
                        true => mode_color,
                        false => Color::White,
                    })),
            );

        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}
