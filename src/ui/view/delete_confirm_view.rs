use crate::app::{App, Mode};

use super::View;
use super::home_list::HomeListView;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{Frame, layout::Rect};

pub struct DeleteConfirmView;

impl View for DeleteConfirmView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        HomeListView.draw(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => app.session.mode = Mode::Home,
            KeyCode::Enter => {
                match app.delete_selected() {
                    Ok(()) => app.toast("deleted", true),
                    Err(err) => app.toast(err.to_string(), false),
                }
                app.session.mode = Mode::Home;
            }
            _ => {}
        }
        Ok(())
    }
}
