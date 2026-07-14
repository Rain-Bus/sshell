mod action_menu;
mod cred_form;
mod cred_list;
mod form;
mod home_list;
mod import;
mod settings;

use crate::app::{App, FormAction, FormNav};
use crate::ui::BLUE;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Cell, Row},
};

/// A View represents a full screen that handles both rendering and key events.
pub trait View {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect);
    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()>;
    fn hints(&self) -> Vec<(&'static str, &'static str)> {
        vec![]
    }
    fn title(&self) -> &'static str {
        ""
    }
}

pub use action_menu::ActionMenuView;
pub use cred_form::CredFormView;
pub use cred_list::CredListView;
pub use form::FormView;
pub use home_list::{DeleteConfirmView, HomeListView, QuickSelectView, SearchView};
pub use import::ImportView;
pub use settings::SettingsView;

/// Scroll a 1:1 row list so the selected index stays visible.
/// `area_height` includes the block borders and table header.
pub fn scroll_rows<'a>(rows: Vec<Row<'a>>, selected: usize, area_height: u16) -> Vec<Row<'a>> {
    scroll_indexed_rows(rows, &[selected], 0, area_height)
}

/// Scroll a row list that mixes data rows with non-data rows (e.g. section headers).
/// `entry_row` maps each data index to its visual row index in `rows`.
/// `selected` is the currently selected data index.
pub fn scroll_indexed_rows<'a>(
    rows: Vec<Row<'a>>,
    entry_row: &[usize],
    selected: usize,
    area_height: u16,
) -> Vec<Row<'a>> {
    let visible = area_height.saturating_sub(3) as usize; // 2 borders + 1 header
    let total = rows.len();
    if visible == 0 || total <= visible || entry_row.is_empty() {
        return rows;
    }
    let sel_row = entry_row[selected.min(entry_row.len() - 1)];
    let scroll = if sel_row < visible / 2 {
        0
    } else if sel_row + visible / 2 >= total {
        total.saturating_sub(visible)
    } else {
        sel_row - visible / 2
    };
    rows.into_iter().skip(scroll).take(visible).collect()
}

/// Common form key handler. Returns Some(action) for Toggle/Save/Cancel,
/// None for text-editing and navigation keys that are fully handled here.
pub fn handle_form_nav<F: FormNav>(form: &mut F, key: KeyEvent) -> Option<FormAction> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Down => {
            form.nav_next();
            None
        }
        KeyCode::Up => {
            form.nav_prev();
            None
        }
        KeyCode::Tab if form.active_is_toggle() => Some(FormAction::Toggle),
        KeyCode::Enter => Some(FormAction::Save),
        KeyCode::Esc => Some(FormAction::Cancel),

        KeyCode::Backspace if form.active_is_text() => {
            form.delete_char();
            None
        }
        KeyCode::Delete if form.active_is_text() => {
            form.delete_next_char();
            None
        }
        KeyCode::Left if form.active_is_text() => {
            form.move_cursor_left();
            None
        }
        KeyCode::Right if form.active_is_text() => {
            form.move_cursor_right();
            None
        }
        KeyCode::Home if form.active_is_text() => {
            form.cursor_home();
            None
        }
        KeyCode::End if form.active_is_text() => {
            form.cursor_end();
            None
        }
        KeyCode::Char('a') if ctrl && form.active_is_text() => {
            form.cursor_home();
            None
        }
        KeyCode::Char('e') if ctrl && form.active_is_text() => {
            form.cursor_end();
            None
        }
        KeyCode::Char('u') if ctrl && form.active_is_text() => {
            form.clear_field();
            None
        }
        KeyCode::Char(' ') if form.active_is_text() => {
            form.insert_char(' ');
            None
        }
        KeyCode::Char(c) if !ctrl && form.active_is_text() => {
            form.insert_char(c);
            None
        }
        _ => None,
    }
}

/// A section header row with a label and count, used in table views.
pub fn section_row(label: &str, count: usize) -> Row<'static> {
    Row::new([
        Cell::from(format!("  {label} ({count})"))
            .style(Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
    ])
    .height(1)
}
