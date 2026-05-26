mod action_menu;
mod cred_form;
mod cred_list;
mod form;
mod home_list;
mod import;
mod settings;

use crate::app::App;
use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::Rect,
    widgets::Row,
};

/// A View represents a full screen that handles both rendering and key events.
pub trait View {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect);
    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()>;
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
    let visible = area_height.saturating_sub(3) as usize; // 2 borders + 1 header
    let total = rows.len();
    if visible == 0 || total <= visible {
        return rows;
    }
    let scroll = if selected < visible / 2 {
        0
    } else if selected + visible / 2 >= total {
        total.saturating_sub(visible)
    } else {
        selected - visible / 2
    };
    rows.into_iter().skip(scroll).take(visible).collect()
}
