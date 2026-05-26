use crate::app::{App, Mode};

use super::View;
use super::home_list::HomeListView;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Frame, layout::Rect};

pub struct SearchView;

impl View for SearchView {
    fn draw(&self, frame: &mut Frame<'_>, app: &App, area: Rect) {
        HomeListView.draw(frame, app, area);
    }

    fn handle_key(&self, app: &mut App, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.session.mode = Mode::Home;
            }
            KeyCode::Char('j') => app.move_selection(1),
            KeyCode::Char('k') => app.move_selection(-1),
            KeyCode::Down => app.move_selection(1),
            KeyCode::Up => app.move_selection(-1),
            KeyCode::Backspace => {
                app.session.home.search.pop();
                app.session.home.selected = 0;
            }
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                app.session.home.search.push(c);
                app.session.home.selected = 0;
            }
            _ => {}
        }
        Ok(())
    }
}
